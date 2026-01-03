use melotui_core::{AppMsg, AppState, NavMode, UIMode};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
        // If help is open, we trap keys to only allow closing it.
        if state.ui.show_help {
            return match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => Some(AppMsg::ToggleHelp),
                _ => None, // Swallow all other events
            };
        }
        // 1. Command Mode (Higher Priority)
        if state.ui.mode == UIMode::Command {
            return match key.code {
                KeyCode::Esc => Some(AppMsg::ExitCommandMode),
                KeyCode::Enter => Some(AppMsg::SubmitCommand(state.ui.command_buffer.clone())),
                KeyCode::Backspace => Some(AppMsg::BackspaceCommand),
                KeyCode::Char(c) => Some(AppMsg::UpdateCommand(c)),
                _ => None,
            };
        }

        // 2. Global / Normal Mode
        match key.code {
            // --- Global Lifecycle ---
            KeyCode::Char('?') => Some(AppMsg::ToggleHelp),
            KeyCode::Char('q') => Some(AppMsg::RequestQuit),
            KeyCode::Char(':') => Some(AppMsg::EnterCommandMode),
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(AppMsg::RequestRescanLibrary)
            }
            KeyCode::F(5) => Some(AppMsg::RequestRescanLibrary), // Moved Rescan here
            KeyCode::Esc => Some(AppMsg::RequestCancelDownload),

            // --- Navigation ---
            // Left Arrow / '[' : Move Focus Left
            KeyCode::Left | KeyCode::Char('[') => match state.nav.mode {
                NavMode::Tracks => Some(AppMsg::SwitchToPlaylists),
                NavMode::TrackInfo => Some(AppMsg::SwitchToTracks),
                _ => None,
            },
            // Right Arrow / ']' : Move Focus Right
            KeyCode::Right | KeyCode::Char(']') => match state.nav.mode {
                NavMode::Playlists => Some(AppMsg::SwitchToTracks),
                NavMode::Tracks => Some(AppMsg::SwitchToTrackInfo),
                _ => None,
            },
            // Up/Down
            KeyCode::Up | KeyCode::Char('k') => Some(AppMsg::MoveUp),
            KeyCode::Down | KeyCode::Char('j') => Some(AppMsg::MoveDown),

            // Actions
            KeyCode::Enter => Some(AppMsg::RequestPlaySelected),
            KeyCode::Tab | KeyCode::Char('{') | KeyCode::Char('}') => Some(AppMsg::ToggleArt),

            // --- Audio Control ---
            KeyCode::Char(' ') => Some(AppMsg::RequestTogglePlayback),
            KeyCode::Char('x') => Some(AppMsg::RequestStop),
            KeyCode::Char('h') => Some(AppMsg::RequestSeek { offset_seconds: -5 }),
            KeyCode::Char('l') => Some(AppMsg::RequestSeek { offset_seconds: 5 }),

            // --- DSP Effects ---
            // Volume (+/-)
            KeyCode::Char('+') | KeyCode::Char('=') => Some(AppMsg::AdjustVolume(0.05)),
            KeyCode::Char('-') | KeyCode::Char('_') => Some(AppMsg::AdjustVolume(-0.05)),

            // Speed (s/S)
            KeyCode::Char('s') => Some(AppMsg::AdjustSpeed(0.1)),
            KeyCode::Char('S') => Some(AppMsg::AdjustSpeed(-0.1)),

            // Bass (b/B)
            KeyCode::Char('b') => Some(AppMsg::AdjustBass(0.1)),
            KeyCode::Char('B') => Some(AppMsg::AdjustBass(-0.1)),

            // Treble (t/T)
            KeyCode::Char('t') => Some(AppMsg::AdjustTreble(0.1)),
            KeyCode::Char('T') => Some(AppMsg::AdjustTreble(-0.1)),

            // Reverb (r/R) - NOW AVAILABLE
            KeyCode::Char('r') => Some(AppMsg::AdjustReverb(0.05)),
            KeyCode::Char('R') => Some(AppMsg::AdjustReverb(-0.05)),

            // Echo (e/E)
            KeyCode::Char('e') => Some(AppMsg::AdjustEcho(0.05)),
            KeyCode::Char('E') => Some(AppMsg::AdjustEcho(-0.05)),

            _ => None,
        }
    }

    /// Returns data to build a Help UI without hardcoding strings in the view
    pub fn get_help_docs() -> Vec<KeyHelp> {
        vec![
            KeyHelp {
                key: "Space",
                desc: "Toggle Playback",
                category: "Playback",
            },
            KeyHelp {
                key: "Enter",
                desc: "Play Selected",
                category: "Playback",
            },
            KeyHelp {
                key: "x",
                desc: "Stop",
                category: "Playback",
            },
            KeyHelp {
                key: "h/l",
                desc: "Seek -/+ 5s",
                category: "Playback",
            },
            KeyHelp {
                key: "j/k",
                desc: "Move Up/Down",
                category: "Navigation",
            },
            KeyHelp {
                key: "←/→",
                desc: "Switch Columns",
                category: "Navigation",
            },
            KeyHelp {
                key: "Tab",
                desc: "Toggle Art/Vis",
                category: "Navigation",
            },
            KeyHelp {
                key: "+/-",
                desc: "Volume",
                category: "Audio FX",
            },
            KeyHelp {
                key: "s/S",
                desc: "Speed",
                category: "Audio FX",
            },
            KeyHelp {
                key: "b/B",
                desc: "Bass",
                category: "Audio FX",
            },
            KeyHelp {
                key: "t/T",
                desc: "Treble",
                category: "Audio FX",
            },
            KeyHelp {
                key: "r/R",
                desc: "Reverb",
                category: "Audio FX",
            },
            KeyHelp {
                key: "e/E",
                desc: "Echo",
                category: "Audio FX",
            },
            KeyHelp {
                key: "F5",
                desc: "Rescan Library",
                category: "System",
            },
            KeyHelp {
                key: ":",
                desc: "Command Mode",
                category: "System",
            },
            KeyHelp {
                key: "q",
                desc: "Quit",
                category: "System",
            },
        ]
    }
}
