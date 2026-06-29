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

// Dispatch handlers (no-op in reducer, intercepted by engine bridge)
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
