#[derive(Debug, Clone)]
pub enum AudioMsg {
    AdjustVolume(f32),
    AdjustSpeed(f32),
    AdjustBass(f32),
    AdjustTreble(f32),
    AdjustReverb(f32),
    AdjustEcho(f32),
    DispatchSetVolume(f32),
    DispatchSetSpeed(f32),
    DispatchSetBass(f32),
    DispatchSetTreble(f32),
    DispatchSetReverb(f32),
    DispatchSetEcho(f32),
}
