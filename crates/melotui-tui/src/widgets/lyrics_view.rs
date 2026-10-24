use melotui_core::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
};
use ratatui_image::Image;

use crate::image_cache::ImageCache;
use crate::lyrics::bidi;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, cache: &mut ImageCache) {
    let t = crate::theme::get_theme(state);

    // Minimalist Sound Control Panel Lyrics
    let block = Block::default()
        .title(" Lyrics ")
        .title_style(Style::default().add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border_inactive))
        .padding(Padding::horizontal(2));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(search_state) = state.lyrics_search_state() {
        draw_search(frame, inner, search_state, t);
        return;
    }

    // Each line with its colour: lines already sung dimmed, the current one in
    // the accent colour, the ones still to come in between.
    let active_index = state.active_lyrics_line_index();
    let lines: Vec<(&str, Color)> = match state.lyrics_data() {
        melotui_core::state::lyrics::LyricsData::Synced(lyrics) => lyrics
            .get_timed_lines()
            .iter()
            .enumerate()
            .map(|(i, (_time, text))| {
                let color = match active_index {
                    Some(active) if i == active => t.accent_primary,
                    Some(active) if i < active => t.fg_muted,
                    _ => t.fg_secondary,
                };
                (&**text, color)
            })
            .collect(),
        melotui_core::state::lyrics::LyricsData::Unsynced(text) => {
            text.lines().map(|line| (line, t.fg_muted)).collect()
        }
        melotui_core::state::lyrics::LyricsData::None => vec![("No lyrics found.", t.fg_muted)],
    };

    let viewport_height = inner.height as usize;
    let scroll_offset = active_index.map_or(0, |idx| idx.saturating_sub(viewport_height / 2));

    let texts: Vec<&str> = lines.iter().map(|(text, _)| *text).collect();
    let picker = &cache.picker;
    let shaped = &mut cache.lyrics;

    if shaped.should_shape(picker, &texts) {
        shaped.begin();
        for (row, (text, color)) in lines
            .iter()
            .skip(scroll_offset)
            .take(viewport_height)
            .enumerate()
        {
            if text.trim().is_empty() {
                continue;
            }
            let row_area = Rect {
                y: inner.y + row as u16,
                height: 1,
                ..inner
            };
            match shaped.line(picker, text, *color, inner.width) {
                Some(image) => frame.render_widget(Image::new(image), row_area),
                None => frame.render_widget(
                    Paragraph::new(text_line(text, *color)).alignment(Alignment::Center),
                    row_area,
                ),
            }
        }
        shaped.end();
    } else {
        shaped.clear();
        let text_lines: Vec<Line> = lines
            .iter()
            .map(|(text, color)| text_line(text, *color))
            .collect();
        let paragraph = Paragraph::new(text_lines)
            .alignment(Alignment::Center)
            .scroll((scroll_offset as u16, 0));
        frame.render_widget(paragraph, inner);
    }
}

/// A lyric line as terminal text, bold, with right-to-left runs in the order
/// the terminal needs.
fn text_line(text: &str, color: Color) -> Line<'static> {
    Line::from(Span::styled(
        bidi::for_terminal(text).into_owned(),
        Style::default().add_modifier(Modifier::BOLD).fg(color),
    ))
}

fn draw_search(
    frame: &mut Frame,
    inner: Rect,
    search_state: &melotui_core::state::lyrics::LyricsSearchState,
    t: &crate::theme::Theme,
) {
    // Render search selector in two columns
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let item_style = |is_selected: bool| {
        if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(t.accent_primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.fg_primary)
        }
    };

    // LRCLIB (Left column)
    let mut lrclib_lines = vec![Line::from(vec![Span::styled(
        "LRCLIB",
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(t.accent_info),
    )])];
    for (i, res) in search_state.lrclib_results.iter().enumerate() {
        let is_selected = search_state.selected_column == 0 && search_state.selected_row == i;
        let prefix = if res.is_synced { "[S] " } else { "[U] " };
        lrclib_lines.push(Line::styled(
            format!(
                "{}{}: {}",
                prefix,
                bidi::for_terminal(&res.artist_name),
                bidi::for_terminal(&res.track_name)
            ),
            item_style(is_selected),
        ));
    }

    // OVH (Right column)
    let mut ovh_lines = vec![Line::from(vec![Span::styled(
        "OVH",
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(t.accent_info),
    )])];
    for (i, res) in search_state.ovh_results.iter().enumerate() {
        let is_selected = search_state.selected_column == 1 && search_state.selected_row == i;
        ovh_lines.push(Line::styled(
            format!(
                "{}: {}",
                bidi::for_terminal(&res.artist_name),
                bidi::for_terminal(&res.track_name)
            ),
            item_style(is_selected),
        ));
    }

    let left_para = Paragraph::new(lrclib_lines).alignment(Alignment::Left);
    let right_para = Paragraph::new(ovh_lines).alignment(Alignment::Left);

    frame.render_widget(left_para, chunks[0]);
    frame.render_widget(right_para, chunks[1]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lyrics::LyricsRenderer;
    use melotui_core::library::snapshot::Library;
    use melotui_core::message::playback::PlaybackMsg;
    use melotui_core::{AppMsg, reducer};
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};
    use ratatui_image::picker::ProtocolType;
    use std::collections::HashMap;

    const LYRICS: &str = "[00:01.00]कभी कभी मेरे दिल में\n\
                          [00:05.00]تمہارے بغیر ہم\n\
                          [00:09.00]Never gonna give you up\n";

    /// App state with `lrc` loaded, the way playback loads it: from a `.lrc`
    /// file next to the track.
    fn state_with_lyrics(lrc: &str, name: &str) -> AppState {
        let dir =
            std::env::temp_dir().join(format!("melotui-lyrics-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("song.lrc"), lrc).unwrap();
        let library = Library::new(vec![dir.clone()], vec![], HashMap::new());
        let mut state = AppState::new(library, &melotui_core::config::AppConfig::default());
        let path = dir.join("song.mp3").to_string_lossy().into_owned();
        reducer::reduce(
            &mut state,
            AppMsg::Playback(PlaybackMsg::AudioStarted { path }),
        );
        std::fs::remove_dir_all(&dir).ok();
        state
    }

    /// The frame as drawn, skipped cells included — the backend's copy only
    /// has what was actually written to the terminal.
    fn render(state: &AppState, cache: &mut ImageCache) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(50, 8)).unwrap();
        let frame = terminal.draw(|f| draw(f, f.area(), state, cache)).unwrap();
        frame.buffer.clone()
    }

    #[test]
    fn complex_script_lyrics_are_drawn_as_images_on_kitty() {
        let state = state_with_lyrics(LYRICS, "kitty");
        let mut cache = ImageCache::empty();
        cache.picker.set_protocol_type(ProtocolType::Kitty);
        cache.lyrics = LyricsRenderer::new(true);
        cache.lyrics.wait_for_fonts();

        let buffer = render(&state, &mut cache);
        // Inside the border and the two-column padding, each lyric row is one
        // kitty image, drawn as unicode placeholders from its first cell —
        // the English line too, so the panel reads as one piece.
        for row in 1..=3 {
            let first = buffer[(3, row)].symbol();
            assert!(
                first.contains('\u{10EEEE}'),
                "row {row} is not an image: {first:?}"
            );
            assert!(
                buffer[(4, row)].skip,
                "row {row}: the rest of the row belongs to the image"
            );
        }
    }

    #[test]
    fn without_graphics_urdu_is_drawn_as_text_in_the_order_the_terminal_needs() {
        let state = state_with_lyrics(LYRICS, "text");
        let mut cache = ImageCache::empty();

        let buffer = render(&state, &mut cache);
        let row = |y: u16| (0..50).map(|x| buffer[(x, y)].symbol()).collect::<String>();
        // Joined and in visual order — or left as it is under a terminal that
        // reorders right-to-left text itself, as GNOME Terminal and Konsole do.
        let expected = crate::lyrics::bidi::for_terminal("تمہارے بغیر ہم");
        assert!(row(2).contains(&*expected), "{:?}", row(2));
        assert!(row(3).contains("Never gonna give you up"));
    }
}
