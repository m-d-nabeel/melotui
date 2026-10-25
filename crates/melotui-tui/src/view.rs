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
        command_view, confirm_view, dsp_studio_view, help_view, notification_popup, playback_view,
        playlist_picker_view, preset_picker_view, sidebar, sound_control_view, visualizer_view,
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

    if state.ui_show_dsp_studio() {
        let playback_height: u16 = 3;
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(playback_height)])
            .split(area);

        dsp_studio_view::draw(frame, layout[0], state, image_cache);
        playback_view::draw_compact(frame, layout[1], state);
    } else {
        // Dispatch to focus mode layout
        match state.focus_mode() {
            FocusMode::Full => draw_full(frame, area, state, image_cache),
            FocusMode::Visualizer => draw_visualizer_only(frame, area, state, image_cache),
            FocusMode::Playback => draw_playback_only(frame, area, state),
            FocusMode::VisualizerPlayback => draw_viz_playback(frame, area, state, image_cache),
            FocusMode::LyricsPlayback => draw_lyrics_playback(frame, area, state, image_cache),
        }
    }

    // Overlays render on top regardless of focus mode
    draw_overlays(frame, area, state, registry);
    fit_escape_cells(frame.buffer_mut());
}

/// Marks each cell holding an escape sequence — an image, as kitty
/// placeholders, a sixel or an iTerm2 image — as one column wide.
///
/// ratatui measures a cell by the text in it and skips that many columns
/// when working out what to send; an image row's escape codes measure as
/// dozens, so the rows below it were skipped and never sent, and only the
/// first row of every image showed.
fn fit_escape_cells(buffer: &mut ratatui::buffer::Buffer) {
    use ratatui::buffer::CellDiffOption;
    for cell in &mut buffer.content {
        if cell.diff_option == CellDiffOption::None && cell.symbol().contains('\x1b') {
            cell.set_diff_option(CellDiffOption::ForcedWidth(std::num::NonZeroU16::MIN));
        }
    }
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

    let playback_height = if state.ui_show_playback() { 7 } else { 0 };

    let control_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(playback_height)])
        .split(right_vertical[1]);

    sidebar::draw(frame, horizontal[0], state, image_cache);
    visualizer_view::draw(frame, right_vertical[0], state, image_cache);

    if state.ui_show_lyrics() {
        crate::widgets::lyrics_view::draw(
            frame,
            control_chunks[0],
            state,
            image_cache,
            state.lyrics_zoom(),
        );
    } else {
        sound_control_view::draw(frame, control_chunks[0], state);
    }

    if state.ui_show_playback() {
        playback_view::draw(frame, control_chunks[1], state);
    }
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
    if !state.ui_show_playback() {
        return; // Nothing to draw
    }

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
    let playback_height = if state.ui_show_playback() { 7 } else { 0 };
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(playback_height)])
        .split(area);

    visualizer_view::draw(frame, layout[0], state, image_cache);
    if state.ui_show_playback() {
        playback_view::draw(frame, layout[1], state);
    }
}

/// Lyrics top + playback bottom, no sidebar or mixer
fn draw_lyrics_playback(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    image_cache: &mut ImageCache,
) {
    let playback_height = if state.ui_show_playback() { 7 } else { 0 };
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(playback_height)])
        .split(area);

    crate::widgets::lyrics_view::draw(
        frame,
        layout[0],
        state,
        image_cache,
        state.lyrics_zoom_focus(),
    );
    if state.ui_show_playback() {
        playback_view::draw(frame, layout[1], state);
    }
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
        let [_, overlay] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(command_view::height(state)),
            ])
            .areas(area);
        frame.render_widget(Clear, overlay);
        command_view::draw(frame, overlay, state);
    }

    if *state.ui_mode() == UIMode::PlaylistPicker {
        frame.render_widget(dim_block.clone(), area);
        playlist_picker_view::draw(frame, area, state);
    }

    if *state.ui_mode() == UIMode::PresetPicker {
        frame.render_widget(dim_block.clone(), area);
        preset_picker_view::draw(frame, area, state);
    }

    if *state.ui_mode() == UIMode::Confirm {
        frame.render_widget(dim_block.clone(), area);
        confirm_view::draw(frame, area, state);
    }

    if state.ui_show_help() {
        frame.render_widget(dim_block.clone(), area);
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

#[cfg(test)]
mod image_tests {
    use super::fit_escape_cells;
    use image::{DynamicImage, RgbaImage};
    use ratatui::{Terminal, backend::TestBackend, layout::Rect};
    use ratatui_image::{
        Image, Resize,
        picker::{Picker, ProtocolType},
    };

    /// Draws a 20 × 5 cell image through ratatui-image, as the album art is,
    /// and returns what the terminal was sent.
    fn sent(protocol: ProtocolType) -> ratatui::buffer::Buffer {
        let mut picker = Picker::halfblocks();
        picker.set_protocol_type(protocol);
        let (w, h) = picker.font_size();
        let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(
            20 * u32::from(w),
            5 * u32::from(h),
            image::Rgba([200, 100, 50, 255]),
        ));
        let area = Rect::new(0, 0, 20, 5);
        let art = picker.new_protocol(image, area, Resize::Fit(None)).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(30, 7)).unwrap();
        terminal
            .draw(|f| {
                f.render_widget(Image::new(&art), Rect::new(2, 1, 20, 5));
                fit_escape_cells(f.buffer_mut());
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    #[test]
    fn every_row_of_the_album_art_reaches_the_terminal() {
        let sent = sent(ProtocolType::Kitty);
        for y in 1..=5 {
            assert!(
                sent[(2, y)].symbol().contains('\u{10EEEE}'),
                "row {y} of the art was never written: {:?}",
                sent[(2, y)].symbol()
            );
        }
    }

    #[test]
    fn an_iterm2_image_is_sent_whole() {
        let sent = sent(ProtocolType::Iterm2);
        assert!(sent[(2, 1)].symbol().contains("]1337;File="));
    }
}
