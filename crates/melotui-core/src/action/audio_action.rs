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

pub fn handle_macro_slowed_reverb(state: &mut AppState) -> Vec<AppMsg> {
    state.session.audio.speed = 0.85;
    state.session.audio.reverb = 0.45;
    state.session.audio.bass = 1.3;
    state.session.audio.width = 1.3;
    state.session.audio.lowpass = 3500.0;
    state.session.audio.highpass = 20.0;
    state.session.audio.wobble = 0.15;
    vec![
        AppMsg::Audio(AudioMsg::DispatchSetSpeed(0.85)),
        AppMsg::Audio(AudioMsg::DispatchSetReverb(0.45)),
        AppMsg::Audio(AudioMsg::DispatchSetBass(1.3)),
        AppMsg::Audio(AudioMsg::DispatchSetWidth(1.3)),
        AppMsg::Audio(AudioMsg::DispatchSetLowpass(3500.0)),
        AppMsg::Audio(AudioMsg::DispatchSetHighpass(20.0)),
        AppMsg::Audio(AudioMsg::DispatchSetWobble(0.15)),
    ]
}

pub fn handle_macro_nightcore(state: &mut AppState) -> Vec<AppMsg> {
    state.session.audio.speed = 1.25;
    state.session.audio.treble = 1.8;
    state.session.audio.bass = 1.2;
    state.session.audio.width = 1.4;
    state.session.audio.highpass = 80.0;
    state.session.audio.exciter = 1.5;
    state.session.audio.lowpass = 20000.0;
    vec![
        AppMsg::Audio(AudioMsg::DispatchSetSpeed(1.25)),
        AppMsg::Audio(AudioMsg::DispatchSetTreble(1.8)),
        AppMsg::Audio(AudioMsg::DispatchSetBass(1.2)),
        AppMsg::Audio(AudioMsg::DispatchSetWidth(1.4)),
        AppMsg::Audio(AudioMsg::DispatchSetHighpass(80.0)),
        AppMsg::Audio(AudioMsg::DispatchSetExciter(1.5)),
        AppMsg::Audio(AudioMsg::DispatchSetLowpass(20000.0)),
    ]
}

pub fn handle_macro_spatial(state: &mut AppState) -> Vec<AppMsg> {
    state.session.audio.auto_pan_rate = 0.25;
    state.session.audio.auto_pan_depth = 0.80;
    state.session.audio.ping_pong = 0.35;
    state.session.audio.width = 1.8;
    state.session.audio.reverb = 0.20;
    state.session.audio.phaser = 0.15;
    vec![
        AppMsg::Audio(AudioMsg::DispatchSetAutoPanRate(0.25)),
        AppMsg::Audio(AudioMsg::DispatchSetAutoPanDepth(0.80)),
        AppMsg::Audio(AudioMsg::DispatchSetPingPong(0.35)),
        AppMsg::Audio(AudioMsg::DispatchSetWidth(1.8)),
        AppMsg::Audio(AudioMsg::DispatchSetReverb(0.20)),
        AppMsg::Audio(AudioMsg::DispatchSetPhaser(0.15)),
    ]
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
