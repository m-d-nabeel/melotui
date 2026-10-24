//! Karaoke and stereo width: the mid/side stage.

use fundsp::DEFAULT_SR;
use fundsp::buffer::{BufferMut, BufferRef};
use fundsp::hacker32::*;
use fundsp::signal::SignalFrame;
use fundsp::svf::SvfCoefs;

use super::AudioParams;
use super::dsp::{Glide, SvfState, glide_coeff};

/// Below this the karaoke leaves the centre alone.
///
/// Bass and kick drum are centre-panned in nearly every mix, for the same
/// reason the vocal is, and cancelling the whole centre took them out with
/// it: at full karaoke a centred 60 Hz bass line measured 229 dB down. A
/// karaoke track with no bottom end is not much of a backing track. Above the
/// crossover is where the vocal lives.
const KARAOKE_CROSSOVER_HZ: f32 = 150.0;
const MAX_WIDTH: f32 = 3.0;

/// Karaoke and width, glided.
///
/// The centre is split at [`KARAOKE_CROSSOVER_HZ`] as `mid − lowpass(mid)`,
/// which is complementary by construction: the two halves sum back to the
/// centre exactly, so the split itself colours nothing.
#[derive(Clone)]
pub struct StereoImage {
    karaoke: Shared,
    width: Shared,
    k: Glide,
    w: Glide,
    low: SvfState,
    low_coefs: SvfCoefs<f32>,
    sample_rate: f32,
    coeff: f32,
}

impl StereoImage {
    pub fn new(params: &AudioParams) -> Self {
        let mut image = Self {
            karaoke: params.karaoke.clone(),
            width: params.width.clone(),
            k: Glide::new(0.0),
            w: Glide::new(1.0),
            low: SvfState::default(),
            low_coefs: SvfCoefs::default(),
            sample_rate: DEFAULT_SR as f32,
            coeff: glide_coeff(DEFAULT_SR as f32),
        };
        image.set_sample_rate(DEFAULT_SR);
        image
    }

    fn targets(&self) -> (f32, f32) {
        (
            self.karaoke.value().clamp(0.0, 1.0),
            self.width.value().clamp(0.0, MAX_WIDTH),
        )
    }

    fn process_sample(&mut self, left: f32, right: f32) -> (f32, f32) {
        let (k, w) = self.targets();
        self.k.step(k, self.coeff);
        self.w.step(w, self.coeff);
        let (k, w) = (self.k.value(), self.w.value());

        let mid = (left + right) * 0.5;
        // Run on every sample, so the crossover is already settled whenever
        // the karaoke comes in.
        let mid_low = self.low.tick(&self.low_coefs, mid);

        // At rest — no karaoke, natural width — this stage is the identity,
        // and is kept exactly that rather than rebuilt from mid and side.
        if k == 0.0 && w == 1.0 {
            return (left, right);
        }
        let mid = mid - k * (mid - mid_low);
        let side = (left - right) * 0.5 * w;
        (mid + side, mid - side)
    }
}

impl AudioUnit for StereoImage {
    fn reset(&mut self) {
        let (k, w) = self.targets();
        self.k.snap(k);
        self.w.snap(w);
        self.low.clear();
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = (sample_rate as f32).max(1.0);
        self.coeff = glide_coeff(self.sample_rate);
        self.low_coefs = SvfCoefs::lowpass(self.sample_rate, KARAOKE_CROSSOVER_HZ, 0.707);
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
        0x57e2_e010_1a6e_0001
    }

    fn footprint(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    fn image(setup: impl Fn(&AudioParams)) -> StereoImage {
        let params = AudioParams::new();
        setup(&params);
        let mut s = StereoImage::new(&params);
        s.set_sample_rate(48_000.0);
        s
    }

    /// Steady-state level of a tone fed in with the given left/right
    /// amplitudes, as (left, right) output amplitude.
    fn levels(s: &mut StereoImage, hz: f32, l_amp: f32, r_amp: f32) -> (f32, f32) {
        let (mut el, mut er, mut n) = (0.0f64, 0.0f64, 0);
        for i in 0..48_000 {
            let v = (i as f32 / 48_000.0 * hz * TAU).sin();
            let (l, r) = s.process_sample(v * l_amp, v * r_amp);
            if i >= 24_000 {
                el += (l as f64).powi(2);
                er += (r as f64).powi(2);
                n += 1;
            }
        }
        (
            (2.0 * el / n as f64).sqrt() as f32,
            (2.0 * er / n as f64).sqrt() as f32,
        )
    }

    #[test]
    fn at_rest_it_is_exactly_the_identity() {
        let mut s = image(|_| {});
        for i in 0..2_048 {
            let (l, r) = (
                ((i * 31) % 97) as f32 / 97.0 - 0.5,
                ((i * 17) % 89) as f32 / 89.0 - 0.5,
            );
            assert_eq!(s.process_sample(l, r), (l, r));
        }
    }

    #[test]
    fn karaoke_takes_out_the_vocal_and_leaves_the_bass() {
        let mut s = image(|p| p.karaoke.set_value(1.0));
        let (bass, _) = levels(&mut s, 60.0, 0.5, 0.5);
        let (vocal, _) = levels(&mut s, 1_000.0, 0.5, 0.5);
        let (guitar, _) = levels(&mut s, 1_000.0, 0.5, 0.0);
        let db = |x: f32| 20.0 * (x / 0.5).log10();
        assert!(
            db(bass) > -1.5,
            "a centred bass line lost {:.1} dB",
            -db(bass)
        );
        assert!(
            db(vocal) < -30.0,
            "a centred vocal is only {:.1} dB down",
            -db(vocal)
        );
        assert!(
            db(guitar) > -7.0,
            "a hard-left guitar should survive (it is half side): {:.1} dB",
            db(guitar)
        );
    }

    #[test]
    fn width_scales_only_the_side() {
        // Zero width is mono; centred material is untouched at any width.
        let mut mono = image(|p| p.width.set_value(0.0));
        let (l, r) = levels(&mut mono, 440.0, 0.5, 0.0);
        assert!(
            (l - r).abs() < 1e-3 && (l - 0.25).abs() < 1e-3,
            "zero width puts a hard-left tone in the middle at half level, got {l}/{r}"
        );
        let mut wide = image(|p| p.width.set_value(3.0));
        let (l, r) = levels(&mut wide, 440.0, 0.5, 0.5);
        assert!(
            (l - 0.5).abs() < 1e-3 && (r - 0.5).abs() < 1e-3,
            "width must not touch centred material, got {l}/{r}"
        );
    }
}
