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
    PresetPicker,
    Confirm,
    LyricsSelector,
    DspStudio,
    Mixer,
    Global,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyAction {
    // system
    Quit,
    OpenHelp,
    CloseHelp,
    HelpDown,
    HelpUp,
    HelpPageDown,
    HelpPageUp,
    HelpTop,
    HelpBottom,
    EnterCommandMode,
    ExitCommandMode,
    SubmitCommand,
    Autocomplete,
    AutocompleteReverse,
    CommandHistoryPrev,
    CommandHistoryNext,

    // navigation
    TogglePanel,
    ToggleVisualizer,
    CycleVisualizerStyle,
    ToggleLyrics,
    TogglePlaybackPanel,
    MoveUp,
    MoveDown,
    MoveTop,
    MoveBottom,
    MovePageUp,
    MovePageDown,
    ToggleTrackInfo,
    SeekForwardLong,
    SeekBackwardLong,
    Mute,
    SpeedReset,
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
    ToggleSort,

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

    // preset picker
    EnterPresetPicker,
    ExitPresetPicker,
    SubmitPresetPicker,
    PresetPickerMoveUp,
    PresetPickerMoveDown,
    PresetPickerBackspace,

    // lyrics selector
    LyricsSelectorUp,
    LyricsSelectorDown,
    LyricsSelectorLeft,
    LyricsSelectorRight,
    LyricsSelectorSubmit,
    LyricsSelectorCancel,

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

    // DSP Studio
    ToggleDspStudio,
    DspSelectUp,
    DspSelectDown,
    DspSelectLeft,
    DspSelectRight,
    DspAdjustUp,
    DspAdjustDown,
    DspResetControl,
    DspMacroSlowedReverb,
    DspMacroBassBoosted,
    DspMacroSpatial,
    DspMacroResetAll,
    MixerUp,
    MixerDown,
    MixerLeft,
    MixerRight,
    LeaveMixer,
}

impl KeyAction {
    pub fn description(&self) -> &'static str {
        match self {
            KeyAction::Quit => "Quit",
            KeyAction::OpenHelp => "Help",
            KeyAction::CloseHelp => "Close",
            KeyAction::HelpDown => "Scroll Down",
            KeyAction::HelpUp => "Scroll Up",
            KeyAction::HelpPageDown => "Page Down",
            KeyAction::HelpPageUp => "Page Up",
            KeyAction::HelpTop => "Top",
            KeyAction::HelpBottom => "Bottom",
            KeyAction::EnterCommandMode => "Command",
            KeyAction::ExitCommandMode => "Close",
            KeyAction::SubmitCommand => "Submit",
            KeyAction::Autocomplete => "Autocomplete",
            KeyAction::AutocompleteReverse => "Autocomplete Reverse",
            KeyAction::CommandHistoryPrev => "History Prev",
            KeyAction::CommandHistoryNext => "History Next",
            KeyAction::TogglePanel => "Switch Panel",
            KeyAction::ToggleVisualizer => "Art / Visualizer",
            KeyAction::CycleVisualizerStyle => "Visualizer Style",
            KeyAction::ToggleLyrics => "Lyrics",
            KeyAction::TogglePlaybackPanel => "Playback Panel",
            KeyAction::MoveUp => "Up",
            KeyAction::MoveDown => "Down",
            KeyAction::MoveTop => "Top",
            KeyAction::MoveBottom => "Bottom",
            KeyAction::MovePageUp => "Page Up",
            KeyAction::MovePageDown => "Page Down",
            KeyAction::ToggleTrackInfo => "Track Info",
            KeyAction::SeekForwardLong => "Seek Forward 1 min",
            KeyAction::SeekBackwardLong => "Seek Back 1 min",
            KeyAction::Mute => "Mute",
            KeyAction::SpeedReset => "Normal Speed",
            KeyAction::Interact => "Play/Open",
            KeyAction::ToggleFolderExpand => "Expand",
            KeyAction::ShowTrackInfo => "Track Info",
            KeyAction::HideTrackInfo => "Close",
            KeyAction::PlaySelected => "Play",
            KeyAction::PlayNextTrack => "Next Track",
            KeyAction::PlayPrevTrack => "Prev Track",
            KeyAction::PlayPause => "Play/Pause",
            KeyAction::Stop => "Stop",
            KeyAction::SeekForward => "Seek Forward",
            KeyAction::SeekBackward => "Seek Back",
            KeyAction::ToggleShuffle => "Shuffle",
            KeyAction::ToggleSort => "Sort",
            KeyAction::CycleRepeat => "Repeat",
            KeyAction::AddToQueue => "Add Queue",
            KeyAction::QueuePlayNext => "Play Next",
            KeyAction::RemoveFromQueue => "Remove",
            KeyAction::ClearQueue => "Clear All",
            KeyAction::ScanLibrary => "Scan",
            KeyAction::CancelDownload => "Cancel",
            KeyAction::ToggleFavorite => "Favorite",
            KeyAction::SearchNext => "Next Match",
            KeyAction::SearchPrev => "Prev Match",
            KeyAction::VolUp => "Vol+",
            KeyAction::VolDown => "Vol-",
            KeyAction::SpeedUp => "Speed+",
            KeyAction::SpeedDown => "Speed-",
            KeyAction::BassUp => "Bass+",
            KeyAction::BassDown => "Bass-",
            KeyAction::TrebleUp => "Treble+",
            KeyAction::TrebleDown => "Treble-",
            KeyAction::ReverbUp => "Reverb+",
            KeyAction::ReverbDown => "Reverb-",
            KeyAction::EchoUp => "Echo+",
            KeyAction::EchoDown => "Echo-",
            KeyAction::EnterPlaylistPicker => "Add to Playlist",
            KeyAction::ExitPlaylistPicker => "Close",
            KeyAction::SubmitPlaylistPicker => "Submit",
            KeyAction::PlaylistPickerMoveUp => "Up",
            KeyAction::PlaylistPickerMoveDown => "Down",
            KeyAction::PlaylistPickerBackspace => "Backspace",
            KeyAction::EnterPresetPicker => "Presets",
            KeyAction::ExitPresetPicker => "Close",
            KeyAction::SubmitPresetPicker => "Submit",
            KeyAction::PresetPickerMoveUp => "Up",
            KeyAction::PresetPickerMoveDown => "Down",
            KeyAction::PresetPickerBackspace => "Backspace",
            KeyAction::LyricsSelectorUp => "Up",
            KeyAction::LyricsSelectorDown => "Down",
            KeyAction::LyricsSelectorLeft => "Left",
            KeyAction::LyricsSelectorRight => "Right",
            KeyAction::LyricsSelectorSubmit => "Select",
            KeyAction::LyricsSelectorCancel => "Cancel",
            KeyAction::DeletePlaylist => "Delete Playlist",
            KeyAction::RemoveFromPlaylist => "Rm from Playlist",
            KeyAction::ConfirmYes => "Confirm",
            KeyAction::ConfirmNo => "Cancel",
            KeyAction::CycleFocusMode => "Focus Mode",
            KeyAction::ShrinkSidebar => "Sidebar-",
            KeyAction::GrowSidebar => "Sidebar+",
            KeyAction::ToggleDspStudio => "DSP Studio",
            KeyAction::DspSelectUp => "DSP Up",
            KeyAction::DspSelectDown => "DSP Down",
            KeyAction::DspSelectLeft => "DSP Left",
            KeyAction::DspSelectRight => "DSP Right",
            KeyAction::DspAdjustUp => "Increase",
            KeyAction::DspAdjustDown => "Decrease",
            KeyAction::DspResetControl => "Reset",
            KeyAction::DspMacroSlowedReverb => "Slowed+Reverb",
            KeyAction::DspMacroBassBoosted => "Bass Boosted",
            KeyAction::DspMacroSpatial => "3D Spatial",
            KeyAction::DspMacroResetAll => "Reset All DSP",
            KeyAction::MixerUp => "Up",
            KeyAction::MixerDown => "Down",
            KeyAction::MixerLeft => "Left",
            KeyAction::MixerRight => "Right",
            KeyAction::LeaveMixer => "Leave",
        }
    }
}

impl KeyAction {
    pub fn to_msg(&self, config: &melotui_core::config::AppConfig) -> Option<AppMsg> {
        match self {
            KeyAction::Quit => Some(AppMsg::RequestQuit),
            KeyAction::OpenHelp => Some(AppMsg::UI(UIMsg::OpenHelp)),
            KeyAction::CloseHelp => Some(AppMsg::UI(UIMsg::CloseHelp)),
            // How far it can go is filled in by whoever drew it.
            KeyAction::HelpDown => Some(AppMsg::UI(UIMsg::ScrollHelp { by: 1, max: 0 })),
            KeyAction::HelpUp => Some(AppMsg::UI(UIMsg::ScrollHelp { by: -1, max: 0 })),
            KeyAction::HelpPageDown => Some(AppMsg::UI(UIMsg::ScrollHelp { by: 15, max: 0 })),
            KeyAction::HelpPageUp => Some(AppMsg::UI(UIMsg::ScrollHelp { by: -15, max: 0 })),
            KeyAction::HelpTop => Some(AppMsg::UI(UIMsg::ScrollHelp {
                by: i32::MIN,
                max: 0,
            })),
            KeyAction::HelpBottom => Some(AppMsg::UI(UIMsg::ScrollHelp {
                by: i32::MAX,
                max: 0,
            })),

            KeyAction::EnterCommandMode => Some(AppMsg::UI(UIMsg::EnterCommandMode)),
            KeyAction::ExitCommandMode => Some(AppMsg::UI(UIMsg::ExitCommandMode)),

            KeyAction::MoveUp => Some(AppMsg::Navigation(NavigationMsg::MoveUp)),
            KeyAction::MoveDown => Some(AppMsg::Navigation(NavigationMsg::MoveDown)),
            KeyAction::MoveTop => Some(AppMsg::Navigation(NavigationMsg::MoveTop)),
            KeyAction::MoveBottom => Some(AppMsg::Navigation(NavigationMsg::MoveBottom)),
            KeyAction::MovePageUp => {
                Some(AppMsg::Navigation(NavigationMsg::MovePage { down: false }))
            }
            KeyAction::MovePageDown => {
                Some(AppMsg::Navigation(NavigationMsg::MovePage { down: true }))
            }
            KeyAction::ToggleTrackInfo => Some(AppMsg::Navigation(NavigationMsg::ToggleTrackInfo)),
            KeyAction::SeekForwardLong => {
                Some(AppMsg::Playback(PlaybackMsg::RequestSeekRelative(60)))
            }
            KeyAction::SeekBackwardLong => {
                Some(AppMsg::Playback(PlaybackMsg::RequestSeekRelative(-60)))
            }
            KeyAction::Mute => Some(AppMsg::Audio(AudioMsg::ToggleMute)),
            KeyAction::SpeedReset => Some(AppMsg::Audio(AudioMsg::ResetSpeed)),
            KeyAction::TogglePanel => Some(AppMsg::Navigation(NavigationMsg::ToggleLibraryQueue)),
            KeyAction::ToggleVisualizer => Some(AppMsg::Navigation(NavigationMsg::ToggleArt)),
            KeyAction::CycleVisualizerStyle => {
                Some(AppMsg::Navigation(NavigationMsg::CycleVisualizerStyle))
            }
            KeyAction::ToggleLyrics => Some(AppMsg::UI(UIMsg::ToggleLyrics)),
            KeyAction::TogglePlaybackPanel => Some(AppMsg::UI(UIMsg::TogglePlaybackPanel)),
            KeyAction::ToggleFolderExpand => {
                Some(AppMsg::Navigation(NavigationMsg::ToggleTreeExpand))
            }
            KeyAction::LyricsSelectorUp => Some(AppMsg::UI(UIMsg::LyricsSelectorMoveUp)),
            KeyAction::LyricsSelectorDown => Some(AppMsg::UI(UIMsg::LyricsSelectorMoveDown)),
            KeyAction::LyricsSelectorLeft => Some(AppMsg::UI(UIMsg::LyricsSelectorMoveLeft)),
            KeyAction::LyricsSelectorRight => Some(AppMsg::UI(UIMsg::LyricsSelectorMoveRight)),
            KeyAction::LyricsSelectorSubmit => Some(AppMsg::UI(UIMsg::LyricsSelectorSubmit)),
            KeyAction::LyricsSelectorCancel => Some(AppMsg::UI(UIMsg::LyricsSelectorCancel)),
            KeyAction::Interact => None, // handled in input.rs
            KeyAction::ShowTrackInfo => Some(AppMsg::Navigation(NavigationMsg::ToggleTrackInfo)),
            KeyAction::HideTrackInfo => Some(AppMsg::Navigation(NavigationMsg::ToggleTrackInfo)),

            KeyAction::PlaySelected => Some(AppMsg::Playback(PlaybackMsg::RequestPlaySelected)),
            KeyAction::PlayNextTrack => Some(AppMsg::Playback(PlaybackMsg::RequestPlayNext)),
            KeyAction::PlayPrevTrack => Some(AppMsg::Playback(PlaybackMsg::RequestPlayPrev)),
            KeyAction::PlayPause => Some(AppMsg::Playback(PlaybackMsg::RequestTogglePlayback)),
            KeyAction::Stop => Some(AppMsg::Playback(PlaybackMsg::RequestStop)),
            KeyAction::SeekForward => Some(AppMsg::Playback(PlaybackMsg::RequestSeekRelative(
                config.audio.seek_step,
            ))),
            KeyAction::SeekBackward => Some(AppMsg::Playback(PlaybackMsg::RequestSeekRelative(
                -config.audio.seek_step,
            ))),

            KeyAction::ToggleShuffle => Some(AppMsg::Library(LibraryMsg::ToggleShuffle)),
            KeyAction::ToggleSort => Some(AppMsg::Library(LibraryMsg::ToggleSort)),
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
            KeyAction::Autocomplete => Some(AppMsg::UI(UIMsg::Autocomplete)),
            KeyAction::AutocompleteReverse => Some(AppMsg::UI(UIMsg::AutocompleteReverse)),
            KeyAction::CommandHistoryPrev => Some(AppMsg::UI(UIMsg::CommandHistoryPrev)),
            KeyAction::CommandHistoryNext => Some(AppMsg::UI(UIMsg::CommandHistoryNext)),
            KeyAction::VolUp => Some(AppMsg::Audio(AudioMsg::AdjustVolume(
                config.audio.volume_step,
            ))),
            KeyAction::VolDown => Some(AppMsg::Audio(AudioMsg::AdjustVolume(
                -config.audio.volume_step,
            ))),
            KeyAction::SpeedUp => Some(AppMsg::Audio(AudioMsg::AdjustSpeed(
                config.audio.speed_step,
            ))),
            KeyAction::SpeedDown => Some(AppMsg::Audio(AudioMsg::AdjustSpeed(
                -config.audio.speed_step,
            ))),
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
            KeyAction::EnterPresetPicker => Some(AppMsg::UI(UIMsg::EnterPresetPicker)),
            KeyAction::ExitPresetPicker => Some(AppMsg::UI(UIMsg::ExitPresetPicker)),
            KeyAction::SubmitPresetPicker => Some(AppMsg::UI(UIMsg::SubmitPresetPicker)),
            KeyAction::PresetPickerMoveUp => Some(AppMsg::UI(UIMsg::PresetPickerMoveUp)),
            KeyAction::PresetPickerMoveDown => Some(AppMsg::UI(UIMsg::PresetPickerMoveDown)),
            KeyAction::PresetPickerBackspace => Some(AppMsg::UI(UIMsg::PresetPickerBackspace)),
            KeyAction::DeletePlaylist => Some(AppMsg::UI(UIMsg::DeletePlaylistConfirm)),
            KeyAction::RemoveFromPlaylist => None, // id resolved in input.rs
            KeyAction::ConfirmYes => Some(AppMsg::UI(UIMsg::ConfirmYes)),
            KeyAction::ConfirmNo => Some(AppMsg::UI(UIMsg::ConfirmNo)),
            KeyAction::CycleFocusMode => Some(AppMsg::UI(UIMsg::CycleFocusMode)),
            KeyAction::ShrinkSidebar => Some(AppMsg::UI(UIMsg::AdjustSidebarWidth(-2))),
            KeyAction::GrowSidebar => Some(AppMsg::UI(UIMsg::AdjustSidebarWidth(2))),
            KeyAction::ToggleDspStudio => Some(AppMsg::UI(UIMsg::ToggleDspStudio)),
            KeyAction::DspSelectUp => Some(AppMsg::UI(UIMsg::DspSelectUp)),
            KeyAction::DspSelectDown => Some(AppMsg::UI(UIMsg::DspSelectDown)),
            KeyAction::DspSelectLeft => Some(AppMsg::UI(UIMsg::DspSelectLeft)),
            KeyAction::DspSelectRight => Some(AppMsg::UI(UIMsg::DspSelectRight)),
            KeyAction::DspAdjustUp => Some(AppMsg::UI(UIMsg::DspAdjustUp)),
            KeyAction::DspAdjustDown => Some(AppMsg::UI(UIMsg::DspAdjustDown)),
            KeyAction::DspResetControl => Some(AppMsg::UI(UIMsg::DspResetControl)),
            KeyAction::DspMacroSlowedReverb => Some(AppMsg::UI(UIMsg::DspMacroSlowedReverb)),
            KeyAction::DspMacroBassBoosted => Some(AppMsg::UI(UIMsg::DspMacroBassBoosted)),
            KeyAction::DspMacroSpatial => Some(AppMsg::UI(UIMsg::DspMacroSpatial)),
            KeyAction::DspMacroResetAll => Some(AppMsg::UI(UIMsg::DspMacroResetAll)),
            KeyAction::MixerUp => Some(AppMsg::UI(UIMsg::MixerSelect {
                columns: 0,
                rows: -1,
            })),
            KeyAction::MixerDown => Some(AppMsg::UI(UIMsg::MixerSelect {
                columns: 0,
                rows: 1,
            })),
            KeyAction::MixerLeft => Some(AppMsg::UI(UIMsg::MixerSelect {
                columns: -1,
                rows: 0,
            })),
            KeyAction::MixerRight => Some(AppMsg::UI(UIMsg::MixerSelect {
                columns: 1,
                rows: 0,
            })),
            KeyAction::LeaveMixer => Some(AppMsg::Navigation(NavigationMsg::LeaveMixer)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub key: KeyCode,
    pub modifiers: KeyModifiers,
    pub action: KeyAction,
}

impl KeyBinding {
    pub fn new(code: KeyCode, action: KeyAction) -> Self {
        Self {
            key: code,
            modifiers: KeyModifiers::empty(),
            action,
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
                "preset" | "presetpicker" => KeyContext::PresetPicker,
                "confirm" => KeyContext::Confirm,
                "dspstudio" | "dsp" => KeyContext::DspStudio,
                "mixer" => KeyContext::Mixer,
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
                            // Inherit description from defaults to avoid breaking Help UI grouping
                            // Remove default bindings with the exact same key to avoid double-trigger
                            if let Some(ctx_bindings) = self.bindings.get_mut(&ctx) {
                                ctx_bindings.retain(|b| !(b.key == code && b.modifiers == mods));
                                ctx_bindings.push(KeyBinding {
                                    key: code,
                                    modifiers: mods,
                                    action,
                                });
                            } else {
                                self.bindings.insert(
                                    ctx,
                                    vec![KeyBinding {
                                        key: code,
                                        modifiers: mods,
                                        action,
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
                KeyBinding::new(KeyCode::Esc, KeyAction::CloseHelp),
                KeyBinding::new(KeyCode::Char('q'), KeyAction::CloseHelp),
                KeyBinding::new(KeyCode::Char('?'), KeyAction::CloseHelp),
                KeyBinding::new(KeyCode::Char('j'), KeyAction::HelpDown),
                KeyBinding::new(KeyCode::Down, KeyAction::HelpDown),
                KeyBinding::new(KeyCode::Char('k'), KeyAction::HelpUp),
                KeyBinding::new(KeyCode::Up, KeyAction::HelpUp),
                KeyBinding::new(KeyCode::PageDown, KeyAction::HelpPageDown),
                KeyBinding::new(KeyCode::Char(' '), KeyAction::HelpPageDown),
                KeyBinding::new(KeyCode::PageUp, KeyAction::HelpPageUp),
                KeyBinding::new(KeyCode::Char('g'), KeyAction::HelpTop),
                KeyBinding::new(KeyCode::Home, KeyAction::HelpTop),
                // A capital comes as its small letter with shift held.
                KeyBinding::new(KeyCode::Char('g'), KeyAction::HelpBottom)
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::End, KeyAction::HelpBottom),
            ],
        );

        bindings.insert(
            KeyContext::TrackInfo,
            vec![KeyBinding::new(
                KeyCode::Char('i'),
                KeyAction::ToggleTrackInfo,
            )],
        );

        // command context
        bindings.insert(
            KeyContext::CommandMode,
            vec![
                KeyBinding::new(KeyCode::Esc, KeyAction::ExitCommandMode),
                KeyBinding::new(KeyCode::Enter, KeyAction::SubmitCommand),
                KeyBinding::new(KeyCode::Tab, KeyAction::Autocomplete),
                KeyBinding::new(KeyCode::BackTab, KeyAction::AutocompleteReverse)
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::Up, KeyAction::CommandHistoryPrev),
                KeyBinding::new(KeyCode::Down, KeyAction::CommandHistoryNext),
            ],
        );

        // library context
        bindings.insert(
            KeyContext::Library,
            vec![
                KeyBinding::new(KeyCode::Tab, KeyAction::TogglePanel),
                KeyBinding::new(KeyCode::Char('j'), KeyAction::MoveDown),
                KeyBinding::new(KeyCode::Down, KeyAction::MoveDown),
                KeyBinding::new(KeyCode::Char('k'), KeyAction::MoveUp),
                KeyBinding::new(KeyCode::Up, KeyAction::MoveUp),
                KeyBinding::new(KeyCode::Char('g'), KeyAction::MoveTop),
                KeyBinding::new(KeyCode::Home, KeyAction::MoveTop),
                KeyBinding::new(KeyCode::Char('g'), KeyAction::MoveBottom)
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::End, KeyAction::MoveBottom),
                KeyBinding::new(KeyCode::PageUp, KeyAction::MovePageUp),
                KeyBinding::new(KeyCode::PageDown, KeyAction::MovePageDown),
                KeyBinding::new(KeyCode::Enter, KeyAction::Interact),
                KeyBinding::new(KeyCode::Char('a'), KeyAction::AddToQueue),
                KeyBinding::new(KeyCode::Char('a'), KeyAction::QueuePlayNext)
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::Char('f'), KeyAction::ToggleFavorite),
                KeyBinding::new(KeyCode::Char('n'), KeyAction::SearchNext),
                KeyBinding::new(KeyCode::Char('n'), KeyAction::SearchPrev)
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::Char('p'), KeyAction::EnterPlaylistPicker),
                KeyBinding::new(KeyCode::Char('p'), KeyAction::RemoveFromPlaylist)
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::Char('d'), KeyAction::DeletePlaylist)
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::Char('o'), KeyAction::ToggleSort),
            ],
        );

        // playlist picker context
        bindings.insert(
            KeyContext::PlaylistPicker,
            vec![
                KeyBinding::new(KeyCode::Esc, KeyAction::ExitPlaylistPicker),
                KeyBinding::new(KeyCode::Enter, KeyAction::SubmitPlaylistPicker),
                KeyBinding::new(KeyCode::Down, KeyAction::PlaylistPickerMoveDown),
                KeyBinding::new(KeyCode::Up, KeyAction::PlaylistPickerMoveUp),
                KeyBinding::new(KeyCode::Backspace, KeyAction::PlaylistPickerBackspace),
            ],
        );

        // preset picker
        bindings.insert(
            KeyContext::PresetPicker,
            vec![
                KeyBinding::new(KeyCode::Esc, KeyAction::ExitPresetPicker),
                KeyBinding::new(KeyCode::Enter, KeyAction::SubmitPresetPicker),
                KeyBinding::new(KeyCode::Down, KeyAction::PresetPickerMoveDown),
                KeyBinding::new(KeyCode::Up, KeyAction::PresetPickerMoveUp),
                KeyBinding::new(KeyCode::Backspace, KeyAction::PresetPickerBackspace),
            ],
        );

        // confirm dialog
        bindings.insert(
            KeyContext::Confirm,
            vec![
                KeyBinding::new(KeyCode::Char('y'), KeyAction::ConfirmYes),
                KeyBinding::new(KeyCode::Char('n'), KeyAction::ConfirmNo),
                KeyBinding::new(KeyCode::Esc, KeyAction::ConfirmNo),
            ],
        );

        // queue context
        bindings.insert(
            KeyContext::Queue,
            vec![
                KeyBinding::new(KeyCode::Tab, KeyAction::TogglePanel),
                KeyBinding::new(KeyCode::Char('j'), KeyAction::MoveDown),
                KeyBinding::new(KeyCode::Down, KeyAction::MoveDown),
                KeyBinding::new(KeyCode::Char('k'), KeyAction::MoveUp),
                KeyBinding::new(KeyCode::Up, KeyAction::MoveUp),
                KeyBinding::new(KeyCode::Char('g'), KeyAction::MoveTop),
                KeyBinding::new(KeyCode::Home, KeyAction::MoveTop),
                KeyBinding::new(KeyCode::Char('g'), KeyAction::MoveBottom)
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::End, KeyAction::MoveBottom),
                KeyBinding::new(KeyCode::PageUp, KeyAction::MovePageUp),
                KeyBinding::new(KeyCode::PageDown, KeyAction::MovePageDown),
                KeyBinding::new(KeyCode::Enter, KeyAction::Interact),
                KeyBinding::new(KeyCode::Char('d'), KeyAction::RemoveFromQueue),
                KeyBinding::new(KeyCode::Delete, KeyAction::RemoveFromQueue),
                KeyBinding::new(KeyCode::Char('d'), KeyAction::ClearQueue)
                    .with_mod(KeyModifiers::SHIFT),
            ],
        );

        // lyrics selector context
        bindings.insert(
            KeyContext::LyricsSelector,
            vec![
                KeyBinding::new(KeyCode::Up, KeyAction::LyricsSelectorUp),
                KeyBinding::new(KeyCode::Char('k'), KeyAction::LyricsSelectorUp),
                KeyBinding::new(KeyCode::Down, KeyAction::LyricsSelectorDown),
                KeyBinding::new(KeyCode::Char('j'), KeyAction::LyricsSelectorDown),
                KeyBinding::new(KeyCode::Left, KeyAction::LyricsSelectorLeft),
                KeyBinding::new(KeyCode::Char('h'), KeyAction::LyricsSelectorLeft),
                KeyBinding::new(KeyCode::Right, KeyAction::LyricsSelectorRight),
                KeyBinding::new(KeyCode::Char('l'), KeyAction::LyricsSelectorRight),
                KeyBinding::new(KeyCode::Enter, KeyAction::LyricsSelectorSubmit),
                KeyBinding::new(KeyCode::Esc, KeyAction::LyricsSelectorCancel),
            ],
        );

        // global context — the keys cmus, ncmpcpp and mpv share where they
        // agree: space pauses, < > change track, arrows seek, [ ] set the
        // speed, s and r shuffle and repeat, i shows the track, l the lyrics.
        // Effects are adjusted where they show: in the mixer, moved on to
        // with Tab, and in the DSP studio.
        bindings.insert(
            KeyContext::Global,
            vec![
                KeyBinding::new(KeyCode::Char(' '), KeyAction::PlayPause),
                KeyBinding::new(KeyCode::Enter, KeyAction::PlaySelected),
                KeyBinding::new(KeyCode::Char('>'), KeyAction::PlayNextTrack),
                KeyBinding::new(KeyCode::Right, KeyAction::PlayNextTrack)
                    .with_mod(KeyModifiers::CONTROL),
                KeyBinding::new(KeyCode::Char('<'), KeyAction::PlayPrevTrack),
                KeyBinding::new(KeyCode::Left, KeyAction::PlayPrevTrack)
                    .with_mod(KeyModifiers::CONTROL),
                KeyBinding::new(KeyCode::Right, KeyAction::SeekForward),
                KeyBinding::new(KeyCode::Left, KeyAction::SeekBackward),
                KeyBinding::new(KeyCode::Char('.'), KeyAction::SeekForwardLong),
                KeyBinding::new(KeyCode::Char(','), KeyAction::SeekBackwardLong),
                KeyBinding::new(KeyCode::Char('s'), KeyAction::Stop).with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::Char('+'), KeyAction::VolUp),
                KeyBinding::new(KeyCode::Char('='), KeyAction::VolUp),
                KeyBinding::new(KeyCode::Char('-'), KeyAction::VolDown),
                KeyBinding::new(KeyCode::Char('m'), KeyAction::Mute),
                KeyBinding::new(KeyCode::Char(']'), KeyAction::SpeedUp),
                KeyBinding::new(KeyCode::Char('['), KeyAction::SpeedDown),
                KeyBinding::new(KeyCode::Backspace, KeyAction::SpeedReset),
                KeyBinding::new(KeyCode::Char('s'), KeyAction::ToggleShuffle),
                KeyBinding::new(KeyCode::Char('r'), KeyAction::CycleRepeat),
                KeyBinding::new(KeyCode::Char('i'), KeyAction::ToggleTrackInfo),
                KeyBinding::new(KeyCode::Char('l'), KeyAction::ToggleLyrics),
                KeyBinding::new(KeyCode::Char('v'), KeyAction::ToggleVisualizer),
                KeyBinding::new(KeyCode::Char('v'), KeyAction::CycleVisualizerStyle)
                    .with_mod(KeyModifiers::SHIFT),
                KeyBinding::new(KeyCode::F(3), KeyAction::CycleVisualizerStyle),
                KeyBinding::new(KeyCode::F(1), KeyAction::CycleFocusMode),
                KeyBinding::new(KeyCode::F(2), KeyAction::ToggleDspStudio),
                KeyBinding::new(KeyCode::Char('l'), KeyAction::GrowSidebar)
                    .with_mod(KeyModifiers::CONTROL),
                KeyBinding::new(KeyCode::Char('h'), KeyAction::ShrinkSidebar)
                    .with_mod(KeyModifiers::CONTROL),
                KeyBinding::new(KeyCode::F(5), KeyAction::ScanLibrary),
                KeyBinding::new(KeyCode::Char(':'), KeyAction::EnterCommandMode),
                KeyBinding::new(KeyCode::Char('?'), KeyAction::OpenHelp),
                KeyBinding::new(KeyCode::Esc, KeyAction::CancelDownload),
                KeyBinding::new(KeyCode::Char('q'), KeyAction::Quit),
            ],
        );

        // the mixer, moved on to with Tab: as in the DSP studio, the arrows
        // pick a control and + - adjust it
        bindings.insert(
            KeyContext::Mixer,
            vec![
                KeyBinding::new(KeyCode::Tab, KeyAction::TogglePanel),
                KeyBinding::new(KeyCode::Esc, KeyAction::LeaveMixer),
                KeyBinding::new(KeyCode::Up, KeyAction::MixerUp),
                KeyBinding::new(KeyCode::Char('k'), KeyAction::MixerUp),
                KeyBinding::new(KeyCode::Down, KeyAction::MixerDown),
                KeyBinding::new(KeyCode::Char('j'), KeyAction::MixerDown),
                KeyBinding::new(KeyCode::Left, KeyAction::MixerLeft),
                KeyBinding::new(KeyCode::Char('h'), KeyAction::MixerLeft),
                KeyBinding::new(KeyCode::Right, KeyAction::MixerRight),
                KeyBinding::new(KeyCode::Char('l'), KeyAction::MixerRight),
                KeyBinding::new(KeyCode::Char('+'), KeyAction::DspAdjustUp),
                KeyBinding::new(KeyCode::Char('='), KeyAction::DspAdjustUp),
                KeyBinding::new(KeyCode::Char('-'), KeyAction::DspAdjustDown),
                KeyBinding::new(KeyCode::Char('_'), KeyAction::DspAdjustDown),
                KeyBinding::new(KeyCode::Char('r'), KeyAction::DspResetControl),
                KeyBinding::new(KeyCode::Backspace, KeyAction::DspResetControl),
            ],
        );

        // DSP Studio context
        bindings.insert(
            KeyContext::DspStudio,
            vec![
                KeyBinding::new(KeyCode::F(2), KeyAction::ToggleDspStudio),
                KeyBinding::new(KeyCode::Esc, KeyAction::ToggleDspStudio),
                // Navigation (Arrows + Vim spatial)
                KeyBinding::new(KeyCode::Up, KeyAction::DspSelectUp),
                KeyBinding::new(KeyCode::Char('k'), KeyAction::DspSelectUp),
                KeyBinding::new(KeyCode::Down, KeyAction::DspSelectDown),
                KeyBinding::new(KeyCode::Char('j'), KeyAction::DspSelectDown),
                KeyBinding::new(KeyCode::Left, KeyAction::DspSelectLeft),
                KeyBinding::new(KeyCode::Char('h'), KeyAction::DspSelectLeft),
                KeyBinding::new(KeyCode::Right, KeyAction::DspSelectRight),
                KeyBinding::new(KeyCode::Char('l'), KeyAction::DspSelectRight),
                // Value adjust (+/-)
                KeyBinding::new(KeyCode::Char('+'), KeyAction::DspAdjustUp),
                KeyBinding::new(KeyCode::Char('='), KeyAction::DspAdjustUp),
                KeyBinding::new(KeyCode::Char('-'), KeyAction::DspAdjustDown),
                KeyBinding::new(KeyCode::Char('_'), KeyAction::DspAdjustDown),
                KeyBinding::new(KeyCode::Char('r'), KeyAction::DspResetControl),
                KeyBinding::new(KeyCode::Backspace, KeyAction::DspResetControl),
                KeyBinding::new(KeyCode::Char('1'), KeyAction::DspMacroSlowedReverb),
                KeyBinding::new(KeyCode::Char('2'), KeyAction::DspMacroBassBoosted),
                KeyBinding::new(KeyCode::Char('3'), KeyAction::DspMacroSpatial),
                KeyBinding::new(KeyCode::Char('0'), KeyAction::DspMacroResetAll),
                KeyBinding::new(KeyCode::Char('p'), KeyAction::EnterPresetPicker),
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
        // "s" = "Quit"
        let mut global_map = HashMap::new();
        global_map.insert("<Space>".to_string(), "ToggleVisualizer".to_string());
        global_map.insert("s".to_string(), "Quit".to_string());

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

        // 2. 's' was shuffle by default; that binding is wiped out, and
        // Shift+S, a different key, keeps stopping.
        let s_bindings: Vec<_> = new_global
            .iter()
            .filter(|b| b.key == KeyCode::Char('s') && b.modifiers.is_empty())
            .collect();
        assert_eq!(
            s_bindings.len(),
            1,
            "The default 's' binding should be wiped out"
        );
        assert_eq!(s_bindings[0].action, KeyAction::Quit);
        assert!(new_global.iter().any(|b| b.key == KeyCode::Char('s')
            && b.modifiers == KeyModifiers::SHIFT
            && b.action == KeyAction::Stop));

        // 3. An unaffected binding like 'r' (CycleRepeat) is still intact.
        let r_binding = new_global
            .iter()
            .find(|b| b.key == KeyCode::Char('r'))
            .unwrap();
        assert_eq!(r_binding.action, KeyAction::CycleRepeat);
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
