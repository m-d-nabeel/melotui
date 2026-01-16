use crate::{AppMsg, AppState, message::audio::AudioMsg};

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
    let new_treble = (state.treble() + delta).clamp(0.0, 5.0);
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
