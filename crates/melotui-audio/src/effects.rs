use fundsp::hacker32::*;
use fundsp::net::Net;

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
        }
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
    let headroom = constant(0.707);

    // high-pass filter (bypassed at 20 Hz)
    let hp_node = (pass() | var(&params.highpass) | constant(0.707)) >> highpass();

    // bass: low shelf @ 80Hz (cascaded for steeper slope)
    let bass_node = ((pass() * headroom) | constant(80.0) | constant(0.707) | var(&params.bass))
        >> lowshelf()
        >> (pass() | constant(60.0) | constant(0.8) | constant(3.5))
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

    // lowpass → echo → output
    let lp_id = net.push(Box::new(lp_node));
    let echo_id = net.push(Box::new(echo_node));

    net.pipe_all(prev_id, lp_id);
    net.pipe_all(lp_id, echo_id);
    net.pipe_output(echo_id);

    net
}

pub fn create_audio_graph(channels: u16, params: &AudioParams) -> Box<dyn AudioUnit> {
    // ── Stereo Reverb ────────────────────────────────────
    let wet_vol = var(&params.reverb);
    let wet_stereo = wet_vol.clone() | wet_vol;
    let dry_gain = constant(1.0) - var(&params.reverb);
    let dry_stereo = dry_gain.clone() | dry_gain;

    let stereo_pass = pass() | pass();
    let dry_path = stereo_pass.clone() * dry_stereo;

    let reverb_unit = reverb_stereo(10.0, 3.0, 0.5);
    let wet_path = reverb_unit * wet_stereo;

    let mixer_core = dry_path + wet_path;
    let stereo_mixer = (stereo_pass.clone() ^ stereo_pass.clone()) >> mixer_core;

    // ── True Stereo Equal-Power Pan + Auto-Pan LFO ───────
    let pan_static = var(&params.pan);
    let lfo = (var(&params.auto_pan_rate) >> sine()) * var(&params.auto_pan_depth);
    let combined_pan = (pan_static + lfo) >> map(|p: &Frame<f32, U1>| p[0].clamp(-1.0, 1.0));

    let cos_map = map(|p: &Frame<f32, U1>| {
        let angle = (p[0] + 1.0) * std::f32::consts::FRAC_PI_4;
        angle.cos()
    });
    let sin_map = map(|p: &Frame<f32, U1>| {
        let angle = (p[0] + 1.0) * std::f32::consts::FRAC_PI_4;
        angle.sin()
    });

    let pan_stereo = combined_pan >> (cos_map ^ sin_map);

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

    // ── Limiter ──────────────────────────────────────────
    let limiter_stereo = limiter(0.001, 0.1) | limiter(0.001, 0.1);

    let mono_chain = build_mono_chain(params);

    if channels == 1 {
        Box::new(
            mono_chain
                >> split::<U2>()
                >> (stereo_pass.clone() * pan_stereo)
                >> stereo_mixer
                >> make_ms_matrix()
                >> limiter_stereo.clone(),
        )
    } else {
        let mono_chain2 = build_mono_chain(params);
        Box::new(
            (mono_chain | mono_chain2)
                >> (stereo_pass.clone() * pan_stereo)
                >> stereo_mixer
                >> make_ms_matrix()
                >> limiter_stereo,
        )
    }
}
