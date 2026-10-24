use crate::{AppMsg, domain::TrackId};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UIMode {
    #[default]
    Normal,
    Command,
    PlaylistPicker,
    PresetPicker,
    Confirm,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FocusMode {
    #[default]
    Full, // sidebar + visualizer + mixer + playback
    Visualizer,         // visualizer fills entire screen
    Playback,           // minimal now-playing strip centered
    VisualizerPlayback, // visualizer + playback, no sidebar/mixer
    LyricsPlayback,     // lyrics + playback
}

impl FocusMode {
    pub fn cycle(self) -> Self {
        match self {
            FocusMode::Full => FocusMode::Visualizer,
            FocusMode::Visualizer => FocusMode::Playback,
            FocusMode::Playback => FocusMode::VisualizerPlayback,
            FocusMode::VisualizerPlayback => FocusMode::LyricsPlayback,
            FocusMode::LyricsPlayback => FocusMode::Full,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            FocusMode::Full => "Full",
            FocusMode::Visualizer => "Visualizer",
            FocusMode::Playback => "Playback",
            FocusMode::VisualizerPlayback => "Viz+Play",
            FocusMode::LyricsPlayback => "Lyrics+Play",
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
            "lyrics" | "l" | "lp" | "lyricsplay" => Ok(FocusMode::LyricsPlayback),
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

#[derive(Debug, Clone, Default)]
pub struct PresetPickerState {
    pub query: String,
    pub selection: usize,
    pub scroll_offset: usize,
    pub presets: Vec<String>,
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
    pub preset_picker: PresetPickerState,
    /// track waiting to be added after async playlist creation
    pub pending_add_track: Option<TrackId>,
    pub confirm_dialog: Option<ConfirmDialog>,

    pub show_help: bool,
    pub show_lyrics: bool,
    pub show_playback: bool,
    pub show_dsp_studio: bool,
    pub dsp_selected_control: usize, // 0..18 (10 EQ bands + 9 sliders)
    pub scrolloff: usize,
    pub terminal_size: (u16, u16),
    pub sidebar_width: u16,
    pub focus_mode: FocusMode,
    /// Lyrics zoom in the lyrics panel, and in the full lyrics view.
    pub lyrics_zoom: f32,
    pub lyrics_zoom_focus: f32,
    /// Saved presets' names, read when the command bar opens, for completion.
    pub command_presets: Vec<String>,
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
            preset_picker: PresetPickerState::default(),
            pending_add_track: None,
            confirm_dialog: None,
            show_help: false,
            show_lyrics: false,
            show_playback: true,
            show_dsp_studio: false,
            dsp_selected_control: 0,
            scrolloff: 8,
            terminal_size: (0, 0),
            sidebar_width: 28,
            focus_mode: FocusMode::default(),
            lyrics_zoom: 2.0,
            lyrics_zoom_focus: 3.0,
            command_presets: Vec::new(),
            theme_overrides: std::collections::HashMap::new(),
        }
    }
}
