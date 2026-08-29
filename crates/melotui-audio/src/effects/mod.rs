pub mod limiter;
pub mod spatializer;

use fundsp::DEFAULT_SR;
use fundsp::hacker32::*;
use fundsp::net::Net;

use limiter::LinkedLimiter;
use spatializer::StereoSpatializer;

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

impl AudioParams {
    /// Largest boost, in dB, that the tone controls and EQ can apply at any one
    /// frequency.
    ///
    /// The shelves and the bell filters are the only stages that add level, so
    /// this is the worst case the rest of the chain has to survive.
    fn max_boost_db(&self) -> f32 {
        let shelf_db = |g: f32| 20.0 * g.max(0.001).log10();
        let mut peak = shelf_db(self.bass.value()).max(shelf_db(self.treble.value()));
        for band in &self.eq_bands {
            peak = peak.max(band.value());
        }
        peak.max(0.0)
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
    pub fn headroom_trim(&self) -> f32 {
        // Empirical. Measured against a -1 dBFS master: at 0.45 the ordinary
        // settings (bass up to 3.0, a single EQ band) stop reaching the limiter
        // at all, while the resulting tone stays within 0.3 dB of what letting
        // the limiter do the work produces. Compensating further only throws
        // away level -- the limiter is transparent enough now to be a safety
        // net rather than a permanent processor.
        const COMPENSATION: f32 = 0.45;
        let boost = self.max_boost_db();
        if boost < 0.01 {
            return 1.0;
        }
        10f32.powf(-boost * COMPENSATION / 20.0)
    }
}

/// Center frequencies for the 10-band parametric EQ.
const EQ_FREQS: [f32; 10] = [
    32.0, 64.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

/// Build the mono DSP chain as a Net (dynamic audio graph).
///
/// Pipeline: highpass → bass_shelf → treble_shelf → exciter → bitcrush → wobble → 10-band EQ → lowpass → echo
fn build_mono_chain(params: &AudioParams) -> Net {
    // high-pass filter (bypassed at 20 Hz)
    let hp_node = (pass() | var(&params.highpass) | constant(0.707)) >> highpass();

    // bass: cascaded shelves, neutral at the default gain of 1.0.
    let bass_gain_1 = var(&params.bass) >> map(|g: &Frame<f32, U1>| g[0].max(0.001).sqrt());
    let bass_gain_2 = var(&params.bass) >> map(|g: &Frame<f32, U1>| g[0].max(0.001).sqrt());
    let bass_node = (pass() | constant(80.0) | constant(0.707) | bass_gain_1)
        >> lowshelf()
        >> (pass() | constant(60.0) | constant(0.8) | bass_gain_2)
        >> lowshelf();

    // treble: high shelf @ 3000Hz
    let treble_node =
        (pass() | constant(3000.0) | constant(0.7) | var(&params.treble)) >> highshelf();

    // sub-bass exciter
    let exciter_shared = params.exciter.clone();
    let exciter_node = (pass() ^ pass())
        >> (pass()
            | ((pass() | constant(120.0) | constant(0.707))
                >> lowpass()
                >> map(move |f: &Frame<f32, U1>| {
                    let e = exciter_shared.value();
                    if e <= 0.001 {
                        0.0
                    } else {
                        (f[0] * (1.0 + 2.0 * e)).tanh() * e * 0.25
                    }
                })))
        >> map(|f: &Frame<f32, U2>| f[0] + f[1]);

    // bitcrusher
    let bc_shared = params.bitcrush.clone();
    let bc_node = map(move |f: &Frame<f32, U1>| {
        let b = bc_shared.value();
        if b <= 0.001 {
            f[0]
        } else {
            let steps = 65536.0 * (1.0 - 0.999 * b);
            (f[0] * steps).round() / steps
        }
    });

    // lo-fi wobble (tape flutter)
    let wobble_shared = params.wobble.clone();
    let wobble_node = (pass()
        ^ ((pass()
            | ((constant(4.0) >> sine()) * (var(&params.wobble) * constant(0.003))
                + constant(0.005)))
            >> tap(0.0, 0.01)))
        >> map(move |f: &Frame<f32, U2>| {
            let w = wobble_shared.value();
            if w <= 0.001 {
                f[0]
            } else {
                f[0] * (1.0 - w * 0.5) + f[1] * (w * 0.5)
            }
        });

    // low-pass filter (bypassed at 20000 Hz)
    let lp_node = (pass() | var(&params.lowpass) | constant(0.707)) >> lowpass();

    // echo: simple feedback delay
    let echo_node = pass() & ((pass() * var(&params.echo)) >> delay(0.5));

    // phaser: sweeping all-pass notches
    let phaser_shared = params.phaser.clone();
    let phaser_unit = phaser(0.7, |t| sin_hz(0.2, t) * 0.5 + 0.5);
    let phaser_node = (pass() ^ (pass() >> phaser_unit))
        >> map(move |f: &Frame<f32, U2>| {
            let p = phaser_shared.value();
            if p <= 0.001 {
                f[0]
            } else {
                f[0] * (1.0 - p * 0.5) + f[1] * (p * 0.5)
            }
        });

    // Build the chain using Net for dynamic composition
    let mut net = Net::new(1, 1);

    // Add all nodes
    let hp_id = net.push(Box::new(hp_node));
    let bass_id = net.push(Box::new(bass_node));
    let treble_id = net.push(Box::new(treble_node));
    let exciter_id = net.push(Box::new(exciter_node));
    let bc_id = net.push(Box::new(bc_node));
    let wobble_id = net.push(Box::new(wobble_node));

    // Connect: input → hp → bass → treble → exciter → bc → wobble
    net.pipe_input(hp_id);
    net.pipe_all(hp_id, bass_id);
    net.pipe_all(bass_id, treble_id);
    net.pipe_all(treble_id, exciter_id);
    net.pipe_all(exciter_id, bc_id);
    net.pipe_all(bc_id, wobble_id);

    // 10-band parametric EQ (bell filters)
    let mut prev_id = wobble_id;
    for (i, &freq) in EQ_FREQS.iter().enumerate() {
        let band_gain = var(&params.eq_bands[i]);
        let gain_linear = band_gain
            >> map(|g: &Frame<f32, U1>| {
                let db = g[0];
                (10.0_f32).powf(db / 20.0)
            });
        let bell_node = (pass() | constant(freq) | constant(1.0) | gain_linear) >> bell();
        let bell_id = net.push(Box::new(bell_node));
        net.pipe_all(prev_id, bell_id);
        prev_id = bell_id;
    }

    // lowpass → echo → phaser → output
    let lp_id = net.push(Box::new(lp_node));
    let echo_id = net.push(Box::new(echo_node));
    let phaser_id = net.push(Box::new(phaser_node));

    net.pipe_all(prev_id, lp_id);
    net.pipe_all(lp_id, echo_id);
    net.pipe_all(echo_id, phaser_id);
    net.pipe_output(phaser_id);

    net
}

pub fn create_audio_graph(channels: u16, params: &AudioParams) -> Box<dyn AudioUnit> {
    // ── Stereo Reverb ────────────────────────────────────
    // Equal-power crossfade. A linear one (dry = 1 - r) drops the direct sound
    // by 3 dB at only 30% wet, which reads as the track going quiet and mushy
    // instead of gaining space; the wet signal is decorrelated, so power, not
    // amplitude, is what adds up.
    let wet_vol = var(&params.reverb) >> map(|r: &Frame<f32, U1>| r[0].clamp(0.0, 1.0).sqrt());
    let wet_stereo = wet_vol.clone() | wet_vol;
    let dry_gain =
        var(&params.reverb) >> map(|r: &Frame<f32, U1>| (1.0 - r[0].clamp(0.0, 1.0)).sqrt());
    let dry_stereo = dry_gain.clone() | dry_gain;

    let stereo_pass = pass() | pass();
    let dry_path = stereo_pass.clone() * dry_stereo;

    let reverb_unit = reverb_stereo(10.0, 3.0, 0.5);
    let wet_path = reverb_unit * wet_stereo;

    let mixer_core = dry_path + wet_path;
    let stereo_mixer = (stereo_pass.clone() ^ stereo_pass.clone()) >> mixer_core;

    let spatializer = || Net::wrap(Box::new(StereoSpatializer::new(params)));

    // ── Mid/Side Matrix (Karaoke & Stereo Widener) ───────
    let make_ms_matrix = || {
        let karaoke_shared = params.karaoke.clone();
        let width_shared = params.width.clone();
        map(move |f: &Frame<f32, U2>| {
            let k = karaoke_shared.value();
            let w = width_shared.value();
            if k <= 0.001 && (w - 1.0).abs() <= 0.001 {
                return *f;
            }
            let l = f[0];
            let r = f[1];
            let mid = (l + r) * 0.5 * (1.0 - k);
            let side = (l - r) * 0.5 * w;
            let mut out = *f;
            out[0] = mid + side;
            out[1] = mid - side;
            out
        })
    };

    // ── Ping-Pong Stereo Delay ───────────────────────────
    let make_ping_pong_mixer = || {
        let pp_gain = var(&params.ping_pong);
        let left_wet = Net::wrap(Box::new(
            (pass() * pp_gain.clone())
                >> ((delay(0.25) * constant(0.7)) & (delay(0.75) * constant(0.3))),
        ));
        let right_wet = Net::wrap(Box::new(
            (pass() * pp_gain) >> ((delay(0.50) * constant(0.5)) & (delay(1.00) * constant(0.15))),
        ));
        let wet_path = left_wet | right_wet;

        let dry_gain =
            var(&params.ping_pong) >> map(|p: &Frame<f32, U1>| (1.0 - p[0].clamp(0.0, 1.0)).sqrt());
        let dry_path = Net::wrap(Box::new((pass() | pass()) * (dry_gain.clone() | dry_gain)));

        let mixer_core = dry_path + wet_path;
        let stereo_pass = pass() | pass();
        (stereo_pass.clone() ^ stereo_pass) >> mixer_core
    };

    // ── Limiter ──────────────────────────────────────────
    // One detector across both channels; see effects::limiter.
    let limiter_stereo = || Net::wrap(Box::new(LinkedLimiter::new(DEFAULT_SR as f32)));

    // ── Input trim ───────────────────────────────────────
    // Gain staging: pull the signal down by most of whatever the tone controls
    // and EQ are about to add, so a boost re-balances the spectrum instead of
    // pinning the limiter and dragging the midrange down with it.
    let trim = params.headroom_trim();
    let trim_stereo = || (constant(trim) * pass()) | (constant(trim) * pass());

    let mono_chain = build_mono_chain(params);

    if channels == 1 {
        Box::new(
            mono_chain
                >> split::<U2>()
                >> trim_stereo()
                >> spatializer()
                >> stereo_mixer
                >> make_ping_pong_mixer()
                >> make_ms_matrix()
                >> limiter_stereo(),
        )
    } else {
        let mono_chain2 = build_mono_chain(params);
        Box::new(
            (mono_chain | mono_chain2)
                >> trim_stereo()
                >> spatializer()
                >> stereo_mixer
                >> make_ping_pong_mixer()
                >> make_ms_matrix()
                >> limiter_stereo(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fundsp::buffer::BufferVec;

    const SR: f32 = 48_000.0;

    fn run(unit: &mut Box<dyn AudioUnit>, input: &[(f32, f32)]) -> Vec<(f32, f32)> {
        let mut inb = BufferVec::new(2);
        let mut outb = BufferVec::new(2);
        let mut out = Vec::with_capacity(input.len());
        for chunk in input.chunks(64) {
            {
                let mut w = inb.buffer_mut();
                for (i, &(l, r)) in chunk.iter().enumerate() {
                    w.channel_f32_mut(0)[i] = l;
                    w.channel_f32_mut(1)[i] = r;
                }
            }
            {
                let r = inb.buffer_ref();
                let mut w = outb.buffer_mut();
                unit.process(chunk.len(), &r, &mut w);
            }
            let r = outb.buffer_ref();
            for i in 0..chunk.len() {
                out.push((r.channel_f32(0)[i], r.channel_f32(1)[i]));
            }
        }
        out
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
                (t * 60.0 * std::f32::consts::TAU).sin() * 0.55
                    + (t * 900.0 * std::f32::consts::TAU).sin() * 0.25
                    + (t * 5_000.0 * std::f32::consts::TAU).sin() * 0.09
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
        let params = AudioParams::new();
        assert_eq!(params.headroom_trim(), 1.0, "no boost, no trim");

        params.eq_bands[4].set_value(12.0);
        let trimmed = params.headroom_trim();
        assert!(
            trimmed < 1.0 && trimmed > 0.1,
            "trim {trimmed} out of range"
        );

        // A cut must not attenuate — there is nothing to make room for.
        let cut = AudioParams::new();
        cut.eq_bands[4].set_value(-12.0);
        assert_eq!(cut.headroom_trim(), 1.0);
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
