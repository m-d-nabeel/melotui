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
        }
    }
}

/// Center frequencies for the 10-band parametric EQ.
const EQ_FREQS: [f32; 10] = [
    32.0, 64.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

/// Build the mono DSP chain as a Net (dynamic audio graph).
///
/// Pipeline: highpass → bass_shelf → treble_shelf → 10-band EQ → lowpass → echo
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

    // Connect: input → hp → bass → treble
    net.pipe_input(hp_id);
    net.pipe_all(hp_id, bass_id);
    net.pipe_all(bass_id, treble_id);

    // 10-band parametric EQ (bell filters)
    let mut prev_id = treble_id;
    for i in 0..10 {
        let freq = EQ_FREQS[i];
        let band_gain = var(&params.eq_bands[i]);
        let gain_linear = band_gain >> map(|g: &Frame<f32, U1>| {
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
    // stereo reverb
    let wet_vol = var(&params.reverb);
    let wet_stereo = wet_vol.clone() | wet_vol;
    let dry_gain = constant(1.0) - var(&params.reverb);
    let dry_stereo = dry_gain.clone() | dry_gain;

    let stereo_pass = pass() | pass();
    // dry path: scale original signal
    let dry_path = stereo_pass.clone() * dry_stereo;

    // reverb unit
    let reverb_unit = reverb_stereo(10.0, 3.0, 0.5);
    let wet_path = reverb_unit * wet_stereo;

    let mixer_core = dry_path + wet_path;
    // mix dry/wet signals
    let stereo_mixer = (stereo_pass.clone() ^ stereo_pass.clone()) >> mixer_core;

    // pan: convert -1..1 to left/right gain
    // pan law: equal power. left = cos((pan+1)*pi/4), right = sin((pan+1)*pi/4)
    let pan_val = var(&params.pan);
    let pan_left = pan_val.clone() >> map(|p: &Frame<f32, U1>| {
        let angle = (p[0] + 1.0) * std::f32::consts::FRAC_PI_4;
        angle.cos()
    });
    let pan_right = pan_val >> map(|p: &Frame<f32, U1>| {
        let angle = (p[0] + 1.0) * std::f32::consts::FRAC_PI_4;
        angle.sin()
    });
    let pan_stereo = pan_left | pan_right;

    // ceiling to prevent clipping
    let limiter_stereo = limiter(0.001, 0.1) | limiter(0.001, 0.1);

    let mono_chain = build_mono_chain(params);

    if channels == 1 {
        Box::new(mono_chain >> split::<U2>() >> (stereo_pass.clone() * pan_stereo) >> stereo_mixer >> limiter_stereo)
    } else {
        let mono_chain2 = build_mono_chain(params);
        Box::new((mono_chain | mono_chain2) >> (stereo_pass.clone() * pan_stereo) >> stereo_mixer >> limiter_stereo)
    }
}
