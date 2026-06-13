use melotui_core::message::ui::UIMsg;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::Event;
use ratatui::crossterm::terminal::{
    ClearType, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::crossterm::{cursor, event, execute, terminal};
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
    pub key_registry: crate::keybinding::KeyRegistry,
}

impl Tui {
    pub fn new(config: &melotui_core::config::AppConfig) -> anyhow::Result<(Self, Vec<String>)> {
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen, cursor::Hide)?;
        let backend = CrosstermBackend::new(stdout());
        let terminal = Terminal::new(backend)?;

        tracing::info!("TUI initialized");

        let mut key_registry = crate::keybinding::KeyRegistry::default();
        let errors = key_registry.apply_overrides(&config.keymap_overrides);

        Ok((
            Self {
                terminal,
                image_cache: ImageCache::new(),
                key_registry,
            },
            errors,
        ))
    }

    pub fn draw(&mut self, state: &AppState) -> anyhow::Result<()> {
        self.terminal.draw(|frame| {
            self.image_cache.update(state);
            view::draw(frame, state, &mut self.image_cache, &self.key_registry);
        })?;
        Ok(())
    }

    pub fn shutdown(self) -> anyhow::Result<()> {
        // Drop self first to release the terminal backend,
        // then restore_terminal() handles the rest via Drop.
        drop(self);
        tracing::info!("TUI shutdown complete");
        Ok(())
    }

    pub fn size(&self) -> anyhow::Result<(u16, u16)> {
        let size = self.terminal.size()?;
        Ok((size.width, size.height))
    }

    pub fn poll_event(
        &self,
        state: &AppState,
        timeout: Duration,
    ) -> anyhow::Result<Option<AppMsg>> {
        let start = std::time::Instant::now();
        let mut current_timeout = timeout;

        while event::poll(current_timeout)? {
            let msg = match event::read()? {
                Event::Key(key) => InputHandler::handle(key, state, &self.key_registry),
                Event::Resize(w, h) => Some(AppMsg::UI(UIMsg::Resize(w, h))),
                _ => None,
            };

            if msg.is_some() {
                return Ok(msg);
            }

            // We consumed an event but it resulted in no AppMsg (e.g. unmapped key,
            // key release, or mouse event). We must keep polling so we don't falsely
            // return Ok(None) and break the coalescing drain loop in runtime.rs.
            let elapsed = start.elapsed();
            current_timeout = timeout.saturating_sub(elapsed);
        }

        Ok(None)
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = restore_terminal();
    }
}

/// Restore terminal to normal mode.
///
/// Best-effort: tries ALL cleanup steps even if one fails.
/// This ensures the terminal is usable after exit, panic, or error.
/// Called from Drop, panic hook, and normal shutdown.
pub fn restore_terminal() -> anyhow::Result<()> {
    let mut out = stdout();

    // Step 1: Clear the alternate screen to remove any image protocol artifacts
    // (Kitty/Sixel protocol data can linger and corrupt the shell after exit)
    let _ = execute!(out, terminal::Clear(ClearType::All));

    // Step 2: Leave alternate screen and show cursor
    let _ = execute!(out, LeaveAlternateScreen, cursor::Show);

    // Step 3: Disable raw mode (must be last — restores line buffering)
    let _ = disable_raw_mode();

    Ok(())
}
