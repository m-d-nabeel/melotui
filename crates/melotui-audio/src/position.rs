//! Where in the file the audio taken from it has got to.
//!
//! The playback clock runs by the wall: started when a track is queued, it
//! says how long that has been. That is smooth, and close, but it is not the
//! audio — a device that stalls, a machine that was busy, and the clock has
//! run on while the sound did not, for the rest of the track. Counting the
//! frames the device has actually taken from the decoder gives where the
//! audio is, in steps of whatever the device takes at once; the clock is
//! held to that (see `TimeKeeper::hold_to`), and keeps its smoothness.

use rodio::{Source, source::SeekError};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

/// How many frames are counted before the count is published: often enough
/// to be current, without an atomic store for every sample.
const PUBLISH_EVERY: u32 = 64;

/// The position of a [`Counted`] source, read from another thread.
#[derive(Clone, Debug, Default)]
pub struct Position(Arc<AtomicU64>);

impl Position {
    /// How far into the file the audio taken so far reaches.
    pub fn get(&self) -> Duration {
        Duration::from_micros(self.0.load(Ordering::Relaxed))
    }

    fn set(&self, position: Duration) {
        self.0.store(position.as_micros() as u64, Ordering::Relaxed);
    }
}

/// Counts the frames taken from a source — the decoder, before the effects
/// and before the speed control, so that a frame is a frame of the file.
pub struct Counted<S> {
    source: S,
    position: Position,
    /// Where counting started from — the last seek, or the last change of
    /// sample rate — in microseconds, and the frames counted since, at
    /// `rate`. Whole numbers throughout: a running sum of fractions is a
    /// microsecond out before the first second is over.
    base_micros: u64,
    frames: u64,
    rate: u32,
    /// Frames counted since the position was last published.
    pending: u32,
    /// Position within the current frame.
    channel: u16,
}

impl<S: Source<Item = f32>> Counted<S> {
    pub fn new(source: S) -> (Self, Position) {
        let position = Position::default();
        let counted = Self {
            rate: source.sample_rate().get(),
            source,
            position: position.clone(),
            base_micros: 0,
            frames: 0,
            pending: 0,
            channel: 0,
        };
        (counted, position)
    }

    fn publish(&mut self) {
        self.frames += self.pending as u64;
        self.pending = 0;
        let micros = self.base_micros + self.frames * 1_000_000 / self.rate as u64;
        self.position.0.store(micros, Ordering::Relaxed);
        // A stream may change rate part way: what was counted at the old
        // one is kept as it stands, and counting goes on at the new.
        let rate = self.source.sample_rate().get();
        if rate != self.rate {
            self.base_micros = micros;
            self.frames = 0;
            self.rate = rate;
        }
    }
}

impl<S: Source<Item = f32>> Iterator for Counted<S> {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.source.next();
        if sample.is_none() {
            // The end: what is left uncounted is counted.
            if self.pending > 0 {
                self.publish();
            }
            return None;
        }
        self.channel += 1;
        if self.channel >= self.source.channels().get() {
            self.channel = 0;
            self.pending += 1;
            if self.pending >= PUBLISH_EVERY {
                self.publish();
            }
        }
        sample
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.source.size_hint()
    }
}

impl<S: Source<Item = f32>> Source for Counted<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.source.current_span_len()
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.source.channels()
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.source.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.source.try_seek(pos)?;
        self.base_micros = pos.as_micros() as u64;
        self.frames = 0;
        self.rate = self.source.sample_rate().get();
        self.pending = 0;
        self.channel = 0;
        self.position.set(pos);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Silent stereo at 48 kHz that can seek.
    struct Stereo {
        left: usize,
    }

    impl Iterator for Stereo {
        type Item = f32;
        fn next(&mut self) -> Option<f32> {
            self.left = self.left.checked_sub(1)?;
            Some(0.0)
        }
    }

    impl Source for Stereo {
        fn current_span_len(&self) -> Option<usize> {
            None
        }
        fn channels(&self) -> rodio::ChannelCount {
            rodio::nz!(2)
        }
        fn sample_rate(&self) -> rodio::SampleRate {
            rodio::nz!(48_000)
        }
        fn total_duration(&self) -> Option<Duration> {
            None
        }
        fn try_seek(&mut self, _: Duration) -> Result<(), SeekError> {
            Ok(())
        }
    }

    #[test]
    fn the_position_is_the_frames_taken_not_the_samples() {
        let (mut counted, position) = Counted::new(Stereo { left: 1_000_000 });
        assert_eq!(position.get(), Duration::ZERO);
        // One second of stereo is 96,000 samples.
        for _ in 0..96_000 {
            counted.next();
        }
        assert_eq!(position.get(), Duration::from_secs(1));
    }

    #[test]
    fn a_seek_moves_the_position_and_counting_goes_on_from_there() {
        let (mut counted, position) = Counted::new(Stereo { left: 1_000_000 });
        for _ in 0..9_600 {
            counted.next();
        }
        counted.try_seek(Duration::from_secs(90)).unwrap();
        assert_eq!(position.get(), Duration::from_secs(90));
        for _ in 0..48_000 {
            counted.next();
        }
        assert_eq!(position.get(), Duration::from_millis(90_500));
    }

    #[test]
    fn the_last_frames_of_a_track_are_counted_too() {
        // 100 frames: one lot published, 36 left over at the end.
        let (counted, position) = Counted::new(Stereo { left: 200 });
        assert_eq!(counted.count(), 200);
        let at = position.get().as_micros() as i64;
        assert!((at - 2_083).abs() <= 1, "{at} µs for 100 frames at 48 kHz");
    }
}
