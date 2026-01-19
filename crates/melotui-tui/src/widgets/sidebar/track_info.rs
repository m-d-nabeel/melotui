use melotui_core::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    // 1. Determine Selection Context
    let track_id = if state.is_library_focused() {
        state.get_selected_track_id_in_sidebar()
    } else {
        state.get_queue_selection()
    };

    let track_id = match track_id {
        Some(id) => id,
        None => return, // Nothing selected
    };

    let track = match state.library.get_track(track_id) {
        Some(t) => t,
        None => return,
    };

    // 2. Container
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" 󰋽 Track Details ");

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // 3. Layout (Art Top, Info Bottom)
    let art_height = if state.track_info_art.is_some() {
        15
    } else {
        0
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(art_height), // Art
            Constraint::Min(10),            // Metadata
        ])
        .split(inner_area);

    // 4. ASCII Art
    if let Some(art) = &state.track_info_art {
        let art_widget = Paragraph::new(art.clone()).alignment(Alignment::Center);
        frame.render_widget(art_widget, chunks[0]);
    }

    // 5. Metadata Lines
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

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, chunks[1]);
}
