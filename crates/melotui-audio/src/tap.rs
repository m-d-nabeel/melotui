use ringbuf::traits::Producer;
use rodio::{Source, source::SeekError};
use std::time::Duration;

const BATCH_LEN: usize = 64;

/// Taps the playing stream and copies it into the visualizer's ring buffer.
///
/// The stream we sit on is **interleaved** (`L, R, L, R, …`), but the analyser
/// downstream treats what it receives as a single mono channel sampled at
/// `sample_rate()`. Forwarding the interleaved samples verbatim therefore fed it
/// a signal at twice the real rate, and every frequency it reported came out
/// half of its true value — a 1 kHz tone read as ~504 Hz. So downmix to mono
/// here, one value per frame, which also halves the traffic through the ring.
pub struct SpectrumSniffer<I, P>
where
    I: Source<Item = f32>,
    P: Producer<Item = f32>,
{
    input: I,
    producer: P,
    batch: [f32; BATCH_LEN],
    batch_pos: usize,

    /// Channel count of `input`, used to fold each frame down to mono.
    channels: u16,
    frame_sum: f32,
    frame_pos: u16,
}

impl<I, P> SpectrumSniffer<I, P>
where
    I: Source<Item = f32>,
    P: Producer<Item = f32>,
{
    pub fn new(input: I, producer: P) -> Self {
        let channels = input.channels().max(1);
        Self {
            input,
            producer,
            batch: [0.0; BATCH_LEN],
            batch_pos: 0,
            channels,
            frame_sum: 0.0,
            frame_pos: 0,
        }
    }

    /// Drops a partially accumulated frame. Called on seek so the downmix cannot
    /// stay stuck half a frame out of alignment.
    fn reset_frame(&mut self) {
        self.frame_sum = 0.0;
        self.frame_pos = 0;
        self.batch_pos = 0;
    }
}

impl<I, P> Iterator for SpectrumSniffer<I, P>
where
    I: Source<Item = f32>,
    P: Producer<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.input.next()?;

        self.frame_sum += sample;
        self.frame_pos += 1;

        if self.frame_pos >= self.channels {
            self.batch[self.batch_pos] = self.frame_sum / self.channels as f32;
            self.batch_pos += 1;
            self.frame_sum = 0.0;
            self.frame_pos = 0;

            if self.batch_pos >= BATCH_LEN {
                // Non-blocking: a full ring just drops the batch. The visualizer
                // falling behind must never stall the audio thread.
                let _ = self.producer.push_slice(&self.batch);
                self.batch_pos = 0;
            }
        }

        Some(sample)
    }
}

impl<I, P> Source for SpectrumSniffer<I, P>
where
    I: Source<Item = f32>,
    P: Producer<Item = f32>,
{
    fn current_span_len(&self) -> Option<usize> {
        self.input.current_span_len()
    }
    fn channels(&self) -> u16 {
        self.input.channels()
    }
    fn sample_rate(&self) -> u32 {
        self.input.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration()
    }
    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.input.try_seek(pos)?;
        self.reset_frame();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ringbuf::{
        HeapRb,
        traits::{Consumer, Split},
    };

    /// Interleaved stereo source: left channel = `l`, right channel = `r`.
    struct Stereo {
        l: f32,
        r: f32,
        n: usize,
        len: usize,
    }

    impl Iterator for Stereo {
        type Item = f32;
        fn next(&mut self) -> Option<f32> {
            if self.n >= self.len {
                return None;
            }
            let v = if self.n.is_multiple_of(2) { self.l } else { self.r };
            self.n += 1;
            Some(v)
        }
    }

    impl Source for Stereo {
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
    fn tap_emits_one_mono_value_per_stereo_frame() {
        let (prod, mut cons) = HeapRb::<f32>::new(1024).split();
        // 128 interleaved samples == 64 frames == exactly one batch.
        let mut sniffer = SpectrumSniffer::new(
            Stereo {
                l: 1.0,
                r: 0.0,
                n: 0,
                len: 128,
            },
            prod,
        );

        let passed_through: Vec<f32> = sniffer.by_ref().collect();

        // The audio itself must be untouched.
        assert_eq!(passed_through.len(), 128);
        assert_eq!(passed_through[0], 1.0);
        assert_eq!(passed_through[1], 0.0);

        // The tap must see 64 mono values, each the average of L and R.
        let tapped: Vec<f32> = std::iter::from_fn(|| cons.try_pop()).collect();
        assert_eq!(tapped.len(), 64, "one value per frame, not per sample");
        assert!(tapped.iter().all(|&v| (v - 0.5).abs() < 1e-6));
    }
}
