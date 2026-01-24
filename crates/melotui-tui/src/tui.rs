use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::Event;
use ratatui::crossterm::terminal::{LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use ratatui::crossterm::{event, execute};
use ratatui::{Terminal, crossterm::terminal::EnterAlternateScreen};
use std::io::{Stdout, stdout};
use std::time::Duration;

use crate::image_cache::ImageCache;
use crate::{input::InputHandler, view};
use melotui_core::AppMsg;
use melotui_core::AppState;

pub enum UIEvent {
    Quit,
    None,
}

pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    pub image_cache: ImageCache,
}

impl Tui {
    pub fn new() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout());
        let terminal = Terminal::new(backend)?;

        tracing::info!("TUI initialized");

        Ok(Self {
            terminal,
            image_cache: ImageCache::new(),
        })
    }

    pub fn draw(&mut self, state: &AppState) -> anyhow::Result<()> {
        self.terminal.draw(|frame| {
            self.image_cache.update(state);
            view::draw(frame, state, &mut self.image_cache);
        })?;
        Ok(())
    }

    pub fn shutdown(mut self) -> anyhow::Result<()> {
        disable_raw_mode()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;

        tracing::info!("TUI shutdown complete");
        Ok(())
    }

    pub fn poll_event(&self, state: &AppState) -> anyhow::Result<Option<AppMsg>> {
        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
        {
            return Ok(InputHandler::handle(key, state));
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
