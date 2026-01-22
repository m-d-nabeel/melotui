use ratatui::crossterm::event::{KeyCode, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    AppMsg,
    message::{
        audio::AudioMsg, download::DownloadMsg, library::LibraryMsg, navigation::NavigationMsg,
        playback::PlaybackMsg, ui::UIMsg,
    },
};

// Contexts for key bindings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyContext {
    Help,
    CommandMode,
    Library,
    Queue,
    Global,
}

// Actions users can perform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyAction {
    // System
    Quit,
    ToggleHelp,
    CloseHelp,
    EnterCommandMode,
    ExitCommandMode,
    SubmitCommand,
    BackspaceCommand,

    // Navigation
    TogglePanel,
    ToggleVisualizer,
    MoveUp,
    MoveDown,
    ToggleTreeExpand,
    ShowTrackInfo,
    HideTrackInfo,

    // Playback
    PlaySelected,
    PlayPause,
    Stop,
    SeekForward,
    SeekBackward,
    ToggleShuffle,
    CycleRepeat,

    // Library/Queue
    AddToQueue,
    PlayNext,
    RemoveFromQueue,
    ClearQueue,
    RescanLibrary,
    CancelDownload,

    // Audio
    VolUp,
    VolDown,
    SpeedUp,
    SpeedDown,
    BassUp,
    BassDown,
    TrebleUp,
    TrebleDown,
    ReverbUp,
    ReverbDown,
    EchoUp,
    EchoDown,
}

const OFFSET_SECONDS: i64 = 10;

impl KeyAction {
    pub fn to_msg(&self) -> Option<AppMsg> {
        match self {
            KeyAction::Quit => Some(AppMsg::RequestQuit),
            KeyAction::ToggleHelp => Some(AppMsg::UI(UIMsg::ToggleHelp)),
            // Toggles help off
            KeyAction::CloseHelp => Some(AppMsg::UI(UIMsg::ToggleHelp)),

            KeyAction::EnterCommandMode => Some(AppMsg::UI(UIMsg::EnterCommandMode)),
            KeyAction::ExitCommandMode => Some(AppMsg::UI(UIMsg::ExitCommandMode)),
            KeyAction::BackspaceCommand => Some(AppMsg::UI(UIMsg::BackspaceCommand)),

            KeyAction::MoveUp => Some(AppMsg::Navigation(NavigationMsg::MoveUp)),
            KeyAction::MoveDown => Some(AppMsg::Navigation(NavigationMsg::MoveDown)),
            KeyAction::TogglePanel => Some(AppMsg::Navigation(NavigationMsg::ToggleLibraryQueue)),
            KeyAction::ToggleVisualizer => Some(AppMsg::Navigation(NavigationMsg::ToggleArt)),
            KeyAction::ToggleTreeExpand => {
                Some(AppMsg::Navigation(NavigationMsg::ToggleTreeExpand))
            }
            KeyAction::ShowTrackInfo => Some(AppMsg::Navigation(NavigationMsg::ToggleTrackInfo)),
            KeyAction::HideTrackInfo => Some(AppMsg::Navigation(NavigationMsg::ToggleTrackInfo)),

            KeyAction::PlaySelected => Some(AppMsg::Playback(PlaybackMsg::RequestPlaySelected)),
            KeyAction::PlayPause => Some(AppMsg::Playback(PlaybackMsg::RequestTogglePlayback)),
            KeyAction::Stop => Some(AppMsg::Playback(PlaybackMsg::RequestStop)),
            KeyAction::SeekForward => Some(AppMsg::Playback(PlaybackMsg::RequestSeek {
                offset_seconds: OFFSET_SECONDS,
            })),
            KeyAction::SeekBackward => Some(AppMsg::Playback(PlaybackMsg::RequestSeek {
                offset_seconds: -OFFSET_SECONDS,
            })),

            KeyAction::ToggleShuffle => Some(AppMsg::Library(LibraryMsg::ToggleShuffle)),
            KeyAction::CycleRepeat => Some(AppMsg::Library(LibraryMsg::ToggleRepeatMode)),

            KeyAction::RemoveFromQueue => Some(AppMsg::Library(LibraryMsg::RemoveFromQueue)),
            KeyAction::ClearQueue => Some(AppMsg::Library(LibraryMsg::ClearQueue)),
            KeyAction::RescanLibrary => Some(AppMsg::Library(LibraryMsg::RequestRescanLibrary)),
            KeyAction::CancelDownload => Some(AppMsg::Download(DownloadMsg::RequestCancelDownload)),

            // Audio FX
            KeyAction::VolUp => Some(AppMsg::Audio(AudioMsg::AdjustVolume(0.05))),
            KeyAction::VolDown => Some(AppMsg::Audio(AudioMsg::AdjustVolume(-0.05))),
            KeyAction::SpeedUp => Some(AppMsg::Audio(AudioMsg::AdjustSpeed(0.1))),
            KeyAction::SpeedDown => Some(AppMsg::Audio(AudioMsg::AdjustSpeed(-0.1))),
            KeyAction::BassUp => Some(AppMsg::Audio(AudioMsg::AdjustBass(0.1))),
            KeyAction::BassDown => Some(AppMsg::Audio(AudioMsg::AdjustBass(-0.1))),
            KeyAction::TrebleUp => Some(AppMsg::Audio(AudioMsg::AdjustTreble(0.1))),
            KeyAction::TrebleDown => Some(AppMsg::Audio(AudioMsg::AdjustTreble(-0.1))),
            KeyAction::ReverbUp => Some(AppMsg::Audio(AudioMsg::AdjustReverb(0.05))),
            KeyAction::ReverbDown => Some(AppMsg::Audio(AudioMsg::AdjustReverb(-0.05))),
            KeyAction::EchoUp => Some(AppMsg::Audio(AudioMsg::AdjustEcho(0.05))),
            KeyAction::EchoDown => Some(AppMsg::Audio(AudioMsg::AdjustEcho(-0.05))),

            _ => None, // SubmitCommand, AddToQueue, PlayNext
        }
    }
}

// Key binding structure
#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub key: KeyCode,
    pub modifiers: KeyModifiers,
    pub action: KeyAction,
    pub desc: &'static str,
}

impl KeyBinding {
    pub fn new(code: KeyCode, action: KeyAction, desc: &'static str) -> Self {
        Self {
            key: code,
            modifiers: KeyModifiers::empty(),
            action,
            desc,
        }
    }

    pub fn with_mod(mut self, modifiers: KeyModifiers) -> Self {
        self.modifiers = modifiers;
        self
    }
}

// Registry of key bindings
pub struct KeyRegistry {
    pub bindings: HashMap<KeyContext, Vec<KeyBinding>>,
}

impl Default for KeyRegistry {
    fn default() -> Self {
        let mut bindings = HashMap::new();

        // Help overlay
        bindings.insert(
            KeyContext::Help,
            vec![
                KeyBinding::new(KeyCode::Esc, KeyAction::CloseHelp, "Close"),
                KeyBinding::new(KeyCode::Char('q'), KeyAction::CloseHelp, "Close"),
                KeyBinding::new(KeyCode::Char('?'), KeyAction::CloseHelp, "Close"),
            ],
        );

        // Command mode
        bindings.insert(
            KeyContext::CommandMode,
            vec![
                KeyBinding::new(KeyCode::Esc, KeyAction::ExitCommandMode, "Exit"),
                KeyBinding::new(KeyCode::Enter, KeyAction::SubmitCommand, "Execute"),
                KeyBinding::new(KeyCode::Backspace, KeyAction::BackspaceCommand, "Delete"),
            ],
        );

        // Library panel
        bindings.insert(
            KeyContext::Library,
            vec![
                KeyBinding::new(KeyCode::Char('j'), KeyAction::MoveDown, "Down"),
                KeyBinding::new(KeyCode::Down, KeyAction::MoveDown, "Down"),
                KeyBinding::new(KeyCode::Char('k'), KeyAction::MoveUp, "Up"),
                KeyBinding::new(KeyCode::Up, KeyAction::MoveUp, "Up"),
                KeyBinding::new(KeyCode::Enter, KeyAction::ToggleTreeExpand, "Expand"),
                KeyBinding::new(KeyCode::Char('['), KeyAction::HideTrackInfo, "Back"),
                KeyBinding::new(KeyCode::Char(']'), KeyAction::ShowTrackInfo, "Track Info"),
                KeyBinding::new(KeyCode::Char('a'), KeyAction::AddToQueue, "Add Queue"),
                KeyBinding::new(KeyCode::Char('A'), KeyAction::PlayNext, "Play Next")
                    .with_mod(KeyModifiers::SHIFT),
            ],
        );

        // Queue panel
        bindings.insert(
            KeyContext::Queue,
            vec![
                KeyBinding::new(KeyCode::Char('j'), KeyAction::MoveDown, "Down"),
                KeyBinding::new(KeyCode::Down, KeyAction::MoveDown, "Down"),
                KeyBinding::new(KeyCode::Char('k'), KeyAction::MoveUp, "Up"),
                KeyBinding::new(KeyCode::Up, KeyAction::MoveUp, "Up"),
                KeyBinding::new(KeyCode::Enter, KeyAction::ToggleTreeExpand, "Expand"),
                KeyBinding::new(KeyCode::Char('['), KeyAction::HideTrackInfo, "Back"),
                KeyBinding::new(KeyCode::Char(']'), KeyAction::ShowTrackInfo, "Track Info"),
                KeyBinding::new(KeyCode::Char('d'), KeyAction::RemoveFromQueue, "Remove"),
                KeyBinding::new(KeyCode::Delete, KeyAction::RemoveFromQueue, "Remove"),
                KeyBinding::new(KeyCode::Char('D'), KeyAction::ClearQueue, "Clear All")
                    .with_mod(KeyModifiers::SHIFT),
            ],
        );

        // Global
        bindings.insert(
            KeyContext::Global,
            vec![
                KeyBinding::new(KeyCode::Char('q'), KeyAction::Quit, "Quit"),
                KeyBinding::new(KeyCode::Char('?'), KeyAction::ToggleHelp, "Help"),
                KeyBinding::new(KeyCode::Char(':'), KeyAction::EnterCommandMode, "Command"),
                KeyBinding::new(KeyCode::Char(' '), KeyAction::PlayPause, "Play/Pause"),
                KeyBinding::new(KeyCode::Enter, KeyAction::PlaySelected, "Play"),
                KeyBinding::new(KeyCode::Char('x'), KeyAction::Stop, "Stop"),
                KeyBinding::new(KeyCode::F(5), KeyAction::RescanLibrary, "Rescan"),
                KeyBinding::new(KeyCode::Tab, KeyAction::TogglePanel, "Switch Panel"),
                KeyBinding::new(KeyCode::BackTab, KeyAction::ToggleVisualizer, "Visualizer")
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::Char(','), KeyAction::SeekBackward, "Seek -5s"),
                KeyBinding::new(KeyCode::Char('<'), KeyAction::SeekBackward, "Seek -5s")
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::Char('.'), KeyAction::SeekForward, "Seek +5s"),
                KeyBinding::new(KeyCode::Char('>'), KeyAction::SeekForward, "Seek +5s")
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::Esc, KeyAction::CancelDownload, "Cancel DL"),
                KeyBinding::new(KeyCode::Char('+'), KeyAction::VolUp, "Vol+"),
                KeyBinding::new(KeyCode::Char('='), KeyAction::VolUp, "Vol+"),
                KeyBinding::new(KeyCode::Char('-'), KeyAction::VolDown, "Vol-"),
                KeyBinding::new(KeyCode::Char('_'), KeyAction::VolDown, "Vol-")
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::Char('z'), KeyAction::ToggleShuffle, "Shuffle"),
                KeyBinding::new(KeyCode::Char('c'), KeyAction::CycleRepeat, "Repeat"),
                KeyBinding::new(KeyCode::Char('s'), KeyAction::SpeedUp, "Speed+"),
                KeyBinding::new(KeyCode::Char('S'), KeyAction::SpeedDown, "Speed-")
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::Char('b'), KeyAction::BassUp, "Bass+"),
                KeyBinding::new(KeyCode::Char('B'), KeyAction::BassDown, "Bass-")
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::Char('t'), KeyAction::TrebleUp, "Treble+"),
                KeyBinding::new(KeyCode::Char('T'), KeyAction::TrebleDown, "Treble-")
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::Char('r'), KeyAction::ReverbUp, "Reverb+"),
                KeyBinding::new(KeyCode::Char('R'), KeyAction::ReverbDown, "Reverb-")
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::Char('e'), KeyAction::EchoUp, "Echo+"),
                KeyBinding::new(KeyCode::Char('E'), KeyAction::EchoDown, "Echo-")
                    .with_mod(KeyModifiers::SHIFT),
            ],
        );

        Self { bindings }
    }
}
