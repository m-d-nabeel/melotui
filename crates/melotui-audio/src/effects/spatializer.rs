//! Headphone spatialiser: static pan plus an optional orbiting auto-pan.
//!
//! # Why this was rewritten
//!
//! The previous version summed the input to mono (`(left + right) * 0.5`), fed
//! that single signal through one delay line, and crossfaded up to **80%** of it
//! back over the dry signal. Everything that only exists as a difference between
//! the channels — reverb tails, room ambience, wide-panned instruments — was
//! therefore attenuated by up to 14 dB the moment the effect was switched on.
//! Measured with the left channel carrying a 400 Hz tone and the right a 700 Hz
//! tone, channel separation collapsed from 50 dB to 7.5 dB at depth 0.5, and the
//! bleed was **in phase**: mono summing, not widening.
//!
//! # What it does now
//!
//! Each channel keeps its own delay line and its own direct path at unity, so
//! the stereo difference survives intact. Externalisation comes from *crossfeed*
//! — a small, delayed, low-passed amount of each channel fed to the opposite
//! ear, which is how a real head works and how analogue crossfeed circuits are
//! built. Because the crossfeed is low-passed, it only touches the low end,
//! where the ears genuinely cannot resolve direction; everything above the
//! shadow cutoff keeps its full separation.
//!
//! Direction is cued the way it physically is: an interaural time difference
//! (the far ear hears it later), an interaural level difference, and head
//! shadowing (the far ear hears it duller).

use fundsp::buffer::{BufferMut, BufferRef};
use fundsp::hacker32::*;
use fundsp::signal::SignalFrame;
use fundsp::DEFAULT_SR;
use std::f32::consts::TAU;

use super::AudioParams;

/// Maximum interaural time difference, i.e. the head's width in seconds.
const MAX_ITD_SECONDS: f32 = 0.000_65;
/// Above this, the far ear is shadowed by the head. Also the crossfeed cutoff:
/// crossfeeding high frequencies is what would collapse the image.
const SHADOW_CUTOFF_HZ: f32 = 700.0;
/// Crossfeed at full depth. Analogue crossfeed circuits sit near -9 dB; this is
/// -11 dB, and only below the shadow cutoff.
const MAX_CROSSFEED: f32 = 0.28;
/// How fast the pan position follows its target, per sample at 48 kHz.
const PAN_SMOOTHING_HZ: f32 = 12.0;
/// Strength of the front/back spectral cue. Real head-related front/back
/// differences are a few dB of shading in the top octaves. The first version of
/// this blended in a 2.5 kHz low-pass at 50%, which audibly dulled the midrange
/// every time the image swung behind the head — the same complaint the mono
/// summing caused, arriving by a different route.
const MAX_REAR_BLEND: f32 = 0.22;
/// Upper cutoff of the front/back shading filter, kept well clear of the
/// midrange so vocals and guitars are not touched.
const REAR_CUTOFF_HZ: f32 = 6_000.0;

#[derive(Clone)]
pub struct StereoSpatializer {
    pan: Shared,
    auto_pan_rate: Shared,
    auto_pan_depth: Shared,
    sample_rate: f32,

    phase: f32,
    smoothed_pan: f32,
    pan_coeff: f32,

    /// One delay line per channel: the whole point is that they stay separate.
    delay_l: Vec<f32>,
    delay_r: Vec<f32>,
    write_idx: usize,

    /// One-pole states for the shadowing low-pass on each crossfeed path.
    shadow_l: f32,
    shadow_r: f32,
    /// Shadow states for the front/back spectral cue.
    rear_l: f32,
    rear_r: f32,
}

impl StereoSpatializer {
    pub fn new(params: &AudioParams) -> Self {
        let mut s = Self {
            pan: params.pan.clone(),
            auto_pan_rate: params.auto_pan_rate.clone(),
            auto_pan_depth: params.auto_pan_depth.clone(),
            sample_rate: DEFAULT_SR as f32,
            phase: 0.0,
            smoothed_pan: 0.0,
            pan_coeff: 0.0,
            delay_l: Vec::new(),
            delay_r: Vec::new(),
            write_idx: 0,
            shadow_l: 0.0,
            shadow_r: 0.0,
            rear_l: 0.0,
            rear_r: 0.0,
        };
        s.reconfigure();
        s
    }

    fn reconfigure(&mut self) {
        let len = Ord::max((self.sample_rate * MAX_ITD_SECONDS).ceil() as usize + 4, 4);
        self.delay_l = vec![0.0; len];
        self.delay_r = vec![0.0; len];
        self.write_idx = 0;
        self.pan_coeff = 1.0 - (-TAU * PAN_SMOOTHING_HZ / self.sample_rate).exp();
    }

    /// Constant-power balance, so sweeping across centre does not dip.
    fn balance_gains(pan: f32) -> (f32, f32) {
        let angle = (pan.clamp(-1.0, 1.0) + 1.0) * 0.25 * TAU * 0.5; // 0..π/2
        (angle.cos(), angle.sin())
    }

    fn read(line: &[f32], write_idx: usize, delay_samples: f32) -> f32 {
        let len = line.len();
        let max = (len - 2) as f32;
        let mut pos = write_idx as f32 - delay_samples.clamp(0.0, max);
        while pos < 0.0 {
            pos += len as f32;
        }
        let i0 = (pos.floor() as usize) % len;
        let i1 = (i0 + 1) % len;
        let frac = pos - pos.floor();
        line[i0] * (1.0 - frac) + line[i1] * frac
    }

    /// One-pole low-pass; the head's shadow on the far ear.
    fn shadow(state: &mut f32, sample: f32, cutoff_hz: f32, sample_rate: f32) -> f32 {
        let alpha = 1.0 - (-TAU * cutoff_hz / sample_rate).exp();
        *state += alpha * (sample - *state);
        *state
    }

    fn process_sample(&mut self, left: f32, right: f32) -> (f32, f32) {
        let rate = self.auto_pan_rate.value().clamp(0.0, 2.0);
        let depth = self.auto_pan_depth.value().clamp(0.0, 1.0);
        let static_pan = self.pan.value().clamp(-1.0, 1.0);

        let orbiting = depth > 0.001 && rate > 0.001;
        let auto = if orbiting {
            self.phase = (self.phase + rate / self.sample_rate).fract();
            (self.phase * TAU).sin() * depth
        } else {
            0.0
        };

        let target_pan = (static_pan + auto).clamp(-1.0, 1.0);
        self.smoothed_pan += (target_pan - self.smoothed_pan) * self.pan_coeff;
        let pan = self.smoothed_pan;
        let (gain_l, gain_r) = Self::balance_gains(pan);

        // Nothing asked for: stay bit-transparent rather than paying for a
        // spatialiser that is not doing anything.
        if !orbiting && static_pan.abs() < 0.001 && depth < 0.001 {
            return (left, right);
        }

        self.delay_l[self.write_idx] = left;
        self.delay_r[self.write_idx] = right;

        // The ear further from the source hears it later.
        let itd = self.sample_rate * MAX_ITD_SECONDS;
        let delay_l = pan.max(0.0) * itd;
        let delay_r = (-pan).max(0.0) * itd;
        let direct_l = Self::read(&self.delay_l, self.write_idx, delay_l);
        let direct_r = Self::read(&self.delay_r, self.write_idx, delay_r);

        // Crossfeed carries the *opposite* channel, delayed by the full head
        // width and shadowed. Because it is low-passed, separation above the
        // cutoff is untouched — which is what preserves the stereo image.
        let crossfeed = MAX_CROSSFEED * depth.max(static_pan.abs());
        let (mut out_l, mut out_r) = if crossfeed > 0.001 {
            let cross_r = Self::shadow(
                &mut self.shadow_l,
                Self::read(&self.delay_r, self.write_idx, itd),
                SHADOW_CUTOFF_HZ,
                self.sample_rate,
            );
            let cross_l = Self::shadow(
                &mut self.shadow_r,
                Self::read(&self.delay_l, self.write_idx, itd),
                SHADOW_CUTOFF_HZ,
                self.sample_rate,
            );
            // Normalised so adding crossfeed does not raise the level.
            let norm = 1.0 / (1.0 + crossfeed);
            (
                (direct_l + cross_r * crossfeed) * norm,
                (direct_r + cross_l * crossfeed) * norm,
            )
        } else {
            (direct_l, direct_r)
        };

        self.write_idx = (self.write_idx + 1) % self.delay_l.len();

        // Front/back is cued spectrally, not by level: sources behind the head
        // lose top end. Applied per channel to its own signal, so unlike the
        // old rear-centre cue it costs no separation.
        if orbiting {
            let behind = ((self.phase * TAU).cos() + 1.0) * 0.5 * depth;
            let rear = behind * (1.0 - pan.abs()).powi(2) * MAX_REAR_BLEND;
            if rear > 0.001 {
                let cutoff = REAR_CUTOFF_HZ - 2_000.0 * rear;
                let dull_l = Self::shadow(&mut self.rear_l, out_l, cutoff, self.sample_rate);
                let dull_r = Self::shadow(&mut self.rear_r, out_r, cutoff, self.sample_rate);
                out_l = out_l * (1.0 - rear) + dull_l * rear;
                out_r = out_r * (1.0 - rear) + dull_r * rear;
            }
        }

        (out_l * gain_l * std::f32::consts::SQRT_2, out_r * gain_r * std::f32::consts::SQRT_2)
    }
}

impl AudioUnit for StereoSpatializer {
    fn reset(&mut self) {
        self.phase = 0.0;
        self.smoothed_pan = self.pan.value().clamp(-1.0, 1.0);
        self.delay_l.fill(0.0);
        self.delay_r.fill(0.0);
        self.write_idx = 0;
        self.shadow_l = 0.0;
        self.shadow_r = 0.0;
        self.rear_l = 0.0;
        self.rear_r = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = (sample_rate as f32).max(1.0);
        self.reconfigure();
        self.reset();
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
        let mut output = SignalFrame::new(2);
        output.set(0, input.at(0).combine_nonlinear(input.at(1), 0.0));
        output.set(1, input.at(1).combine_nonlinear(input.at(0), 0.0));
        output
    }

    fn get_id(&self) -> u64 {
        0x05a7_1a11_05fa_71a1
    }

    fn footprint(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.0001,
            "expected {expected}, got {actual}"
        );
    }

    fn spatializer(setup: impl Fn(&AudioParams)) -> StereoSpatializer {
        let params = AudioParams::new();
        setup(&params);
        let mut s = StereoSpatializer::new(&params);
        s.set_sample_rate(48_000.0);
        s
    }

    /// Bleed of `fb` (right-channel-only) into the left output, in dB relative
    /// to the wanted left signal at `fa`. 0 dB would be fully mono.
    ///
    /// Measured with a static pan. The orbiting auto-pan sweeps the ITD, which
    /// Doppler-shifts a steady tone by around a hertz and smears it out of a
    /// narrow analysis bin — that would under-read the wanted signal and make
    /// the effect look far worse than it is. Width under orbit is covered by
    /// `orbiting_does_not_narrow_the_image` instead, which is immune to it.
    fn channel_bleed_db(depth: f32, fa: f32, fb: f32) -> f32 {
        const SR: f32 = 48_000.0;
        let mut s = spatializer(|p| {
            p.auto_pan_depth.set_value(depth);
            p.auto_pan_rate.set_value(0.0);
            p.pan.set_value(depth);
        });

        let mut out_l = Vec::with_capacity(48_000);
        for i in 0..48_000 {
            let t = i as f32 / SR;
            let mut o = [0.0; 2];
            s.tick(&[(t * fa * TAU).sin() * 0.25, (t * fb * TAU).sin() * 0.25], &mut o);
            out_l.push(o[0]);
        }

        let amp = |sig: &[f32], f: f32| {
            let s = &sig[sig.len() / 3..];
            let (mut re, mut im) = (0.0f64, 0.0f64);
            for (i, &v) in s.iter().enumerate() {
                let ph = (i as f64) / SR as f64 * f as f64 * std::f64::consts::TAU;
                re += v as f64 * ph.cos();
                im += v as f64 * ph.sin();
            }
            (2.0 * (re * re + im * im).sqrt() / s.len() as f64) as f32
        };
        20.0 * (amp(&out_l, fb).max(1e-12) / amp(&out_l, fa).max(1e-12)).log10()
    }

    /// Ratio of side (stereo-only) to mid (shared) energy, in dB.
    fn side_over_mid_db(out: &[(f32, f32)]) -> f32 {
        let skip = out.len() / 4;
        let (mut m, mut s) = (0.0f64, 0.0f64);
        for &(l, r) in &out[skip..] {
            m += (((l + r) * 0.5) as f64).powi(2);
            s += (((l - r) * 0.5) as f64).powi(2);
        }
        20.0 * ((s / m).sqrt() as f32).max(1e-12).log10()
    }

    #[test]
    fn is_transparent_when_nothing_is_engaged() {
        // Pan centred and no orbit: the spatialiser must pass audio through
        // untouched rather than run a delay line for no reason.
        let mut s = spatializer(|_| {});
        let mut out = [0.0, 0.0];
        s.tick(&[0.8, -0.2], &mut out);
        assert_close(out[0], 0.8);
        assert_close(out[1], -0.2);
    }

    #[test]
    fn centre_pan_does_not_attenuate() {
        let mut s = spatializer(|p| p.pan.set_value(0.0));
        let mut out = [0.0, 0.0];
        s.tick(&[0.5, 0.5], &mut out);
        assert_close(out[0], 0.5);
        assert_close(out[1], 0.5);
    }

    #[test]
    fn hard_pan_moves_the_image() {
        let mut s = spatializer(|p| p.pan.set_value(1.0));
        let mut out = [0.0, 0.0];
        for _ in 0..8_000 {
            s.tick(&[0.5, 0.5], &mut out);
        }
        assert!(
            out[1].abs() > out[0].abs() * 4.0,
            "hard right should be dominated by the right channel, got {out:?}"
        );
    }

    #[test]
    fn the_midrange_keeps_its_separation() {
        // This is the band the stereo image actually lives in. The old
        // mono-summing implementation measured about -7 dB of bleed here at
        // full depth, which is very nearly mono.
        for depth in [0.25f32, 0.5, 0.75, 1.0] {
            let mid = channel_bleed_db(depth, 400.0, 3_000.0);
            let high = channel_bleed_db(depth, 400.0, 8_000.0);
            assert!(mid < -20.0, "depth {depth}: {mid:.1} dB of bleed at 3 kHz");
            assert!(high < -30.0, "depth {depth}: {high:.1} dB of bleed at 8 kHz");
        }
    }

    #[test]
    fn crossfeed_stays_within_its_budget_and_is_off_at_zero() {
        // Below the shadow cutoff the ears genuinely share content — that is
        // what externalises the image on headphones — but it must stay bounded,
        // and must not engage at all when the effect is off.
        assert!(
            channel_bleed_db(0.0, 400.0, 700.0) < -60.0,
            "with the effect off the channels must not touch"
        );
        for depth in [0.25f32, 0.5, 0.75, 1.0] {
            let low = channel_bleed_db(depth, 400.0, 700.0);
            assert!(low < -12.0, "depth {depth}: {low:.1} dB of crossfeed is too much");
        }
    }

    #[test]
    fn orbiting_does_not_narrow_the_image() {
        // Decorrelated input, so side and mid energy start out comparable.
        // Whatever the effect does to the image, it must not shrink the side
        // channel — that is the "background goes dead" failure.
        let (mut a, mut b) = (12345u32, 987654u32);
        let rnd = |s: &mut u32| {
            *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
            ((*s >> 8) as f32 / 8_388_608.0) - 1.0
        };
        let input: Vec<(f32, f32)> = (0..96_000)
            .map(|_| (rnd(&mut a) * 0.3, rnd(&mut b) * 0.3))
            .collect();
        let reference = side_over_mid_db(&input);

        for depth in [0.25f32, 0.5, 0.75, 1.0] {
            for rate in [0.0f32, 0.15] {
                let mut s = spatializer(|p| {
                    p.auto_pan_depth.set_value(depth);
                    p.auto_pan_rate.set_value(rate);
                });
                let out: Vec<(f32, f32)> = input
                    .iter()
                    .map(|&(l, r)| {
                        let mut o = [0.0; 2];
                        s.tick(&[l, r], &mut o);
                        (o[0], o[1])
                    })
                    .collect();
                let width = side_over_mid_db(&out);
                assert!(
                    width > reference - 3.0,
                    "depth {depth} rate {rate}: width fell from {reference:.2} to {width:.2} dB"
                );
            }
        }
    }

    #[test]
    fn auto_pan_actually_moves_the_image() {
        let mut s = spatializer(|p| {
            p.auto_pan_depth.set_value(0.9);
            p.auto_pan_rate.set_value(1.0);
        });
        let mut min_ratio = f32::MAX;
        let mut max_ratio = f32::MIN;
        let mut out = [0.0, 0.0];
        for i in 0..48_000 {
            let v = (i as f32 / 48_000.0 * 500.0 * TAU).sin() * 0.3;
            s.tick(&[v, v], &mut out);
            if i > 8_000 && out[1].abs() > 1e-6 {
                let ratio = out[0].abs() / out[1].abs();
                min_ratio = min_ratio.min(ratio);
                max_ratio = max_ratio.max(ratio);
            }
        }
        assert!(
            max_ratio / min_ratio > 2.0,
            "the image should sweep, L/R ratio spanned {min_ratio}..{max_ratio}"
        );
    }

    #[test]
    fn stays_finite_and_bounded_under_full_settings() {
        let mut s = spatializer(|p| {
            p.auto_pan_depth.set_value(1.0);
            p.auto_pan_rate.set_value(2.0);
            p.pan.set_value(-1.0);
        });
        let mut out = [0.0, 0.0];
        for i in 0..48_000 {
            let v = (i as f32 / 48_000.0 * 100.0 * TAU).sin();
            s.tick(&[v, -v], &mut out);
            assert!(out[0].is_finite() && out[1].is_finite());
            assert!(out[0].abs() < 4.0 && out[1].abs() < 4.0, "runaway gain: {out:?}");
        }
    }
}
