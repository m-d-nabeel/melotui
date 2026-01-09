use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum PlaybackMsg {
    RequestPlaySelected,
    RequestTogglePlayback,
    RequestStop,
    RequestSeek { offset_seconds: i64 },

    DispatchPlay { path: PathBuf },
    DispatchPause,
    DispatchResume,
    DispatchStop,
    DispatchSeek { position_ms: u64 },
    DispatchSetVisualizer { active: bool },

    AudioProgress { position_ms: u64 },
    AudioStarted { path: String },
    AudioPaused,
    AudioResumed,
    AudioFinished,
    AudioError { error: String },
}
