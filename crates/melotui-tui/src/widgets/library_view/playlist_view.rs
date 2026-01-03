use melotui_core::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, Cell, Row, Table, TableState},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let selected_idx = state.nav.selected_playlist;

    // Convert Playlists to Table Rows
    let rows: Vec<Row> = state
        .library
        .playlists
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let is_selected = i == selected_idx;

            // Column 1: Playlist Name
            let name_cell = Cell::from(format!("  {}", p.name)).style(if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            });

            // Column 2: Track Count (Right aligned visually by the column constraint)
            let count_line = Line::from(format!("{} songs ", p.tracks.len()))
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Right);
            let count_cell = Cell::from(count_line);

            let row = Row::new(vec![name_cell, count_cell]);

            if is_selected {
                row.style(Style::default().bg(Color::DarkGray))
            } else {
                row
            }
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Fill(1), // remaining space
            Constraint::Min(15), // minimum 15 chars
        ],
    )
    .block(
        Block::default()
            .title(" Playlists ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Blue)),
    )
    .row_highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_stateful_widget(
        table,
        area,
        &mut TableState::default().with_selected(Some(selected_idx)),
    );
}
