use melotui_core::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph, Wrap},
};
use ratatui_image::StatefulImage;

use crate::{format_duration, image_cache::ImageCache, theme::Theme};

fn theme() -> &'static Theme {
    static THEME: std::sync::OnceLock<Theme> = std::sync::OnceLock::new();
    THEME.get_or_init(Theme::default)
}

const MAX_ART_HEIGHT_PERCENT: u16 = 40;
const MIN_ART_HEIGHT: u16 = 15;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, cache: &mut ImageCache) {
    let art_state = &mut cache.track_info;
    let picker = &cache.picker;

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

    let t = theme();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border_inactive))
        .title(" Track Info ")
        .title_style(
            Style::default()
                .fg(t.accent_primary)
                .add_modifier(Modifier::BOLD),
        )
        .padding(Padding::new(2, 2, 1, 1));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Art + info split - Always allocate space for art to prevent layout jumping
    let art_height = std::cmp::max(
        MIN_ART_HEIGHT,
        inner_area.height * MAX_ART_HEIGHT_PERCENT / 100,
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(art_height), Constraint::Min(4)])
        .split(inner_area);

    // Album art or placeholder
    if art_state.source.is_some() {
        let art_area = chunks[0];
        art_state.update_protocol(picker, art_area);

        if let Some(protocol) = &mut art_state.protocol {
            let image = StatefulImage::default().resize(ratatui_image::Resize::Fit(None));
            frame.render_stateful_widget(image, art_area, protocol);
        }
    } else {
        let art_area = chunks[0];
        let placeholder = Paragraph::new("♪")
            .alignment(Alignment::Center)
            .style(Style::default().fg(t.fg_muted).add_modifier(Modifier::BOLD));

        let y_offset = art_area.height.saturating_sub(1) / 2;
        let centered_rect = Rect {
            x: art_area.x,
            y: art_area.y + y_offset,
            width: art_area.width,
            height: 1,
        };

        frame.render_widget(placeholder, centered_rect);
    }

    // Adaptive info area — degrade gracefully based on available height
    let info_area = chunks[1];
    let available = info_area.height as usize;

    if available < 3 {
        // Ultra-compact: just title
        render_title(frame, info_area, track, t);
        return;
    }

    if available < 8 {
        // Compact: title + artist + album
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Title
                Constraint::Length(1), // Artist
                Constraint::Length(1), // Album
                Constraint::Min(0),    // Flex spacer
            ])
            .split(info_area);

        render_title(frame, layout[0], track, t);
        render_artist(frame, layout[1], track, t);
        render_album(frame, layout[2], track, t);
        return;
    }

    if available < 14 {
        // Medium: title + artist + album + badges + details
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Spacer
                Constraint::Length(2), // Title
                Constraint::Length(1), // Artist
                Constraint::Length(1), // Album
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Badges
                Constraint::Length(1), // Details
                Constraint::Min(0),    // Flex
            ])
            .split(info_area);

        render_title(frame, layout[1], track, t);
        render_artist(frame, layout[2], track, t);
        render_album(frame, layout[3], track, t);
        render_badges(frame, layout[5], track, t);
        render_details(frame, layout[6], track, t);
        return;
    }

    // Full: everything
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Spacer
            Constraint::Length(2), // Title
            Constraint::Length(1), // Artist
            Constraint::Length(1), // Album
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Badges
            Constraint::Length(1), // Details
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Separator
            Constraint::Min(4),    // File Info
        ])
        .split(info_area);

    render_title(frame, layout[1], track, t);
    render_artist(frame, layout[2], track, t);
    render_album(frame, layout[3], track, t);
    render_badges(frame, layout[5], track, t);
    render_details(frame, layout[6], track, t);

    // Separator
    let sep = Line::from(Span::styled(
        "─".repeat(info_area.width as usize),
        Style::default().fg(t.fg_muted),
    ));
    frame.render_widget(Paragraph::new(sep).alignment(Alignment::Center), layout[8]);

    // File info
    render_file_info(frame, layout[9], track, t);
}

// ── Render helpers ──────────────────────────────────────────────────────────

fn render_title(frame: &mut Frame, area: Rect, track: &melotui_core::domain::TrackMeta, t: &Theme) {
    let line = Line::from(Span::styled(
        &track.title,
        Style::default()
            .fg(t.accent_primary)
            .add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(Paragraph::new(line).alignment(Alignment::Center), area);
}

fn render_artist(
    frame: &mut Frame,
    area: Rect,
    track: &melotui_core::domain::TrackMeta,
    t: &Theme,
) {
    let artist_str = track.artist.as_deref().unwrap_or("Unknown Artist");
    let line = Line::from(Span::styled(artist_str, Style::default().fg(t.fg_primary)));
    frame.render_widget(Paragraph::new(line).alignment(Alignment::Center), area);
}

fn render_album(frame: &mut Frame, area: Rect, track: &melotui_core::domain::TrackMeta, t: &Theme) {
    if let Some(album) = &track.album {
        let line = Line::from(Span::styled(
            album,
            Style::default()
                .fg(t.fg_secondary)
                .add_modifier(Modifier::ITALIC),
        ));
        frame.render_widget(Paragraph::new(line).alignment(Alignment::Center), area);
    }
}

fn render_badges(
    frame: &mut Frame,
    area: Rect,
    track: &melotui_core::domain::TrackMeta,
    t: &Theme,
) {
    let bitrate = track.bitrate.unwrap_or(0);
    let sample = track.sample_rate.unwrap_or(0);
    let format = track.format.to_uppercase();

    let mut parts = Vec::new();
    let fmt_color = if format == "FLAC" || format == "WAV" {
        t.badge_lossless
    } else {
        t.badge_lossy
    };
    parts.push(Span::styled(
        format!(" {} ", format),
        Style::default()
            .bg(fmt_color)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    ));

    if bitrate > 0 {
        parts.push(Span::raw(" "));
        parts.push(Span::styled(
            format!(" {} kbps ", bitrate),
            Style::default().bg(t.badge_detail_bg).fg(t.fg_primary),
        ));
    }

    if sample > 0 {
        parts.push(Span::raw(" "));
        parts.push(Span::styled(
            format!(" {} Hz ", sample),
            Style::default().bg(t.badge_detail_bg).fg(t.fg_primary),
        ));
    }

    frame.render_widget(
        Paragraph::new(Line::from(parts)).alignment(Alignment::Center),
        area,
    );
}

fn render_details(
    frame: &mut Frame,
    area: Rect,
    track: &melotui_core::domain::TrackMeta,
    t: &Theme,
) {
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
            .alignment(Alignment::Center)
            .style(Style::default().fg(t.fg_muted)),
        area,
    );
}

fn render_file_info(
    frame: &mut Frame,
    area: Rect,
    track: &melotui_core::domain::TrackMeta,
    t: &Theme,
) {
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
            Span::styled("📁 Folder: ", Style::default().fg(t.accent_primary)),
            Span::raw(parent_folder),
        ]),
        Line::from(vec![
            Span::styled("📄 File:   ", Style::default().fg(t.accent_primary)),
            Span::raw(filename),
        ]),
        Line::from(vec![
            Span::styled("💾 Size:   ", Style::default().fg(t.accent_primary)),
            Span::raw(format!("{:.2} MB", size_mb)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("📍 Path:   ", Style::default().fg(t.fg_muted)),
            Span::styled(
                track.file_path.to_string_lossy(),
                Style::default()
                    .fg(t.fg_muted)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]),
    ];

    frame.render_widget(Paragraph::new(info_text).wrap(Wrap { trim: true }), area);
}
