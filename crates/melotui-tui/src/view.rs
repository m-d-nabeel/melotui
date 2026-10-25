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
    help_scroll_max: &std::cell::Cell<u16>,
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

    // Before anything is drawn over the images.
    crate::lyrics::kitty::spread_rows(frame.buffer_mut());

    // Overlays render on top regardless of focus mode
    draw_overlays(frame, area, state, registry, help_scroll_max);
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
    help_scroll_max: &std::cell::Cell<u16>,
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
        help_view::draw(frame, area, state, registry, help_scroll_max);
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
                draw(f, &state, &mut image_cache, &registry, &Default::default());
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
                draw(f, &state, &mut image_cache, &registry, &Default::default());
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
    use crate::lyrics::kitty::spread_rows;
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
                spread_rows(f.buffer_mut());
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

    #[test]
    fn a_popup_over_the_album_art_leaves_no_trace_when_it_goes() {
        use ratatui::widgets::{Block, Borders, Clear, Paragraph};
        let mut picker = Picker::halfblocks();
        picker.set_protocol_type(ProtocolType::Kitty);
        let (w, h) = picker.font_size();
        let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(
            20 * u32::from(w),
            5 * u32::from(h),
            image::Rgba([200, 100, 50, 255]),
        ));
        let art_area = Rect::new(2, 1, 20, 5);
        let art = picker
            .new_protocol(image, art_area, Resize::Fit(None))
            .unwrap();
        let popup = Rect::new(12, 2, 15, 3);
        let mut terminal = Terminal::new(TestBackend::new(30, 7)).unwrap();
        let mut frame = |with_popup: bool| {
            terminal
                .draw(|f| {
                    f.render_widget(Image::new(&art), art_area);
                    spread_rows(f.buffer_mut());
                    if with_popup {
                        f.render_widget(Clear, popup);
                        f.render_widget(
                            Paragraph::new("Visualizer").block(Block::new().borders(Borders::ALL)),
                            popup,
                        );
                    }
                    fit_escape_cells(f.buffer_mut());
                })
                .unwrap();
            terminal.backend().buffer().clone()
        };

        frame(false);
        let shown = frame(true);
        assert_eq!(
            shown[(13, 3)].symbol(),
            "V",
            "the popup is drawn over the art"
        );
        let after = frame(false);
        let under = popup.intersection(art_area);
        for y in under.top()..under.bottom() {
            for x in under.left()..under.right() {
                assert!(
                    after[(x, y)].symbol().contains('\u{10EEEE}'),
                    "({x}, {y}) still shows the popup: {:?}",
                    after[(x, y)].symbol()
                );
            }
        }
        for y in art_area.top()..art_area.bottom() {
            for x in art_area.left()..art_area.right() {
                assert!(
                    after[(x, y)].symbol().contains('\u{10EEEE}'),
                    "({x}, {y}) is not the art: {:?}",
                    after[(x, y)].symbol()
                );
            }
        }
        assert_eq!(
            after[(25, 3)].symbol(),
            " ",
            "and off the art, it is cleared"
        );
    }

    /// A terminal, as far as what ends up on screen goes: it prints each
    /// character where the cursor is and moves on, follows the cursor
    /// movement ratatui and ratatui-image send, puts kitty's placeholder
    /// diacritics on the cell before them, and skips kitty's image commands,
    /// which show nothing by themselves. Unlike `TestBackend`, which files
    /// each cell where ratatui says, this sees what a row of placeholders
    /// written from one cell paints over.
    struct Screen {
        width: u16,
        cells: Vec<(String, Option<ratatui::style::Color>)>,
        cursor: (u16, u16),
        saved: (u16, u16),
        fg: Option<ratatui::style::Color>,
        last: Option<usize>,
    }

    impl Screen {
        fn new(width: u16, height: u16) -> Self {
            Self {
                width,
                cells: vec![(" ".to_string(), None); usize::from(width) * usize::from(height)],
                cursor: (0, 0),
                saved: (0, 0),
                fg: None,
                last: None,
            }
        }

        fn feed(&mut self, bytes: &[u8]) {
            let text = std::str::from_utf8(bytes).unwrap();
            let mut chars = text.chars().peekable();
            while let Some(c) = chars.next() {
                if c != '\x1b' {
                    self.print(c);
                    continue;
                }
                match chars.next() {
                    Some('[') => {
                        let mut params = String::new();
                        let mut last = ' ';
                        for c in chars.by_ref() {
                            if ('\x40'..='\x7e').contains(&c) {
                                last = c;
                                break;
                            }
                            params.push(c);
                        }
                        self.csi(&params, last);
                    }
                    // Strings — kitty's APC image commands among them — run to
                    // ESC \ and show nothing.
                    Some('_' | 'P' | ']') => {
                        while let Some(c) = chars.next() {
                            if c == '\x1b' && chars.peek() == Some(&'\\') {
                                chars.next();
                                break;
                            }
                        }
                    }
                    Some('7') => self.saved = self.cursor,
                    Some('8') => self.cursor = self.saved,
                    _ => {}
                }
            }
        }

        fn csi(&mut self, params: &str, last: char) {
            let arg = |i: usize| -> u16 {
                params
                    .split(';')
                    .nth(i)
                    .and_then(|p| p.parse().ok())
                    .filter(|&n| n > 0)
                    .unwrap_or(1)
            };
            match last {
                'H' => self.cursor = (arg(1) - 1, arg(0) - 1),
                's' => self.saved = self.cursor,
                'u' => self.cursor = self.saved,
                'C' => self.cursor.0 = self.cursor.0.saturating_add(arg(0)),
                'B' => self.cursor.1 = self.cursor.1.saturating_add(arg(0)),
                'm' => self.sgr(params),
                _ => {}
            }
        }

        fn sgr(&mut self, params: &str) {
            use ratatui::style::Color;
            let p: Vec<&str> = params.split(';').collect();
            let mut i = 0;
            while i < p.len() {
                match (p[i], p.get(i + 1).copied()) {
                    ("" | "0" | "39", _) => self.fg = None,
                    ("38", Some("2")) => {
                        let ch = |k: usize| p[i + k].parse().unwrap();
                        self.fg = Some(Color::Rgb(ch(2), ch(3), ch(4)));
                        i += 4;
                    }
                    ("38", Some("5")) => {
                        self.fg = Some(Color::Indexed(p[i + 2].parse().unwrap()));
                        i += 2;
                    }
                    ("48" | "58", Some("2")) => i += 4,
                    ("48" | "58", Some("5")) => i += 2,
                    _ => {}
                }
                i += 1;
            }
        }

        fn print(&mut self, c: char) {
            if crate::lyrics::kitty::DIACRITICS.contains(&c) {
                if let Some(i) = self.last {
                    self.cells[i].0.push(c);
                }
                return;
            }
            let (x, y) = self.cursor;
            let i = usize::from(y) * usize::from(self.width) + usize::from(x);
            if x < self.width && i < self.cells.len() {
                self.cells[i] = (c.to_string(), self.fg);
                self.last = Some(i);
            } else {
                self.last = None;
            }
            self.cursor.0 = self.cursor.0.saturating_add(1);
        }

        fn at(&self, x: u16, y: u16) -> &(String, Option<ratatui::style::Color>) {
            &self.cells[usize::from(y) * usize::from(self.width) + usize::from(x)]
        }
    }

    /// The bytes sent to the terminal, kept where the test can read them.
    #[derive(Clone, Default)]
    struct Sent(std::rc::Rc<std::cell::RefCell<Vec<u8>>>);

    impl std::io::Write for Sent {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// What of `symbol` shows on screen: all but its escape sequences.
    fn visible(symbol: &str) -> String {
        let mut screen = Screen::new(64, 1);
        screen.feed(symbol.as_bytes());
        let width = screen.cursor.0.min(64);
        (0..width).map(|x| screen.at(x, 0).0.clone()).collect()
    }

    #[test]
    fn the_terminal_shows_what_was_drawn_as_popups_and_art_change_together() {
        use crate::widgets::notification_popup;
        use melotui_core::{AppState, library::snapshot::Library, state::NotificationKind};
        use ratatui::{
            Terminal, TerminalOptions, Viewport,
            backend::CrosstermBackend,
            buffer::{Buffer, CellDiffOption},
        };
        use ratatui_image::StatefulImage;

        let (width, height) = (60, 16);
        let mut picker = Picker::halfblocks();
        picker.set_protocol_type(ProtocolType::Kitty);
        let (w, h) = picker.font_size();
        let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(
            60 * u32::from(w),
            14 * u32::from(h),
            image::Rgba([90, 140, 60, 255]),
        ));
        let mut art = picker.new_resize_protocol(image);
        let mut state = AppState::new(
            Library::new(vec![], vec![], std::collections::HashMap::new()),
            &melotui_core::config::AppConfig::default(),
        );
        let sent = Sent::default();
        let mut terminal = Terminal::with_options(
            CrosstermBackend::new(sent.clone()),
            TerminalOptions {
                viewport: Viewport::Fixed(Rect::new(0, 0, width, height)),
            },
        )
        .unwrap();
        let mut screen = Screen::new(width, height);

        // As the app's frames go: the art (narrower once the layout changes),
        // then the spreading pass, then whatever pops up over it.
        let mut frame = |state: &AppState, art_width: u16, step: &str| {
            let mut drawn = Buffer::empty(Rect::new(0, 0, width, height));
            terminal
                .draw(|f| {
                    let image = StatefulImage::default().resize(Resize::Fit(None));
                    f.render_stateful_widget(image, Rect::new(1, 1, art_width, 14), &mut art);
                    spread_rows(f.buffer_mut());
                    notification_popup::draw(f, f.area(), state);
                    fit_escape_cells(f.buffer_mut());
                    drawn = f.buffer_mut().clone();
                })
                .unwrap();
            screen.feed(&sent.0.take());
            for y in 0..height {
                for x in 0..width {
                    let cell = &drawn[(x, y)];
                    #[allow(deprecated)]
                    let skipped = cell.skip || cell.diff_option == CellDiffOption::Skip;
                    assert!(
                        !skipped,
                        "{step}: ({x}, {y}) is left for another cell to draw"
                    );
                    let (shown, fg) = screen.at(x, y);
                    assert_eq!(
                        *shown,
                        visible(cell.symbol()),
                        "{step}: ({x}, {y}) shows something other than what was drawn"
                    );
                    if shown.starts_with('\u{10EEEE}') {
                        assert_eq!(*fg, Some(cell.fg), "{step}: ({x}, {y}) is another image");
                    }
                }
            }
        };

        frame(&state, 58, "the art alone");
        state.set_notification(NotificationKind::Info, "Focus: Lyrics");
        frame(&state, 58, "a notification over it");
        // A focus change: the art is drawn again at a new size while the
        // notification's text changes, which is when, sent a row at a time,
        // the art painted over everything of the popup but its new text.
        state.set_notification(NotificationKind::Info, "Full");
        frame(&state, 40, "the art redrawn under a new notification");
        state.clear_current_notification();
        frame(&state, 40, "the notification gone");
    }
}
