pub mod audio;
pub mod download;
pub mod notification;
pub mod session;
pub mod ui;
pub mod visualizer;

// Re-export specific structs for easier access
pub use audio::AudioState;
pub use download::{DownloadJob, DownloadState, DownloadStatus};
pub use notification::{Notification, NotificationKind, NotificationState};
pub use session::SessionState;
pub use ui::{UIMode, UIState};
pub use visualizer::{SpectrumStyle, VisualizerMode};
