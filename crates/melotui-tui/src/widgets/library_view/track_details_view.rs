use crate::format_duration;
use melotui_core::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let p_idx = state.session.nav.library.selected_playlist_idx;

    if p_idx >= state.library.playlists.len() {
        return;
    }
    let playlist = &state.library.playlists[p_idx];

    let t_idx = state.session.nav.library.selected_tracks_curs[p_idx];
    if t_idx >= playlist.tracks.len() {
        return;
    }

    let track = &playlist.tracks[t_idx];

    // 1. Container Block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Track Info ");

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // 2. Layout: Split into Top (Art) and Bottom (Info)
    let art_height = if state.track_info_art.is_some() {
        15
    } else {
        0
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(art_height), // Dynamic Art Section
            Constraint::Min(10),            // Metadata Section
        ])
        .split(inner_area);

    // 3. Render Album Art
    if let Some(art) = &state.track_info_art {
        let art_widget = Paragraph::new(art.clone())
            .alignment(Alignment::Center)
            .block(Block::default());
        frame.render_widget(art_widget, chunks[0]);
    }

    // 4. Build Visual Content
    let mut lines = vec![
        // --- HEADER: Title & Artist ---
        Line::from(""), // Top padding
        Line::from(Span::styled(
            &track.title,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            track.artist.as_deref().unwrap_or("Unknown Artist"),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::ITALIC),
        )),
        // Separator
        Line::from(Span::styled("──────", Style::default().fg(Color::DarkGray))),
        Line::from(""),
    ];

    // --- METADATA (Icon based) ---
    // Album
    if let Some(album) = &track.album {
        lines.push(Line::from(vec![
            Span::styled("💿 ", Style::default().fg(Color::Cyan)),
            Span::styled(album, Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from("")); // Spacer
    }

    // Genre
    if let Some(genre) = &track.genre {
        lines.push(Line::from(vec![
            Span::styled("🎹 ", Style::default().fg(Color::Cyan)),
            Span::styled(genre, Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(""));
    }

    // Year
    if let Some(year) = track.year {
        lines.push(Line::from(vec![
            Span::styled("📅 ", Style::default().fg(Color::Cyan)),
            Span::styled(year.to_string(), Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(""));
    }

    // --- TECH SPECS ---
    // Separator
    lines.push(Line::from(Span::styled(
        "──────",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));

    // Audio Format & Bitrate
    let bitrate = track.bitrate.unwrap_or(0);
    let format_str = format!("{} • {} kbps", track.format, bitrate);
    lines.push(Line::from(vec![
        Span::styled("🔊 ", Style::default().fg(Color::Green)),
        Span::styled(format_str, Style::default().fg(Color::Gray)),
    ]));

    // Sample Rate
    if let Some(sample) = track.sample_rate {
        lines.push(Line::from(vec![
            Span::styled("📉 ", Style::default().fg(Color::Green)),
            Span::styled(format!("{} Hz", sample), Style::default().fg(Color::Gray)),
        ]));
    }

    // Duration
    lines.push(Line::from(vec![
        Span::styled("⏱  ", Style::default().fg(Color::Green)),
        Span::styled(
            format_duration(track.duration_ms),
            Style::default().fg(Color::Gray),
        ),
    ]));

    // --- FOOTER: File Name (Dimmed) ---
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "File:",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        track
            .file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
        Style::default().fg(Color::DarkGray),
    )));

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, chunks[1]);
}
