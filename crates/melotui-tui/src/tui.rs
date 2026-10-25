use melotui_core::message::ui::UIMsg;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::Event;
use ratatui::crossterm::terminal::{
    ClearType, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::crossterm::{cursor, event, execute, terminal};
use ratatui::{Terminal, crossterm::terminal::EnterAlternateScreen};
use std::io::{Stdout, stdout};
use std::sync::mpsc;
use std::time::{Duration, Instant};

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
    /// How far the help can scroll, as last drawn: the keys that scroll it
    /// go no further.
    help_scroll_max: std::cell::Cell<u16>,
    /// Terminal events, read on a thread of their own.
    events: mpsc::Receiver<std::io::Result<Event>>,
}

/// Reads terminal events on a thread of its own, so the app's loop only ever
/// waits on a channel.
///
/// crossterm's reader never returns once the terminal hangs up — it retries
/// the failing read forever — and the loop blocked in it then never got round
/// to quitting. With the reading here, the loop keeps its own time whatever
/// happens to the terminal.
fn spawn_input_thread() -> std::io::Result<mpsc::Receiver<std::io::Result<Event>>> {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("terminal-input".into())
        .spawn(move || {
            loop {
                let event = event::read();
                let failed = event.is_err();
                if tx.send(event).is_err() || failed {
                    break;
                }
            }
        })?;
    Ok(rx)
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

        // The image cache asks the terminal about itself and reads the replies
        // from stdin, so input is only read from here on.
        let image_cache = ImageCache::new(config.ui.lyrics_shaping);
        let events = spawn_input_thread()?;

        Ok((
            Self {
                terminal,
                image_cache,
                key_registry,
                help_scroll_max: std::cell::Cell::new(0),
                events,
            },
            errors,
        ))
    }

    pub fn draw(&mut self, state: &AppState) -> anyhow::Result<()> {
        self.terminal.draw(|frame| {
            self.image_cache.update(state);
            view::draw(
                frame,
                state,
                &mut self.image_cache,
                &self.key_registry,
                &self.help_scroll_max,
            );
        })?;
        // Lyric images that left the screen in that frame are freed now, with
        // the frame written and nothing else going to the terminal.
        self.image_cache
            .lyrics
            .flush_retired(self.terminal.backend_mut())?;
        Ok(())
    }

    pub fn shutdown(mut self) -> anyhow::Result<()> {
        // A terminal that has gone cannot be restored, and ratatui's own clean
        // up would eprintln! about the cursor — a panic, with stderr gone too.
        if self.terminal.show_cursor().is_err() {
            tracing::info!("Terminal gone; nothing to restore");
            std::mem::forget(self);
            return Ok(());
        }
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

    /// The next event that means something to the app, waiting at most
    /// `timeout` for one. An error means the terminal is gone.
    pub fn poll_event(
        &self,
        state: &AppState,
        timeout: Duration,
    ) -> anyhow::Result<Option<AppMsg>> {
        let deadline = Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let event = match self.events.recv_timeout(left) {
                Ok(event) => event?,
                Err(mpsc::RecvTimeoutError::Timeout) => return Ok(None),
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    anyhow::bail!("the terminal input closed")
                }
            };
            let msg = match event {
                Event::Key(key) => match InputHandler::handle(key, state, &self.key_registry) {
                    Some(AppMsg::UI(UIMsg::ScrollHelp { by, .. })) => {
                        Some(AppMsg::UI(UIMsg::ScrollHelp {
                            by,
                            max: self.help_scroll_max.get(),
                        }))
                    }
                    msg => msg,
                },
                Event::Resize(w, h) => Some(AppMsg::UI(UIMsg::Resize(w, h))),
                _ => None,
            };
            // An event that means nothing — an unmapped key, a key release, the
            // mouse — is passed over, and the wait goes on for the time left.
            if msg.is_some() {
                return Ok(msg);
            }
            if left.is_zero() {
                return Ok(None);
            }
        }
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
