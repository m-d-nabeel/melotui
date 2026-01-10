use crate::{AppMsg, AppState, message::audio::AudioMsg};

pub fn handle_adjust_volume(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    state.session.audio.volume = (state.session.audio.volume + delta).clamp(0.0, 1.5);
    vec![AppMsg::Audio(AudioMsg::DispatchSetVolume(
        state.session.audio.volume,
    ))]
}

pub fn handle_adjust_speed(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    state.session.audio.speed = (state.session.audio.speed + delta).clamp(0.5, 2.0);
    vec![AppMsg::Audio(AudioMsg::DispatchSetSpeed(
        state.session.audio.speed,
    ))]
}

pub fn handle_adjust_bass(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    state.session.audio.bass = (state.session.audio.bass + delta).clamp(0.1, 5.0);
    vec![AppMsg::Audio(AudioMsg::DispatchSetBass(
        state.session.audio.bass,
    ))]
}

pub fn handle_adjust_treble(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    state.session.audio.treble = (state.session.audio.treble + delta).clamp(0.0, 5.0);
    vec![AppMsg::Audio(AudioMsg::DispatchSetTreble(
        state.session.audio.treble,
    ))]
}

pub fn handle_adjust_reverb(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    state.session.audio.reverb = (state.session.audio.reverb + delta).clamp(0.0, 1.0);
    vec![AppMsg::Audio(AudioMsg::DispatchSetReverb(
        state.session.audio.reverb,
    ))]
}

pub fn handle_adjust_echo(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    state.session.audio.echo = (state.session.audio.echo + delta).clamp(0.0, 0.9);
    vec![AppMsg::Audio(AudioMsg::DispatchSetEcho(
        state.session.audio.echo,
    ))]
}

// Dispatch handlers that do nothing in reducer
pub fn handle_dispatch_set_volume(_state: &mut AppState, _volume: f32) -> Vec<AppMsg> {
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
