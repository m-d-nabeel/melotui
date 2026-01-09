use crate::{AppMsg, AppState};

pub fn handle_adjust_volume(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    state.audio.volume = (state.audio.volume + delta).clamp(0.0, 1.5);
    vec![AppMsg::DispatchSetVolume(state.audio.volume)]
}

pub fn handle_adjust_speed(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    state.audio.speed = (state.audio.speed + delta).clamp(0.5, 2.0);
    vec![AppMsg::DispatchSetSpeed(state.audio.speed)]
}

pub fn handle_adjust_bass(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    state.audio.bass = (state.audio.bass + delta).clamp(0.1, 5.0);
    vec![AppMsg::DispatchSetBass(state.audio.bass)]
}

pub fn handle_adjust_treble(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    state.audio.treble = (state.audio.treble + delta).clamp(0.0, 5.0);
    vec![AppMsg::DispatchSetTreble(state.audio.treble)]
}

pub fn handle_adjust_reverb(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    state.audio.reverb = (state.audio.reverb + delta).clamp(0.0, 1.0);
    vec![AppMsg::DispatchSetReverb(state.audio.reverb)]
}

pub fn handle_adjust_echo(state: &mut AppState, delta: f32) -> Vec<AppMsg> {
    state.audio.echo = (state.audio.echo + delta).clamp(0.0, 0.9);
    vec![AppMsg::DispatchSetEcho(state.audio.echo)]
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
