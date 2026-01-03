use crate::format_duration;
use melotui_core::{AppState, NavMode};
use ratatui::layout::{Alignment, Direction, Layout};
use ratatui::{
    Frame,
    layout::{Constraint, Rect}, // Removed Alignment, Direction, Layout
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    match state.nav.mode {
        NavMode::Playlists => draw_playlists(frame, area, state),
        NavMode::Tracks => draw_tracks(frame, area, state),
        NavMode::TrackInfo => draw_track_info(frame, area, state),
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

fn draw_track_info(frame: &mut Frame, area: Rect, state: &AppState) {
    let p_idx = state.nav.selected_playlist;
    if p_idx >= state.library.playlists.len() {
        return;
    }
    let playlist = &state.library.playlists[p_idx];

    let t_idx = state.nav.selected_tracks[p_idx];
    if t_idx >= playlist.tracks.len() {
        return;
    }

    let track = &playlist.tracks[t_idx];

    // 1. Container Block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Track Info ");

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // 2. Layout: Split into Top (Art) and Bottom (Info)
    let art_height = if state.track_info_art.is_some() {
        15
    } else {
        0
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(art_height), // Dynamic Art Section
            Constraint::Min(10),            // Metadata Section
        ])
        .split(inner_area);

    // 3. Render Album Art
    if let Some(art) = &state.track_info_art {
        let art_widget = Paragraph::new(art.clone())
            .alignment(Alignment::Center)
            .block(Block::default());
        frame.render_widget(art_widget, chunks[0]);
    }

    // 4. Build Visual Content
    let mut lines = vec![
        // --- HEADER: Title & Artist ---
        Line::from(""), // Top padding
        Line::from(Span::styled(
            &track.title,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            track.artist.as_deref().unwrap_or("Unknown Artist"),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::ITALIC),
        )),
        // Separator
        Line::from(Span::styled("──────", Style::default().fg(Color::DarkGray))),
        Line::from(""),
    ];

    // --- METADATA (Icon based) ---
    // Album
    if let Some(album) = &track.album {
        lines.push(Line::from(vec![
            Span::styled("💿 ", Style::default().fg(Color::Cyan)),
            Span::styled(album, Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from("")); // Spacer
    }

    // Genre
    if let Some(genre) = &track.genre {
        lines.push(Line::from(vec![
            Span::styled("🎹 ", Style::default().fg(Color::Cyan)),
            Span::styled(genre, Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(""));
    }

    // Year
    if let Some(year) = track.year {
        lines.push(Line::from(vec![
            Span::styled("📅 ", Style::default().fg(Color::Cyan)),
            Span::styled(year.to_string(), Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(""));
    }

    // --- TECH SPECS ---
    // Separator
    lines.push(Line::from(Span::styled(
        "──────",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));

    // Audio Format & Bitrate
    let bitrate = track.bitrate.unwrap_or(0);
    let format_str = format!("{} • {} kbps", track.format, bitrate);
    lines.push(Line::from(vec![
        Span::styled("🔊 ", Style::default().fg(Color::Green)),
        Span::styled(format_str, Style::default().fg(Color::Gray)),
    ]));

    // Sample Rate
    if let Some(sample) = track.sample_rate {
        lines.push(Line::from(vec![
            Span::styled("📉 ", Style::default().fg(Color::Green)),
            Span::styled(format!("{} Hz", sample), Style::default().fg(Color::Gray)),
        ]));
    }

    // Duration
    lines.push(Line::from(vec![
        Span::styled("⏱  ", Style::default().fg(Color::Green)),
        Span::styled(
            format_duration(track.duration_ms),
            Style::default().fg(Color::Gray),
        ),
    ]));

    // --- FOOTER: File Name (Dimmed) ---
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "File:",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        track
            .file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
        Style::default().fg(Color::DarkGray),
    )));

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, chunks[1]);
}
