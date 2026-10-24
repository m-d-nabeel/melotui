//! The input trim, kept up to date with the tone controls.

use fundsp::DEFAULT_SR;
use fundsp::buffer::{BufferMut, BufferRef};
use fundsp::hacker32::*;
use fundsp::signal::SignalFrame;

use super::dsp::{Glide, glide_coeff};
use super::{AudioParams, headroom_trim};

/// Gain staging that follows the tone controls while the track plays.
///
/// The trim pulls the signal down by most of whatever the shelves and EQ are
/// about to add, so that a boost re-balances the spectrum instead of pinning
/// the limiter. It used to be worked out once, when the graph was built at the
/// start of each track, so a boost made during a track got no trim until the
/// next one: bass 5.0 set mid-track came out 6.3 dB louder than the same
/// setting made before pressing play, straight into the limiter. Now it is
/// worked out again whenever one of the settings it depends on changes, and
/// glides at the same pace as the filters it compensates.
#[derive(Clone)]
pub struct HeadroomTrim {
    bass: Shared,
    treble: Shared,
    eq: [Shared; 10],
    /// The settings the current target was worked out from.
    settings: [f32; 12],
    target: f32,
    gain: Glide,
    coeff: f32,
}

impl HeadroomTrim {
    pub fn new(params: &AudioParams) -> Self {
        let mut trim = Self {
            bass: params.bass.clone(),
            treble: params.treble.clone(),
            eq: params.eq_bands.clone(),
            settings: [f32::NAN; 12],
            target: 1.0,
            gain: Glide::new(1.0),
            coeff: glide_coeff(DEFAULT_SR as f32),
        };
        trim.reset();
        trim
    }

    fn read(&self) -> [f32; 12] {
        let mut settings = [0.0; 12];
        settings[0] = self.bass.value();
        settings[1] = self.treble.value();
        for (s, band) in settings[2..].iter_mut().zip(&self.eq) {
            *s = band.value();
        }
        settings
    }

    fn update_target(&mut self) {
        let settings = self.read();
        if settings != self.settings {
            self.settings = settings;
            self.target = headroom_trim(settings[0], settings[1], &settings[2..]);
        }
    }

    fn process_sample(&mut self, left: f32, right: f32) -> (f32, f32) {
        self.update_target();
        self.gain.step(self.target, self.coeff);
        let g = self.gain.value();
        (left * g, right * g)
    }
}

impl AudioUnit for HeadroomTrim {
    fn reset(&mut self) {
        self.update_target();
        self.gain.snap(self.target);
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.coeff = glide_coeff(sample_rate as f32);
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
        output.set(0, input.at(0).distort(0.0));
        output.set(1, input.at(1).distort(0.0));
        output
    }

    fn get_id(&self) -> u64 {
        0x7e1a_d200_7e1a_0001
    }

    fn footprint(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}
