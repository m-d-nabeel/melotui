use ratatui::crossterm::event::{KeyCode, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use melotui_core::{
    AppMsg,
    message::{
        audio::AudioMsg, library::LibraryMsg, navigation::NavigationMsg, playback::PlaybackMsg,
        ui::UIMsg,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyContext {
    Help,
    CommandMode,
    Library,
    Queue,
    TrackInfo,
    PlaylistPicker,
    Confirm,
    Global,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyAction {
    // system
    Quit,
    OpenHelp,
    CloseHelp,
    EnterCommandMode,
    ExitCommandMode,
    SubmitCommand,

    // navigation
    TogglePanel,
    ToggleVisualizer,
    MoveUp,
    MoveDown,
    Interact,
    ToggleFolderExpand,
    ShowTrackInfo,
    HideTrackInfo,

    // playback
    PlaySelected,
    PlayNextTrack,
    PlayPrevTrack,
    PlayPause,
    Stop,
    SeekForward,
    SeekBackward,
    ToggleShuffle,
    CycleRepeat,

    // library/queue
    AddToQueue,
    QueuePlayNext,
    RemoveFromQueue,
    ClearQueue,
    ScanLibrary,
    CancelDownload,
    ToggleFavorite,
    SearchNext,
    SearchPrev,

    // audio
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

    // playlist picker
    EnterPlaylistPicker,
    ExitPlaylistPicker,
    SubmitPlaylistPicker,
    PlaylistPickerMoveUp,
    PlaylistPickerMoveDown,
    PlaylistPickerBackspace,

    // playlist management
    DeletePlaylist,
    RemoveFromPlaylist,

    // confirm dialog
    ConfirmYes,
    ConfirmNo,

    // focus mode
    CycleFocusMode,

    // panel resize
    ShrinkSidebar,
    GrowSidebar,
}

const OFFSET_SECONDS: i64 = 5;

impl KeyAction {
    pub fn to_msg(&self) -> Option<AppMsg> {
        match self {
            KeyAction::Quit => Some(AppMsg::RequestQuit),
            KeyAction::OpenHelp => Some(AppMsg::UI(UIMsg::OpenHelp)),
            KeyAction::CloseHelp => Some(AppMsg::UI(UIMsg::CloseHelp)),

            KeyAction::EnterCommandMode => Some(AppMsg::UI(UIMsg::EnterCommandMode)),
            KeyAction::ExitCommandMode => Some(AppMsg::UI(UIMsg::ExitCommandMode)),

            KeyAction::MoveUp => Some(AppMsg::Navigation(NavigationMsg::MoveUp)),
            KeyAction::MoveDown => Some(AppMsg::Navigation(NavigationMsg::MoveDown)),
            KeyAction::TogglePanel => Some(AppMsg::Navigation(NavigationMsg::ToggleLibraryQueue)),
            KeyAction::ToggleVisualizer => Some(AppMsg::Navigation(NavigationMsg::ToggleArt)),
            KeyAction::ToggleFolderExpand => {
                Some(AppMsg::Navigation(NavigationMsg::ToggleTreeExpand))
            }
            KeyAction::Interact => None, // handled in input.rs
            KeyAction::ShowTrackInfo => Some(AppMsg::Navigation(NavigationMsg::ToggleTrackInfo)),
            KeyAction::HideTrackInfo => Some(AppMsg::Navigation(NavigationMsg::ToggleTrackInfo)),

            KeyAction::PlaySelected => Some(AppMsg::Playback(PlaybackMsg::RequestPlaySelected)),
            KeyAction::PlayNextTrack => Some(AppMsg::Playback(PlaybackMsg::RequestPlayNext)),
            KeyAction::PlayPrevTrack => Some(AppMsg::Playback(PlaybackMsg::RequestPlayPrev)),
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
            KeyAction::ScanLibrary => Some(AppMsg::Library(LibraryMsg::RequestScanLibrary)),
            KeyAction::CancelDownload => None, // conditional in input.rs
            KeyAction::ToggleFavorite => None, // id resolved in input.rs
            KeyAction::SearchNext => None,     // conditional in input.rs
            KeyAction::SearchPrev => None,     // conditional in input.rs
            KeyAction::AddToQueue => None,     // id resolved in input.rs
            KeyAction::QueuePlayNext => None,  // id resolved in input.rs
            KeyAction::SubmitCommand => None,  // uses command buffer
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

            KeyAction::EnterPlaylistPicker => Some(AppMsg::UI(UIMsg::EnterPlaylistPicker)),
            KeyAction::ExitPlaylistPicker => Some(AppMsg::UI(UIMsg::ExitPlaylistPicker)),
            KeyAction::SubmitPlaylistPicker => Some(AppMsg::UI(UIMsg::SubmitPlaylistPicker)),
            KeyAction::PlaylistPickerMoveUp => Some(AppMsg::UI(UIMsg::PlaylistPickerMoveUp)),
            KeyAction::PlaylistPickerMoveDown => Some(AppMsg::UI(UIMsg::PlaylistPickerMoveDown)),
            KeyAction::PlaylistPickerBackspace => Some(AppMsg::UI(UIMsg::PlaylistPickerBackspace)),
            KeyAction::DeletePlaylist => Some(AppMsg::UI(UIMsg::DeletePlaylistConfirm)),
            KeyAction::RemoveFromPlaylist => None, // id resolved in input.rs
            KeyAction::ConfirmYes => Some(AppMsg::UI(UIMsg::ConfirmYes)),
            KeyAction::ConfirmNo => Some(AppMsg::UI(UIMsg::ConfirmNo)),
            KeyAction::CycleFocusMode => Some(AppMsg::UI(UIMsg::CycleFocusMode)),
            KeyAction::ShrinkSidebar => Some(AppMsg::UI(UIMsg::AdjustSidebarWidth(-2))),
            KeyAction::GrowSidebar => Some(AppMsg::UI(UIMsg::AdjustSidebarWidth(2))),
        }
    }
}

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

// key binding registry
#[derive(Debug, Clone)]
pub struct KeyRegistry {
    pub bindings: HashMap<KeyContext, Vec<KeyBinding>>,
}

impl KeyRegistry {
    pub fn apply_overrides(
        &mut self,
        overrides: &HashMap<String, HashMap<String, String>>,
    ) -> Vec<String> {
        let mut errors = Vec::new();
        use crate::key_parser::parse_key;

        for (ctx_str, maps) in overrides {
            let ctx = match ctx_str.to_lowercase().as_str() {
                "help" => KeyContext::Help,
                "command" => KeyContext::CommandMode,
                "library" => KeyContext::Library,
                "queue" => KeyContext::Queue,
                "trackinfo" => KeyContext::TrackInfo,
                "playlist" | "playlistpicker" => KeyContext::PlaylistPicker,
                "confirm" => KeyContext::Confirm,
                "global" => KeyContext::Global,
                _ => {
                    let err = format!("Unknown keymap context: {}", ctx_str);
                    tracing::warn!("{}", err);
                    errors.push(err);
                    continue;
                }
            };

            // Remove any existing bindings that map to the overridden keys
            // to allow silent overrides.
            for (key_str, action_str) in maps {
                if let Ok((code, mods)) = parse_key(key_str) {
                    let toml_doc = format!("action = \"{}\"", action_str);
                    if let Ok(mut wrapper) =
                        toml::from_str::<std::collections::HashMap<String, KeyAction>>(&toml_doc)
                    {
                        if let Some(action) = wrapper.remove("action") {
                            // Remove default bindings with the exact same key to avoid double-trigger
                            if let Some(ctx_bindings) = self.bindings.get_mut(&ctx) {
                                ctx_bindings.retain(|b| !(b.key == code && b.modifiers == mods));
                                ctx_bindings.push(KeyBinding {
                                    key: code,
                                    modifiers: mods,
                                    action,
                                    desc: "User Custom",
                                });
                            } else {
                                self.bindings.insert(
                                    ctx,
                                    vec![KeyBinding {
                                        key: code,
                                        modifiers: mods,
                                        action,
                                        desc: "User Custom",
                                    }],
                                );
                            }
                        } else {
                            let err = format!("Unknown action '{}'", action_str);
                            tracing::error!("{}", err);
                            errors.push(err);
                        }
                    } else {
                        let err = format!("Failed to parse action '{}'", action_str);
                        tracing::error!("{}", err);
                        errors.push(err);
                    }
                } else {
                    let err = format!("Invalid key sequence '{}'", key_str);
                    tracing::error!("{}", err);
                    errors.push(err);
                }
            }
        }
        errors
    }
}

impl Default for KeyRegistry {
    fn default() -> Self {
        let mut bindings = HashMap::new();

        // help context
        bindings.insert(
            KeyContext::Help,
            vec![
                KeyBinding::new(KeyCode::Esc, KeyAction::CloseHelp, "Close"),
                KeyBinding::new(KeyCode::Char('q'), KeyAction::CloseHelp, "Close"),
            ],
        );

        bindings.insert(
            KeyContext::TrackInfo,
            vec![
                KeyBinding::new(
                    KeyCode::Char('['),
                    KeyAction::HideTrackInfo,
                    "Show/Hide track info",
                ),
                KeyBinding::new(
                    KeyCode::Char(']'),
                    KeyAction::ShowTrackInfo,
                    "Show/Hide track info",
                ),
            ],
        );

        // command context
        bindings.insert(
            KeyContext::CommandMode,
            vec![
                KeyBinding::new(KeyCode::Esc, KeyAction::ExitCommandMode, "Exit"),
                KeyBinding::new(KeyCode::Enter, KeyAction::SubmitCommand, "Execute"),
            ],
        );

        // library context
        bindings.insert(
            KeyContext::Library,
            vec![
                KeyBinding::new(KeyCode::Tab, KeyAction::TogglePanel, "Switch Panel"),
                KeyBinding::new(KeyCode::Char('j'), KeyAction::MoveDown, "Down"),
                KeyBinding::new(KeyCode::Down, KeyAction::MoveDown, "Down"),
                KeyBinding::new(KeyCode::Char('k'), KeyAction::MoveUp, "Up"),
                KeyBinding::new(KeyCode::Up, KeyAction::MoveUp, "Up"),
                KeyBinding::new(KeyCode::Enter, KeyAction::Interact, "Expand"),
                KeyBinding::new(KeyCode::Char('['), KeyAction::HideTrackInfo, "Back"),
                KeyBinding::new(KeyCode::Char(']'), KeyAction::ShowTrackInfo, "Track Info"),
                KeyBinding::new(KeyCode::Char('a'), KeyAction::AddToQueue, "Add Queue"),
                KeyBinding::new(KeyCode::Char('a'), KeyAction::QueuePlayNext, "Play Next")
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::Char('f'), KeyAction::ToggleFavorite, "Favorite"),
                KeyBinding::new(KeyCode::Char('n'), KeyAction::SearchNext, "Next Match"),
                KeyBinding::new(KeyCode::Char('n'), KeyAction::SearchPrev, "Prev Match")
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(
                    KeyCode::Char('p'),
                    KeyAction::EnterPlaylistPicker,
                    "Add to Playlist",
                ),
                KeyBinding::new(
                    KeyCode::Char('p'),
                    KeyAction::RemoveFromPlaylist,
                    "Remove from Playlist",
                )
                .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(
                    KeyCode::Char('d'),
                    KeyAction::DeletePlaylist,
                    "Delete Playlist",
                )
                .with_mod(KeyModifiers::SHIFT),
            ],
        );

        // playlist picker context
        bindings.insert(
            KeyContext::PlaylistPicker,
            vec![
                KeyBinding::new(KeyCode::Esc, KeyAction::ExitPlaylistPicker, "Back"),
                KeyBinding::new(KeyCode::Enter, KeyAction::SubmitPlaylistPicker, "Select"),
                KeyBinding::new(KeyCode::Down, KeyAction::PlaylistPickerMoveDown, "Down"),
                KeyBinding::new(KeyCode::Up, KeyAction::PlaylistPickerMoveUp, "Up"),
                KeyBinding::new(
                    KeyCode::Backspace,
                    KeyAction::PlaylistPickerBackspace,
                    "Delete",
                ),
            ],
        );

        // confirm dialog
        bindings.insert(
            KeyContext::Confirm,
            vec![
                KeyBinding::new(KeyCode::Char('y'), KeyAction::ConfirmYes, "Yes"),
                KeyBinding::new(KeyCode::Char('n'), KeyAction::ConfirmNo, "No"),
                KeyBinding::new(KeyCode::Esc, KeyAction::ConfirmNo, "Cancel"),
            ],
        );

        // queue context
        bindings.insert(
            KeyContext::Queue,
            vec![
                KeyBinding::new(KeyCode::Tab, KeyAction::TogglePanel, "Switch Panel"),
                KeyBinding::new(KeyCode::Char('j'), KeyAction::MoveDown, "Down"),
                KeyBinding::new(KeyCode::Down, KeyAction::MoveDown, "Down"),
                KeyBinding::new(KeyCode::Char('k'), KeyAction::MoveUp, "Up"),
                KeyBinding::new(KeyCode::Up, KeyAction::MoveUp, "Up"),
                KeyBinding::new(KeyCode::Enter, KeyAction::Interact, "Expand"),
                KeyBinding::new(KeyCode::Char('['), KeyAction::HideTrackInfo, "Back"),
                KeyBinding::new(KeyCode::Char(']'), KeyAction::ShowTrackInfo, "Track Info"),
                KeyBinding::new(KeyCode::Char('d'), KeyAction::RemoveFromQueue, "Remove"),
                KeyBinding::new(KeyCode::Delete, KeyAction::RemoveFromQueue, "Remove"),
                KeyBinding::new(KeyCode::Char('d'), KeyAction::ClearQueue, "Clear All")
                    .with_mod(KeyModifiers::SHIFT),
            ],
        );

        // global context
        bindings.insert(
            KeyContext::Global,
            vec![
                KeyBinding::new(KeyCode::Char('q'), KeyAction::Quit, "Quit"),
                KeyBinding::new(KeyCode::Char('?'), KeyAction::OpenHelp, "Help"),
                KeyBinding::new(KeyCode::Char(':'), KeyAction::EnterCommandMode, "Command"),
                KeyBinding::new(KeyCode::Char(' '), KeyAction::PlayPause, "Play/Pause"),
                KeyBinding::new(KeyCode::Enter, KeyAction::PlaySelected, "Play"),
                KeyBinding::new(KeyCode::Left, KeyAction::PlayPrevTrack, "Prev Track")
                    .with_mod(KeyModifiers::CONTROL),
                KeyBinding::new(KeyCode::Right, KeyAction::PlayNextTrack, "Next Track")
                    .with_mod(KeyModifiers::CONTROL),
                KeyBinding::new(KeyCode::Char('x'), KeyAction::Stop, "Stop"),
                KeyBinding::new(KeyCode::F(5), KeyAction::ScanLibrary, "Scan"),
                KeyBinding::new(KeyCode::BackTab, KeyAction::ToggleVisualizer, "Visualizer")
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::Char(','), KeyAction::SeekBackward, "Seek -5s"),
                KeyBinding::new(KeyCode::Char('<'), KeyAction::SeekBackward, "Seek -5s"),
                KeyBinding::new(KeyCode::Char('.'), KeyAction::SeekForward, "Seek +5s"),
                KeyBinding::new(KeyCode::Char('>'), KeyAction::SeekForward, "Seek +5s"),
                KeyBinding::new(KeyCode::Esc, KeyAction::CancelDownload, "Cancel DL"),
                KeyBinding::new(KeyCode::Char('+'), KeyAction::VolUp, "Vol+"),
                KeyBinding::new(KeyCode::Char('='), KeyAction::VolUp, "Vol+"),
                KeyBinding::new(KeyCode::Char('-'), KeyAction::VolDown, "Vol-"),
                KeyBinding::new(KeyCode::Char('_'), KeyAction::VolDown, "Vol-"),
                KeyBinding::new(KeyCode::Char('z'), KeyAction::ToggleShuffle, "Shuffle"),
                KeyBinding::new(KeyCode::Char('c'), KeyAction::CycleRepeat, "Repeat"),
                KeyBinding::new(KeyCode::Char('s'), KeyAction::SpeedUp, "Speed+"),
                KeyBinding::new(KeyCode::Char('s'), KeyAction::SpeedDown, "Speed-")
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::Char('b'), KeyAction::BassUp, "Bass+"),
                KeyBinding::new(KeyCode::Char('b'), KeyAction::BassDown, "Bass-")
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::Char('t'), KeyAction::TrebleUp, "Treble+"),
                KeyBinding::new(KeyCode::Char('t'), KeyAction::TrebleDown, "Treble-")
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::Char('r'), KeyAction::ReverbUp, "Reverb+"),
                KeyBinding::new(KeyCode::Char('r'), KeyAction::ReverbDown, "Reverb-")
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::Char('e'), KeyAction::EchoUp, "Echo+"),
                KeyBinding::new(KeyCode::Char('e'), KeyAction::EchoDown, "Echo-")
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::F(1), KeyAction::CycleFocusMode, "Focus Mode"),
                KeyBinding::new(KeyCode::Char('h'), KeyAction::ShrinkSidebar, "Sidebar-")
                    .with_mod(KeyModifiers::CONTROL),
                KeyBinding::new(KeyCode::Char('l'), KeyAction::GrowSidebar, "Sidebar+")
                    .with_mod(KeyModifiers::CONTROL),
            ],
        );

        Self { bindings }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_apply_overrides_conflict_resolution() {
        let mut registry = KeyRegistry::default();

        // Validate the default state: <Space> maps to PlayPause in Global
        let global_bindings = registry.bindings.get(&KeyContext::Global).unwrap();
        let space_binding = global_bindings
            .iter()
            .find(|b| b.key == KeyCode::Char(' '))
            .unwrap();
        assert_eq!(space_binding.action, KeyAction::PlayPause);

        // Simulate reading ~/.config/melotui/keymaps.toml
        // [global]
        // "<Space>" = "ToggleVisualizer"
        // "x" = "Quit"
        let mut global_map = HashMap::new();
        global_map.insert("<Space>".to_string(), "ToggleVisualizer".to_string());
        global_map.insert("x".to_string(), "Quit".to_string());

        let mut overrides = HashMap::new();
        overrides.insert("global".to_string(), global_map);

        // Apply overrides
        registry.apply_overrides(&overrides);

        // Validate the new state
        let new_global = registry.bindings.get(&KeyContext::Global).unwrap();

        // 1. Ensure <Space> now maps to ToggleVisualizer, and the old PlayPause mapping on Space is DEAD.
        let space_bindings: Vec<_> = new_global
            .iter()
            .filter(|b| b.key == KeyCode::Char(' '))
            .collect();
        assert_eq!(
            space_bindings.len(),
            1,
            "There should only be one action bound to space to prevent double triggers"
        );
        assert_eq!(space_bindings[0].action, KeyAction::ToggleVisualizer);

        // 2. Ensure 'x' now maps to Quit.
        // Wait, 'x' was mapped to Stop by default! The default Stop on 'x' should be erased.
        let x_bindings: Vec<_> = new_global
            .iter()
            .filter(|b| b.key == KeyCode::Char('x'))
            .collect();
        assert_eq!(
            x_bindings.len(),
            1,
            "The default 'x' binding should be wiped out"
        );
        assert_eq!(x_bindings[0].action, KeyAction::Quit);

        // 3. Ensure an unaffected binding like 'c' (CycleRepeat) is still intact
        let c_binding = new_global
            .iter()
            .find(|b| b.key == KeyCode::Char('c'))
            .unwrap();
        assert_eq!(c_binding.action, KeyAction::CycleRepeat);
    }

    #[test]
    fn test_apply_overrides_shifted_symbols() {
        let mut registry = KeyRegistry::default();

        let mut global_map = HashMap::new();
        global_map.insert("<S-.>".to_string(), "SeekForward".to_string());
        global_map.insert("<S-,>".to_string(), "SeekBackward".to_string());
        global_map.insert(".".to_string(), "SeekForward".to_string());
        global_map.insert(",".to_string(), "SeekBackward".to_string());

        let mut overrides = HashMap::new();
        overrides.insert("global".to_string(), global_map);

        registry.apply_overrides(&overrides);
        let new_global = registry.bindings.get(&KeyContext::Global).unwrap();

        // Check if raw `.` and `,` are successfully mapped to SeekForward / SeekBackward
        let dot_bindings: Vec<_> = new_global
            .iter()
            .filter(|b| b.key == KeyCode::Char('.') && b.modifiers.is_empty())
            .collect();
        assert_eq!(dot_bindings.len(), 1);
        assert_eq!(dot_bindings[0].action, KeyAction::SeekForward);

        let comma_bindings: Vec<_> = new_global
            .iter()
            .filter(|b| b.key == KeyCode::Char(',') && b.modifiers.is_empty())
            .collect();
        assert_eq!(comma_bindings.len(), 1);
        assert_eq!(comma_bindings[0].action, KeyAction::SeekBackward);

        // Check if shifted `<S-.>` and `<S-,>` are successfully mapped
        let s_dot_bindings: Vec<_> = new_global
            .iter()
            .filter(|b| b.key == KeyCode::Char('.') && b.modifiers.contains(KeyModifiers::SHIFT))
            .collect();
        assert_eq!(s_dot_bindings.len(), 1);
        assert_eq!(s_dot_bindings[0].action, KeyAction::SeekForward);

        let s_comma_bindings: Vec<_> = new_global
            .iter()
            .filter(|b| b.key == KeyCode::Char(',') && b.modifiers.contains(KeyModifiers::SHIFT))
            .collect();
        assert_eq!(s_comma_bindings.len(), 1);
        assert_eq!(s_comma_bindings[0].action, KeyAction::SeekBackward);
    }
}
