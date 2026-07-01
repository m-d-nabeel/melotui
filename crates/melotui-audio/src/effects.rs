use fundsp::hacker32::*;
use fundsp::net::Net;
use fundsp::{
    DEFAULT_SR,
    buffer::{BufferMut, BufferRef},
    signal::SignalFrame,
};
use std::f32::consts::TAU;

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

/// Center frequencies for the 10-band parametric EQ.
const EQ_FREQS: [f32; 10] = [
    32.0, 64.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

#[derive(Clone)]
struct StereoSpatializer {
    pan: Shared,
    auto_pan_rate: Shared,
    auto_pan_depth: Shared,
    sample_rate: f32,
    phase: f32,
    smoothed_pan: f32,
    delay_buffer: Vec<f32>,
    write_idx: usize,
    shadow_l: f32,
    shadow_r: f32,
    rear_l: f32,
    rear_r: f32,
}

impl StereoSpatializer {
    const MAX_ITD_SECONDS: f32 = 0.000_65;
    const SHADOW_CUTOFF_HZ: f32 = 4_200.0;

    fn new(params: &AudioParams) -> Self {
        let mut spatializer = Self {
            pan: params.pan.clone(),
            auto_pan_rate: params.auto_pan_rate.clone(),
            auto_pan_depth: params.auto_pan_depth.clone(),
            sample_rate: DEFAULT_SR as f32,
            phase: 0.0,
            smoothed_pan: 0.0,
            delay_buffer: Vec::new(),
            write_idx: 0,
            shadow_l: 0.0,
            shadow_r: 0.0,
            rear_l: 0.0,
            rear_r: 0.0,
        };
        spatializer.resize_delay_buffer();
        spatializer
    }

    fn resize_delay_buffer(&mut self) {
        let len = (self.sample_rate * Self::MAX_ITD_SECONDS).ceil() as usize + 4;
        self.delay_buffer.resize(std::cmp::max(len, 4), 0.0);
        self.write_idx = std::cmp::min(self.write_idx, self.delay_buffer.len() - 1);
    }

    fn balance_gains(pan: f32) -> (f32, f32) {
        if pan < 0.0 {
            (1.0, (1.0 + pan).max(0.0).sqrt())
        } else {
            ((1.0 - pan).max(0.0).sqrt(), 1.0)
        }
    }

    fn read_delay(&self, delay_samples: f32) -> f32 {
        let len = self.delay_buffer.len() as f32;
        let mut pos = self.write_idx as f32 - delay_samples.clamp(0.0, len - 2.0);
        while pos < 0.0 {
            pos += len;
        }

        let i0 = pos.floor() as usize % self.delay_buffer.len();
        let i1 = (i0 + 1) % self.delay_buffer.len();
        let frac = pos - pos.floor();
        self.delay_buffer[i0] * (1.0 - frac) + self.delay_buffer[i1] * frac
    }

    fn shadow_sample(state: &mut f32, sample: f32, amount: f32, sample_rate: f32) -> f32 {
        let alpha = 1.0 - (-TAU * Self::SHADOW_CUTOFF_HZ / sample_rate).exp();
        *state += alpha * (sample - *state);
        sample * (1.0 - amount) + *state * amount
    }

    fn process_sample(&mut self, left: f32, right: f32) -> (f32, f32) {
        let rate = self.auto_pan_rate.value().clamp(0.0, 2.0);
        let depth = self.auto_pan_depth.value().clamp(0.0, 1.0);
        let orbit_phase = self.phase * TAU;
        let auto = if depth > 0.001 && rate > 0.001 {
            self.phase = (self.phase + rate / self.sample_rate).fract();
            (self.phase * TAU).sin() * depth
        } else {
            0.0
        };
        let rear_orbit = if depth > 0.001 && rate > 0.001 {
            // The left/right pan is sine-shaped. Cosine gives the two center crossings
            // different front/back character so the orbit does not flatten behind the head.
            ((orbit_phase.cos() + 1.0) * 0.5) * depth
        } else {
            0.0
        };

        let target_pan = (self.pan.value() + auto).clamp(-1.0, 1.0);
        self.smoothed_pan += (target_pan - self.smoothed_pan) * 0.02;
        let pan = self.smoothed_pan;
        let lateral = pan.abs();
        let (left_gain, right_gain) = Self::balance_gains(pan);

        let dry_l = left * left_gain;
        let dry_r = right * right_gain;
        let auto_active = depth > 0.001 && (rate > 0.001 || self.pan.value().abs() > 0.001);
        if !auto_active {
            return (dry_l, dry_r);
        }

        let mono = (left + right) * 0.5;
        self.delay_buffer[self.write_idx] = mono;

        let max_delay = self.sample_rate * Self::MAX_ITD_SECONDS * depth;
        let left_delay = pan.max(0.0) * max_delay;
        let right_delay = (-pan).max(0.0) * max_delay;
        let mut spatial_l = self.read_delay(left_delay);
        let mut spatial_r = self.read_delay(right_delay);

        let shadow = 0.45 * lateral * depth;
        spatial_l = Self::shadow_sample(
            &mut self.shadow_l,
            spatial_l,
            pan.max(0.0) * shadow,
            self.sample_rate,
        );
        spatial_r = Self::shadow_sample(
            &mut self.shadow_r,
            spatial_r,
            (-pan).max(0.0) * shadow,
            self.sample_rate,
        );
        spatial_l *= left_gain;
        spatial_r *= right_gain;

        self.write_idx = (self.write_idx + 1) % self.delay_buffer.len();

        let spatial_mix = (0.25 + depth * 0.55).min(0.8);
        let dry_mix = 1.0 - spatial_mix;
        let makeup = 1.0 + spatial_mix * 0.08;
        let mut out_l = (dry_l * dry_mix + spatial_l * spatial_mix) * makeup;
        let mut out_r = (dry_r * dry_mix + spatial_r * spatial_mix) * makeup;

        let rear_amount = rear_orbit * (1.0 - lateral).powi(2) * 0.38;
        if rear_amount > 0.001 {
            let rear_delay = max_delay * (0.45 + 0.25 * rear_orbit);
            let rear_mono = self.read_delay(rear_delay);
            let rear_left = Self::shadow_sample(
                &mut self.rear_l,
                rear_mono + right * 0.18,
                rear_amount * 0.75,
                self.sample_rate,
            );
            let rear_right = Self::shadow_sample(
                &mut self.rear_r,
                rear_mono + left * 0.18,
                rear_amount * 0.75,
                self.sample_rate,
            );

            out_l = out_l * (1.0 - rear_amount) + rear_left * rear_amount;
            out_r = out_r * (1.0 - rear_amount) + rear_right * rear_amount;
        }

        (out_l, out_r)
    }
}

impl AudioUnit for StereoSpatializer {
    fn reset(&mut self) {
        self.phase = 0.0;
        self.smoothed_pan = self.pan.value().clamp(-1.0, 1.0);
        self.delay_buffer.fill(0.0);
        self.write_idx = 0;
        self.shadow_l = 0.0;
        self.shadow_r = 0.0;
        self.rear_l = 0.0;
        self.rear_r = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = (sample_rate as f32).max(1.0);
        self.resize_delay_buffer();
        self.reset();
    }

    fn tick(&mut self, input: &[f32], output: &mut [f32]) {
        let (left, right) = self.process_sample(input[0], input[1]);
        output[0] = left;
        output[1] = right;
    }

    fn process(&mut self, size: usize, input: &BufferRef, output: &mut BufferMut) {
        for i in 0..size {
            let (left, right) =
                self.process_sample(input.channel_f32(0)[i], input.channel_f32(1)[i]);
            output.channel_f32_mut(0)[i] = left;
            output.channel_f32_mut(1)[i] = right;
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

        let dry_gain = constant(1.0) - var(&params.ping_pong);
        let dry_path = Net::wrap(Box::new((pass() | pass()) * (dry_gain.clone() | dry_gain)));

        let mixer_core = dry_path + wet_path;
        let stereo_pass = pass() | pass();
        (stereo_pass.clone() ^ stereo_pass) >> mixer_core
    };

    // ── Limiter ──────────────────────────────────────────
    let limiter_stereo = limiter(0.001, 0.1) | limiter(0.001, 0.1);

    let mono_chain = build_mono_chain(params);

    if channels == 1 {
        Box::new(
            mono_chain
                >> split::<U2>()
                >> spatializer()
                >> stereo_mixer
                >> make_ping_pong_mixer()
                >> make_ms_matrix()
                >> limiter_stereo.clone(),
        )
    } else {
        let mono_chain2 = build_mono_chain(params);
        Box::new(
            (mono_chain | mono_chain2)
                >> spatializer()
                >> stereo_mixer
                >> make_ping_pong_mixer()
                >> make_ms_matrix()
                >> limiter_stereo,
        )
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

    #[test]
    fn spatializer_default_preserves_stereo_sample() {
        let params = AudioParams::new();
        let mut spatializer = StereoSpatializer::new(&params);
        spatializer.reset();

        let mut output = [0.0, 0.0];
        spatializer.tick(&[0.8, -0.2], &mut output);

        assert_close(output[0], 0.8);
        assert_close(output[1], -0.2);
    }

    #[test]
    fn spatializer_center_pan_does_not_attenuate() {
        let params = AudioParams::new();
        params.pan.set_value(0.0);
        let mut spatializer = StereoSpatializer::new(&params);
        spatializer.reset();

        let mut output = [0.0, 0.0];
        spatializer.tick(&[0.5, 0.5], &mut output);

        assert_close(output[0], 0.5);
        assert_close(output[1], 0.5);
    }

    #[test]
    fn spatializer_hard_pan_keeps_near_channel_level() {
        let params = AudioParams::new();
        params.pan.set_value(1.0);
        let mut spatializer = StereoSpatializer::new(&params);
        spatializer.reset();

        let mut output = [0.0, 0.0];
        spatializer.tick(&[0.5, 0.5], &mut output);

        assert!(output[0].abs() < 0.0001);
        assert_close(output[1], 0.5);
    }

    #[test]
    fn spatializer_auto_pan_adds_rear_center_cue() {
        let params = AudioParams::new();
        params.auto_pan_rate.set_value(0.10);
        params.auto_pan_depth.set_value(0.90);
        let mut spatializer = StereoSpatializer::new(&params);
        spatializer.reset();

        let mut output = [0.0, 0.0];
        spatializer.tick(&[0.5, 0.5], &mut output);

        assert!(output[0].is_finite());
        assert!(output[1].is_finite());
        assert!(
            (output[0] - 0.5).abs() > 0.001 || (output[1] - 0.5).abs() > 0.001,
            "rear cue should alter the otherwise unchanged center image"
        );
    }
}
