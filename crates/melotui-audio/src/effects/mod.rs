pub mod control;
pub mod dsp;
pub mod gate;
pub mod image;
pub mod limiter;
pub mod spatializer;
pub mod tone;
pub mod trim;

use fundsp::DEFAULT_SR;
use fundsp::hacker32::*;
use fundsp::net::Net;

use control::control;
use gate::OnDemand;
use image::StereoImage;
use limiter::LinkedLimiter;
use spatializer::StereoSpatializer;
use tone::ToneFilters;
use trim::HeadroomTrim;

pub struct AudioParams {
    pub bass: Shared,
    pub treble: Shared,
    pub reverb: Shared,
    pub echo: Shared,
    pub pan: Shared,
    pub lowpass: Shared,
    pub highpass: Shared,
    pub eq_bands: [Shared; 10],
    pub auto_pan_rate: Shared,
    pub auto_pan_depth: Shared,
    pub wobble: Shared,
    pub karaoke: Shared,
    pub width: Shared,
    pub bitcrush: Shared,
    pub exciter: Shared,
    pub ping_pong: Shared,
    pub phaser: Shared,
}

impl AudioParams {
    pub fn new() -> Self {
        Self {
            bass: shared(1.0),
            treble: shared(1.0),
            reverb: shared(0.0),
            echo: shared(0.0),
            pan: shared(0.0),
            lowpass: shared(20000.0),
            highpass: shared(20.0),
            eq_bands: std::array::from_fn(|_| shared(0.0)),
            auto_pan_rate: shared(0.0),
            auto_pan_depth: shared(0.0),
            wobble: shared(0.0),
            karaoke: shared(0.0),
            width: shared(1.0),
            bitcrush: shared(0.0),
            exciter: shared(0.0),
            ping_pong: shared(0.0),
            phaser: shared(0.0),
        }
    }

    pub fn is_all_default(&self) -> bool {
        (self.bass.value() - 1.0).abs() < 0.001
            && (self.treble.value() - 1.0).abs() < 0.001
            && self.reverb.value() < 0.001
            && self.echo.value() < 0.001
            && self.pan.value().abs() < 0.001
            && (self.lowpass.value() - 20000.0).abs() < 1.0
            && (self.highpass.value() - 20.0).abs() < 1.0
            && self.eq_bands.iter().all(|b| b.value().abs() < 0.001)
            && self.auto_pan_rate.value() < 0.001
            && self.auto_pan_depth.value() < 0.001
            && self.wobble.value() < 0.001
            && self.karaoke.value() < 0.001
            && (self.width.value() - 1.0).abs() < 0.001
            && self.bitcrush.value() < 0.001
            && self.exciter.value() < 0.001
            && self.ping_pong.value() < 0.001
            && self.phaser.value() < 0.001
    }
}

/// Largest boost, in dB, that the tone controls and EQ can apply at any one
/// frequency, given their settings.
///
/// The shelves and the bell filters are the only stages that add level, so
/// this is the worst case the rest of the chain has to survive.
fn max_boost_db(bass: f32, treble: f32, eq_db: &[f32]) -> f32 {
    let shelf_db = |g: f32| 20.0 * g.max(0.001).log10();
    let shelves = shelf_db(bass).max(shelf_db(treble));
    eq_db.iter().copied().fold(shelves, f32::max).max(0.0)
}

/// Input attenuation applied ahead of the chain so that boosting a band
/// changes the *balance* between bands rather than simply driving the
/// limiter.
///
/// Without this, asking for +6 dB on every EQ band on a track mastered to
/// -1 dBFS produced +0.6 dB of actual output: the limiter absorbed the rest
/// and paid for it by ducking the midrange. Compensating the full boost
/// would instead make every setting quieter than bypass, so we take most of
/// it and leave the limiter a little work to do.
pub(crate) fn headroom_trim(bass: f32, treble: f32, eq_db: &[f32]) -> f32 {
    // Empirical. Measured against a -1 dBFS master: at 0.45 the ordinary
    // settings (bass up to 3.0, a single EQ band) stop reaching the limiter
    // at all, while the resulting tone stays within 0.3 dB of what letting
    // the limiter do the work produces. Compensating further only throws
    // away level -- the limiter is transparent enough now to be a safety
    // net rather than a permanent processor.
    const COMPENSATION: f32 = 0.45;
    let boost = max_boost_db(bass, treble, eq_db);
    if boost < 0.01 {
        return 1.0;
    }
    10f32.powf(-boost * COMPENSATION / 20.0)
}

/// Ceiling of the exciter control.
const MAX_EXCITER: f32 = 5.0;

/// Wow: the slow pitch drift of a tape running at an uneven speed.
const WOW_HZ: f32 = 0.7;
/// Peak extra delay the wow sweeps through at full depth. About 0.9% of pitch
/// at its fastest: a tape that is plainly warped, but still music.
const WOW_SECONDS: f32 = 0.002;
/// Flutter: the faster, shallower waver on top of the wow.
const FLUTTER_HZ: f32 = 6.0;
const FLUTTER_SECONDS: f32 = 0.000_08;
/// Depth over which the wobble fades in from the dry signal.
///
/// At the bottom of its travel the wobbled signal is all but identical to the
/// dry one — the delay it reads at sweeps from zero — so fading between the
/// two there costs nothing, and above it the output is the wobbled signal
/// alone.
const WOBBLE_FADE_IN: f32 = 0.15;

/// The echo's repeat time.
const ECHO_SECONDS: f32 = 0.5;
/// Each repeat comes back duller than the one before, as a tape echo's do.
/// It is what keeps a run of repeats sounding like echoes rather than a loop.
const ECHO_DAMPING_HZ: f32 = 3_000.0;
/// Share of each repeat fed back into the next, per unit of the echo
/// control. At the low settings the presets use this is a single faint
/// repeat, as it always was; turned up, it is a decaying train.
const ECHO_FEEDBACK: f32 = 0.5;

/// The ping-pong's repeats, as (seconds, gain), alternating left, right,
/// left, right.
const PING_PONG_TAPS: [(f32, f32); 4] = [(0.25, 0.7), (0.50, 0.5), (0.75, 0.3), (1.00, 0.15)];

/// Tape wow and flutter.
///
/// This was a 50/50 blend of the dry signal with a copy run through a delay
/// swept by ±3 ms at 4 Hz, which is not tape: it is a chorus, comb filtering
/// and all, with a vibrato of ±7.5% — over a semitone — at full depth. And
/// fundsp gives every oscillator a pseudorandom starting phase, so the two
/// channels' sweeps ran out of step, and identical left and right input came
/// out up to 0.3 apart.
///
/// Here the wobbled signal replaces the dry one rather than blending with it,
/// the delay it reads sweeps up from zero, and both sweeps start from the
/// same phase in every channel. The control works through its square root,
/// so the bottom of its travel — where the presets sit — is a waver you can
/// hear rather than none at all.
fn wobble(params: &AudioParams) -> An<impl AudioNode<Inputs = U1, Outputs = U1> + use<>> {
    let depth = || control(&params.wobble, |w| w.clamp(0.0, 1.0).sqrt());
    // `sine_phase(0.25)` is a cosine from a fixed starting point, so
    // `1 - cos` sweeps the delay up from zero, identically in every channel.
    let sweep = |hz: f32, seconds: f32| {
        (constant(hz) >> sine_phase(0.25)) >> map(move |c: &Frame<f32, U1>| seconds * (1.0 - c[0]))
    };
    let delay_time = (sweep(WOW_HZ, WOW_SECONDS) + sweep(FLUTTER_HZ, FLUTTER_SECONDS)) * depth();
    let wobbled = (pass() | delay_time) >> tap(0.0, 2.0 * (WOW_SECONDS + FLUTTER_SECONDS));
    (pass() ^ wobbled ^ (sink() >> depth()))
        >> map(|f: &Frame<f32, U3>| {
            let mix = (f[2] / WOBBLE_FADE_IN).min(1.0);
            f[0] + (f[1] - f[0]) * mix
        })
}

/// A tape-style echo: the repeats feed back, each one duller than the last.
///
/// The comment above this used to say "feedback delay", but there was no
/// feedback: it was a single repeat at half a second.
fn echo(params: &AudioParams) -> An<impl AudioNode<Inputs = U1, Outputs = U1> + use<>> {
    let send = control(&params.echo, |e| e.clamp(0.0, 0.9));
    let feedback = control(&params.echo, |e| ECHO_FEEDBACK * e.clamp(0.0, 0.9));
    pass()
        & ((pass() * send)
            >> feedback2(
                delay(ECHO_SECONDS) >> lowpass_hz(ECHO_DAMPING_HZ, 0.6),
                pass() * feedback,
            ))
}

/// The phaser, blended in by its control.
fn phaser_mix(params: &AudioParams) -> An<impl AudioNode<Inputs = U1, Outputs = U1> + use<>> {
    let depth = control(&params.phaser, |p| p.clamp(0.0, 1.0));
    (pass() ^ phaser(0.7, |t| sin_hz(0.2, t) * 0.5 + 0.5) ^ (sink() >> depth))
        >> map(|f: &Frame<f32, U3>| {
            // fundsp's phaser already sums the dry signal with its all-pass
            // chain, so the dry signal has to come down as the phaser comes
            // up: at full depth this is half of each, the classic even mix
            // whose notches go all the way down. Blending the phaser's output
            // against the dry signal, as this did, put the dry signal in
            // twice and left the notches shallow.
            let p = f[2];
            f[0] * (1.0 - p) + f[1] * (0.5 * p)
        })
}

/// A true ping-pong: one send from both channels, its repeats alternating
/// between the sides.
///
/// Each channel used to feed only its own repeats, which bounced a centred
/// source but left anything panned echoing where it already was. And the dry
/// signal was faded out as the ping-pong came up, so at full the music itself
/// was gone and only the echoes were left.
fn ping_pong(params: &AudioParams) -> An<impl AudioNode<Inputs = U2, Outputs = U2> + use<>> {
    let send = control(&params.ping_pong, |p| 0.5 * p.clamp(0.0, 1.0));
    let [(t1, g1), (t2, g2), (t3, g3), (t4, g4)] = PING_PONG_TAPS;
    let left = (delay(t1) * g1) & (delay(t3) * g3);
    let right = (delay(t2) * g2) & (delay(t4) * g4);
    (pass() | pass()) & (((pass() + pass()) * send) >> (left ^ right))
}

/// Build the mono DSP chain.
///
/// Pipeline: highpass → bass → treble → exciter → bitcrush → wobble → 10-band
/// EQ → lowpass → echo → phaser
fn build_mono_chain(params: &AudioParams) -> Net {
    // Sub-bass exciter: a saturated copy of everything under 120 Hz, added
    // back on top.
    let exciter = control(&params.exciter, |e| e.clamp(0.0, MAX_EXCITER));
    let exciter_node = (pass() ^ lowpass_hz(120.0, 0.707) ^ (sink() >> exciter))
        >> map(|f: &Frame<f32, U3>| {
            let e = f[2];
            if e <= 0.0 {
                f[0]
            } else {
                f[0] + (f[1] * (1.0 + 2.0 * e)).tanh() * e * 0.25
            }
        });

    let crush = control(&params.bitcrush, |b| b.clamp(0.0, 1.0));
    let bitcrush_node = (pass() | crush)
        >> map(|f: &Frame<f32, U2>| {
            let b = f[1];
            if b <= 0.001 {
                f[0]
            } else {
                let steps = 65536.0 * (1.0 - 0.999 * b);
                (f[0] * steps).round() / steps
            }
        });

    Net::wrap(Box::new(ToneFilters::input(params)))
        >> exciter_node
        >> bitcrush_node
        >> wobble(params)
        >> Net::wrap(Box::new(ToneFilters::output(params)))
        >> echo(params)
        >> phaser_mix(params)
}

pub fn create_audio_graph(channels: u16, params: &AudioParams) -> Box<dyn AudioUnit> {
    // ── Stereo Reverb ────────────────────────────────────
    // Equal-power crossfade. A linear one (dry = 1 - r) drops the direct sound
    // by 3 dB at only 30% wet, which reads as the track going quiet and mushy
    // instead of gaining space; the wet signal is decorrelated, so power, not
    // amplitude, is what adds up.
    let wet = || control(&params.reverb, |r| r.clamp(0.0, 1.0).sqrt());
    let dry = || control(&params.reverb, |r| (1.0 - r.clamp(0.0, 1.0)).sqrt());
    let dry_path = (pass() | pass()) * (dry() | dry());
    // Run only while the reverb is turned up: at zero its output is
    // multiplied by zero anyway, and it was a third of the graph's cost.
    let reverb = OnDemand::new(reverb_stereo(10.0, 3.0, 0.5), &params.reverb);
    let wet_path = Net::wrap(Box::new(reverb)) * (wet() | wet());
    let stereo_mixer = ((pass() | pass()) ^ (pass() | pass())) >> (dry_path + wet_path);

    // ── Input trim ───────────────────────────────────────
    // Gain staging: pull the signal down by most of whatever the tone controls
    // and EQ are about to add, so a boost re-balances the spectrum instead of
    // pinning the limiter and dragging the midrange down with it.
    let trim = Net::wrap(Box::new(HeadroomTrim::new(params)));

    let spatializer = Net::wrap(Box::new(StereoSpatializer::new(params)));

    // ── Mid/Side: karaoke and stereo width ───────────────
    let stereo_image = Net::wrap(Box::new(StereoImage::new(params)));

    // ── Limiter ──────────────────────────────────────────
    // One detector across both channels; see effects::limiter.
    let limiter = Net::wrap(Box::new(LinkedLimiter::new(DEFAULT_SR as f32)));

    let input = if channels == 1 {
        build_mono_chain(params) >> split::<U2>()
    } else {
        build_mono_chain(params) | build_mono_chain(params)
    };
    Box::new(
        input
            >> trim
            >> spatializer
            >> stereo_mixer
            >> ping_pong(params)
            >> stereo_image
            >> limiter,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use fundsp::buffer::BufferVec;
    use std::f32::consts::TAU;

    const SR: f32 = 48_000.0;

    fn run(unit: &mut Box<dyn AudioUnit>, input: &[(f32, f32)]) -> Vec<(f32, f32)> {
        run_with(unit, input.len(), |_| (), |i| input[i])
    }

    /// Runs `unit` for `n` frames in 64-frame blocks, the way `FundspSource`
    /// does, calling `at(i)` before the block starting at frame `i` and taking
    /// the input for frame `i` from `signal(i)`.
    fn run_with(
        unit: &mut Box<dyn AudioUnit>,
        n: usize,
        mut at: impl FnMut(usize),
        signal: impl Fn(usize) -> (f32, f32),
    ) -> Vec<(f32, f32)> {
        let mut inb = BufferVec::new(2);
        let mut outb = BufferVec::new(2);
        let mut out = Vec::with_capacity(n);
        let mut start = 0;
        while start < n {
            at(start);
            let len = std::cmp::min(64, n - start);
            {
                let mut w = inb.buffer_mut();
                for j in 0..len {
                    let (l, r) = signal(start + j);
                    w.channel_f32_mut(0)[j] = l;
                    w.channel_f32_mut(1)[j] = r;
                }
            }
            {
                let r = inb.buffer_ref();
                let mut w = outb.buffer_mut();
                unit.process(len, &r, &mut w);
            }
            let r = outb.buffer_ref();
            for j in 0..len {
                out.push((r.channel_f32(0)[j], r.channel_f32(1)[j]));
            }
            start += len;
        }
        out
    }

    fn graph(sample_rate: f32, setup: impl Fn(&AudioParams)) -> (Box<dyn AudioUnit>, AudioParams) {
        let params = AudioParams::new();
        setup(&params);
        let mut unit = create_audio_graph(2, &params);
        unit.reset();
        unit.set_sample_rate(sample_rate as f64);
        (unit, params)
    }

    /// Runs the whole graph over `input`, a function of the frame index,
    /// calling `at(i)` before the block starting at frame `i`.
    fn play(
        unit: &mut Box<dyn AudioUnit>,
        n: usize,
        input: impl Fn(usize) -> (f32, f32),
        at: impl FnMut(usize),
    ) -> Vec<(f32, f32)> {
        run_with(unit, n, at, input)
    }

    fn db(x: f32) -> f32 {
        20.0 * x.max(1e-12).log10()
    }

    fn amplitude_at(sig: &[f32], freq: f32) -> f32 {
        let s = &sig[sig.len() / 3..];
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (i, &v) in s.iter().enumerate() {
            let ph = (i as f64) / SR as f64 * freq as f64 * std::f64::consts::TAU;
            re += v as f64 * ph.cos();
            im += v as f64 * ph.sin();
        }
        (2.0 * (re * re + im * im).sqrt() / s.len() as f64) as f32
    }

    /// Bass, midrange and treble tones, normalised to a chosen peak level.
    ///
    /// A third of a second: the analysis window still covers ~14 cycles of the
    /// 60 Hz tone, and the whole graph is slow enough in a debug build that
    /// length matters for how long the suite takes.
    fn probe(peak_dbfs: f32) -> Vec<(f32, f32)> {
        let raw: Vec<f32> = (0..16_384)
            .map(|i| {
                let t = i as f32 / SR;
                (t * 60.0 * TAU).sin() * 0.55
                    + (t * 900.0 * TAU).sin() * 0.25
                    + (t * 5_000.0 * TAU).sin() * 0.09
            })
            .collect();
        let target = 10f32.powf(peak_dbfs / 20.0);
        let peak = raw.iter().fold(0.0f32, |a, &v| a.max(v.abs()));
        raw.iter()
            .map(|&v| (v * target / peak, v * target / peak))
            .collect()
    }

    /// dB difference between the 60 Hz and 900 Hz components of the left output
    /// — i.e. how much bass the chain delivered relative to the midrange.
    fn bass_to_mid_db(params: &AudioParams, peak_dbfs: f32) -> f32 {
        let input = probe(peak_dbfs);
        let mut unit = create_audio_graph(2, params);
        unit.reset();
        unit.set_sample_rate(SR as f64);
        let out = run(&mut unit, &input);
        let l: Vec<f32> = out.iter().map(|x| x.0).collect();
        db(amplitude_at(&l, 60.0)) - db(amplitude_at(&l, 900.0))
    }

    /// Largest second difference in either channel over `range`: a click is a
    /// spike in it that a smooth signal does not have.
    fn roughness(out: &[(f32, f32)], range: std::ops::Range<usize>) -> f32 {
        let d2 = |a: f32, b: f32, c: f32| (c - 2.0 * b + a).abs();
        out[range]
            .windows(3)
            .map(|w| d2(w[0].0, w[1].0, w[2].0).max(d2(w[0].1, w[1].1, w[2].1)))
            .fold(0.0, f32::max)
    }

    #[test]
    fn is_transparent_when_every_control_is_at_its_default() {
        let params = AudioParams::new();
        assert!(params.is_all_default());
        let input = probe(-12.0);
        let mut unit = create_audio_graph(2, &params);
        unit.reset();
        unit.set_sample_rate(SR as f64);
        let out = run(&mut unit, &input);

        let l: Vec<f32> = out.iter().map(|x| x.0).collect();
        let inl: Vec<f32> = input.iter().map(|x| x.0).collect();
        for freq in [60.0f32, 900.0, 5_000.0] {
            let delta = db(amplitude_at(&l, freq)) - db(amplitude_at(&inl, freq));
            assert!(
                delta.abs() < 0.5,
                "{freq} Hz moved by {delta:.2} dB with nothing engaged"
            );
        }
    }

    #[test]
    fn at_its_defaults_the_running_graph_is_a_pure_delay() {
        // The bypass skips the graph when every control is at its default, but
        // the graph runs at its defaults too — whenever any one other control
        // is engaged, every stage it is not using has to be exactly nothing.
        // The parked 20 Hz high-pass and 20 kHz low-pass alone used to take
        // 3 dB off both ends of the spectrum. What is left is the limiter's
        // 5 ms lookahead and nothing else, sample for sample.
        for sr in [44_100.0f32, 48_000.0] {
            let (mut unit, _) = graph(sr, |_| {});
            let latency = unit.latency().unwrap_or(0.0).round() as usize;
            let input = |i: usize| {
                let l = ((i * 7_919) % 10_007) as f32 / 10_007.0 - 0.5;
                let r = ((i * 104_729) % 10_009) as f32 / 10_009.0 - 0.5;
                (l * 0.6, r * 0.6)
            };
            let out = play(&mut unit, 24_000, input, |_| {});
            let worst = (latency..out.len())
                .map(|i| {
                    let (l, r) = input(i - latency);
                    (out[i].0 - l).abs().max((out[i].1 - r).abs())
                })
                .fold(0.0f32, f32::max);
            assert_eq!(latency, (0.005 * sr) as usize, "sr {sr}: the lookahead");
            assert!(worst < 1e-6, "sr {sr}: drifted {worst:e} from the input");
        }
    }

    #[test]
    fn every_sample_rate_survives_every_effect() {
        // At 11.025, 22.05, 24 and 32 kHz the graph's output used to be NaN —
        // silence — the moment any effect was on: the 16 kHz EQ band and the
        // 20 kHz low-pass were at or beyond Nyquist.
        for sr in [
            8_000.0f32, 11_025.0, 16_000.0, 22_050.0, 24_000.0, 32_000.0, 44_100.0, 48_000.0,
            96_000.0, 192_000.0,
        ] {
            for engaged in [false, true] {
                let (mut unit, _) = graph(sr, |p| {
                    p.reverb.set_value(0.01);
                    if engaged {
                        p.bass.set_value(3.0);
                        p.treble.set_value(2.0);
                        p.highpass.set_value(40.0);
                        p.lowpass.set_value(15_000.0);
                        for band in &p.eq_bands {
                            band.set_value(6.0);
                        }
                        p.exciter.set_value(1.0);
                        p.wobble.set_value(0.5);
                        p.echo.set_value(0.4);
                        p.phaser.set_value(0.5);
                        p.ping_pong.set_value(0.3);
                        p.karaoke.set_value(0.3);
                        p.width.set_value(1.5);
                    }
                });
                let tone = |i: usize| {
                    let v = (i as f32 / sr * 300.0 * TAU).sin() * 0.3;
                    (v, v)
                };
                let out = play(&mut unit, sr as usize / 8, tone, |_| {});
                let tail = &out[out.len() / 2..];
                assert!(
                    tail.iter().all(|(l, r)| l.is_finite() && r.is_finite()),
                    "sr {sr}, engaged {engaged}: non-finite output"
                );
                let peak = tail.iter().fold(0.0f32, |a, (l, _)| a.max(l.abs()));
                assert!(peak > 0.05, "sr {sr}, engaged {engaged}: silent ({peak})");
            }
        }
    }

    #[test]
    fn tone_shaping_does_not_depend_on_how_loud_the_track_is() {
        // This is what the input trim buys. Without it the limiter absorbed a
        // level-dependent share of every boost, so the same setting sounded
        // different on a quiet track than on a loud one — and on a loud one it
        // mostly just ducked the midrange.
        let quiet_ref = bass_to_mid_db(&AudioParams::new(), -20.0);
        let loud_ref = bass_to_mid_db(&AudioParams::new(), -1.0);

        for bass in [2.0f32, 3.0, 5.0] {
            let params = AudioParams::new();
            params.bass.set_value(bass);
            let quiet = bass_to_mid_db(&params, -20.0) - quiet_ref;
            let loud = bass_to_mid_db(&params, -1.0) - loud_ref;
            assert!(
                (quiet - loud).abs() < 1.0,
                "bass {bass}: delivered {quiet:.2} dB on a quiet track but \
                 {loud:.2} dB on a loud one"
            );
            assert!(
                quiet > 2.0,
                "bass {bass}: only {quiet:.2} dB of actual boost"
            );
        }
    }

    #[test]
    fn headroom_trim_tracks_the_largest_boost() {
        let flat = [0.0; 10];
        assert_eq!(headroom_trim(1.0, 1.0, &flat), 1.0, "no boost, no trim");

        let mut boosted = flat;
        boosted[4] = 12.0;
        let trimmed = headroom_trim(1.0, 1.0, &boosted);
        assert!(
            trimmed < 1.0 && trimmed > 0.1,
            "trim {trimmed} out of range"
        );

        // A cut must not attenuate — there is nothing to make room for.
        let mut cut = flat;
        cut[4] = -12.0;
        assert_eq!(headroom_trim(1.0, 1.0, &cut), 1.0);
    }

    #[test]
    fn a_boost_made_mid_track_is_trimmed_like_one_made_before_it() {
        // The trim used to be worked out once, when the graph was built at the
        // start of the track, so a boost made while it played got none: bass
        // 5.0 set mid-track came out 6.3 dB louder than the same setting made
        // before pressing play, straight into the limiter.
        let level = |unit: &mut Box<dyn AudioUnit>, change: &dyn Fn()| {
            let tone = |i: usize| {
                let v = (i as f32 / SR * 60.0 * TAU).sin() * 0.1;
                (v, v)
            };
            let out = play(unit, 48_000, tone, |i| {
                if i == 0 {
                    change()
                }
            });
            let l: Vec<f32> = out.iter().map(|x| x.0).collect();
            db(amplitude_at(&l, 60.0))
        };
        let (mut before, _) = graph(SR, |p| p.bass.set_value(5.0));
        let from_the_start = level(&mut before, &|| {});
        let (mut during, p) = graph(SR, |_| {});
        let mid_track = level(&mut during, &|| p.bass.set_value(5.0));
        assert!(
            (from_the_start - mid_track).abs() < 0.3,
            "bass 5.0: {from_the_start:.2} dB set before the track, {mid_track:.2} dB set during it"
        );
    }

    #[test]
    fn changing_any_control_is_a_glide_not_a_click() {
        // Every control scales something in the signal path, so a setting that
        // moves in one step is a gain step unless it is smoothed. Measured
        // before they were, one keypress on the karaoke was 152 times rougher
        // than the music either side of it, the EQ 136, the reverb 103, the
        // wobble 83, the exciter 71. The change lands on a block boundary — the
        // only place the graph can see it — near a peak of the tone, where a
        // step is largest.
        const AT: usize = 12_096;
        // Faded in, because the echoes replay whatever the tone starts with:
        // a tone that began abruptly would have its own first corner come
        // back half a second later and be blamed on the change.
        let tone = |i: usize| {
            let fade = (i as f32 / 4_800.0).min(1.0);
            let v = (i as f32 / SR * 150.0 * TAU).sin() * 0.5 * fade * fade * (3.0 - 2.0 * fade);
            (v, v * 0.7)
        };
        type Setter = fn(&AudioParams);
        let cases: [(&str, Setter, Setter); 16] = [
            (
                "bass",
                |p| p.bass.set_value(1.0001),
                |p| p.bass.set_value(1.1),
            ),
            (
                "treble",
                |p| p.treble.set_value(1.0001),
                |p| p.treble.set_value(1.1),
            ),
            (
                "an EQ band",
                |p| p.eq_bands[2].set_value(0.001),
                |p| p.eq_bands[2].set_value(1.0),
            ),
            (
                "reverb",
                |p| p.reverb.set_value(0.30),
                |p| p.reverb.set_value(0.35),
            ),
            ("reverb, switched on", |_| {}, |p| p.reverb.set_value(0.05)),
            (
                "reverb, switched off",
                |p| p.reverb.set_value(0.05),
                |p| p.reverb.set_value(0.0),
            ),
            (
                "echo",
                |p| p.echo.set_value(0.30),
                |p| p.echo.set_value(0.35),
            ),
            (
                "width",
                |p| p.width.set_value(1.0001),
                |p| p.width.set_value(1.1),
            ),
            (
                "karaoke",
                |p| p.karaoke.set_value(0.5),
                |p| p.karaoke.set_value(0.55),
            ),
            (
                "wobble",
                |p| p.wobble.set_value(0.5),
                |p| p.wobble.set_value(0.55),
            ),
            (
                "phaser",
                |p| p.phaser.set_value(0.5),
                |p| p.phaser.set_value(0.55),
            ),
            (
                "exciter",
                |p| p.exciter.set_value(1.0),
                |p| p.exciter.set_value(1.2),
            ),
            (
                "ping-pong",
                |p| p.ping_pong.set_value(0.3),
                |p| p.ping_pong.set_value(0.35),
            ),
            (
                "bitcrush",
                |p| p.bitcrush.set_value(0.3),
                |p| p.bitcrush.set_value(0.35),
            ),
            (
                "low-pass",
                |p| p.lowpass.set_value(10_000.0),
                |p| p.lowpass.set_value(9_500.0),
            ),
            (
                "high-pass",
                |p| p.highpass.set_value(200.0),
                |p| p.highpass.set_value(250.0),
            ),
        ];
        for (name, setup, change) in cases {
            let (mut unit, p) = graph(SR, setup);
            let mut changed = false;
            let out = play(&mut unit, 36_000, tone, |i| {
                if i == AT {
                    change(&p);
                    changed = true;
                }
            });
            assert!(changed, "{name}: the change never happened");
            let steady = roughness(&out, 6_000..AT).max(roughness(&out, 30_000..36_000));
            let ratio = roughness(&out, AT - 2..AT + 12_000) / steady;
            assert!(
                ratio < 4.0,
                "{name}: {ratio:.1}x rougher across the change than either side of it"
            );
        }
    }

    #[test]
    fn ping_pong_keeps_the_music_and_bounces_it_between_the_sides() {
        // A hard-left source, ping-pong all the way up. The music has to still
        // be there, and the repeats have to cross to the right.
        let (mut unit, _) = graph(SR, |p| p.ping_pong.set_value(1.0));
        let burst = |i: usize| {
            let v = if i < 4_800 {
                (i as f32 / SR * 440.0 * TAU).sin() * 0.3
            } else {
                0.0
            };
            (v, 0.0)
        };
        let out = play(&mut unit, 36_000, burst, |_| {});
        let peak = |range: std::ops::Range<usize>, right: bool| {
            out[range]
                .iter()
                .fold(0.0f32, |a, &(l, r)| a.max(if right { r } else { l }.abs()))
        };
        assert!(
            peak(1_000..4_800, false) > 0.25,
            "the music itself is gone: {:.3}",
            peak(1_000..4_800, false)
        );
        // The first repeat (0.25 s) comes back left, the second (0.5 s) right.
        assert!(
            peak(24_500..28_000, true) > 0.05,
            "a hard-left source never bounced to the right: {:.3}",
            peak(24_500..28_000, true)
        );
    }

    #[test]
    fn the_echo_repeats_each_time_quieter_and_duller() {
        // On the echo alone: a click with plenty of top end, then silence.
        let params = AudioParams::new();
        params.echo.set_value(0.8);
        let mut echo = echo(&params);
        echo.set_sample_rate(SR as f64);
        echo.reset();
        let out: Vec<f32> = (0..3 * SR as usize)
            .map(|i| echo.filter_mono(if i == 0 { 0.5 } else { 0.0 }))
            .collect();
        let half = (ECHO_SECONDS * SR) as usize;
        let window = |k: usize| &out[k * half..(k + 1) * half];
        let energy = |k: usize| window(k).iter().map(|&x| (x as f64).powi(2)).sum::<f64>();
        let brightness = |k: usize| {
            let w = window(k);
            let diff: f64 = w.windows(2).map(|p| ((p[1] - p[0]) as f64).powi(2)).sum();
            diff / energy(k).max(1e-30)
        };
        let (first, second, third) = (energy(1), energy(2), energy(3));
        assert!(
            first > 1e-4 && second > 1e-6 && third > 1e-8,
            "the echo should repeat, not stop after one: {first:e}, {second:e}, {third:e}"
        );
        assert!(
            second < first * 0.5 && third < second * 0.5,
            "each repeat should be quieter: {first:e}, {second:e}, {third:e}"
        );
        assert!(
            brightness(2) < brightness(1),
            "each repeat should be duller than the last"
        );
    }

    #[test]
    fn the_phaser_cuts_deep_notches_at_full_depth() {
        // Blending the phaser's output — which already contains the dry
        // signal — against the dry signal again left its deepest notch at
        // -4.5 dB, which is barely a notch at all. The even mix reaches
        // -15 dB; fundsp's feedback keeps the cancellation from being total.
        //
        // Measured on the phaser alone: its impulse response, taken at a
        // moment when its sweep is quasi-static, read at many frequencies.
        let params = AudioParams::new();
        params.phaser.set_value(1.0);
        let mut ph = phaser_mix(&params);
        ph.set_sample_rate(SR as f64);
        ph.reset();
        for _ in 0..24_000 {
            ph.filter_mono(0.0);
        }
        let ir: Vec<f32> = (0..4_096)
            .map(|i| ph.filter_mono(if i == 0 { 1.0 } else { 0.0 }))
            .collect();
        let gain_db = |f: f32| {
            let (mut re, mut im) = (0.0f64, 0.0f64);
            for (n, &h) in ir.iter().enumerate() {
                let w = n as f64 * f as f64 / SR as f64 * std::f64::consts::TAU;
                re += h as f64 * w.cos();
                im -= h as f64 * w.sin();
            }
            10.0 * (re * re + im * im).max(1e-30).log10() as f32
        };
        let deepest = (0..400)
            .map(|i| gain_db(100.0 * 1.0135f32.powi(i)))
            .fold(f32::MAX, f32::min);
        assert!(
            deepest < -10.0,
            "at full depth the phaser's deepest notch is only {deepest:.1} dB"
        );
    }

    #[test]
    fn the_wobble_moves_both_channels_together() {
        // Identical left and right in, identical left and right out: the two
        // channels' sweeps used to start at pseudorandom phases and drift
        // apart by up to 0.3.
        let (mut unit, _) = graph(SR, |p| p.wobble.set_value(1.0));
        let tone = |i: usize| {
            let v = (i as f32 / SR * 440.0 * TAU).sin() * 0.3;
            (v, v)
        };
        let out = play(&mut unit, 48_000, tone, |_| {});
        let worst = out
            .iter()
            .map(|(l, r)| (l - r).abs())
            .fold(0.0f32, f32::max);
        assert!(worst < 1e-6, "left and right wobbled apart by {worst}");
    }

    /// Largest deviation of a 1 kHz tone's period from 1 ms through the
    /// wobble alone, as a fraction: how far the pitch wavers. Long enough to
    /// take in a whole cycle of the wow.
    fn wobble_pitch_swing(setting: f32) -> f32 {
        let params = AudioParams::new();
        params.wobble.set_value(setting);
        let mut node = wobble(&params);
        node.set_sample_rate(SR as f64);
        node.reset();
        let out: Vec<f32> = (0..(1.8 * SR) as usize)
            .map(|i| node.filter_mono((i as f32 / SR * 1_000.0 * TAU).sin() * 0.3))
            .collect();
        let mut crossings = Vec::new();
        for i in 1..out.len() {
            let (a, b) = (out[i - 1], out[i]);
            if a < 0.0 && b >= 0.0 {
                crossings.push(i as f32 - 1.0 + a / (a - b));
            }
        }
        crossings
            .windows(2)
            .map(|w| ((w[1] - w[0]) / (SR / 1_000.0) - 1.0).abs())
            .fold(0.0, f32::max)
    }

    #[test]
    fn the_wobble_wavers_the_pitch_like_tape_rather_than_a_vibrato() {
        // Full depth used to be a ±7.5% vibrato — over a semitone. Tape wow
        // and flutter at their worst are around one percent.
        let full = wobble_pitch_swing(1.0);
        assert!(
            (0.006..0.02).contains(&full),
            "full wobble swings the pitch by {:.2}%",
            full * 100.0
        );
        // And the bottom of the control is still a waver you can hear: the
        // slowed-and-reverb preset sits at 0.04.
        let gentle = wobble_pitch_swing(0.04);
        assert!(
            (0.0005..0.005).contains(&gentle),
            "wobble 0.04 swings the pitch by {:.3}%",
            gentle * 100.0
        );
    }

    /// Not a pass/fail test — a measuring tool, like the spatialiser's, for
    /// the whole graph.
    ///
    /// Measured on the machine this was written on, at 48 kHz: about 1.2% of
    /// one core with the graph running and every control at rest, 1.6% on the
    /// bass-boosted preset, 2.8% with everything on. Measured the same way,
    /// that is 57%, 38% and 15% less than before the tone filters stopped
    /// working on bands that were doing nothing and the reverb stopped running
    /// while it was off. Run with
    /// `cargo test --release -p melotui-audio -- --ignored --nocapture`.
    #[test]
    #[ignore = "measurement, not a regression guard; timing-dependent"]
    fn bench_graph_cost() {
        type Setter = fn(&AudioParams);
        let cases: [(&str, Setter); 3] = [
            ("at rest", |_| {}),
            ("bass-boosted preset", |p| {
                p.bass.set_value(1.75);
                p.treble.set_value(1.08);
                p.width.set_value(1.12);
                p.highpass.set_value(28.0);
                p.lowpass.set_value(18_000.0);
                p.exciter.set_value(0.45);
                for (band, db) in p
                    .eq_bands
                    .iter()
                    .zip([4.5, 4.0, 2.5, 0.8, 0.0, 0.0, 0.0, 0.4, 0.6, 0.0])
                {
                    band.set_value(db);
                }
            }),
            ("everything on", |p| {
                p.bass.set_value(2.0);
                p.reverb.set_value(0.4);
                p.echo.set_value(0.3);
                p.ping_pong.set_value(0.3);
                p.phaser.set_value(0.5);
                p.wobble.set_value(0.3);
                p.karaoke.set_value(0.3);
                p.width.set_value(1.3);
                p.exciter.set_value(0.5);
                p.bitcrush.set_value(0.1);
                p.auto_pan_depth.set_value(0.9);
                p.auto_pan_rate.set_value(0.1);
                for band in &p.eq_bands {
                    band.set_value(2.0);
                }
            }),
        ];
        let music = |i: usize| {
            let v = (i as f32 / SR * 220.0 * TAU).sin() * 0.3;
            (v, v * 0.8)
        };
        for (name, setup) in cases {
            let (mut unit, _) = graph(SR, setup);
            play(&mut unit, SR as usize, music, |_| {});
            let start = std::time::Instant::now();
            let out = play(&mut unit, 10 * SR as usize, music, |_| {});
            let elapsed = start.elapsed().as_secs_f64();
            std::hint::black_box(out);
            println!(
                "{name:>20}: {:.2}% of one core (realtime budget)",
                elapsed / 10.0 * 100.0
            );
        }
    }

    #[test]
    fn output_stays_below_full_scale_under_everything_at_once() {
        let params = AudioParams::new();
        params.bass.set_value(5.0);
        params.treble.set_value(3.0);
        for b in &params.eq_bands {
            b.set_value(9.0);
        }
        params.reverb.set_value(0.6);
        params.echo.set_value(0.5);
        params.ping_pong.set_value(0.5);
        params.exciter.set_value(1.0);
        params.auto_pan_rate.set_value(1.0);
        params.auto_pan_depth.set_value(1.0);

        let input = probe(-1.0);
        let mut unit = create_audio_graph(2, &params);
        unit.reset();
        unit.set_sample_rate(SR as f64);
        let out = run(&mut unit, &input);

        let peak = out
            .iter()
            .skip(4_000)
            .fold(0.0f32, |a, &(l, r)| a.max(l.abs()).max(r.abs()));
        assert!(peak <= 1.0, "clipped at {peak}");
        assert!(
            out.iter().all(|&(l, r)| l.is_finite() && r.is_finite()),
            "graph produced non-finite output"
        );
    }
}
