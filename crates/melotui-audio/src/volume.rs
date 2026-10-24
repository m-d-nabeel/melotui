//! Volume, applied as a glide rather than a step.

use fundsp::hacker32::Shared;
use rodio::{Source, source::SeekError};
use std::time::Duration;

use crate::effects::dsp::{Glide, glide_coeff};

/// Applies the volume setting to the stream, gliding to each new setting.
///
/// rodio's own volume control jumps to a new setting at its next 5 ms control
/// tick, and a jump in gain is a click: one press of the volume key on loud
/// material was a step of several hundredths in the signal. This glides
/// instead, once per frame so both channels of a frame always share a gain.
///
/// It sits after the visualizer tap, where rodio's control was, so the
/// spectrum still shows the music rather than the volume knob.
pub struct VolumeRamp<S> {
    source: S,
    volume: Shared,
    gain: Glide,
    coeff: f32,
    sample_rate: u32,
    channels: u16,
    /// Position within the current frame.
    position: u16,
}

impl<S: Source<Item = f32>> VolumeRamp<S> {
    pub fn new(source: S, volume: Shared) -> Self {
        let sample_rate = source.sample_rate();
        Self {
            // A new track starts at the current volume, not faded up from
            // silence.
            gain: Glide::new(volume.value()),
            coeff: glide_coeff(sample_rate as f32),
            channels: source.channels().max(1),
            sample_rate,
            source,
            volume,
            position: 0,
        }
    }
}

impl<S: Source<Item = f32>> Iterator for VolumeRamp<S> {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.source.next()?;
        if self.position == 0 {
            let sample_rate = self.source.sample_rate();
            if sample_rate != self.sample_rate {
                self.sample_rate = sample_rate;
                self.coeff = glide_coeff(sample_rate as f32);
            }
            self.gain.step(self.volume.value(), self.coeff);
        }
        self.position = (self.position + 1) % self.channels;
        Some(sample * self.gain.value())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.source.size_hint()
    }
}

impl<S: Source<Item = f32>> Source for VolumeRamp<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.source.current_span_len()
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.source.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.source.try_seek(pos)?;
        self.position = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fundsp::hacker32::shared;

    /// Endless stereo source at a constant level.
    struct Steady;

    impl Iterator for Steady {
        type Item = f32;
        fn next(&mut self) -> Option<f32> {
            Some(0.8)
        }
    }

    impl Source for Steady {
        fn current_span_len(&self) -> Option<usize> {
            None
        }
        fn channels(&self) -> u16 {
            2
        }
        fn sample_rate(&self) -> u32 {
            48_000
        }
        fn total_duration(&self) -> Option<Duration> {
            None
        }
    }

    #[test]
    fn a_new_track_starts_at_the_current_volume() {
        let volume = shared(0.5);
        let mut ramp = VolumeRamp::new(Steady, volume);
        assert_eq!(ramp.next(), Some(0.4));
    }

    #[test]
    fn a_volume_change_is_a_glide_not_a_step() {
        let volume = shared(1.0);
        let mut ramp = VolumeRamp::new(Steady, volume.clone());
        let mut previous = ramp.next().unwrap();
        ramp.next();
        volume.set_value(0.5);
        let mut worst = 0.0f32;
        for _ in 0..48_000 {
            let left = ramp.next().unwrap();
            let right = ramp.next().unwrap();
            assert_eq!(left, right, "both channels of a frame share a gain");
            worst = worst.max((left - previous).abs());
            previous = left;
        }
        assert!(
            (previous - 0.4).abs() < 1e-6,
            "arrives at the new volume: {previous}"
        );
        assert!(
            worst < 0.001,
            "the level moved by {worst} in one frame; rodio's step was 0.4"
        );
    }
}
