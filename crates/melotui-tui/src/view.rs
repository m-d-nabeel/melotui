use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Clear, Paragraph},
};

use melotui_core::{AppState, UIMode, state::ui::FocusMode};

use crate::{
    image_cache::ImageCache,
    theme::Theme,
    widgets::{
        command_view, confirm_view, help_view, notification_popup, playback_view,
        playlist_picker_view, sidebar, sound_control_view, visualizer_view,
    },
};

const MIN_WIDTH: u16 = 60;
const MIN_HEIGHT: u16 = 10;

pub fn draw(
    frame: &mut Frame,
    state: &AppState,
    image_cache: &mut ImageCache,
    registry: &crate::keybinding::KeyRegistry,
) {
    let area = frame.area();

    // Minimum terminal size guard
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        draw_too_small(frame, area);
        return;
    }

    // Dispatch to focus mode layout
    match state.focus_mode() {
        FocusMode::Full => draw_full(frame, area, state, image_cache),
        FocusMode::Visualizer => draw_visualizer_only(frame, area, state, image_cache),
        FocusMode::Playback => draw_playback_only(frame, area, state),
        FocusMode::VisualizerPlayback => draw_viz_playback(frame, area, state, image_cache),
    }

    // Overlays render on top regardless of focus mode
    draw_overlays(frame, area, state, registry);
}

/// Full layout: sidebar + visualizer + mixer + playback
fn draw_full(frame: &mut Frame, area: Rect, state: &AppState, image_cache: &mut ImageCache) {
    let sidebar_width = state.sidebar_width();
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(sidebar_width), Constraint::Fill(1)])
        .split(area);

    let right_vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(14)])
        .split(horizontal[1]);

    let control_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(7)])
        .split(right_vertical[1]);

    sidebar::draw(frame, horizontal[0], state, image_cache);
    visualizer_view::draw(frame, right_vertical[0], state, image_cache);
    sound_control_view::draw(frame, control_chunks[0], state);
    playback_view::draw(frame, control_chunks[1], state);
}

/// Visualizer fills entire area
fn draw_visualizer_only(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    image_cache: &mut ImageCache,
) {
    visualizer_view::draw(frame, area, state, image_cache);
}

/// Minimal playback strip centered vertically
fn draw_playback_only(frame: &mut Frame, area: Rect, state: &AppState) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(7),
            Constraint::Fill(1),
        ])
        .split(area);

    playback_view::draw(frame, layout[1], state);
}

/// Visualizer top + playback bottom, no sidebar or mixer
fn draw_viz_playback(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    image_cache: &mut ImageCache,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(7)])
        .split(area);

    visualizer_view::draw(frame, layout[0], state, image_cache);
    playback_view::draw(frame, layout[1], state);
}

/// Overlays: command bar, playlist picker, confirm dialog, help, notifications
fn draw_overlays(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    registry: &crate::keybinding::KeyRegistry,
) {
    notification_popup::draw(frame, frame.area(), state);

    // Shared dim overlay — uses a dark background that works on all terminals
    // (Modifier::DIM is ignored by many terminal emulators)
    let dim_block = Block::default().style(Style::default().bg(Color::Rgb(10, 10, 15)));

    if *state.ui_mode() == UIMode::Command {
        frame.render_widget(dim_block.clone(), area);
        let overlay = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(area)[1];
        frame.render_widget(Clear, overlay);
        command_view::draw(frame, overlay, state);
    }

    if *state.ui_mode() == UIMode::PlaylistPicker {
        frame.render_widget(dim_block.clone(), area);
        playlist_picker_view::draw(frame, area, state);
    }

    if *state.ui_mode() == UIMode::Confirm {
        frame.render_widget(dim_block.clone(), area);
        confirm_view::draw(frame, area, state);
    }

    if state.ui_show_help() {
        frame.render_widget(dim_block, area);
        help_view::draw(frame, area, state, registry);
    }
}

fn draw_too_small(frame: &mut Frame, area: Rect) {
    let theme = Theme::default();
    let msg = Paragraph::new(format!(
        "Terminal too small\nMinimum: {}×{}\nCurrent: {}×{}",
        MIN_WIDTH, MIN_HEIGHT, area.width, area.height
    ))
    .alignment(Alignment::Center)
    .style(Style::default().fg(theme.accent_warning));

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(4),
            Constraint::Fill(1),
        ])
        .split(area);

    frame.render_widget(msg, popup_layout[1]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use melotui_core::library::snapshot::Library;
    use ratatui::{Terminal, backend::TestBackend};
    use std::collections::HashMap;

    fn test_state() -> AppState {
        let root = std::path::PathBuf::from("/tmp");
        let library = Library::new(vec![root], vec![], HashMap::new());
        AppState::new(library, &melotui_core::config::AppConfig::default())
    }

    #[test]
    fn test_viewport_too_small() {
        let state = test_state();
        let mut image_cache = ImageCache::empty();

        // Terminal smaller than MIN_WIDTH (60)
        let backend = TestBackend::new(50, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let registry = crate::keybinding::KeyRegistry::default();
                draw(f, &state, &mut image_cache, &registry);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        // Look for the "Terminal too small" text
        let content = (0..50)
            .map(|x| buffer[(x, 10)].symbol().to_string())
            .collect::<String>();
        assert!(
            content.contains("Terminal too small")
                || (0..20).any(|y| (0..50)
                    .map(|x| buffer[(x, y)].symbol().to_string())
                    .collect::<String>()
                    .contains("Terminal too small"))
        );
    }

    #[test]
    fn test_viewport_sufficient_size() {
        let state = test_state();
        let mut image_cache = ImageCache::empty();

        // Terminal larger than minimum (60x10)
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let registry = crate::keybinding::KeyRegistry::default();
                draw(f, &state, &mut image_cache, &registry);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        // Should not contain "Terminal too small"
        let full_text = (0..24)
            .map(|y| {
                (0..80)
                    .map(|x| buffer[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<String>>()
            .join("\n");
        assert!(!full_text.contains("Terminal too small"));
    }
}
