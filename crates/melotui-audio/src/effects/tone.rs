//! The tone filters: the high-pass, the bass and treble shelves, the ten-band
//! EQ and the low-pass.
//!
//! These were fundsp filter nodes fed straight from the shared parameters, and
//! between them they got five things wrong.
//!
//! * **Nyquist.** Nothing kept a filter below half the sample rate. The 16 kHz
//!   EQ band sits exactly on Nyquist for a 32 kHz file and the 20 kHz low-pass
//!   is beyond it for anything under 40 kHz, and a filter tuned there fills
//!   with infinities. At 11.025, 22.05, 24 and 32 kHz the graph's output was
//!   NaN — silence — the moment any effect was switched on.
//! * **Steps.** Every setting went straight into the coefficients, so a 1 dB
//!   nudge of an EQ band measured 136 times rougher than the music around it.
//! * **Defaults that were not off.** The high-pass parked at 20 Hz and the
//!   low-pass parked at 20 kHz were still filters, 3 dB down at both ends,
//!   whenever any other effect had the graph running.
//! * **Treble at zero.** fundsp's shelf takes an amplitude gain above zero. At
//!   zero its integrators freeze and the output is whatever they held:
//!   silence, or a DC offset.
//! * **Cost.** The EQ converted all ten gains from decibels with `powf` on
//!   every sample of both channels, whether or not anything had moved.
//!
//! The coefficients are still fundsp's own, so every filter here has exactly
//! the response its fundsp node had; what changed is everything around them.

use fundsp::DEFAULT_SR;
use fundsp::buffer::{BufferMut, BufferRef};
use fundsp::hacker32::*;
use fundsp::signal::SignalFrame;
use fundsp::svf::SvfCoefs;

use super::AudioParams;
use super::dsp::{Glide, NYQUIST_GUARD, SvfState, glide_coeff};

/// Lowest amplitude gain a shelf is given: -20 dB. The bass control already
/// stopped here; treble went down to zero, which is not a gain a shelf can
/// have.
const MIN_SHELF_GAIN: f32 = 0.1;

/// The high-pass control's resting value, where it means "no high-pass".
const HIGHPASS_OFF_HZ: f32 = 20.0;
/// The low-pass control's resting value, where it means "no low-pass".
const LOWPASS_OFF_HZ: f32 = 20_000.0;
/// How close to its resting value a cutoff counts as off. The tolerance
/// `AudioParams::is_all_default` uses, so the graph's own bypass and these
/// filters agree on what off is.
const OFF_TOLERANCE_HZ: f32 = 1.0;
/// Q of the high- and low-pass: Butterworth, maximally flat.
const EDGE_Q: f32 = 0.707;

/// Centre frequencies for the 10-band parametric EQ.
pub const EQ_FREQS: [f32; 10] = [
    32.0, 64.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];
const EQ_Q: f32 = 1.0;

fn bass_gain(setting: f32) -> f32 {
    // Two cascaded shelves, each carrying half the boost in decibels.
    setting.max(MIN_SHELF_GAIN).sqrt()
}

fn treble_gain(setting: f32) -> f32 {
    setting.max(MIN_SHELF_GAIN)
}

fn db_to_gain(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

#[derive(Clone, Copy)]
enum Edge {
    Highpass,
    Lowpass,
}

/// A high- or low-pass whose control's resting value means "off".
///
/// Off is a crossfade to the dry signal rather than a cutoff parked at the
/// edge of hearing, because a filter parked there is still a filter. The
/// cutoff keeps following the control while the filter is out, so it is
/// already in place by the time the filter fades back in.
#[derive(Clone)]
struct EdgeFilter {
    edge: Edge,
    source: Shared,
    cutoff: Glide,
    engaged: Glide,
    coefs: SvfCoefs<f32>,
    state: SvfState,
}

impl EdgeFilter {
    fn new(edge: Edge, source: &Shared) -> Self {
        Self {
            edge,
            source: source.clone(),
            cutoff: Glide::new(0.0),
            engaged: Glide::new(0.0),
            coefs: SvfCoefs::default(),
            state: SvfState::default(),
        }
    }

    fn is_on(&self, setting: f32) -> bool {
        match self.edge {
            Edge::Highpass => setting > HIGHPASS_OFF_HZ + OFF_TOLERANCE_HZ,
            Edge::Lowpass => setting < LOWPASS_OFF_HZ - OFF_TOLERANCE_HZ,
        }
    }

    fn targets(&self, sample_rate: f32) -> (f32, f32) {
        let setting = self.source.value();
        // `max` then `min` rather than `clamp`, which panics if a nonsensical
        // sample rate ever put the ceiling under the floor.
        let hz = setting
            .max(HIGHPASS_OFF_HZ)
            .min(NYQUIST_GUARD * sample_rate);
        (hz, if self.is_on(setting) { 1.0 } else { 0.0 })
    }

    fn tune(&mut self, sample_rate: f32) {
        let hz = self.cutoff.value();
        self.coefs = match self.edge {
            Edge::Highpass => SvfCoefs::highpass(sample_rate, hz, EDGE_Q),
            Edge::Lowpass => SvfCoefs::lowpass(sample_rate, hz, EDGE_Q),
        };
    }

    fn reset(&mut self, sample_rate: f32) {
        let (hz, engaged) = self.targets(sample_rate);
        self.cutoff.snap(hz);
        self.engaged.snap(engaged);
        self.tune(sample_rate);
        self.state.clear();
    }

    fn process(&mut self, x: f32, sample_rate: f32, coeff: f32) -> f32 {
        let (hz, engaged) = self.targets(sample_rate);
        if self.cutoff.step(hz, coeff) {
            self.tune(sample_rate);
        }
        self.engaged.step(engaged, coeff);
        let mix = self.engaged.value();
        if mix == 0.0 {
            // Fully out: the dry signal exactly, and a clean state to fade
            // back in from.
            self.state.clear();
            return x;
        }
        let y = self.state.tick(&self.coefs, x);
        if mix == 1.0 {
            return y;
        }
        x + (y - x) * mix
    }
}

#[derive(Clone, Copy)]
enum Shape {
    LowShelf,
    HighShelf,
    Bell,
}

/// A shelf or bell whose gain comes from a control.
#[derive(Clone)]
struct GainFilter {
    shape: Shape,
    hz: f32,
    q: f32,
    source: Shared,
    /// The control's units into amplitude gain.
    to_gain: fn(f32) -> f32,
    setting: Glide,
    coefs: SvfCoefs<f32>,
    state: SvfState,
    /// Whether this band can exist at the current sample rate. One that
    /// cannot has no audio to act on, and tuning it anyway is how the graph
    /// came to output NaN.
    in_range: bool,
    /// Settled at unity gain, i.e. exactly the identity, and skipped.
    transparent: bool,
}

impl GainFilter {
    fn new(shape: Shape, hz: f32, q: f32, source: &Shared, to_gain: fn(f32) -> f32) -> Self {
        Self {
            shape,
            hz,
            q,
            source: source.clone(),
            to_gain,
            setting: Glide::new(0.0),
            coefs: SvfCoefs::default(),
            state: SvfState::default(),
            in_range: false,
            transparent: true,
        }
    }

    fn tune(&mut self, sample_rate: f32) {
        let gain = (self.to_gain)(self.setting.value());
        self.coefs = match self.shape {
            Shape::LowShelf => SvfCoefs::lowshelf(sample_rate, self.hz, self.q, gain),
            Shape::HighShelf => SvfCoefs::highshelf(sample_rate, self.hz, self.q, gain),
            Shape::Bell => SvfCoefs::bell(sample_rate, self.hz, self.q, gain),
        };
        // At unity every one of these shapes is exactly the identity, so a
        // band at rest there can be skipped outright. That is the common
        // case — the tone controls untouched while some other effect has the
        // graph running — and it costs nothing.
        self.transparent = self.setting.is_settled() && gain == 1.0;
        if self.transparent {
            self.state.clear();
        }
    }

    fn reset(&mut self, sample_rate: f32) {
        self.in_range = self.hz <= NYQUIST_GUARD * sample_rate;
        self.setting.snap(self.source.value());
        self.tune(sample_rate);
        self.state.clear();
    }

    fn process(&mut self, x: f32, sample_rate: f32, coeff: f32) -> f32 {
        if !self.in_range {
            return x;
        }
        if self.setting.step(self.source.value(), coeff) {
            self.tune(sample_rate);
        }
        if self.transparent {
            return x;
        }
        self.state.tick(&self.coefs, x)
    }
}

#[derive(Clone)]
enum Section {
    Edge(EdgeFilter),
    Gain(GainFilter),
}

/// A run of tone filters, as one unit.
#[derive(Clone)]
pub struct ToneFilters {
    sections: Vec<Section>,
    sample_rate: f32,
    coeff: f32,
}

impl ToneFilters {
    /// The filters ahead of the exciter: high-pass, bass, treble.
    pub fn input(params: &AudioParams) -> Self {
        Self::new(vec![
            Section::Edge(EdgeFilter::new(Edge::Highpass, &params.highpass)),
            Section::Gain(GainFilter::new(
                Shape::LowShelf,
                80.0,
                0.707,
                &params.bass,
                bass_gain,
            )),
            Section::Gain(GainFilter::new(
                Shape::LowShelf,
                60.0,
                0.8,
                &params.bass,
                bass_gain,
            )),
            Section::Gain(GainFilter::new(
                Shape::HighShelf,
                3000.0,
                0.7,
                &params.treble,
                treble_gain,
            )),
        ])
    }

    /// The filters after the wobble: the ten EQ bands, then the low-pass.
    pub fn output(params: &AudioParams) -> Self {
        let mut sections: Vec<Section> = EQ_FREQS
            .iter()
            .zip(&params.eq_bands)
            .map(|(&hz, band)| {
                Section::Gain(GainFilter::new(Shape::Bell, hz, EQ_Q, band, db_to_gain))
            })
            .collect();
        sections.push(Section::Edge(EdgeFilter::new(
            Edge::Lowpass,
            &params.lowpass,
        )));
        Self::new(sections)
    }

    fn new(sections: Vec<Section>) -> Self {
        let mut filters = Self {
            sections,
            sample_rate: DEFAULT_SR as f32,
            coeff: glide_coeff(DEFAULT_SR as f32),
        };
        filters.reset();
        filters
    }

    fn process_sample(&mut self, x: f32) -> f32 {
        let (sample_rate, coeff) = (self.sample_rate, self.coeff);
        self.sections
            .iter_mut()
            .fold(x, |x, section| match section {
                Section::Edge(f) => f.process(x, sample_rate, coeff),
                Section::Gain(f) => f.process(x, sample_rate, coeff),
            })
    }
}

impl AudioUnit for ToneFilters {
    fn reset(&mut self) {
        let sample_rate = self.sample_rate;
        for section in &mut self.sections {
            match section {
                Section::Edge(f) => f.reset(sample_rate),
                Section::Gain(f) => f.reset(sample_rate),
            }
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = (sample_rate as f32).max(1.0);
        self.coeff = glide_coeff(self.sample_rate);
        self.reset();
    }

    fn tick(&mut self, input: &[f32], output: &mut [f32]) {
        output[0] = self.process_sample(input[0]);
    }

    fn process(&mut self, size: usize, input: &BufferRef, output: &mut BufferMut) {
        for i in 0..size {
            output.channel_f32_mut(0)[i] = self.process_sample(input.channel_f32(0)[i]);
        }
    }

    fn inputs(&self) -> usize {
        1
    }

    fn outputs(&self) -> usize {
        1
    }

    fn route(&mut self, input: &SignalFrame, _frequency: f64) -> SignalFrame {
        let mut output = SignalFrame::new(1);
        output.set(0, input.at(0).distort(0.0));
        output
    }

    fn get_id(&self) -> u64 {
        0x70ae_f117_e125_0001
    }

    fn footprint(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    const RATES: [f32; 10] = [
        8_000.0, 11_025.0, 16_000.0, 22_050.0, 24_000.0, 32_000.0, 44_100.0, 48_000.0, 96_000.0,
        192_000.0,
    ];

    fn filters(
        which: fn(&AudioParams) -> ToneFilters,
        sample_rate: f32,
        setup: impl Fn(&AudioParams),
    ) -> (ToneFilters, AudioParams) {
        let params = AudioParams::new();
        setup(&params);
        let mut f = which(&params);
        f.set_sample_rate(sample_rate as f64);
        (f, params)
    }

    /// Steady-state gain of `f` at `hz`, in dB.
    fn gain_db(f: &mut ToneFilters, hz: f32, sample_rate: f32) -> f32 {
        let n = Ord::max(sample_rate as usize, 8_000);
        let (mut num, mut den) = (0.0f64, 0.0f64);
        for i in 0..n {
            let x = (i as f32 / sample_rate * hz * TAU).sin();
            let y = f.process_sample(x);
            if i >= n / 2 {
                num += (y as f64).powi(2);
                den += (x as f64).powi(2);
            }
        }
        10.0 * (num / den).log10() as f32
    }

    #[test]
    fn at_rest_both_runs_are_exactly_transparent_at_every_sample_rate() {
        // The defaults mean "off", and off has to mean the dry signal, bit for
        // bit: the parked 20 Hz high-pass and 20 kHz low-pass used to take
        // 3 dB off both ends of the spectrum whenever the graph was running.
        for sr in RATES {
            for which in [ToneFilters::input, ToneFilters::output] {
                let (mut f, _) = filters(which, sr, |_| {});
                for i in 0..4_096 {
                    let x = ((i * 7919) % 2_000) as f32 / 1_000.0 - 1.0;
                    assert_eq!(f.process_sample(x), x, "sr {sr}: sample {i} changed");
                }
            }
        }
    }

    #[test]
    fn every_setting_stays_finite_at_every_sample_rate() {
        // Every control at an extreme, at every rate a file might come in at.
        // A band above Nyquist, or a cutoff beyond it, used to fill the filter
        // with infinities, and one NaN silences the whole graph.
        for sr in RATES {
            let (mut a, _) = filters(ToneFilters::input, sr, |p| {
                p.highpass.set_value(5_000.0);
                p.bass.set_value(5.0);
                p.treble.set_value(5.0);
            });
            let (mut b, _) = filters(ToneFilters::output, sr, |p| {
                p.lowpass.set_value(200.0);
                for band in &p.eq_bands {
                    band.set_value(12.0);
                }
            });
            for i in 0..(sr as usize) {
                let x = (i as f32 / sr * 997.0 * TAU).sin() * 0.5;
                let y = b.process_sample(a.process_sample(x));
                assert!(y.is_finite(), "sr {sr}: sample {i} is {y}");
            }
        }
    }

    #[test]
    fn treble_turned_all_the_way_down_is_a_cut_not_silence() {
        // At zero, fundsp's shelf freezes and outputs whatever its
        // integrators held. Floored at -20 dB it is a deep cut that still
        // leaves the music playing.
        let (mut f, _) = filters(ToneFilters::input, 48_000.0, |p| p.treble.set_value(0.0));
        let low = gain_db(&mut f, 200.0, 48_000.0);
        let high = gain_db(&mut f, 12_000.0, 48_000.0);
        assert!(low.abs() < 1.0, "the bass is untouched: {low:.1} dB");
        assert!(
            (-22.0..-15.0).contains(&high),
            "the top end is cut, not gone: {high:.1} dB"
        );
    }

    #[test]
    fn the_filters_sound_exactly_like_the_fundsp_nodes_they_replaced() {
        // Everything here changed around the filters, not in them: once a
        // setting has arrived, the output has to match the old fundsp chain
        // sample for sample, or the tone of every preset has quietly moved.
        let eq_db = [4.5, -3.0, 2.5, 0.8, -6.0, 1.0, 0.0, 0.4, 0.6, 3.0];
        for sr in [44_100.0f32, 48_000.0, 96_000.0] {
            let (mut input, _) = filters(ToneFilters::input, sr, |p| {
                p.highpass.set_value(120.0);
                p.bass.set_value(3.0);
                p.treble.set_value(0.5);
            });
            let mut old_input = highpass_hz(120.0, 0.707)
                >> lowshelf_hz(80.0, 0.707, 3.0f32.sqrt())
                >> lowshelf_hz(60.0, 0.8, 3.0f32.sqrt())
                >> highshelf_hz(3_000.0, 0.7, 0.5);
            old_input.set_sample_rate(sr as f64);

            let (mut output, _) = filters(ToneFilters::output, sr, |p| {
                for (band, db) in p.eq_bands.iter().zip(eq_db) {
                    band.set_value(db);
                }
                p.lowpass.set_value(6_000.0);
            });
            let mut old_output = Net::wrap(Box::new(pass()));
            for (&hz, db) in EQ_FREQS.iter().zip(eq_db) {
                old_output = old_output >> bell_hz(hz, EQ_Q, db_to_gain(db));
            }
            let mut old_output = old_output >> lowpass_hz(6_000.0, 0.707);
            old_output.set_sample_rate(sr as f64);

            let mut worst = 0.0f32;
            for i in 0..(sr as usize) {
                let x = (i as f32 / sr * 211.0 * TAU).sin() * 0.4
                    + (i as f32 / sr * 3_517.0 * TAU).sin() * 0.2;
                let new = output.process_sample(input.process_sample(x));
                let old = old_output.filter_mono(old_input.filter_mono(x));
                worst = worst.max((new - old).abs());
            }
            assert!(
                worst < 1e-5,
                "sr {sr}: drifted {worst:e} from the fundsp chain"
            );
        }
    }

    #[test]
    fn a_band_beyond_nyquist_does_nothing() {
        // At 22.05 kHz there is no 16 kHz to boost. The band is left out
        // rather than retuned to somewhere it was never meant to act.
        let sr = 22_050.0;
        let (mut f, _) = filters(ToneFilters::output, sr, |p| p.eq_bands[9].set_value(12.0));
        for f_hz in [1_000.0f32, 5_000.0, 9_000.0] {
            let g = gain_db(&mut f, f_hz, sr);
            assert!(g.abs() < 0.05, "{f_hz} Hz moved by {g:.2} dB");
        }
    }
}
