pub mod audio;
pub mod download;
pub mod navigation;
pub mod notification;
pub mod playback;
pub mod session;
pub mod ui;
pub mod visualizer;

// Re-export specific structs for easier access
pub use audio::AudioState;
pub use download::{DownloadJob, DownloadState, DownloadStatus};
pub use navigation::{
    LeftPanelMode, LibraryNavState, LibraryViewDepth, NavigationState, QueueNavState,
};
pub use notification::{Notification, NotificationKind, NotificationState};
pub use playback::{PlaybackQueueState, PlaybackState, PlaybackStatus, RepeatMode};
pub use session::SessionState;
pub use ui::{UIMode, UIState};
pub use visualizer::{SpectrumStyle, VisualizerMode};
