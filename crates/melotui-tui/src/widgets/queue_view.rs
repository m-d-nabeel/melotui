use crate::format_duration;
use melotui_core::{
    AppState,
    state_registry::{LeftPanelMode, LibraryViewDepth},
};
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Row, Table, TableState},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let queue = &state.playback_queue.tracks;
    let selected_idx = state.nav.queue.selected_idx;

    let lib_tab_text = match state.nav.library.depth {
        LibraryViewDepth::Playlists => " Library ".to_string(),
        LibraryViewDepth::Tracks | LibraryViewDepth::TrackInfo => {
            let p_idx = state.nav.library.selected_playlist_idx;
            if let Some(pl) = state.library.playlists.get(p_idx) {
                let raw = &pl.name;
                if raw.len() > 15 {
                    format!(" {}... ", &raw[0..12])
                } else {
                    format!(" {} ", raw)
                }
            } else {
                " Library ".to_string()
            }
        }
    };

    let title = Line::from(vec![
        // Inactive Library Tab
        Span::styled(lib_tab_text, Style::default().fg(Color::DarkGray)),
        Span::styled("|", Style::default().fg(Color::DarkGray)),
        // Active Queue Tab
        Span::styled(
            " Queue ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let header = Row::new(vec![
        Cell::from(""),
        Cell::from("#"),
        Cell::from("Title"),
        Cell::from("Time"),
    ])
    .style(
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    );

    let rows: Vec<Row> = queue
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let is_playing = state
                .playback
                .current
                .as_ref()
                .is_some_and(|curr| curr.playlist == "Queue" && curr.path == t.file_path);
            let is_selected = i == selected_idx;
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
                Cell::from((i + 1).to_string()).style(Style::default().fg(Color::DarkGray)),
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

    let border_color = if state.nav.active_panel == LeftPanelMode::Queue {
        Color::Magenta
    } else {
        Color::DarkGray
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Length(3),
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
            .border_style(Style::default().fg(border_color)),
    );

    frame.render_stateful_widget(
        table,
        area,
        &mut TableState::default().with_selected(Some(selected_idx)),
    );
}
