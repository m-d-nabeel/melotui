//! Parameters as the graph sees them.

use fundsp::DEFAULT_SR;
use fundsp::hacker32::*;
use fundsp::signal::{Signal, SignalFrame};

use super::dsp::{Glide, glide_coeff};

/// A shared parameter as the graph sees it: glided rather than jumped, and
/// mapped into whatever units the node it feeds wants.
///
/// Reading a `Shared` straight into the signal path, as `var` does, turns every
/// change of setting into a step in some gain, and a step is a click: measured
/// through the full graph, a single keypress on the reverb or the karaoke was
/// over a hundred times rougher than the music around it. This glides in the
/// parameter's own units and only works out the mapping while the value is
/// moving, so a setting standing still — which is nearly always — costs one
/// atomic load per sample.
#[derive(Clone)]
pub struct Control<F> {
    source: Shared,
    map: F,
    glide: Glide,
    output: f32,
    coeff: f32,
}

impl<F: Fn(f32) -> f32 + Clone + Send + Sync> AudioNode for Control<F> {
    const ID: u64 = 0x6c0d_7e01_c0a7_1a11;
    type Inputs = U0;
    type Outputs = U1;

    fn reset(&mut self) {
        // A fresh stream has no earlier output to be continuous with.
        self.glide.snap(self.source.value());
        self.output = (self.map)(self.glide.value());
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.coeff = glide_coeff(sample_rate as f32);
    }

    #[inline]
    fn tick(&mut self, _: &Frame<f32, Self::Inputs>) -> Frame<f32, Self::Outputs> {
        if self.glide.step(self.source.value(), self.coeff) {
            self.output = (self.map)(self.glide.value());
        }
        [self.output].into()
    }

    fn route(&mut self, _input: &SignalFrame, _frequency: f64) -> SignalFrame {
        let mut output = SignalFrame::new(self.outputs());
        output.set(0, Signal::Value(self.output as f64));
        output
    }
}

/// `source`, glided and passed through `map`.
pub fn control<F: Fn(f32) -> f32 + Clone + Send + Sync>(source: &Shared, map: F) -> An<Control<F>> {
    let value = source.value();
    An(Control {
        source: source.clone(),
        output: map(value),
        map,
        glide: Glide::new(value),
        coeff: glide_coeff(DEFAULT_SR as f32),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_control_glides_to_a_new_setting_instead_of_jumping() {
        let p = shared(0.0);
        let mut c = control(&p, |x| x * 2.0);
        c.set_sample_rate(48_000.0);
        c.reset();
        assert_eq!(c.get_mono(), 0.0);

        p.set_value(1.0);
        let first = c.get_mono();
        assert!(
            first > 0.0 && first < 0.01,
            "the first sample after a change moved by {first}, which is a step"
        );
        for _ in 0..48_000 {
            c.get_mono();
        }
        assert_eq!(c.get_mono(), 2.0, "arrives exactly, through the mapping");
    }

    #[test]
    fn a_control_starts_where_its_parameter_is() {
        // After a reset there is no earlier output to glide from, and a
        // control that started from zero would open every stream with the
        // effect fading in from nothing.
        let p = shared(0.7);
        let mut c = control(&p, |x| x);
        c.set_sample_rate(48_000.0);
        c.reset();
        assert_eq!(c.get_mono(), 0.7);
    }
}
