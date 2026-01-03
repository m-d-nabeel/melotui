use crate::format_duration;
use melotui_core::{AppState, state_registry::LeftPanelMode};
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::Line, // Import Line
    widgets::{Block, BorderType, Borders, Cell, Row, Table, TableState},
};

// SIGNATURE CHANGED: Added `title: Line`
pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, title: Line) {
    let queue = &state.playback_queue.tracks;
    let selected_idx = state.nav.queue.selected_idx;

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

    // ... (Row generation logic remains exactly the same as before) ...
    let rows: Vec<Row> = queue
        .iter()
        .enumerate()
        .map(|(i, t)| {
            // ... copy existing row logic ...
            let is_playing = state
                .playback
                .current
                .as_ref()
                .is_some_and(|curr| curr.playlist == "Queue" && curr.path == t.file_path);
            let is_selected = i == selected_idx;

            // Re-use your existing styles here for brevity in this snippet
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

    let widths = [
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Min(10),
        Constraint::Length(6),
    ];

    let border_color = if state.nav.active_panel == LeftPanelMode::Queue {
        Color::Magenta
    } else {
        Color::DarkGray
    };

    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .title(title) // <--- USE THE SHARED TITLE HERE
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
