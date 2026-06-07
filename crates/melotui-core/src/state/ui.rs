use crate::{AppMsg, domain::TrackId};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UIMode {
    #[default]
    Normal,
    Command,
    PlaylistPicker,
    Confirm,
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
    pub command_buffer: String,
    pub playlist_picker: PlaylistPickerState,
    /// track waiting to be added after async playlist creation
    pub pending_add_track: Option<TrackId>,
    pub confirm_dialog: Option<ConfirmDialog>,

    pub show_help: bool,
    pub scrolloff: usize,
    pub terminal_size: (u16, u16),
    pub sidebar_width: u16,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            mode: UIMode::default(),
            command_buffer: String::new(),
            playlist_picker: PlaylistPickerState::default(),
            pending_add_track: None,
            confirm_dialog: None,
            show_help: false,
            scrolloff: 8,
            terminal_size: (0, 0),
            sidebar_width: 28,
        }
    }
}
