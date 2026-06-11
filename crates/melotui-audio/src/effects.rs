use fundsp::hacker32::*;

pub struct AudioParams {
    pub bass: Shared,
    pub treble: Shared,
    pub reverb: Shared,
    pub echo: Shared,
}

impl AudioParams {
    pub fn new() -> Self {
        Self {
            bass: shared(1.0),
            treble: shared(1.0),
            reverb: shared(0.0),
            echo: shared(0.0),
        }
    }
}

pub fn create_audio_graph(channels: u16, params: &AudioParams) -> Box<dyn AudioUnit> {
    let headroom = constant(0.707);

    // bass: low shelf @ 80Hz
    // cascading two filters for steeper slope
    let bass_node = ((pass() * headroom) | constant(80.0) | constant(0.707) | var(&params.bass))
        >> lowshelf()
        >> (pass() | constant(60.0) | constant(0.8) | constant(3.5))
        >> lowshelf();

    // treble: high shelf @ 3000Hz
    let treble_node =
        (pass() | constant(3000.0) | constant(0.7) | var(&params.treble)) >> highshelf();

    // echo: simple feedback delay
    let echo_node = pass() & ((pass() * var(&params.echo)) >> delay(0.5));

    let mono_chain = bass_node >> treble_node >> echo_node;

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
    let stereo_mixer = (stereo_pass.clone() ^ stereo_pass) >> mixer_core;
    // ceiling to prevent clipping
    let limiter_stereo = limiter(0.001, 0.1) | limiter(0.001, 0.1);

    if channels == 1 {
        Box::new(mono_chain >> split() >> stereo_mixer >> limiter_stereo)
    } else {
        let stereo_eq = mono_chain.clone() | mono_chain;
        Box::new(stereo_eq >> stereo_mixer >> limiter_stereo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_params_default() {
        let params = AudioParams::new();
        assert_eq!(params.bass.value(), 1.0);
        assert_eq!(params.treble.value(), 1.0);
        assert_eq!(params.reverb.value(), 0.0);
        assert_eq!(params.echo.value(), 0.0);
    }

    #[test]
    fn test_audio_params_mutation() {
        let params = AudioParams::new();
        params.bass.set_value(2.0);
        params.echo.set_value(0.5);

        assert_eq!(params.bass.value(), 2.0);
        assert_eq!(params.echo.value(), 0.5);
    }

    #[test]
    fn test_create_audio_graph_stereo() {
        let params = AudioParams::new();
        let graph = create_audio_graph(2, &params);

        // Assert we have 2 inputs and 2 outputs (stereo)
        assert_eq!(graph.inputs(), 2);
        assert_eq!(graph.outputs(), 2);
    }

    #[test]
    fn test_create_audio_graph_mono() {
        let params = AudioParams::new();
        let graph = create_audio_graph(1, &params);

        // Mono input, stereo output
        assert_eq!(graph.inputs(), 1);
        assert_eq!(graph.outputs(), 2);
    }
}
