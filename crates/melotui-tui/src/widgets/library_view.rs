use crate::view::format_duration;
use melotui_core::{AppState, NavMode};
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Cell, Row, Table, TableState},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    match state.nav.mode {
        NavMode::Playlists => draw_playlists(frame, area, state),
        NavMode::Tracks => draw_tracks(frame, area, state),
    }
}

fn draw_playlists(frame: &mut Frame, area: Rect, state: &AppState) {
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
            let count_cell = Cell::from(format!("{} songs ", p.tracks.len()))
                .style(Style::default().fg(Color::DarkGray));

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
            Constraint::Percentage(70), // Name
            Constraint::Percentage(30), // Count
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

fn draw_tracks(frame: &mut Frame, area: Rect, state: &AppState) {
    let p_idx = state.nav.selected_playlist;

    if p_idx >= state.library.playlists.len() {
        return;
    }

    let playlist = &state.library.playlists[p_idx];
    let selected_track_idx = state.nav.selected_tracks[p_idx];

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
    )
    .bottom_margin(1);

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

            // Status: Added spaces for "breathing room" (padding)
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
                let gap = "      "; // 6 spaces
                let speed = 3.0; // chars per second
                let delay = 1.5; // Seconds to wait before starting

                // 1. Calculate time since this row was selected
                let active_time = state.time_elapsed - state.last_nav_time;

                // 2. Prepare the full scrolling string
                let full_text = format!("{}{}{}", title, gap, title);
                let cycle_len = title.chars().count() + gap.chars().count();

                // 3. Calculate the Cycle Duration
                // How long does one full scroll take?
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

    // Widths:
    // 1. Status: Fixed width of 2 to provide the "left padding" visual.
    // 2. Title: Min(10) ensures it takes up all remaining available space.
    // 3. Time: Fixed width of 6 for alignment.
    let widths = [
        Constraint::Length(2),
        Constraint::Min(10),
        Constraint::Length(6),
    ];

    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .title(format!(" {} ", playlist.name))
            .title_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if state.nav.mode == NavMode::Tracks {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            }),
    );

    frame.render_stateful_widget(
        table,
        area,
        &mut TableState::default().with_selected(Some(selected_track_idx)),
    );
}
