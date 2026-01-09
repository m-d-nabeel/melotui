use melotui_core::{
    AppState,
    state::{LeftPanelMode, LibraryViewDepth, PlaybackContext},
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Row, Table, TableState},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let selected_idx = state.nav.library.selected_playlist_idx;

    let title = Line::from(vec![
        Span::styled(
            " Library ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("|", Style::default().fg(Color::DarkGray)),
        Span::styled(" Queue ", Style::default().fg(Color::DarkGray)),
    ]);

    let rows: Vec<Row> = state
        .library
        .playlists
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let is_selected = i == selected_idx;

            // Check if this specific playlist is the source of the current song
            let is_active_playlist = state
                .playback
                .current
                .as_ref()
                .is_some_and(|curr| matches!(curr.context, PlaybackContext::Library { ref playlist_name } if playlist_name == &p.name));

            let icon = if is_active_playlist { " 󰐑" } else { " 󰲹" };
            let mut text_style = Style::default().add_modifier(Modifier::BOLD);
            if is_active_playlist {
                text_style = text_style.fg(Color::LightRed);
            } else if is_selected {
                text_style = text_style.fg(Color::Yellow);
            } else {
                text_style = Style::default().fg(Color::Cyan); // Remove BOLD for unselected/inactive
            }

            let name_cell = Cell::from(format!("{} {}", icon, p.name)).style(text_style);

            let count_line =
                Line::from(format!("{} songs ", p.tracks.len())).alignment(Alignment::Right);
            let count_cell = Cell::from(count_line).style(Style::default().fg(Color::DarkGray));

            let row = Row::new(vec![name_cell, count_cell]);

            // 3. Apply Selection Background (Navigation)
            if is_selected {
                row.style(Style::default().bg(Color::Rgb(40, 40, 40)))
            } else {
                row
            }
        })
        .collect();

    let border_color = if state.nav.library.depth == LibraryViewDepth::Playlists
        && state.nav.active_panel == LeftPanelMode::Library
    {
        Color::Yellow
    } else {
        Color::Blue
    };

    let table = Table::new(rows, [Constraint::Fill(1), Constraint::Min(10)]).block(
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
