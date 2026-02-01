use melotui_core::AppState;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};
use ratatui_image::StatefulImage;

use crate::{format_duration, image_cache::ImageCache};

const MAX_ART_HEIGHT_PERCENT: u16 = 40;
const MIN_ART_HEIGHT: u16 = 15;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, cache: &mut ImageCache) {
    // Access state via cache
    let art_state = &mut cache.track_info;
    let picker = &cache.picker;

    // Determine selection context
    let track_id = if state.is_library_focused() {
        state.get_selected_track_id_in_sidebar()
    } else {
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

    // Main Block with subtle styling
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Track Info ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .padding(ratatui::widgets::Padding::new(2, 2, 1, 1));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Layout: Art (Dominant), Info (Bottom)
    let art_height = if art_state.source.is_some() {
        // Use configured percentage of height for art
        std::cmp::max(
            MIN_ART_HEIGHT,
            inner_area.height * MAX_ART_HEIGHT_PERCENT / 100,
        )
    } else {
        0
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(art_height), // Art
            Constraint::Min(8),             // Info Area
        ])
        .split(inner_area);

    // 1. Album Art with Manual Center Crop
    if art_state.source.is_some() {
        let area = chunks[0];
        art_state.update_protocol(picker, area);

        if let Some(protocol) = &mut art_state.protocol {
            let image = StatefulImage::default().resize(ratatui_image::Resize::Fit(None));
            frame.render_stateful_widget(image, area, protocol);
        }
    }

    // 2. Info Area Layout
    let info_area = chunks[1];
    let info_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Spacer
            Constraint::Length(2), // Title (Bold, Large)
            Constraint::Length(1), // Artist
            Constraint::Length(1), // Album
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Badges
            Constraint::Length(1), // Details
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Separator
            Constraint::Min(4),    // File Info (Fill rest)
        ])
        .split(info_area);

    // Title: Bold, Cyan, Centered
    let title_line = Line::from(ratatui::text::Span::styled(
        &track.title,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(
        Paragraph::new(title_line).alignment(ratatui::layout::Alignment::Center),
        info_layout[1],
    );

    // Artist: White, Centered
    let artist_str = track.artist.as_deref().unwrap_or("Unknown Artist");
    let artist_line = Line::from(ratatui::text::Span::styled(
        artist_str,
        Style::default().fg(Color::White),
    ));
    frame.render_widget(
        Paragraph::new(artist_line).alignment(ratatui::layout::Alignment::Center),
        info_layout[2],
    );

    // Album: DarkGray, Centered
    if let Some(album) = &track.album {
        let album_line = Line::from(ratatui::text::Span::styled(
            album,
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
        ));
        frame.render_widget(
            Paragraph::new(album_line).alignment(ratatui::layout::Alignment::Center),
            info_layout[3],
        );
    }

    // Audio Tech Specs Row (Badges)
    let bitrate = track.bitrate.unwrap_or(0);
    let sample = track.sample_rate.unwrap_or(0);
    let format = track.format.to_uppercase();

    let mut badge_parts = Vec::new();
    let fmt_color = if format == "FLAC" || format == "WAV" {
        Color::Green
    } else {
        Color::Blue
    };
    badge_parts.push(ratatui::text::Span::styled(
        format!(" {} ", format),
        Style::default()
            .bg(fmt_color)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    ));

    if bitrate > 0 {
        badge_parts.push(ratatui::text::Span::raw(" "));
        badge_parts.push(ratatui::text::Span::styled(
            format!(" {} kbps ", bitrate),
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ));
    }

    if sample > 0 {
        badge_parts.push(ratatui::text::Span::raw(" "));
        badge_parts.push(ratatui::text::Span::styled(
            format!(" {} Hz ", sample),
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ));
    }
    // Fixed: Always render the widget even if empty (or render placeholder space)
    frame.render_widget(
        Paragraph::new(Line::from(badge_parts)).alignment(ratatui::layout::Alignment::Center),
        info_layout[5],
    );

    // Details Row: Year | Genre | Duration
    let mut details = Vec::new();
    if let Some(y) = track.year {
        details.push(format!("{}", y));
    }
    if let Some(g) = &track.genre {
        details.push(g.clone());
    }
    details.push(format_duration(track.duration_ms));
    let details_str = details.join(" • ");
    frame.render_widget(
        Paragraph::new(details_str)
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        info_layout[6],
    );

    // Separator
    let sep = Line::from(ratatui::text::Span::styled(
        "─".repeat(info_area.width as usize),
        Style::default().fg(Color::DarkGray),
    ));
    frame.render_widget(
        Paragraph::new(sep).alignment(ratatui::layout::Alignment::Center),
        info_layout[8],
    );

    // File Details (The "Fill Space" Section)
    let file_info_area = info_layout[9];
    // Calculate values
    let parent_folder = track
        .file_path
        .parent()
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy())
        .unwrap_or("Unknown".into());
    let filename = track
        .file_path
        .file_name()
        .map(|s| s.to_string_lossy())
        .unwrap_or("Unknown".into());
    let size_mb = track.file_size_bytes as f64 / 1024.0 / 1024.0;

    let info_text = vec![
        Line::from(vec![
            ratatui::text::Span::styled("📁 Folder: ", Style::default().fg(Color::Cyan)),
            ratatui::text::Span::raw(parent_folder),
        ]),
        Line::from(vec![
            ratatui::text::Span::styled("📄 File:   ", Style::default().fg(Color::Cyan)),
            ratatui::text::Span::raw(filename),
        ]),
        Line::from(vec![
            ratatui::text::Span::styled("💾 Size:   ", Style::default().fg(Color::Cyan)),
            ratatui::text::Span::raw(format!("{:.2} MB", size_mb)),
        ]),
        Line::from(""), // Spacer
        Line::from(vec![
            ratatui::text::Span::styled("📍 Path:   ", Style::default().fg(Color::DarkGray)),
            ratatui::text::Span::styled(
                track.file_path.to_string_lossy(),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(info_text).wrap(Wrap { trim: true }),
        file_info_area,
    );
}
