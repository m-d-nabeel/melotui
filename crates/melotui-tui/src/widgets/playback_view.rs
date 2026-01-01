use crate::{
    view::format_duration,
    widgets::progress_bar_view::{ProgressBarConfig, ProgressBarTheme, render_progress_bar},
};
use melotui_core::{AppState, state_registry::PlaybackStatus};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    // 1. Container Block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Now Playing ")
        .title_style(Style::default().add_modifier(Modifier::BOLD))
        .padding(Padding::horizontal(2));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // 2. Layout
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Progress Bar
        ])
        .split(inner_area);

    // --- TOP ROW: Status & Title ---
    let (status_icon, color) = match state.playback.status {
        PlaybackStatus::Playing => ("▶", Color::Green),
        PlaybackStatus::Paused => ("⏸", Color::Yellow),
        PlaybackStatus::Stopped => ("⏹", Color::Red),
    };

    let raw_name = state
        .playback
        .current
        .as_ref()
        .and_then(|t| t.path.file_name())
        .map(|n| n.to_string_lossy())
        .unwrap_or_else(|| "No Track Selected".into());

    let clean_name = raw_name.replace(".mp3", "").replace(".flac", "");

    let title_line = Line::from(vec![
        Span::styled(format!(" {} ", status_icon), Style::default().fg(color)),
        Span::styled(clean_name, Style::default().add_modifier(Modifier::BOLD)),
    ]);

    frame.render_widget(
        Paragraph::new(title_line).alignment(Alignment::Left),
        v_chunks[0],
    );

    // --- BOTTOM ROW: Thicker Custom Bar ---
    let progress_area = v_chunks[2];

    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(10),
            Constraint::Length(6),
        ])
        .split(progress_area);

    // 1. Current Time
    let current_ms = state.playback.progress_ms;
    frame.render_widget(
        Paragraph::new(format_duration(current_ms))
            .alignment(Alignment::Left)
            .style(Style::default().fg(Color::DarkGray)),
        h_chunks[0],
    );

    // 2. Custom Bar Construction
    let duration_ms = state.playback.duration_ms.unwrap_or(0);
    let ratio = if duration_ms > 0 {
        (current_ms as f64 / duration_ms as f64).min(1.0)
    } else {
        0.0
    };

    let bar_width = h_chunks[1].width as usize;
    let pb_cfg = ProgressBarConfig {
        theme: ProgressBarTheme::Capsule,
        color,
        width: bar_width,
        ratio,
    };
    frame.render_widget(render_progress_bar(pb_cfg), h_chunks[1]);
}
