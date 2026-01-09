use crate::format_duration;
use melotui_core::{
    AppState,
    state_registry::{LeftPanelMode, LibraryViewDepth, PlaybackContext},
};
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Row, Table, TableState},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let p_idx = state.nav.library.selected_playlist_idx;
    if p_idx >= state.library.playlists.len() {
        return;
    }
    let playlist = &state.library.playlists[p_idx];
    let selected_track_idx = state.nav.library.selected_tracks_curs[p_idx];

    let raw_name = &playlist.name;
    let display_name = if raw_name.len() > 15 {
        format!("{}...", &raw_name[0..12])
    } else {
        raw_name.clone()
    };

    // 2. Build Title: "[ Playlist... ] | Queue"
    let title = Line::from(vec![
        Span::styled(
            format!(" {} ", display_name),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("|", Style::default().fg(Color::DarkGray)),
        Span::styled(" Queue ", Style::default().fg(Color::DarkGray)),
    ]);

    let header = Row::new(vec![
        Cell::from(""),
        Cell::from("Title"),
        Cell::from("Time"),
    ])
    .style(
        Style::default()
            .fg(Color::LightBlue)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    );

    let rows: Vec<Row> = playlist
        .tracks
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let is_playing = state
                .playback
                .current
                .as_ref()
                .is_some_and(|curr| matches!(curr.context, PlaybackContext::Library { ref playlist_name } if playlist_name == &playlist.name) && curr.path == t.file_path);
            let is_selected = i == selected_track_idx;

            let status = if is_playing {
                " ▶"
            } else if is_selected {
                " ●"
            } else {
                "  "
            };
            let status_style = if is_playing {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let title_style = if is_playing {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            let row = Row::new(vec![
                Cell::from(status).style(status_style),
                Cell::from(t.title.clone()).style(title_style),
                Cell::from(format_duration(t.duration_ms))
                    .style(Style::default().fg(Color::DarkGray)),
            ]);
            if is_selected {
                row.style(Style::default().bg(Color::Rgb(40, 40, 40)))
            } else {
                row
            }
        })
        .collect();

    let border_style = if state.nav.library.depth == LibraryViewDepth::Tracks
        && state.nav.active_panel == LeftPanelMode::Library
    {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Min(10),
            Constraint::Length(6),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style),
    );

    frame.render_stateful_widget(
        table,
        area,
        &mut TableState::default().with_selected(Some(selected_track_idx)),
    );
}
