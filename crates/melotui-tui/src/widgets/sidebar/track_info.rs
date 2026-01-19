use crate::format_duration;
use melotui_core::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let track_id = match state.get_queue_selection() {
        Some(id) => id,
        None => return,
    };

    let track = match state.library.get_track(track_id) {
        Some(t) => t,
        None => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Track Info ");

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let art_height = if state.track_info_art.is_some() {
        15
    } else {
        0
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(art_height), Constraint::Min(10)])
        .split(inner_area);

    if let Some(art) = &state.track_info_art {
        let art_widget = Paragraph::new(art.clone())
            .alignment(Alignment::Center)
            .block(Block::default());
        frame.render_widget(art_widget, chunks[0]);
    }

    let mut lines = vec![
        Line::from(""),
        Line::from(ratatui::text::Span::styled(
            &track.title,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(ratatui::text::Span::styled(
            track.artist.as_deref().unwrap_or("Unknown Artist"),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::ITALIC),
        )),
        Line::from(ratatui::text::Span::styled(
            "──────",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
    ];

    if let Some(album) = &track.album {
        lines.push(Line::from(vec![
            ratatui::text::Span::styled("  ", Style::default().fg(Color::Cyan)),
            ratatui::text::Span::styled(album, Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(""));
    }

    if let Some(genre) = &track.genre {
        lines.push(Line::from(vec![
            ratatui::text::Span::styled("  ", Style::default().fg(Color::Cyan)),
            ratatui::text::Span::styled(genre, Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(""));
    }

    if let Some(year) = track.year {
        lines.push(Line::from(vec![
            ratatui::text::Span::styled("  ", Style::default().fg(Color::Cyan)),
            ratatui::text::Span::styled(year.to_string(), Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(ratatui::text::Span::styled(
        "──────",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));

    let bitrate = track.bitrate.unwrap_or(0);
    let format_str = format!("{} • {} kbps", track.format, bitrate);
    lines.push(Line::from(vec![
        ratatui::text::Span::styled("  ", Style::default().fg(Color::Green)),
        ratatui::text::Span::styled(format_str, Style::default().fg(Color::Gray)),
    ]));

    if let Some(sample) = track.sample_rate {
        lines.push(Line::from(vec![
            ratatui::text::Span::styled("  ", Style::default().fg(Color::Green)),
            ratatui::text::Span::styled(format!("{} Hz", sample), Style::default().fg(Color::Gray)),
        ]));
    }

    lines.push(Line::from(vec![
        ratatui::text::Span::styled("  ", Style::default().fg(Color::Green)),
        ratatui::text::Span::styled(
            format_duration(track.duration_ms),
            Style::default().fg(Color::Gray),
        ),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(ratatui::text::Span::styled(
        "File:",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(ratatui::text::Span::styled(
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
