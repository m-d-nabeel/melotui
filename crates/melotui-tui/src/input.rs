use melotui_core::{
    AppMsg, AppState, UIMode,
    message::{
        audio::AudioMsg, download::DownloadMsg, library::LibraryMsg, navigation::NavigationMsg,
        playback::PlaybackMsg, ui::UIMsg,
    },
    state::{LeftPanelMode, LibraryViewDepth},
};
use ratatui::crossterm::event::{KeyCode, KeyEvent};

/// Helper struct for documentation generation
pub struct KeyHelp {
    pub key: &'static str,
    pub desc: &'static str,
    pub category: &'static str,
}

pub struct InputHandler;

impl InputHandler {
    /// The single entry point for all input
    pub fn handle(key: KeyEvent, state: &AppState) -> Option<AppMsg> {
        // 1. Help Overlay Trap
        if state.ui.show_help {
            return match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
                    Some(AppMsg::UI(UIMsg::ToggleHelp))
                }
                _ => None,
            };
        }

        // 2. Command Mode Trap
        if state.ui.mode == UIMode::Command {
            return match key.code {
                KeyCode::Esc => Some(AppMsg::UI(UIMsg::ExitCommandMode)),
                KeyCode::Enter => Some(AppMsg::UI(UIMsg::SubmitCommand(
                    state.ui.command_buffer.clone(),
                ))),
                KeyCode::Backspace => Some(AppMsg::UI(UIMsg::BackspaceCommand)),
                KeyCode::Char(c) => Some(AppMsg::UI(UIMsg::UpdateCommand(c))),
                _ => None,
            };
        }

        // 3. Normal Mode
        match key.code {
            // ───── Global System ─────
            KeyCode::Char('?') => Some(AppMsg::UI(UIMsg::ToggleHelp)),
            KeyCode::Char('q') => Some(AppMsg::RequestQuit),
            KeyCode::Char(':') => Some(AppMsg::UI(UIMsg::EnterCommandMode)),
            KeyCode::F(5) => Some(AppMsg::Library(LibraryMsg::RequestRescanLibrary)),
            KeyCode::Esc => Some(AppMsg::Download(DownloadMsg::RequestCancelDownload)),

            // ───── Panel & View Switching ─────
            // Tab: Switch between Library and Queue
            KeyCode::Tab => Some(AppMsg::Navigation(NavigationMsg::ToggleLibraryQueue)),
            KeyCode::BackTab => Some(AppMsg::Navigation(NavigationMsg::ToggleArt)),
            KeyCode::Down | KeyCode::Char('j') => Some(AppMsg::Navigation(NavigationMsg::MoveDown)),
            KeyCode::Up | KeyCode::Char('k') => Some(AppMsg::Navigation(NavigationMsg::MoveUp)),

            // Left / Back
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('[') => {
                // If in Queue, maybe switch back to Library? Or just do nothing?
                // For now, let's keep column navigation for Library
                if state.nav.active_panel == LeftPanelMode::Library {
                    match state.nav.library.depth {
                        LibraryViewDepth::Tracks => {
                            Some(AppMsg::Navigation(NavigationMsg::SwitchToPlaylists))
                        }
                        LibraryViewDepth::TrackInfo => {
                            Some(AppMsg::Navigation(NavigationMsg::SwitchToTracks))
                        }
                        _ => None,
                    }
                } else {
                    // If in Queue, Left takes you back to Library
                    // Some(AppMsg::ToggleLibraryQueue)
                    None
                }
            }

            // Right / Enter
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char(']') => {
                if state.nav.active_panel == LeftPanelMode::Library {
                    match state.nav.library.depth {
                        LibraryViewDepth::Playlists => {
                            Some(AppMsg::Navigation(NavigationMsg::SwitchToTracks))
                        }
                        LibraryViewDepth::Tracks => {
                            Some(AppMsg::Navigation(NavigationMsg::SwitchToTrackInfo))
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }

            // Play Selected
            KeyCode::Enter => Some(AppMsg::Playback(PlaybackMsg::RequestPlaySelected)),

            // ───── Queue Management ─────
            // 'a': Add current selection to Queue
            KeyCode::Char('a') => {
                // We need to extract the track from the current selection
                // Logic mirrors RequestPlaySelected but wraps it in AddToQueue
                if state.nav.active_panel == LeftPanelMode::Library
                    && state.nav.library.depth != LibraryViewDepth::Playlists
                {
                    let p_idx = state.nav.library.selected_playlist_idx;
                    // Safe access
                    if let Some(playlist) = state.library.playlists.get(p_idx) {
                        let t_idx = *state
                            .nav
                            .library
                            .selected_tracks_curs
                            .get(p_idx)
                            .unwrap_or(&0);
                        if let Some(track) = playlist.tracks.get(t_idx) {
                            return Some(AppMsg::Library(LibraryMsg::AddToQueue(track.clone())));
                        }
                    }
                }
                None
            }

            // 'A' (Shift+a): Play Next (Add to front of queue)
            KeyCode::Char('A') => {
                if state.nav.active_panel == LeftPanelMode::Library
                    && state.nav.library.depth != LibraryViewDepth::Playlists
                {
                    let p_idx = state.nav.library.selected_playlist_idx;
                    if let Some(playlist) = state.library.playlists.get(p_idx) {
                        let t_idx = *state
                            .nav
                            .library
                            .selected_tracks_curs
                            .get(p_idx)
                            .unwrap_or(&0);
                        if let Some(track) = playlist.tracks.get(t_idx) {
                            return Some(AppMsg::Library(LibraryMsg::PlayNext(track.clone())));
                        }
                    }
                }
                None
            }

            // 'd': Remove from Queue (Only valid in Queue Panel)
            KeyCode::Char('d') | KeyCode::Delete => {
                if state.nav.active_panel == LeftPanelMode::Queue {
                    Some(AppMsg::Library(LibraryMsg::RemoveFromQueue))
                } else {
                    None
                }
            }

            // 'D': Clear Queue
            KeyCode::Char('D') => Some(AppMsg::Library(LibraryMsg::ClearQueue)),

            // ───── Playback Control ─────
            KeyCode::Char(' ') => Some(AppMsg::Playback(PlaybackMsg::RequestTogglePlayback)),
            KeyCode::Char('x') => Some(AppMsg::Playback(PlaybackMsg::RequestStop)),

            // Seeking (using comma/period for < > semantic)
            KeyCode::Char(',') | KeyCode::Char('<') => {
                Some(AppMsg::Playback(PlaybackMsg::RequestSeek {
                    offset_seconds: -5,
                }))
            }
            KeyCode::Char('.') | KeyCode::Char('>') => {
                Some(AppMsg::Playback(PlaybackMsg::RequestSeek {
                    offset_seconds: 5,
                }))
            }

            // Modes
            KeyCode::Char('z') => Some(AppMsg::Library(LibraryMsg::ToggleShuffle)),
            KeyCode::Char('c') => Some(AppMsg::Library(LibraryMsg::ToggleRepeatMode)),

            // ───── Audio FX ─────
            // Volume
            KeyCode::Char('+') | KeyCode::Char('=') => {
                Some(AppMsg::Audio(AudioMsg::AdjustVolume(0.05)))
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                Some(AppMsg::Audio(AudioMsg::AdjustVolume(-0.05)))
            }

            // Speed
            KeyCode::Char('s') => Some(AppMsg::Audio(AudioMsg::AdjustSpeed(0.1))),
            KeyCode::Char('S') => Some(AppMsg::Audio(AudioMsg::AdjustSpeed(-0.1))),

            // Bass
            KeyCode::Char('b') => Some(AppMsg::Audio(AudioMsg::AdjustBass(0.1))),
            KeyCode::Char('B') => Some(AppMsg::Audio(AudioMsg::AdjustBass(-0.1))),

            // Treble
            KeyCode::Char('t') => Some(AppMsg::Audio(AudioMsg::AdjustTreble(0.1))),
            KeyCode::Char('T') => Some(AppMsg::Audio(AudioMsg::AdjustTreble(-0.1))),

            // Reverb
            KeyCode::Char('r') => Some(AppMsg::Audio(AudioMsg::AdjustReverb(0.05))),
            KeyCode::Char('R') => Some(AppMsg::Audio(AudioMsg::AdjustReverb(-0.05))),

            // Echo
            KeyCode::Char('e') => Some(AppMsg::Audio(AudioMsg::AdjustEcho(0.05))),
            KeyCode::Char('E') => Some(AppMsg::Audio(AudioMsg::AdjustEcho(-0.05))),

            _ => None,
        }
    }

    /// Returns data to build a Help UI
    pub fn get_help_docs() -> Vec<KeyHelp> {
        vec![
            KeyHelp {
                key: "Tab",
                desc: "Toggle Lib/Queue",
                category: "General",
            },
            KeyHelp {
                key: "Shift+Tab",
                desc: "Toggle Visualizer",
                category: "General",
            },
            KeyHelp {
                key: "Space",
                desc: "Play/Pause",
                category: "Playback",
            },
            KeyHelp {
                key: "Enter",
                desc: "Play Selected",
                category: "Playback",
            },
            KeyHelp {
                key: "z",
                desc: "Toggle Shuffle",
                category: "Playback",
            },
            KeyHelp {
                key: "c",
                desc: "Cycle Repeat",
                category: "Playback",
            },
            KeyHelp {
                key: "a",
                desc: "Add to Queue",
                category: "Queue",
            },
            KeyHelp {
                key: "Shift+A",
                desc: "Play Next",
                category: "Queue",
            },
            KeyHelp {
                key: "d",
                desc: "Delete (Queue)",
                category: "Queue",
            },
            KeyHelp {
                key: "D",
                desc: "Clear Queue",
                category: "Queue",
            },
            KeyHelp {
                key: "h/j/k/l",
                desc: "Navigate",
                category: "Navigation",
            },
            KeyHelp {
                key: ":",
                desc: "Command Mode",
                category: "System",
            },
        ]
    }
}
