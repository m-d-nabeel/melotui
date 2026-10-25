use std::path::PathBuf;

use crate::state::SpectrumStyle;

#[derive(Debug, Clone)]
pub enum PlaybackMsg {
    RequestPlaySelected,
    RequestTogglePlayback,
    /// Plays: resumes what is paused, or starts the selection; nothing if
    /// already playing. Unlike the toggle, safe to repeat.
    RequestPlay,
    /// Pauses if playing; nothing otherwise.
    RequestPause,
    RequestStop,
    RequestPlayNext,
    RequestPlayPrev,
    RequestSeekRelative(i64),
    RequestSeekAbsolute(u64),

    DispatchPlay {
        path: PathBuf,
    },
    DispatchPause,
    DispatchResume,
    DispatchStop,
    DispatchSeek {
        position_ms: u64,
        epoch: u64,
    },
    DispatchSetVisualizer {
        spectrum_style: Option<SpectrumStyle>,
    },

    AudioProgress {
        position_ms: u64,
        epoch: u64,
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
}
