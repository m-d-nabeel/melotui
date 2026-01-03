use melotui_core::state_registry::UIMode;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{Event, KeyCode};
use ratatui::crossterm::terminal::{LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use ratatui::crossterm::{event, execute};
use ratatui::{Terminal, crossterm::terminal::EnterAlternateScreen};
use std::io::{Stdout, stdout};
use std::time::Duration;

use crate::view;
use melotui_core::AppMsg;
use melotui_core::AppState;

pub enum UIEvent {
    Quit,
    SwitchToPlaylists,
    SwitchToTracks,
    MoveUp,
    MoveDown,
    None,
}

pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Tui {
    /// Initialize terminal (raw mode + alt screen)
    pub fn new() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout());
        let terminal = Terminal::new(backend)?;

        tracing::info!("TUI initialized");

        Ok(Self { terminal })
    }

    /// Main draw call
    pub fn draw(&mut self, state: &AppState) -> anyhow::Result<()> {
        self.terminal.draw(|frame| {
            view::draw(frame, state);
        })?;
        Ok(())
    }

    /// Restore terminal state
    pub fn shutdown(mut self) -> anyhow::Result<()> {
        disable_raw_mode()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;

        tracing::info!("TUI shutdown complete");
        Ok(())
    }

    /// Poll input
    pub fn poll_event(&self, state: &AppState) -> anyhow::Result<Option<AppMsg>> {
        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
        {
            return Ok(match state.ui.mode {
                UIMode::Normal => match key.code {
                    // --- Navigation ---
                    KeyCode::Char('q') => Some(AppMsg::RequestQuit),
                    KeyCode::Char('[') | KeyCode::Left => match state.nav.mode {
                        melotui_core::NavMode::Tracks => Some(AppMsg::SwitchToPlaylists),
                        melotui_core::NavMode::TrackInfo => Some(AppMsg::SwitchToTracks),
                        _ => None,
                    },
                    KeyCode::Char(']') | KeyCode::Right => match state.nav.mode {
                        melotui_core::NavMode::Playlists => Some(AppMsg::SwitchToTracks),
                        melotui_core::NavMode::Tracks => Some(AppMsg::SwitchToTrackInfo),
                        _ => None,
                    },
                    KeyCode::Up | KeyCode::Char('k') => Some(AppMsg::MoveUp),
                    KeyCode::Down | KeyCode::Char('j') => Some(AppMsg::MoveDown),

                    // --- Playback Control ---
                    KeyCode::Enter => Some(AppMsg::RequestPlaySelected),
                    KeyCode::Char(' ') => Some(AppMsg::RequestTogglePlayback),
                    KeyCode::Char('x') => Some(AppMsg::RequestStop),

                    // --- Audio FX Hotkeys ---
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

                    // Reverb (v/V)
                    KeyCode::Char('v') => Some(AppMsg::AdjustReverb(0.05)),
                    KeyCode::Char('V') => Some(AppMsg::AdjustReverb(-0.05)),

                    // Echo (e/E)
                    KeyCode::Char('e') => Some(AppMsg::AdjustEcho(0.05)),
                    KeyCode::Char('E') => Some(AppMsg::AdjustEcho(-0.05)),

                    // Seek -5s / +5s
                    KeyCode::Char('h') => Some(AppMsg::RequestSeek { offset_seconds: -5 }),
                    KeyCode::Char('l') => Some(AppMsg::RequestSeek { offset_seconds: 5 }),

                    // --- Global ---
                    KeyCode::Char('r') => Some(AppMsg::RequestRescanLibrary),
                    KeyCode::Char(':') => Some(AppMsg::EnterCommandMode),
                    KeyCode::Char('{') | KeyCode::Char('}') | KeyCode::Tab => {
                        Some(AppMsg::ToggleArt)
                    }
                    KeyCode::Esc => Some(AppMsg::RequestCancelDownload),
                    _ => None,
                },

                UIMode::Command => match key.code {
                    KeyCode::Esc => Some(AppMsg::ExitCommandMode),
                    KeyCode::Enter => Some(AppMsg::SubmitCommand(state.ui.command_buffer.clone())),
                    KeyCode::Backspace => Some(AppMsg::BackspaceCommand),
                    KeyCode::Char(c) => Some(AppMsg::UpdateCommand(c)),
                    _ => None,
                },
            });
        }
        Ok(None)
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
    }
}
