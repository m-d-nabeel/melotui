use crate::{AppMsg, AppState, message::audio::AudioMsg, state::audio::AudioState};

pub fn handle_adjust_volume(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    let new_volume = (state.volume() + delta).clamp(0.0, 1.5);
    state.session.audio.volume = new_volume;
    vec![AppMsg::Audio(AudioMsg::DispatchSetVolume(new_volume))]
}

pub fn handle_adjust_speed(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    let new_speed = (state.speed() + delta).clamp(0.5, 2.0);
    state.session.audio.speed = new_speed;
    vec![AppMsg::Audio(AudioMsg::DispatchSetSpeed(new_speed))]
}

pub fn handle_adjust_bass(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    let new_bass = (state.bass() + delta).clamp(0.1, 5.0);
    state.session.audio.bass = new_bass;
    vec![AppMsg::Audio(AudioMsg::DispatchSetBass(new_bass))]
}

pub fn handle_adjust_treble(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    let new_treble = (state.treble() + delta).clamp(0.1, 5.0);
    state.session.audio.treble = new_treble;
    vec![AppMsg::Audio(AudioMsg::DispatchSetTreble(new_treble))]
}

pub fn handle_adjust_reverb(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    let new_reverb = (state.reverb() + delta).clamp(0.0, 1.0);
    state.session.audio.reverb = new_reverb;
    vec![AppMsg::Audio(AudioMsg::DispatchSetReverb(new_reverb))]
}

pub fn handle_adjust_echo(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    let new_echo = (state.echo() + delta).clamp(0.0, 0.9);
    state.session.audio.echo = new_echo;
    vec![AppMsg::Audio(AudioMsg::DispatchSetEcho(new_echo))]
}

pub fn handle_adjust_pan(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    let new_pan = (state.pan() + delta).clamp(-1.0, 1.0);
    state.session.audio.pan = new_pan;
    vec![AppMsg::Audio(AudioMsg::DispatchSetPan(new_pan))]
}

pub fn handle_adjust_lowpass(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    let new_lp = (state.lowpass() + delta).clamp(200.0, 20000.0);
    state.session.audio.lowpass = new_lp;
    vec![AppMsg::Audio(AudioMsg::DispatchSetLowpass(new_lp))]
}

pub fn handle_adjust_highpass(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    let new_hp = (state.highpass() + delta).clamp(20.0, 5000.0);
    state.session.audio.highpass = new_hp;
    vec![AppMsg::Audio(AudioMsg::DispatchSetHighpass(new_hp))]
}

pub fn handle_adjust_eq_band(state: &mut AppState, band: usize, delta: f32) -> Vec<AppMsg> {
    let band = band.min(9);
    let new_gain = (state.eq_band(band) + delta).clamp(-12.0, 12.0);
    state.session.audio.eq_bands[band] = new_gain;
    vec![AppMsg::Audio(AudioMsg::DispatchSetEqBand {
        band,
        gain: new_gain,
    })]
}

pub fn handle_adjust_auto_pan_rate(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    let new = (state.auto_pan_rate() + delta).clamp(0.0, 2.0);
    state.session.audio.auto_pan_rate = new;
    vec![AppMsg::Audio(AudioMsg::DispatchSetAutoPanRate(new))]
}

pub fn handle_adjust_auto_pan_depth(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    let new = (state.auto_pan_depth() + delta).clamp(0.0, 1.0);
    state.session.audio.auto_pan_depth = new;
    vec![AppMsg::Audio(AudioMsg::DispatchSetAutoPanDepth(new))]
}

// Dispatch handlers (intercepted by engine bridge)
pub fn handle_dispatch_set_volume(state: &mut AppState, volume: f32) -> Vec<AppMsg> {
    state.session.audio.volume = volume.clamp(0.0, 1.5);
    vec![]
}

pub fn handle_dispatch_set_speed(_state: &mut AppState, _speed: f32) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_dispatch_set_bass(_state: &mut AppState, _bass: f32) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_dispatch_set_treble(_state: &mut AppState, _treble: f32) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_dispatch_set_reverb(_state: &mut AppState, _reverb: f32) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_dispatch_set_echo(_state: &mut AppState, _echo: f32) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_dispatch_set_pan(_state: &mut AppState, _pan: f32) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_dispatch_set_lowpass(_state: &mut AppState, _lp: f32) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_dispatch_set_highpass(_state: &mut AppState, _hp: f32) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_dispatch_set_eq_band(_state: &mut AppState, _band: usize, _gain: f32) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_dispatch_set_auto_pan_rate(_state: &mut AppState, _rate: f32) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_dispatch_set_auto_pan_depth(_state: &mut AppState, _depth: f32) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_adjust_wobble(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    let new_val = (state.wobble() + delta).clamp(0.0, 1.0);
    state.session.audio.wobble = new_val;
    vec![AppMsg::Audio(AudioMsg::DispatchSetWobble(new_val))]
}

pub fn handle_adjust_karaoke(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    let new_val = (state.karaoke() + delta).clamp(0.0, 1.0);
    state.session.audio.karaoke = new_val;
    vec![AppMsg::Audio(AudioMsg::DispatchSetKaraoke(new_val))]
}

pub fn handle_adjust_width(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    let new_val = (state.width() + delta).clamp(0.0, 3.0);
    state.session.audio.width = new_val;
    vec![AppMsg::Audio(AudioMsg::DispatchSetWidth(new_val))]
}

pub fn handle_adjust_bitcrush(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    let new_val = (state.bitcrush() + delta).clamp(0.0, 1.0);
    state.session.audio.bitcrush = new_val;
    vec![AppMsg::Audio(AudioMsg::DispatchSetBitcrush(new_val))]
}

pub fn handle_adjust_exciter(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    let new_val = (state.exciter() + delta).clamp(0.0, 5.0);
    state.session.audio.exciter = new_val;
    vec![AppMsg::Audio(AudioMsg::DispatchSetExciter(new_val))]
}

pub fn handle_adjust_ping_pong(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    let new_val = (state.ping_pong() + delta).clamp(0.0, 1.0);
    state.session.audio.ping_pong = new_val;
    vec![AppMsg::Audio(AudioMsg::DispatchSetPingPong(new_val))]
}

pub fn handle_adjust_phaser(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    let new_val = (state.phaser() + delta).clamp(0.0, 1.0);
    state.session.audio.phaser = new_val;
    vec![AppMsg::Audio(AudioMsg::DispatchSetPhaser(new_val))]
}

pub fn handle_dispatch_set_wobble(_state: &mut AppState, _val: f32) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_dispatch_set_karaoke(_state: &mut AppState, _val: f32) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_dispatch_set_width(_state: &mut AppState, _val: f32) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_dispatch_set_bitcrush(_state: &mut AppState, _val: f32) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_dispatch_set_exciter(_state: &mut AppState, _val: f32) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_dispatch_set_ping_pong(_state: &mut AppState, _val: f32) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_dispatch_set_phaser(_state: &mut AppState, _val: f32) -> Vec<AppMsg> {
    vec![]
}

// ── Macros ──────────────────────────────────────────────

fn apply_macro_profile(state: &mut AppState, mut audio: AudioState) -> Vec<AppMsg> {
    audio.volume = state.session.audio.volume;
    state.session.audio = audio.clone();

    let mut msgs = vec![
        AppMsg::Audio(AudioMsg::DispatchSetSpeed(audio.speed)),
        AppMsg::Audio(AudioMsg::DispatchSetBass(audio.bass)),
        AppMsg::Audio(AudioMsg::DispatchSetTreble(audio.treble)),
        AppMsg::Audio(AudioMsg::DispatchSetReverb(audio.reverb)),
        AppMsg::Audio(AudioMsg::DispatchSetEcho(audio.echo)),
        AppMsg::Audio(AudioMsg::DispatchSetPan(audio.pan)),
        AppMsg::Audio(AudioMsg::DispatchSetLowpass(audio.lowpass)),
        AppMsg::Audio(AudioMsg::DispatchSetHighpass(audio.highpass)),
        AppMsg::Audio(AudioMsg::DispatchSetAutoPanRate(audio.auto_pan_rate)),
        AppMsg::Audio(AudioMsg::DispatchSetAutoPanDepth(audio.auto_pan_depth)),
        AppMsg::Audio(AudioMsg::DispatchSetWobble(audio.wobble)),
        AppMsg::Audio(AudioMsg::DispatchSetKaraoke(audio.karaoke)),
        AppMsg::Audio(AudioMsg::DispatchSetWidth(audio.width)),
        AppMsg::Audio(AudioMsg::DispatchSetBitcrush(audio.bitcrush)),
        AppMsg::Audio(AudioMsg::DispatchSetExciter(audio.exciter)),
        AppMsg::Audio(AudioMsg::DispatchSetPingPong(audio.ping_pong)),
        AppMsg::Audio(AudioMsg::DispatchSetPhaser(audio.phaser)),
    ];
    for (band, gain) in audio.eq_bands.iter().copied().enumerate() {
        msgs.push(AppMsg::Audio(AudioMsg::DispatchSetEqBand { band, gain }));
    }
    msgs
}

pub fn handle_macro_slowed_reverb(state: &mut AppState) -> Vec<AppMsg> {
    let audio = AudioState {
        speed: 0.82,
        bass: 1.12,
        treble: 0.92,
        reverb: 0.34,
        echo: 0.08,
        width: 1.15,
        lowpass: 9000.0,
        wobble: 0.04,
        ..AudioState::default()
    };
    apply_macro_profile(state, audio)
}

pub fn handle_macro_bass_boosted(state: &mut AppState) -> Vec<AppMsg> {
    let audio = AudioState {
        bass: 1.75,
        treble: 1.08,
        width: 1.12,
        highpass: 28.0,
        lowpass: 18000.0,
        exciter: 0.45,
        eq_bands: [4.5, 4.0, 2.5, 0.8, 0.0, 0.0, 0.0, 0.4, 0.6, 0.0],
        ..AudioState::default()
    };
    apply_macro_profile(state, audio)
}

pub fn handle_macro_spatial(state: &mut AppState) -> Vec<AppMsg> {
    let audio = AudioState {
        auto_pan_rate: 0.10,
        auto_pan_depth: 0.90,
        ping_pong: 0.08,
        width: 1.35,
        reverb: 0.08,
        ..AudioState::default()
    };
    apply_macro_profile(state, audio)
}

pub fn handle_macro_reset_all(state: &mut AppState) -> Vec<AppMsg> {
    let def = crate::state::audio::AudioState::default();
    state.session.audio = def.clone();
    let mut msgs = vec![
        AppMsg::Audio(AudioMsg::DispatchSetVolume(def.volume)),
        AppMsg::Audio(AudioMsg::DispatchSetSpeed(def.speed)),
        AppMsg::Audio(AudioMsg::DispatchSetBass(def.bass)),
        AppMsg::Audio(AudioMsg::DispatchSetTreble(def.treble)),
        AppMsg::Audio(AudioMsg::DispatchSetReverb(def.reverb)),
        AppMsg::Audio(AudioMsg::DispatchSetEcho(def.echo)),
        AppMsg::Audio(AudioMsg::DispatchSetPan(def.pan)),
        AppMsg::Audio(AudioMsg::DispatchSetLowpass(def.lowpass)),
        AppMsg::Audio(AudioMsg::DispatchSetHighpass(def.highpass)),
        AppMsg::Audio(AudioMsg::DispatchSetAutoPanRate(def.auto_pan_rate)),
        AppMsg::Audio(AudioMsg::DispatchSetAutoPanDepth(def.auto_pan_depth)),
        AppMsg::Audio(AudioMsg::DispatchSetWobble(def.wobble)),
        AppMsg::Audio(AudioMsg::DispatchSetKaraoke(def.karaoke)),
        AppMsg::Audio(AudioMsg::DispatchSetWidth(def.width)),
        AppMsg::Audio(AudioMsg::DispatchSetBitcrush(def.bitcrush)),
        AppMsg::Audio(AudioMsg::DispatchSetExciter(def.exciter)),
        AppMsg::Audio(AudioMsg::DispatchSetPingPong(def.ping_pong)),
        AppMsg::Audio(AudioMsg::DispatchSetPhaser(def.phaser)),
    ];
    for i in 0..10 {
        msgs.push(AppMsg::Audio(AudioMsg::DispatchSetEqBand {
            band: i,
            gain: 0.0,
        }));
    }
    msgs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::AppConfig, library::snapshot::Library};
    use std::collections::HashMap;

    fn test_state() -> AppState {
        AppState::new(
            Library::new(vec![], vec![], HashMap::new()),
            &AppConfig::default(),
        )
    }

    #[test]
    fn slowed_reverb_macro_applies_complete_profile() {
        let mut state = test_state();
        state.session.audio.auto_pan_depth = 1.0;
        state.session.audio.ping_pong = 1.0;
        state.session.audio.bitcrush = 1.0;

        let msgs = handle_macro_slowed_reverb(&mut state);

        assert_eq!(state.session.audio.speed, 0.82);
        assert_eq!(state.session.audio.reverb, 0.34);
        assert_eq!(state.session.audio.echo, 0.08);
        assert_eq!(state.session.audio.lowpass, 9000.0);
        assert_eq!(state.session.audio.auto_pan_depth, 0.0);
        assert_eq!(state.session.audio.ping_pong, 0.0);
        assert_eq!(state.session.audio.bitcrush, 0.0);
        assert!(matches!(
            msgs.iter()
                .find(|msg| matches!(msg, AppMsg::Audio(AudioMsg::DispatchSetAutoPanDepth(_)))),
            Some(AppMsg::Audio(AudioMsg::DispatchSetAutoPanDepth(0.0)))
        ));
    }

    #[test]
    fn bass_boosted_macro_replaces_nightcore_slot() {
        let mut state = test_state();
        state.session.audio.speed = 1.25;
        state.session.audio.reverb = 0.5;
        state.session.audio.auto_pan_depth = 0.9;

        let msgs = handle_macro_bass_boosted(&mut state);

        assert_eq!(state.session.audio.speed, 1.0);
        assert_eq!(state.session.audio.bass, 1.75);
        assert_eq!(state.session.audio.treble, 1.08);
        assert_eq!(state.session.audio.reverb, 0.0);
        assert_eq!(state.session.audio.auto_pan_depth, 0.0);
        assert_eq!(state.session.audio.eq_bands[0], 4.5);
        assert_eq!(state.session.audio.eq_bands[1], 4.0);
        assert!(matches!(
            msgs.iter().find(|msg| matches!(
                msg,
                AppMsg::Audio(AudioMsg::DispatchSetEqBand { band: 0, .. })
            )),
            Some(AppMsg::Audio(AudioMsg::DispatchSetEqBand {
                band: 0,
                gain: 4.5
            }))
        ));
    }

    #[test]
    fn spatial_macro_is_clean_8d_profile() {
        let mut state = test_state();
        state.session.audio.bass = 2.0;
        state.session.audio.eq_bands = [3.0; 10];
        state.session.audio.wobble = 0.5;

        let msgs = handle_macro_spatial(&mut state);

        assert_eq!(state.session.audio.bass, 1.0);
        assert_eq!(state.session.audio.eq_bands, [0.0; 10]);
        assert_eq!(state.session.audio.wobble, 0.0);
        assert_eq!(state.session.audio.auto_pan_rate, 0.10);
        assert_eq!(state.session.audio.auto_pan_depth, 0.90);
        assert_eq!(state.session.audio.width, 1.35);
        assert_eq!(state.session.audio.reverb, 0.08);
        assert!(matches!(
            msgs.iter()
                .find(|msg| matches!(msg, AppMsg::Audio(AudioMsg::DispatchSetAutoPanRate(_)))),
            Some(AppMsg::Audio(AudioMsg::DispatchSetAutoPanRate(0.10)))
        ));
    }

    #[test]
    fn treble_bottoms_out_at_a_deep_cut_rather_than_zero() {
        // Treble is an amplitude gain on a shelf, and zero is not one a shelf
        // can apply: the DSP froze there and played silence. It now stops at
        // 0.1, -20 dB, where bass already did.
        let mut state = test_state();
        for _ in 0..30 {
            handle_adjust_treble(&mut state, -0.1);
        }
        assert!(
            (state.session.audio.treble - 0.1).abs() < 1e-6,
            "treble went down to {}",
            state.session.audio.treble
        );
    }
}
