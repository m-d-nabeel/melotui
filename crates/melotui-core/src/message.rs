use std::path::PathBuf;

use crate::library::Track;

#[derive(Debug, Clone)]
pub enum AppMsg {
    // ───── Lifecycle ─────
    RequestQuit,
    Noop,

    // ───── Navigation ─────
    SwitchToPlaylists,
    SwitchToTracks,
    SwitchToTrackInfo,
    ToggleLibraryQueue,

    MoveUp,
    MoveDown,
    ToggleArt,
    ToggleHelp,

    // ───── Queue & Repeat ─────
    AddToQueue(Track),
    PlayNext(Track),
    ToggleRepeatMode,
    ToggleShuffle,
    RemoveFromQueue,
    ClearQueue,

    // ───── Library ─────
    RequestRescanLibrary,

    // ───── Playback ─────
    /// User intent (pure)
    RequestPlaySelected,

    /// User intent (pure)
    RequestTogglePlayback,

    /// User intent (pure)
    RequestStop,

    /// User intent (pure)
    RequestSeek {
        offset_seconds: i64,
    },

    /// Side-effect boundary
    DispatchPlay {
        path: PathBuf,
    },

    DispatchSetVolume(f32),
    DispatchSetSpeed(f32),
    DispatchSetBass(f32),
    DispatchSetTreble(f32),
    DispatchSetReverb(f32),
    DispatchSetEcho(f32),

    /// Side-effect boundary
    DispatchPause,

    /// Side-effect boundary
    DispatchResume,

    /// Side-effect boundary
    DispatchStop,

    /// Side-effect boundary
    DispatchSeek {
        position_ms: u64,
    },

    /// Side-effect boundary
    DispatchSetVisualizer {
        active: bool,
    },

    /// External facts
    AudioProgress {
        position_ms: u64,
    },

    AudioStarted {
        path: String,
    },
    AudioPaused,
    AudioResumed,
    AudioFinished,
    AudioError {
        error: String,
    },

    // Audio Controls
    AdjustVolume(f32),
    AdjustSpeed(f32),
    AdjustBass(f32),
    AdjustTreble(f32),
    AdjustReverb(f32),
    AdjustEcho(f32),

    // ───── Command bar ─────
    EnterCommandMode,
    ExitCommandMode,
    UpdateCommand(char),
    BackspaceCommand,
    SubmitCommand(String),

    // ───── Downloads ─────
    RequestDownload {
        query: String,
    },

    BeginNextDownload,

    DispatchDownload {
        query: String,
        download_path: PathBuf,
    },

    DownloadProgress {
        percent: u8,
    },
    DownloadFinished,
    DownloadFailed {
        error: String,
    },

    RequestCancelDownload,
}
