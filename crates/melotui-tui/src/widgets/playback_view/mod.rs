mod progress_bar_view;
mod utils;
use crate::{
    format_duration,
    widgets::playback_view::progress_bar_view::{ProgressBarConfig, ProgressBarTheme},
};
use melotui_core::AppState;
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
        .padding(Padding::horizontal(1)); // Reduce padding slightly to fit more text

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // 2. Layout (3 Rows)
    // Row 1: Title/Artist & Playback Controls
    // Row 2: Next Song & Source Info
    // Row 3: Progress Bar
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top
            Constraint::Length(1), // Middle (Replaces spacer)
            Constraint::Length(1), // Bottom
        ])
        .split(inner_area);

    // --- PREPARE DATA ---
    let current_track = state.playback.current.as_ref().and_then(|id| {
        if id.playlist == "Queue" {
            state
                .playback_queue
                .tracks
                .iter()
                .find(|t| t.file_path == id.path)
        } else {
            state
                .library
                .playlists
                .iter()
                .find(|p| p.name == id.playlist)
                .and_then(|pl| pl.tracks.iter().find(|t| t.file_path == id.path))
        }
    });

    let next_song_name = utils::predict_next_song(state);
    let (status_icon, status_color) = utils::get_status_icon(&state.playback.status);

    // --- ROW 1: Title & Controls ---
    let row1_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Fill(1), Constraint::Min(10)]) // Left: Title, Right: Icons
        .split(v_chunks[0]);

    // Left: [Icon] Title - Artist
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

    // Right: [Shuffle] [Repeat]
    let controls_line = Line::from(vec![
        utils::get_shuffle_icon(state.playback.shuffle),
        Span::raw(" "),
        utils::get_repeat_icon(&state.repeat_mode),
    ]);
    frame.render_widget(
        Paragraph::new(controls_line).alignment(Alignment::Right),
        row1_layout[1],
    );

    // --- ROW 2: Next Song & Source ---
    let row2_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Fill(2), Constraint::Fill(1)])
        .split(v_chunks[1]);

    // Left: "Next: ..."
    let next_text = Line::from(vec![
        Span::styled("Next: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            next_song_name,
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
        ),
    ]);
    frame.render_widget(Paragraph::new(next_text), row2_layout[0]);

    // Right: "From: Queue" or "From: Library"
    let source_name = state
        .playback
        .current
        .as_ref()
        .map(|id| id.playlist.as_str())
        .unwrap_or("-");
    let source_color = if source_name == "Queue" {
        Color::Yellow
    } else {
        Color::Blue
    };

    let source_text = Line::from(vec![
        Span::styled("Src: ", Style::default().fg(Color::DarkGray)),
        Span::styled(source_name, Style::default().fg(source_color)),
    ]);
    frame.render_widget(
        Paragraph::new(source_text).alignment(Alignment::Right),
        row2_layout[1],
    );

    // --- ROW 3: Progress Bar ---
    let progress_area = v_chunks[2];
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(6), // Current Time
            Constraint::Min(10),   // Bar
            Constraint::Length(6), // Total Time
        ])
        .split(progress_area);

    let current_ms = state.playback.progress_ms;
    let duration_ms = current_track
        .map(|t| t.duration_ms)
        .or(state.playback.duration_ms)
        .unwrap_or(0);

    let ratio = if duration_ms > 0 {
        (current_ms as f64 / duration_ms as f64).min(1.0)
    } else {
        0.0
    };

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

// mod progress_bar_view;
// use crate::{
//     format_duration,
//     widgets::playback_view::progress_bar_view::{
//         ProgressBarConfig, ProgressBarTheme, render_progress_bar,
//     },
// };
// use melotui_core::{AppState, state_registry::PlaybackStatus};
// use ratatui::{
//     Frame,
//     layout::{Alignment, Constraint, Direction, Layout, Rect},
//     style::{Color, Modifier, Style},
//     text::{Line, Span},
//     widgets::{Block, BorderType, Borders, Padding, Paragraph},
// };
//
// pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
//     // 1. Container Block
//     let block = Block::default()
//         .borders(Borders::ALL)
//         .border_type(BorderType::Rounded)
//         .title(" Now Playing ")
//         .title_style(Style::default().add_modifier(Modifier::BOLD))
//         .padding(Padding::horizontal(2));
//
//     let inner_area = block.inner(area);
//     frame.render_widget(block, area);
//
//     // 2. Layout
//     let v_chunks = Layout::default()
//         .direction(Direction::Vertical)
//         .constraints([
//             Constraint::Length(1), // Title Line
//             Constraint::Length(1), // Spacer
//             Constraint::Length(1), // Progress Bar Line
//         ])
//         .split(inner_area);
//
//     // --- METADATA LOOKUP ---
//     // We look up the full Track object from the library using the current ID
//     // so we can get the real Title, Artist, and Duration.
//     let current_track = state.playback.current.as_ref().and_then(|id| {
//         if id.playlist == "Queue" {
//             state
//                 .playback_queue
//                 .tracks
//                 .iter()
//                 .find(|t| t.file_path == id.path)
//         } else {
//             state
//                 .library
//                 .playlists
//                 .iter()
//                 .find(|p| p.name == id.playlist)
//                 .and_then(|pl| pl.tracks.iter().find(|t| t.file_path == id.path))
//         }
//     });
//
//     // --- TOP ROW: Status & Song Info ---
//     let (status_icon, color) = match state.playback.status {
//         PlaybackStatus::Playing => ("▶", Color::Green),
//         PlaybackStatus::Paused => ("⏸", Color::Yellow),
//         PlaybackStatus::Stopped => ("⏹", Color::Red),
//     };
//
//     let title_line = if let Some(track) = current_track {
//         // Option A: Rich Metadata (Title - Artist)
//         let artist = track.artist.as_deref().unwrap_or("Unknown Artist");
//         Line::from(vec![
//             Span::styled(format!(" {} ", status_icon), Style::default().fg(color)),
//             Span::styled(&track.title, Style::default().add_modifier(Modifier::BOLD)),
//             Span::styled(" • ", Style::default().fg(Color::DarkGray)),
//             Span::styled(artist, Style::default().fg(Color::Gray)),
//         ])
//     } else {
//         // Option B: Fallback (Filename)
//         let name = state
//             .playback
//             .current
//             .as_ref()
//             .and_then(|t| t.path.file_name())
//             .map(|n| n.to_string_lossy().to_string())
//             .unwrap_or_else(|| "No Track Selected".to_string());
//
//         Line::from(vec![
//             Span::styled(format!(" {} ", status_icon), Style::default().fg(color)),
//             Span::styled(name, Style::default().add_modifier(Modifier::BOLD)),
//         ])
//     };
//
//     frame.render_widget(
//         Paragraph::new(title_line).alignment(Alignment::Left),
//         v_chunks[0],
//     );
//
//     // --- BOTTOM ROW: Progress Area ---
//     let progress_area = v_chunks[2];
//
//     let h_chunks = Layout::default()
//         .direction(Direction::Horizontal)
//         .constraints([
//             Constraint::Length(6), // Current Time
//             Constraint::Min(10),   // Bar (Flexible)
//             Constraint::Length(6), // Total Time
//         ])
//         .split(progress_area);
//
//     // 1. Current Time
//     let current_ms = state.playback.progress_ms;
//     frame.render_widget(
//         Paragraph::new(format_duration(current_ms))
//             .alignment(Alignment::Left)
//             .style(Style::default().fg(Color::DarkGray)),
//         h_chunks[0],
//     );
//
//     // 2. Custom Bar Construction
//     // Prefer the duration from the Library metadata (it is stable and scanned),
//     // fallback to state.playback.duration_ms if needed.
//     let duration_ms = current_track
//         .map(|t| t.duration_ms)
//         .or(state.playback.duration_ms)
//         .unwrap_or(0);
//
//     let ratio = if duration_ms > 0 {
//         (current_ms as f64 / duration_ms as f64).min(1.0)
//     } else {
//         0.0
//     };
//
//     let pb_cfg = ProgressBarConfig {
//         theme: ProgressBarTheme::Capsule, // Or Heavy/Bar
//         color,
//         width: h_chunks[1].width as usize,
//         ratio,
//     };
//     frame.render_widget(render_progress_bar(pb_cfg), h_chunks[1]);
//
//     // 3. Total Time (Correctly Displayed)
//     let total_str = if duration_ms > 0 {
//         format_duration(duration_ms)
//     } else {
//         "--:--".to_string()
//     };
//
//     frame.render_widget(
//         Paragraph::new(total_str)
//             .alignment(Alignment::Right)
//             .style(Style::default().fg(Color::DarkGray)),
//         h_chunks[2],
//     );
// }
