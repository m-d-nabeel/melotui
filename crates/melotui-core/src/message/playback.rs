use std::path::PathBuf;

use crate::state::SpectrumStyle;

#[derive(Debug, Clone)]
pub enum PlaybackMsg {
    RequestPlaySelected,
    RequestTogglePlayback,
    RequestStop,
    RequestPlayNext,
    RequestPlayPrev,
    RequestSeek {
        offset_seconds: i64,
    },

    DispatchPlay {
        path: PathBuf,
    },
    DispatchPause,
    DispatchResume,
    DispatchStop,
    DispatchSeek {
        position_ms: u64,
    },
    DispatchSetVisualizer {
        spectrum_style: Option<SpectrumStyle>,
    },

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
}
