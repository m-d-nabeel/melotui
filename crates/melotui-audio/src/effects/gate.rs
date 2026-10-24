//! Running expensive units only while they are wanted.

use fundsp::DEFAULT_SR;
use fundsp::buffer::{BufferMut, BufferRef, BufferVec};
use fundsp::hacker32::*;
use fundsp::signal::SignalFrame;

/// How long a unit keeps running after its control reaches zero: long enough
/// for the glide that fades its output out to arrive.
const HOLD_SECONDS: f64 = 0.25;
/// How long its input is faded in for when it starts again.
///
/// A unit full of delay lines that restarts empty while the music is
/// mid-waveform opens each line onto a hard edge as the signal first reaches
/// its far end. In the reverb that was a burst of clicks 50 to 80 ms after it
/// was switched on, nine times rougher than the music around it. With the
/// input faded in, the lines fill from silence instead.
const FADE_IN_SECONDS: f64 = 0.03;

/// Runs `unit` only while `amount` is turned up, and outputs silence
/// otherwise.
///
/// For effects whose output is faded in and out by their control downstream,
/// like the reverb: at zero its output is multiplied by zero anyway, and the
/// reverb alone was a third of what the whole graph cost with every control at
/// rest. When it comes back it starts afresh rather than playing out whatever
/// was still ringing when it stopped, which would be a tail from a different
/// moment of the track.
#[derive(Clone)]
pub struct OnDemand<U> {
    unit: U,
    amount: Shared,
    hold: usize,
    remaining: usize,
    running: bool,
    fade_in: usize,
    /// Frames run since the unit last started.
    since_start: usize,
    /// The faded input, block-wise and frame-wise.
    faded: BufferVec,
    faded_frame: Vec<f32>,
}

impl<U: AudioUnit + Clone> OnDemand<U> {
    pub fn new(unit: U, amount: &Shared) -> Self {
        let inputs = unit.inputs();
        let mut gated = Self {
            unit,
            amount: amount.clone(),
            hold: 0,
            remaining: 0,
            running: false,
            fade_in: 0,
            since_start: 0,
            faded: BufferVec::new(inputs),
            faded_frame: vec![0.0; inputs],
        };
        gated.set_sample_rate(DEFAULT_SR);
        gated
    }

    /// Whether the unit should run for the next `frames` frames.
    fn wanted(&mut self, frames: usize) -> bool {
        if self.amount.value() > 0.0 {
            self.remaining = self.hold;
        }
        if self.remaining == 0 {
            self.running = false;
            return false;
        }
        if !self.running {
            self.unit.reset();
            self.running = true;
            self.since_start = 0;
        }
        self.remaining = self.remaining.saturating_sub(frames);
        true
    }

    /// Gain of the input fade, `offset` frames into the current block.
    fn fade(since_start: usize, offset: usize, fade_in: usize) -> f32 {
        let t = ((since_start + offset) as f32 / Ord::max(fade_in, 1) as f32).min(1.0);
        t * t * (3.0 - 2.0 * t)
    }
}

impl<U: AudioUnit + Clone> AudioUnit for OnDemand<U> {
    fn reset(&mut self) {
        self.unit.reset();
        self.running = false;
        self.remaining = 0;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.hold = (HOLD_SECONDS * sample_rate).ceil() as usize;
        self.fade_in = (FADE_IN_SECONDS * sample_rate).ceil() as usize;
        self.unit.set_sample_rate(sample_rate);
        self.reset();
    }

    fn tick(&mut self, input: &[f32], output: &mut [f32]) {
        if !self.wanted(1) {
            output.fill(0.0);
        } else if self.since_start >= self.fade_in {
            self.unit.tick(input, output);
        } else {
            let g = Self::fade(self.since_start, 0, self.fade_in);
            for (faded, &x) in self.faded_frame.iter_mut().zip(input) {
                *faded = x * g;
            }
            self.since_start += 1;
            self.unit.tick(&self.faded_frame, output);
        }
    }

    fn process(&mut self, size: usize, input: &BufferRef, output: &mut BufferMut) {
        if !self.wanted(size) {
            for ch in 0..self.unit.outputs() {
                output.channel_f32_mut(ch)[..size].fill(0.0);
            }
        } else if self.since_start >= self.fade_in {
            self.unit.process(size, input, output);
        } else {
            for ch in 0..self.unit.inputs() {
                let faded = self.faded.channel_f32_mut(ch);
                for (i, (faded, &x)) in faded
                    .iter_mut()
                    .zip(&input.channel_f32(ch)[..size])
                    .enumerate()
                {
                    *faded = x * Self::fade(self.since_start, i, self.fade_in);
                }
            }
            self.since_start += size;
            self.unit.process(size, &self.faded.buffer_ref(), output);
        }
    }

    fn inputs(&self) -> usize {
        self.unit.inputs()
    }

    fn outputs(&self) -> usize {
        self.unit.outputs()
    }

    fn route(&mut self, input: &SignalFrame, frequency: f64) -> SignalFrame {
        self.unit.route(input, frequency)
    }

    fn get_id(&self) -> u64 {
        0x0d3a_4d00_0d3a_4d01
    }

    fn footprint(&self) -> usize {
        std::mem::size_of::<Self>() + self.unit.footprint()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_runs_while_wanted_and_rests_otherwise() {
        let amount = shared(0.0);
        let mut gated = OnDemand::new(dc(1.0), &amount);
        gated.set_sample_rate(48_000.0);
        let mut out = [0.0];
        gated.tick(&[], &mut out);
        assert_eq!(out[0], 0.0, "silent while not wanted");

        amount.set_value(0.5);
        gated.tick(&[], &mut out);
        assert_eq!(out[0], 1.0, "running once wanted");

        // Turned off, it keeps running for the hold so the fade-out it feeds
        // can finish, then rests.
        amount.set_value(0.0);
        for _ in 0..1_000 {
            gated.tick(&[], &mut out);
        }
        assert_eq!(out[0], 1.0, "still running during the hold");
        for _ in 0..12_000 {
            gated.tick(&[], &mut out);
        }
        assert_eq!(out[0], 0.0, "resting after the hold");
    }

    #[test]
    fn it_comes_back_without_what_it_was_holding() {
        // A unit with memory, turned off while the music is still loud — so
        // it stops with its delay line full — then turned back on into
        // silence. What it was holding must not come back out.
        let amount = shared(1.0);
        let mut gated = OnDemand::new(delay(0.1), &amount);
        gated.set_sample_rate(48_000.0);
        let mut out = [0.0];
        for _ in 0..2_400 {
            gated.tick(&[0.7], &mut out);
        }
        amount.set_value(0.0);
        for _ in 0..24_000 {
            gated.tick(&[0.7], &mut out);
        }
        assert_eq!(out[0], 0.0, "resting");
        amount.set_value(1.0);
        let mut worst = 0.0f32;
        for _ in 0..9_600 {
            gated.tick(&[0.0], &mut out);
            worst = worst.max(out[0].abs());
        }
        assert_eq!(worst, 0.0, "it replayed {worst} of what it held");
    }
}
