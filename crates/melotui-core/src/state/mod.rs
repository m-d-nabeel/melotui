pub mod audio;
pub mod download;
pub mod lyrics;
pub mod notification;
pub mod session;
pub mod ui;
pub mod visualizer;

// re-exports
pub use audio::{AudioState, DSP_CONTROL_COUNT, EQ_BAND_FREQS, EQ_BAND_LABELS};
pub use download::{DownloadJob, DownloadState, DownloadStatus};
pub use lyrics::{LyricsData, LyricsState};
pub use notification::{Notification, NotificationKind, NotificationState};
pub use session::SessionState;
pub use ui::{UIMode, UIState};
pub use visualizer::{SpectrumStyle, VisualizerMode};
