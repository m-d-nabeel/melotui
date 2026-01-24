use melotui_core::AppState;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};
use ratatui_image::StatefulImage;

use crate::{format_duration, image_cache::ImageState};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, art_state: &mut ImageState) {
    // Determine selection context
    let track_id = if state.is_library_focused() {
        state.get_selected_track_id_in_sidebar()
    } else {
        // TODO: Consider if this accessor is still needed
        state.get_selected_track_id_in_queue()
    };

    let track_id = match track_id {
        Some(id) => id,
        None => return,
    };

    let track = match state.library.get_track(track_id) {
        Some(t) => t,
        None => return,
    };

    // Container
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" 󰋽 Track Details ");

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Layout
    let art_height = if art_state.protocol.is_some() { 15 } else { 0 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(art_height), Constraint::Min(10)])
        .split(inner_area);

    if let Some(protocol) = &mut art_state.protocol {
        let image = StatefulImage::default();
        frame.render_stateful_widget(image, chunks[0], protocol);
    }

    // Metadata lines
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
    ];

    if let Some(album) = &track.album {
        lines.push(Line::from(vec![
            ratatui::text::Span::styled("󰀥  ", Style::default().fg(Color::Cyan)),
            ratatui::text::Span::styled(album, Style::default().fg(Color::Gray)),
        ]));
    }
    if let Some(year) = track.year {
        lines.push(Line::from(vec![
            ratatui::text::Span::styled("󰃭  ", Style::default().fg(Color::Cyan)),
            ratatui::text::Span::styled(year.to_string(), Style::default().fg(Color::Gray)),
        ]));
    }
    if let Some(genre) = &track.genre {
        lines.push(Line::from(vec![
            ratatui::text::Span::styled("󰽰  ", Style::default().fg(Color::Cyan)),
            ratatui::text::Span::styled(genre, Style::default().fg(Color::Gray)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![ratatui::text::Span::styled(
        format!(
            "{} • {} kbps • {} Hz",
            track.format,
            track.bitrate.unwrap_or(0),
            track.sample_rate.unwrap_or(0)
        ),
        Style::default().fg(Color::DarkGray),
    )]));

    // Duration line
    lines.push(Line::from(vec![
        ratatui::text::Span::styled("Duration: ", Style::default().fg(Color::DarkGray)),
        ratatui::text::Span::styled(
            format_duration(track.duration_ms),
            Style::default().fg(Color::Gray),
        ),
    ]));

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, chunks[1]);
}
