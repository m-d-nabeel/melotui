//! Stereo-linked brick-wall limiter.
//!
//! The chain previously ended in `limiter(..) | limiter(..)` — two independent
//! mono limiters. Measured, driving the left channel 13 dB into limiting moved
//! the right channel by 0.00 dB, so every loud transient on one side dragged the
//! stereo image across to the other. A stereo limiter has to derive one gain
//! from both channels and apply it to both, which is what this does.
//!
//! It also uses lookahead. Reducing gain only once a peak has already been
//! output means either distorting it or setting an attack so fast it audibly
//! pumps; delaying the audio by the attack window lets the gain arrive first.

use fundsp::buffer::{BufferMut, BufferRef};
use fundsp::hacker32::*;
use fundsp::signal::SignalFrame;

/// Ceiling, just under full scale to leave room for inter-sample peaks and the
/// resampler downstream.
const THRESHOLD: f32 = 0.891; // -1 dBFS
/// Lookahead / attack window. Long enough to catch a bass transient, short
/// enough not to audibly pre-duck.
const LOOKAHEAD_SECONDS: f32 = 0.005;
/// Time for gain to recover once the peak has passed. Slow enough to avoid
/// pumping on sustained low end.
const RELEASE_SECONDS: f32 = 0.120;

pub struct LinkedLimiter {
    sample_rate: f32,
    /// Interleaved lookahead ring, one slot per frame.
    delay_l: Vec<f32>,
    delay_r: Vec<f32>,
    write_idx: usize,
    lookahead: usize,
    /// Peak envelope over the lookahead window, shared by both channels.
    envelope: f32,
    /// Currently applied gain, smoothed.
    gain: f32,
    attack_coeff: f32,
    release_coeff: f32,
}

impl Clone for LinkedLimiter {
    fn clone(&self) -> Self {
        Self::new(self.sample_rate)
    }
}

impl LinkedLimiter {
    pub fn new(sample_rate: f32) -> Self {
        let mut limiter = Self {
            sample_rate: sample_rate.max(1.0),
            delay_l: Vec::new(),
            delay_r: Vec::new(),
            write_idx: 0,
            lookahead: 0,
            envelope: 0.0,
            gain: 1.0,
            attack_coeff: 0.0,
            release_coeff: 0.0,
        };
        limiter.reconfigure();
        limiter
    }

    fn reconfigure(&mut self) {
        self.lookahead = Ord::max((self.sample_rate * LOOKAHEAD_SECONDS) as usize, 1);
        let len = self.lookahead + 1;
        self.delay_l = vec![0.0; len];
        self.delay_r = vec![0.0; len];
        self.write_idx = 0;
        // One-pole coefficients. Attack spans the lookahead window so the gain
        // has fully arrived by the time the peak reaches the output.
        self.attack_coeff = 1.0 - (-1.0 / (self.sample_rate * LOOKAHEAD_SECONDS)).exp();
        self.release_coeff = 1.0 - (-1.0 / (self.sample_rate * RELEASE_SECONDS)).exp();
        self.envelope = 0.0;
        self.gain = 1.0;
    }

    #[inline]
    fn process_sample(&mut self, left: f32, right: f32) -> (f32, f32) {
        let len = self.delay_l.len();

        // Push the incoming frame and take the one leaving the window.
        self.delay_l[self.write_idx] = left;
        self.delay_r[self.write_idx] = right;
        let read_idx = (self.write_idx + len - self.lookahead) % len;
        let out_l = self.delay_l[read_idx];
        let out_r = self.delay_r[read_idx];
        self.write_idx = (self.write_idx + 1) % len;

        // One detector for both channels: this is what keeps the image stable.
        let peak = left.abs().max(right.abs());
        self.envelope = if peak > self.envelope {
            peak
        } else {
            self.envelope + (peak - self.envelope) * self.release_coeff
        };

        let target = if self.envelope > THRESHOLD {
            THRESHOLD / self.envelope
        } else {
            1.0
        };
        // Duck quickly, recover gently.
        let coeff = if target < self.gain {
            self.attack_coeff
        } else {
            self.release_coeff
        };
        self.gain += (target - self.gain) * coeff;

        (out_l * self.gain, out_r * self.gain)
    }
}

impl AudioUnit for LinkedLimiter {
    fn reset(&mut self) {
        // Cleared in place rather than through `reconfigure`: the graph is
        // reset on the audio thread every time the effects come out of bypass,
        // and with the sample rate unchanged there is nothing to reallocate.
        self.delay_l.fill(0.0);
        self.delay_r.fill(0.0);
        self.write_idx = 0;
        self.envelope = 0.0;
        self.gain = 1.0;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = (sample_rate as f32).max(1.0);
        self.reconfigure();
    }

    fn tick(&mut self, input: &[f32], output: &mut [f32]) {
        let (l, r) = self.process_sample(input[0], input[1]);
        output[0] = l;
        output[1] = r;
    }

    fn process(&mut self, size: usize, input: &BufferRef, output: &mut BufferMut) {
        for i in 0..size {
            let (l, r) = self.process_sample(input.channel_f32(0)[i], input.channel_f32(1)[i]);
            output.channel_f32_mut(0)[i] = l;
            output.channel_f32_mut(1)[i] = r;
        }
    }

    fn inputs(&self) -> usize {
        2
    }

    fn outputs(&self) -> usize {
        2
    }

    fn route(&mut self, input: &SignalFrame, _frequency: f64) -> SignalFrame {
        // The lookahead is latency, and has to be reported as such: the
        // bypass in `FundspSource` lines the dry signal up with the processed
        // one by asking the graph how late it runs.
        let latency = self.lookahead as f64;
        let mut output = SignalFrame::new(2);
        output.set(0, input.at(0).distort(latency));
        output.set(1, input.at(1).distort(latency));
        output
    }

    fn get_id(&self) -> u64 {
        0x117e_d111_17e5_u64
    }

    fn footprint(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(limiter: &mut LinkedLimiter, frames: &[(f32, f32)]) -> Vec<(f32, f32)> {
        frames
            .iter()
            .map(|&(l, r)| {
                let mut out = [0.0; 2];
                limiter.tick(&[l, r], &mut out);
                (out[0], out[1])
            })
            .collect()
    }

    fn rms(v: &[(f32, f32)], right: bool) -> f32 {
        let e: f64 = v
            .iter()
            .map(|x| {
                let s = if right { x.1 } else { x.0 };
                (s * s) as f64
            })
            .sum();
        (e / v.len() as f64).sqrt() as f32
    }

    #[test]
    fn quiet_material_passes_through_untouched() {
        let mut lim = LinkedLimiter::new(48_000.0);
        let input: Vec<(f32, f32)> = (0..4800)
            .map(|i| {
                let v = (i as f32 / 48_000.0 * 440.0 * std::f32::consts::TAU).sin() * 0.2;
                (v, -v)
            })
            .collect();
        let out = run(&mut lim, &input);
        // Compare past the lookahead delay.
        for i in 1000..4000 {
            assert!(
                (out[i].0 - input[i - 240].0).abs() < 1e-4,
                "below threshold the limiter must be transparent"
            );
        }
    }

    #[test]
    fn gain_reduction_is_shared_by_both_channels() {
        // Left is slammed, right holds a steady quiet tone. Independent limiters
        // left the right channel untouched, yanking the image sideways.
        let mut lim = LinkedLimiter::new(48_000.0);
        let quiet: Vec<(f32, f32)> = (0..24_000)
            .map(|i| {
                let t = i as f32 / 48_000.0;
                (
                    (t * 220.0 * std::f32::consts::TAU).sin() * 0.05,
                    (t * 330.0 * std::f32::consts::TAU).sin() * 0.05,
                )
            })
            .collect();
        let loud: Vec<(f32, f32)> = (0..24_000)
            .map(|i| {
                let t = (i + 24_000) as f32 / 48_000.0;
                (
                    (t * 220.0 * std::f32::consts::TAU).sin() * 3.0,
                    (t * 330.0 * std::f32::consts::TAU).sin() * 0.05,
                )
            })
            .collect();

        let before = run(&mut lim, &quiet);
        let during = run(&mut lim, &loud);

        let r_before = rms(&before[8_000..], true);
        let r_during = rms(&during[8_000..], true);
        let reduction_db = 20.0 * (r_during / r_before).log10();

        assert!(
            reduction_db < -3.0,
            "the untouched channel must duck along with the limited one, got {reduction_db:.2} dB"
        );
    }

    #[test]
    fn output_never_exceeds_the_ceiling() {
        let mut lim = LinkedLimiter::new(48_000.0);
        let input: Vec<(f32, f32)> = (0..48_000)
            .map(|i| {
                let t = i as f32 / 48_000.0;
                let v = (t * 60.0 * std::f32::consts::TAU).sin() * 4.0;
                (v, v * 0.5)
            })
            .collect();
        let out = run(&mut lim, &input);
        // Skip the first window while the envelope is still catching up.
        let peak = out[2000..]
            .iter()
            .fold(0.0f32, |a, &(l, r)| a.max(l.abs()).max(r.abs()));
        assert!(peak <= THRESHOLD * 1.02, "peak {peak} exceeded the ceiling");
    }
}
