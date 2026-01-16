mod progress_bar_view;
mod utils;
use crate::{
    format_duration,
    widgets::playback_view::progress_bar_view::{ProgressBarConfig, ProgressBarTheme},
};
use melotui_core::{AppState, state::RepeatMode};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Flex, Layout, Rect},
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

    // 2. Layout (Responsive with Min and Fill for automatic spacing)
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),  // Title
            Constraint::Fill(1), // Spacer
            Constraint::Min(1),  // Progress Bar
            Constraint::Fill(1), // Spacer
            Constraint::Min(1),  // Next Info
        ])
        .flex(Flex::SpaceEvenly);

    let v_chunks = layout.split(inner_area);

    // --- PREPARE DATA ---
    let current_track = state.get_playing_track();
    let next_song_name = state.predict_next_song();
    let (status_icon, status_color) = utils::get_status_icon(state.playback_status());
    let (source_icon, source_color) = if state.is_playing_from_queue() {
        ("Q", Color::Yellow)
    } else {
        ("♪", Color::Blue)
    };

    // --- ROW 1: Title & Controls ---
    if v_chunks[0].height > 0 {
        let row1_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1), Constraint::Min(10)]) // Left: Title, Right: Icons
            .split(v_chunks[0]);

        // Left: [Status] Title - Artist
        let title_line = if let Some(track) = current_track {
            let artist = track.artist.as_deref().unwrap_or("Unknown");
            Line::from(vec![
                Span::styled(
                    format!("{} ", status_icon),
                    Style::default().fg(status_color),
                ),
                Span::styled(&track.title, Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(" • ", Style::default().fg(Color::DarkGray)),
                Span::styled(artist, Style::default().fg(Color::Gray)),
            ])
        } else {
            Line::from(vec![
                Span::styled(
                    format!("{} ", status_icon),
                    Style::default().fg(status_color),
                ),
                Span::styled(
                    "No Track Selected",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ])
        };
        frame.render_widget(Paragraph::new(title_line), row1_layout[0]);

        // Right: [Source] [Shuffle] [Repeat]
        let controls_line = Line::from(vec![
            Span::styled(source_icon, Style::default().fg(source_color).bold()),
            Span::raw(" "),
            utils::get_shuffle_icon(state.shuffle_active()),
            Span::raw(" "),
            utils::get_repeat_icon(state.repeat_mode()),
        ]);
        frame.render_widget(
            Paragraph::new(controls_line).alignment(Alignment::Right),
            row1_layout[1],
        );
    }

    // --- ROW 3: Progress Bar ---
    if v_chunks[2].height > 0 {
        let progress_area = v_chunks[2];
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(6), // Current Time
                Constraint::Min(10),   // Bar
                Constraint::Length(6), // Total Time
            ])
            .split(progress_area);

        let current_ms = state.progress_ms();
        let duration_ms = state.total_duration_ms();
        let ratio = state.progress_ratio();

        // Render Time Left
        frame.render_widget(
            Paragraph::new(format_duration(current_ms)).style(Style::default().fg(Color::DarkGray)),
            h_chunks[0],
        );

        // Render Bar
        let pb_cfg = ProgressBarConfig {
            theme: ProgressBarTheme::Capsule,
            color: status_color,
            width: h_chunks[1].width as usize,
            ratio,
        };
        frame.render_widget(progress_bar_view::render_progress_bar(pb_cfg), h_chunks[1]);

        // Render Time Right
        let total_str = if duration_ms > 0 {
            format_duration(duration_ms)
        } else {
            "--:--".to_string()
        };
        frame.render_widget(
            Paragraph::new(total_str)
                .alignment(Alignment::Right)
                .style(Style::default().fg(Color::DarkGray)),
            h_chunks[2],
        );
    }

    // --- ROW 5: Next Song Info (dull colors) ---
    if v_chunks[4].height > 0 {
        let next_area = v_chunks[4];
        let (label, name) = match state.repeat_mode() {
            RepeatMode::Track => ("Repeating: ", "Track"),
            RepeatMode::PlayOnce => ("Playing Once: ", "End After Track"),
            RepeatMode::Playlist => ("Up Next (loop): ", next_song_name.as_str()),
            RepeatMode::Off => ("Up Next: ", next_song_name.as_str()),
        };
        let next_text = Line::from(vec![
            Span::styled(label, Style::default().fg(Color::DarkGray)),
            Span::styled(
                name,
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC | Modifier::DIM),
            ),
        ]);
        frame.render_widget(
            Paragraph::new(next_text).alignment(Alignment::Left),
            next_area,
        );
    }
}
