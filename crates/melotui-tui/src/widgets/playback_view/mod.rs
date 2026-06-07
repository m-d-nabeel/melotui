mod progress_bar_view;
mod utils;
use crate::{
    format_duration,
    theme::Theme,
    widgets::playback_view::progress_bar_view::{ProgressBarConfig, ProgressBarTheme},
};
use melotui_core::{AppState, playback::state::RepeatMode};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Flex, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
};

fn theme() -> &'static Theme {
    static THEME: std::sync::OnceLock<Theme> = std::sync::OnceLock::new();
    THEME.get_or_init(Theme::default)
}

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    // Container block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Now Playing ")
        .title_style(Style::default().add_modifier(Modifier::BOLD))
        .padding(Padding::horizontal(2));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Layout with responsive spacing
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

    // Prepare data
    let current_track = state.get_playing_track();
    let next_song_name = state.predict_next_song();
    let (status_icon, status_color) = utils::get_status_icon(state.playback_status());
    let t = theme();
    let (source_icon, source_color) = if state.is_playing_from_queue() {
        ("Q", t.accent_warning)
    } else {
        ("♪", t.accent_info)
    };

    // Row 1: title & controls
    if v_chunks[0].height > 0 {
        let row1_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1), Constraint::Min(10)]) // Left: Title, Right: Icons
            .split(v_chunks[0]);

        // Left: status, title, artist
        let title_line = if let Some(track) = current_track {
            let artist = track.artist.as_deref().unwrap_or("Unknown");
            Line::from(vec![
                Span::styled(
                    format!("{} ", status_icon),
                    Style::default().fg(status_color),
                ),
                Span::styled(&track.title, Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(" • ", Style::default().fg(t.fg_muted)),
                Span::styled(artist, Style::default().fg(t.fg_secondary)),
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

        // Right: source, shuffle, repeat
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

    // Row 3: progress bar
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

        // Render time left
        frame.render_widget(
            Paragraph::new(format_duration(current_ms)).style(Style::default().fg(t.fg_muted)),
            h_chunks[0],
        );

        // Render bar
        let pb_cfg = ProgressBarConfig {
            theme: ProgressBarTheme::Capsule,
            color: status_color,
            width: h_chunks[1].width as usize,
            ratio,
        };
        frame.render_widget(progress_bar_view::render_progress_bar(pb_cfg), h_chunks[1]);

        // Render time right
        let total_str = if duration_ms > 0 {
            format_duration(duration_ms)
        } else {
            "--:--".to_string()
        };
        frame.render_widget(
            Paragraph::new(total_str)
                .alignment(Alignment::Right)
                .style(Style::default().fg(t.fg_muted)),
            h_chunks[2],
        );
    }

    // Row 5: next song info
    if v_chunks[4].height > 0 {
        let next_area = v_chunks[4];
        let (label, name) = match state.repeat_mode() {
            RepeatMode::Track => ("Repeating: ", "Track"),
            RepeatMode::PlayOnce => ("Playing Once: ", "End After Track"),
            RepeatMode::Playlist => ("Up Next (loop): ", next_song_name.as_str()),
            RepeatMode::Off => ("Up Next: ", next_song_name.as_str()),
        };
        let next_text = Line::from(vec![
            Span::styled(label, Style::default().fg(t.fg_muted)),
            Span::styled(
                name,
                Style::default()
                    .fg(t.fg_muted)
                    .add_modifier(Modifier::ITALIC | Modifier::DIM),
            ),
        ]);
        frame.render_widget(
            Paragraph::new(next_text).alignment(Alignment::Left),
            next_area,
        );
    }
}
