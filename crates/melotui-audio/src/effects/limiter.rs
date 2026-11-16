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
use fundsp::prelude32::*;
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
    /// The frames come in so far, counting from the last reset.
    frame: u64,
    /// The smallest gain any frame in the lookahead needs, kept as a queue
    /// of (frame, gain) in which each gain is smaller than every one after
    /// it: the front is the smallest, and a frame is dropped once it has
    /// left the window or a later one needs less.
    needs: std::collections::VecDeque<(u64, f32)>,
    /// That smallest gain over the last `lookahead` frames, and their sum:
    /// a moving average of it, which is the gain smoothed.
    smallest: Vec<f32>,
    smallest_idx: usize,
    smallest_sum: f64,
    gain: f32,
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
            frame: 0,
            needs: std::collections::VecDeque::new(),
            smallest: Vec::new(),
            smallest_idx: 0,
            smallest_sum: 0.0,
            gain: 1.0,
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
        // Room for the whole window, so that nothing is allocated on the
        // audio thread.
        self.needs = std::collections::VecDeque::with_capacity(len + 1);
        self.smallest = vec![1.0; self.lookahead];
        self.release_coeff = 1.0 - (-1.0 / (self.sample_rate * RELEASE_SECONDS)).exp();
        self.clear();
    }

    /// Back to silence, with nothing to hold back.
    fn clear(&mut self) {
        self.delay_l.fill(0.0);
        self.delay_r.fill(0.0);
        self.write_idx = 0;
        self.frame = 0;
        self.needs.clear();
        self.smallest.fill(1.0);
        self.smallest_idx = 0;
        self.smallest_sum = self.lookahead as f64;
        self.gain = 1.0;
    }

    /// Lets `left` and `right` in, and gives out the frame `lookahead` frames
    /// before them at the gain it needs.
    ///
    /// The gain never lets a frame out above the ceiling. Each frame's need
    /// is in the smallest-gain window for the `lookahead` frames it spends in
    /// the delay line, so every value the moving average takes in over that
    /// time is at most its need, and so is the average when it comes out.
    /// Easing toward the need instead — as this did, over the lookahead —
    /// gets only part of the way there by then: a loud start came out a
    /// third over the ceiling for its first 50 ms.
    #[inline]
    fn process_sample(&mut self, left: f32, right: f32) -> (f32, f32) {
        let len = self.delay_l.len();

        self.delay_l[self.write_idx] = left;
        self.delay_r[self.write_idx] = right;
        let read_idx = (self.write_idx + len - self.lookahead) % len;
        let out_l = self.delay_l[read_idx];
        let out_r = self.delay_r[read_idx];
        self.write_idx = (self.write_idx + 1) % len;

        // What this frame needs, and the least any frame in the window does.
        let peak = left.abs().max(right.abs());
        let need = if peak > THRESHOLD {
            THRESHOLD / peak
        } else {
            1.0
        };
        while self.needs.back().is_some_and(|&(_, gain)| gain >= need) {
            self.needs.pop_back();
        }
        self.needs.push_back((self.frame, need));
        while self
            .needs
            .front()
            .is_some_and(|&(frame, _)| frame + (self.lookahead as u64) < self.frame)
        {
            self.needs.pop_front();
        }
        let smallest = self.needs.front().map_or(1.0, |&(_, gain)| gain);
        self.frame += 1;

        // Its moving average, which comes down to a need by the time the
        // frame with it comes out.
        self.smallest_sum += f64::from(smallest) - f64::from(self.smallest[self.smallest_idx]);
        self.smallest[self.smallest_idx] = smallest;
        self.smallest_idx = (self.smallest_idx + 1) % self.lookahead;
        let average = (self.smallest_sum / self.lookahead as f64) as f32;

        // Down with it at once, which is already smooth; back up slowly, so
        // that the low end does not pump between one peak and the next.
        self.gain = if average < self.gain {
            average
        } else {
            self.gain + (average - self.gain) * self.release_coeff
        };

        (out_l * self.gain, out_r * self.gain)
    }
}

impl AudioUnit for LinkedLimiter {
    fn reset(&mut self) {
        // Cleared in place rather than through `reconfigure`: the graph is
        // reset on the audio thread every time the effects come out of bypass,
        // and with the sample rate unchanged there is nothing to reallocate.
        self.clear();
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
        // From the very first frame: a limiter that is only right once it
        // has caught up lets the start of every loud passage through.
        let peak = out
            .iter()
            .fold(0.0f32, |a, &(l, r)| a.max(l.abs()).max(r.abs()));
        assert!(
            peak <= THRESHOLD * 1.0001,
            "peak {peak} exceeded the ceiling"
        );
    }

    #[test]
    fn a_sudden_loud_start_is_held_under_the_ceiling_from_its_first_frame() {
        // Silence, then at once far over: drums out of nowhere, or surround
        // mixed down. Eased toward over the lookahead, the gain had come only
        // part of the way down when the loud frames came out.
        for over in [1.5f32, 3.0, 10.0, 100.0] {
            let mut lim = LinkedLimiter::new(48_000.0);
            let mut input = vec![(0.0, 0.0); 1_000];
            input.extend((0..24_000).map(|i| {
                let v = (i as f32 / 48_000.0 * 220.0 * std::f32::consts::TAU).sin() * over;
                (v, -v)
            }));
            // And a single-frame spike after it has settled.
            input[12_000] = (over * 1.5, 0.0);
            let out = run(&mut lim, &input);
            let peak = out
                .iter()
                .fold(0.0f32, |a, &(l, r)| a.max(l.abs()).max(r.abs()));
            assert!(peak <= THRESHOLD * 1.0001, "{over}x: peaked at {peak}");
            // And limited, not silenced: it settles at the ceiling.
            let settled = out[20_000..]
                .iter()
                .fold(0.0f32, |a, &(l, _)| a.max(l.abs()));
            assert!(settled > THRESHOLD * 0.9, "{over}x: settled at {settled}");
        }
    }

    #[test]
    fn it_recovers_slowly_once_the_loud_passage_is_over() {
        // Released over about 120 ms, not snapped back.
        let mut lim = LinkedLimiter::new(48_000.0);
        let loud: Vec<(f32, f32)> = (0..9_600).map(|_| (2.0, 2.0)).collect();
        let quiet: Vec<(f32, f32)> = (0..48_000).map(|_| (0.5, 0.5)).collect();
        run(&mut lim, &loud);
        let out = run(&mut lim, &quiet);
        // 10 ms after the quiet starts it is still held well down; a second
        // after — eight times the release — it is back to as it is.
        assert!(out[240 + 480].0 < 0.4, "{}", out[240 + 480].0);
        assert!((out[47_000].0 - 0.5).abs() < 1e-3, "{}", out[47_000].0);
    }
}
