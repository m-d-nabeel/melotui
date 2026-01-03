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

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, main_title: Line) {
    let p_idx = state.nav.library.selected_playlist_idx;

    if p_idx >= state.library.playlists.len() {
        return;
    }

    let playlist = &state.library.playlists[p_idx];
    let selected_track_idx = state.nav.library.selected_tracks_curs[p_idx];

    let mut combined_spans = main_title.spans; // Take the tabs
    combined_spans.push(Span::raw(" - "));
    combined_spans.push(Span::styled(
        format!("[ {} ]", playlist.name),
        Style::default().fg(Color::Cyan),
    ));
    let final_title = Line::from(combined_spans);

    // Header: Only Title and Time
    let header = Row::new(vec![
        Cell::from(""), // Status Column (acts as left padding)
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
                .is_some_and(|curr| curr.playlist == playlist.name && curr.path == t.file_path);

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

            let display_title = if is_selected {
                let title = &t.title;
                let gap = " ".repeat(6); // 6 spaces
                let speed = 3.0; // chars per second
                let delay = 3.0; // Seconds to wait before starting

                // 1. Calculate time since this row was selected
                let active_time = state.time_elapsed - state.last_nav_time;

                // 2. Prepare the full scrolling string
                let full_text = format!("{}{}{}", title, gap, title);
                let cycle_len = title.chars().count() + gap.chars().count();

                // 3. Calculate the Cycle Duration
                let scroll_duration = cycle_len as f32 / speed;
                let total_cycle_time = delay + scroll_duration;

                // 4. Determine current phase within the loop
                let phase = active_time % total_cycle_time;

                let offset = if phase < delay {
                    0
                } else {
                    ((phase - delay) * speed) as usize
                };

                // 6. Slice the string
                full_text.chars().skip(offset).take(30).collect::<String>()
            } else {
                t.title.clone()
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

            let duration = format_duration(t.duration_ms);

            let row = Row::new(vec![
                Cell::from(status).style(status_style),
                Cell::from(display_title).style(title_style),
                Cell::from(duration).style(Style::default().fg(Color::DarkGray)),
            ]);

            if is_selected {
                row.style(Style::default().bg(Color::Rgb(40, 40, 40)))
            } else {
                row
            }
        })
        .collect();

    let widths = [
        Constraint::Length(2), // Status
        Constraint::Min(10),   // Title
        Constraint::Length(6), // Time
    ];

    let border_style = if state.nav.library.depth == LibraryViewDepth::Tracks
        && state.nav.active_panel == LeftPanelMode::Library
    {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .title(final_title)
            .title_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
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
