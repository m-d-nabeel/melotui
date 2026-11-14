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
    /// Turn an effect up or down by this much.
    Adjust(crate::state::audio::Effect, f32),
    /// Have the audio thread set an effect to this value.
    DispatchSet(crate::state::audio::Effect, f32),
    AdjustEqBand {
        band: usize,
        delta: f32,
    },
    DispatchSetVolume(f32),
    DispatchSetSpeed(f32),
    DispatchSetEqBand {
        band: usize,
        gain: f32,
    },
}
