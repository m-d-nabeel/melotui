use melotui_core::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
};

use crate::image_cache::ImageCache;
use crate::lyrics::bidi;

/// Draws the lyrics, `zoom` times the text size where the terminal can show
/// them as images.
pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, cache: &mut ImageCache, zoom: f32) {
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

    // The line to keep in the middle: the one being sung, or for plain
    // lyrics, as far down them as the song has got, so they can all be read.
    let anchor = match state.lyrics_data() {
        melotui_core::state::lyrics::LyricsData::Unsynced(_) => {
            unsynced_line(lines.len(), state.progress_ms(), state.total_duration_ms())
        }
        _ => active_index,
    };

    let texts: Vec<&str> = lines.iter().map(|(text, _)| *text).collect();
    let picker = &cache.picker;
    let shaped = &mut cache.lyrics;

    // As one image where the terminal can show it; as text where it cannot,
    // or while the fonts are still loading.
    let drawn = shaped.should_shape(picker, &texts, zoom)
        && shaped.draw(frame, inner, picker, &lines, anchor, zoom);
    if !drawn {
        shaped.clear();
        let viewport_height = inner.height as usize;
        let scroll_offset = anchor.map_or(0, |idx| idx.saturating_sub(viewport_height / 2));
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

/// Of `count` plain lyric lines, the one as far down them as `progress` is
/// through a song `duration` long.
fn unsynced_line(count: usize, progress: u64, duration: u64) -> Option<usize> {
    (duration > 0 && count > 0).then(|| {
        let through = (progress as f64 / duration as f64).clamp(0.0, 1.0);
        (through * (count - 1) as f64).round() as usize
    })
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

    // Scrolled to keep the selected result in sight, both columns together.
    let visible = (inner.height as usize).saturating_sub(1).max(1);
    let skip = search_state.selected_row.saturating_sub(visible - 1);

    // LRCLIB (Left column)
    let mut lrclib_lines = vec![Line::from(vec![Span::styled(
        "LRCLIB",
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(t.accent_info),
    )])];
    for (i, res) in search_state.lrclib_results.iter().enumerate().skip(skip) {
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
    for (i, res) in search_state.ovh_results.iter().enumerate().skip(skip) {
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
        render_at(state, cache, 8, 1.0)
    }

    fn render_at(state: &AppState, cache: &mut ImageCache, height: u16, zoom: f32) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(50, height)).unwrap();
        let frame = terminal
            .draw(|f| draw(f, f.area(), state, cache, zoom))
            .unwrap();
        frame.buffer.clone()
    }

    fn kitty_cache() -> ImageCache {
        let mut cache = ImageCache::empty();
        cache.picker.set_protocol_type(ProtocolType::Kitty);
        cache.lyrics = LyricsRenderer::new(true);
        cache.lyrics.wait_for_fonts();
        cache
    }

    /// Whether row `y` of the panel shows an image.
    fn image_row(buffer: &Buffer, y: u16) -> bool {
        buffer[(3, y)].symbol().contains('\u{10EEEE}')
    }

    #[test]
    fn complex_script_lyrics_are_drawn_as_images_on_kitty() {
        let state = state_with_lyrics(LYRICS, "kitty");
        let mut cache = kitty_cache();

        let buffer = render(&state, &mut cache);
        // Inside the border and the two-column padding, each lyric row is one
        // kitty image, drawn as unicode placeholders, one a cell — the
        // English line too, so the panel reads as one piece.
        for row in 1..=3 {
            let first = buffer[(3, row)].symbol();
            assert!(
                first.contains('\u{10EEEE}'),
                "row {row} is not an image: {first:?}"
            );
            assert!(
                buffer[(4, row)].symbol().starts_with('\u{10EEEE}'),
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

    #[test]
    fn zoomed_lyrics_fill_the_panel_with_as_many_lines_as_fit() {
        let state = state_with_lyrics(LYRICS, "zoomed");
        let mut cache = kitty_cache();
        // Six rows inside the border: three lines, two rows apart.
        let buffer = render_at(&state, &mut cache, 8, 2.0);
        for y in 1..=6 {
            assert!(image_row(&buffer, y), "row {y} is not part of the image");
        }
        assert_eq!(cache.lyrics.shown_lines(), Some(3));
    }

    #[test]
    fn zoomed_lyrics_in_any_script_are_images_and_unzoomed_latin_stays_text() {
        let state = state_with_lyrics("[00:01.00]Never gonna give you up\n", "latin");
        let mut cache = kitty_cache();
        let text = render_at(&state, &mut cache, 8, 1.0);
        let row = (0..50).map(|x| text[(x, 1)].symbol()).collect::<String>();
        assert!(row.contains("Never gonna give you up"), "{row:?}");

        let zoomed = render_at(&state, &mut cache, 8, 2.0);
        assert!(image_row(&zoomed, 1) && image_row(&zoomed, 2));
    }

    #[test]
    fn a_panel_too_short_for_the_zoom_still_shows_the_line_being_sung() {
        let state = state_with_lyrics(LYRICS, "short");
        let mut cache = kitty_cache();
        // Two rows inside the border: smaller than 4x, but there.
        let buffer = render_at(&state, &mut cache, 4, 4.0);
        assert!(image_row(&buffer, 1) && image_row(&buffer, 2));
        assert_eq!(cache.lyrics.shown_lines(), Some(1));
    }

    #[test]
    fn every_row_of_a_lyrics_image_reaches_the_terminal() {
        // Checked in what the terminal is sent, not in the frame: the frame
        // held every row while only the first was written.
        let state = state_with_lyrics(LYRICS, "sent");
        let mut cache = kitty_cache();
        let mut terminal = Terminal::new(TestBackend::new(50, 8)).unwrap();
        terminal
            .draw(|f| draw(f, f.area(), &state, &mut cache, 2.0))
            .unwrap();
        let sent = terminal.backend().buffer();
        for y in 1..=6 {
            assert!(
                sent[(3, y)].symbol().contains('\u{10EEEE}'),
                "row {y} of the image was never written: {:?}",
                sent[(3, y)].symbol()
            );
        }
    }

    #[test]
    fn plain_lyrics_move_down_as_the_song_goes_on() {
        assert_eq!(unsynced_line(41, 0, 200_000), Some(0));
        assert_eq!(unsynced_line(41, 100_000, 200_000), Some(20));
        assert_eq!(unsynced_line(41, 250_000, 200_000), Some(40));
        assert_eq!(unsynced_line(41, 1_000, 0), None, "length unknown");
    }

    #[test]
    fn the_selected_search_result_stays_in_sight() {
        use melotui_core::state::lyrics::{LyricsApi, LyricsCandidate, LyricsSearchState};
        let result = |n: usize| LyricsCandidate {
            api: LyricsApi::Lrclib,
            track_name: format!("Song {n}"),
            artist_name: "Artist".into(),
            is_synced: true,
            provider_id: None,
        };
        let search = LyricsSearchState {
            lrclib_results: (0..8).map(result).collect(),
            ovh_results: vec![],
            selected_column: 0,
            selected_row: 6,
        };
        let theme = crate::theme::Theme::default();
        let mut terminal = Terminal::new(TestBackend::new(60, 5)).unwrap();
        let frame = terminal
            .draw(|f| draw_search(f, f.area(), &search, &theme))
            .unwrap();
        let text: String = frame.buffer.content().iter().map(|c| c.symbol()).collect();
        // A header and four rows: the fourth is the selected one.
        assert!(text.contains("Song 6"), "{text}");
        assert!(
            !text.contains("Song 2") && !text.contains("Song 7"),
            "{text}"
        );
    }
}
