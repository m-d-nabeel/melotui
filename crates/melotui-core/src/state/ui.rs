use crate::{AppMsg, domain::TrackId};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UIMode {
    #[default]
    Normal,
    Command,
    PlaylistPicker,
    Confirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusMode {
    #[default]
    Full, // sidebar + visualizer + mixer + playback
    Visualizer,         // visualizer fills entire screen
    Playback,           // minimal now-playing strip centered
    VisualizerPlayback, // visualizer + playback, no sidebar/mixer
}

impl FocusMode {
    pub fn cycle(self) -> Self {
        match self {
            FocusMode::Full => FocusMode::Visualizer,
            FocusMode::Visualizer => FocusMode::Playback,
            FocusMode::Playback => FocusMode::VisualizerPlayback,
            FocusMode::VisualizerPlayback => FocusMode::Full,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            FocusMode::Full => "Full",
            FocusMode::Visualizer => "Visualizer",
            FocusMode::Playback => "Playback",
            FocusMode::VisualizerPlayback => "Viz+Play",
        }
    }
}

impl std::str::FromStr for FocusMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "full" | "f" | "all" => Ok(FocusMode::Full),
            "viz" | "v" | "visualizer" => Ok(FocusMode::Visualizer),
            "play" | "p" | "playback" => Ok(FocusMode::Playback),
            "vp" | "vizplay" => Ok(FocusMode::VisualizerPlayback),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PlaylistPickerState {
    pub query: String,
    pub selection: usize,
    pub scroll_offset: usize,
    pub track_id: Option<TrackId>,
}

#[derive(Debug, Clone)]
pub struct ConfirmDialog {
    pub message: String,
    pub on_confirm: Box<AppMsg>,
}

#[derive(Debug, Clone)]
pub struct UIState {
    pub mode: UIMode,
    pub command_buffer: tui_input::Input,
    pub autocomplete_cycle: usize,
    pub autocomplete_prefix: Option<String>,
    pub command_history: crate::history::CommandHistory,
    pub history_index: Option<usize>,
    pub playlist_picker: PlaylistPickerState,
    /// track waiting to be added after async playlist creation
    pub pending_add_track: Option<TrackId>,
    pub confirm_dialog: Option<ConfirmDialog>,

    pub show_help: bool,
    pub show_lyrics: bool,
    pub scrolloff: usize,
    pub terminal_size: (u16, u16),
    pub sidebar_width: u16,
    pub focus_mode: FocusMode,
    pub theme_overrides: std::collections::HashMap<String, String>,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            mode: UIMode::default(),
            command_buffer: tui_input::Input::default(),
            autocomplete_cycle: 0,
            autocomplete_prefix: None,
            command_history: crate::history::CommandHistory::new(),
            history_index: None,
            playlist_picker: PlaylistPickerState::default(),
            pending_add_track: None,
            confirm_dialog: None,
            show_help: false,
            show_lyrics: false,
            scrolloff: 8,
            terminal_size: (0, 0),
            sidebar_width: 28,
            focus_mode: FocusMode::default(),
            theme_overrides: std::collections::HashMap::new(),
        }
    }
}
