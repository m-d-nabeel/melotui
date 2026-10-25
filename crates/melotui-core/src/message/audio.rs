#[derive(Debug, Clone)]
pub enum AudioMsg {
    /// Plays through the device with this name, or part of it; `None`
    /// follows the system default.
    SetOutputDevice(Option<String>),
    /// Asks the audio thread which output devices there are.
    RequestOutputDevices,
    /// The output devices there are.
    OutputDevicesListed(Vec<String>),
    /// An output device was opened.
    OutputOpened(String),
    /// What the music comes out of changed: headphones plugged in, say.
    OutputKindChanged(crate::state::audio::OutputKind),
    AdjustVolume(f32),
    /// Mutes, keeping the volume to unmute to, or unmutes.
    ToggleMute,
    AdjustSpeed(f32),
    /// Back to normal speed.
    ResetSpeed,
    AdjustBass(f32),
    AdjustTreble(f32),
    AdjustReverb(f32),
    AdjustEcho(f32),
    AdjustPan(f32),
    AdjustLowpass(f32),
    AdjustHighpass(f32),
    AdjustEqBand {
        band: usize,
        delta: f32,
    },
    AdjustAutoPanRate(f32),
    AdjustAutoPanDepth(f32),
    AdjustWobble(f32),
    AdjustKaraoke(f32),
    AdjustWidth(f32),
    AdjustBitcrush(f32),
    AdjustExciter(f32),
    AdjustPingPong(f32),
    AdjustPhaser(f32),
    DispatchSetVolume(f32),
    DispatchSetSpeed(f32),
    DispatchSetBass(f32),
    DispatchSetTreble(f32),
    DispatchSetReverb(f32),
    DispatchSetEcho(f32),
    DispatchSetPan(f32),
    DispatchSetLowpass(f32),
    DispatchSetHighpass(f32),
    DispatchSetEqBand {
        band: usize,
        gain: f32,
    },
    DispatchSetAutoPanRate(f32),
    DispatchSetAutoPanDepth(f32),
    DispatchSetWobble(f32),
    DispatchSetKaraoke(f32),
    DispatchSetWidth(f32),
    DispatchSetBitcrush(f32),
    DispatchSetExciter(f32),
    DispatchSetPingPong(f32),
    DispatchSetPhaser(f32),
}
