use crate::state::audio::Effect;
use crate::{AppMsg, AppState, message::audio::AudioMsg, state::audio::AudioState};

/// Says what a change came to, unless the mixer, which shows it, is on
/// screen: in the DSP studio, or in the full layout with the lyrics panel
/// not in its place.
fn show_unless_mixer_shown(state: &mut AppState, message: String) {
    if !(state.ui_show_dsp_studio() || state.mixer_on_screen()) {
        state.session.notification.current = Some(
            crate::state::notification::Notification::new(
                crate::state::notification::NotificationKind::Info,
                message,
            )
            .with_duration(2),
        );
    }
}

pub fn handle_adjust_volume(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    // Turned up or down, it is no longer muted.
    state.session.audio.muted_from = None;
    let new_volume = (state.volume() + delta).clamp(0.0, 1.5);
    state.session.audio.volume = new_volume;
    show_unless_mixer_shown(state, format!("Volume {:.0}%", new_volume * 100.0));
    vec![AppMsg::Audio(AudioMsg::DispatchSetVolume(new_volume))]
}

pub fn handle_toggle_mute(state: &mut AppState) -> Vec<AppMsg> {
    let audio = &mut state.session.audio;
    let message = match audio.muted_from.take() {
        Some(volume) => {
            audio.volume = volume;
            format!("Unmuted: {:.0}%", volume * 100.0)
        }
        None => {
            audio.muted_from = Some(audio.volume);
            audio.volume = 0.0;
            "Muted".to_string()
        }
    };
    let volume = audio.volume;
    state.session.notification.current = Some(
        crate::state::notification::Notification::new(
            crate::state::notification::NotificationKind::Info,
            message,
        )
        .with_duration(2),
    );
    vec![AppMsg::Audio(AudioMsg::DispatchSetVolume(volume))]
}

pub fn handle_adjust_speed(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    let new_speed = (state.speed() + delta).clamp(0.5, 2.0);
    set_speed(state, new_speed)
}

pub fn handle_reset_speed(state: &mut AppState) -> Vec<AppMsg> {
    set_speed(state, 1.0)
}

fn set_speed(state: &mut AppState, speed: f32) -> Vec<AppMsg> {
    state.session.audio.speed = speed;
    show_unless_mixer_shown(state, format!("Speed {speed:.2}×"));
    vec![AppMsg::Audio(AudioMsg::DispatchSetSpeed(speed))]
}

/// Turns `effect` up or down by `delta`, within its range.
pub fn handle_adjust_effect(
    state: &mut AppState,
    effect: crate::state::audio::Effect,
    delta: f32,
) -> Vec<AppMsg> {
    let value = effect.get(&state.session.audio) + delta;
    let msg = effect.set(&mut state.session.audio, value);
    let value = effect.get(&state.session.audio);
    // The ones with keys of their own say what they came to; the rest are
    // only ever changed in the mixer or the studio, which show them.
    let said = match effect {
        Effect::Bass => Some(format!("Bass ×{value:.1}")),
        Effect::Treble => Some(format!("Treble ×{value:.1}")),
        Effect::Reverb => Some(format!("Reverb {:.0}%", value * 100.0)),
        Effect::Echo => Some(format!("Echo {:.0}%", value * 100.0)),
        _ => None,
    };
    if let Some(said) = said {
        show_unless_mixer_shown(state, said);
    }
    vec![AppMsg::Audio(msg)]
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

// Dispatch handlers (intercepted by engine bridge)
pub fn handle_dispatch_set_volume(state: &mut AppState, volume: f32) -> Vec<AppMsg> {
    state.session.audio.volume = volume.clamp(0.0, 1.5);
    vec![]
}

pub fn handle_dispatch_set_speed(_state: &mut AppState, _speed: f32) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_dispatch_set_eq_band(_state: &mut AppState, _band: usize, _gain: f32) -> Vec<AppMsg> {
    vec![]
}

// ── Macros ──────────────────────────────────────────────

fn apply_macro_profile(state: &mut AppState, mut audio: AudioState) -> Vec<AppMsg> {
    audio.volume = state.session.audio.volume;
    audio.muted_from = state.session.audio.muted_from;
    state.session.audio = audio.clone();

    let mut msgs = vec![
        AppMsg::Audio(AudioMsg::DispatchSetSpeed(audio.speed)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Bass, audio.bass)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Treble, audio.treble)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Reverb, audio.reverb)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Echo, audio.echo)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Pan, audio.pan)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Lowpass, audio.lowpass)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Highpass, audio.highpass)),
        AppMsg::Audio(AudioMsg::DispatchSet(
            Effect::AutoPanRate,
            audio.auto_pan_rate,
        )),
        AppMsg::Audio(AudioMsg::DispatchSet(
            Effect::AutoPanDepth,
            audio.auto_pan_depth,
        )),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Wobble, audio.wobble)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Karaoke, audio.karaoke)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Width, audio.width)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Bitcrush, audio.bitcrush)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Exciter, audio.exciter)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::PingPong, audio.ping_pong)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Phaser, audio.phaser)),
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
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Bass, def.bass)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Treble, def.treble)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Reverb, def.reverb)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Echo, def.echo)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Pan, def.pan)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Lowpass, def.lowpass)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Highpass, def.highpass)),
        AppMsg::Audio(AudioMsg::DispatchSet(
            Effect::AutoPanRate,
            def.auto_pan_rate,
        )),
        AppMsg::Audio(AudioMsg::DispatchSet(
            Effect::AutoPanDepth,
            def.auto_pan_depth,
        )),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Wobble, def.wobble)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Karaoke, def.karaoke)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Width, def.width)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Bitcrush, def.bitcrush)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Exciter, def.exciter)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::PingPong, def.ping_pong)),
        AppMsg::Audio(AudioMsg::DispatchSet(Effect::Phaser, def.phaser)),
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
            msgs.iter().find(|msg| matches!(
                msg,
                AppMsg::Audio(AudioMsg::DispatchSet(Effect::AutoPanDepth, _))
            )),
            Some(AppMsg::Audio(AudioMsg::DispatchSet(
                Effect::AutoPanDepth,
                0.0
            )))
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
            msgs.iter().find(|msg| matches!(
                msg,
                AppMsg::Audio(AudioMsg::DispatchSet(Effect::AutoPanRate, _))
            )),
            Some(AppMsg::Audio(AudioMsg::DispatchSet(
                Effect::AutoPanRate,
                0.10
            )))
        ));
    }

    #[test]
    fn treble_bottoms_out_at_a_deep_cut_rather_than_zero() {
        // Treble is an amplitude gain on a shelf, and zero is not one a shelf
        // can apply: the DSP froze there and played silence. It now stops at
        // 0.1, -20 dB, where bass already did.
        let mut state = test_state();
        for _ in 0..30 {
            handle_adjust_effect(&mut state, Effect::Treble, -0.1);
        }
        assert!(
            (state.session.audio.treble - 0.1).abs() < 1e-6,
            "treble went down to {}",
            state.session.audio.treble
        );
    }

    #[test]
    fn mute_keeps_the_volume_to_come_back_to() {
        let mut state = test_state();
        state.session.audio.volume = 0.8;
        handle_toggle_mute(&mut state);
        assert_eq!(state.session.audio.volume, 0.0);
        let saved = crate::state::session_store::PersistedSession::from_app_state(&state);
        assert_eq!(
            saved.volume, 0.8,
            "a muted session is saved at the volume to come back to"
        );
        handle_toggle_mute(&mut state);
        assert_eq!(state.session.audio.volume, 0.8);

        // Turned up while muted, it starts from silence and is unmuted.
        handle_toggle_mute(&mut state);
        handle_adjust_volume(&mut state, 0.05);
        assert!((state.session.audio.volume - 0.05).abs() < 1e-6);
        assert_eq!(state.session.audio.muted_from, None);
    }

    #[test]
    fn a_change_is_told_only_where_the_mixer_does_not_show_it() {
        use crate::state::ui::FocusMode;
        let mut state = test_state();
        state.session.ui.focus_mode = FocusMode::Full;
        state.session.ui.show_lyrics = false;
        handle_adjust_speed(&mut state, 0.1);
        assert!(
            state.session.notification.current.is_none(),
            "the mixer shows it"
        );

        state.session.ui.focus_mode = FocusMode::Visualizer;
        handle_adjust_speed(&mut state, 0.1);
        let told = state.session.notification.current.as_ref().unwrap();
        assert_eq!(told.message, "Speed 1.20×");

        handle_reset_speed(&mut state);
        assert_eq!(state.session.audio.speed, 1.0);
        assert_eq!(
            state.session.notification.current.as_ref().unwrap().message,
            "Speed 1.00×"
        );
    }
}
