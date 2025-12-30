use melotui_core::state::UIMode;
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
        // execute!(stdout, EnterAlternateScreen, DisableMouseCapture, Hide)?;
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
                    KeyCode::Char('q') => Some(AppMsg::Quit),
                    KeyCode::Char('[') => Some(AppMsg::SwitchToPlaylists),
                    KeyCode::Char(']') => Some(AppMsg::SwitchToTracks),
                    KeyCode::Up => Some(AppMsg::MoveUp),
                    KeyCode::Down => Some(AppMsg::MoveDown),
                    KeyCode::Char('r') => Some(AppMsg::RescanLibrary),
                    KeyCode::Char(':') => Some(AppMsg::EnterCommandMode),
                    KeyCode::Enter => Some(AppMsg::PlaySelected),
                    KeyCode::Esc => Some(AppMsg::CancelDownload),
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
