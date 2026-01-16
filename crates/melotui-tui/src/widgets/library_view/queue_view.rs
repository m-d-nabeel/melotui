use crate::format_duration;
use melotui_core::{
    AppState,
    state::{LeftPanelMode, LibraryViewDepth},
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, TableState},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let queue = &state.session.playback_queue.tracks;
    let selected_idx = state.session.nav.queue.selected_idx;
    let is_active_panel = state.session.nav.active_panel == LeftPanelMode::Queue;

    // --- 1. CONTAINER & TITLE ---
    let lib_tab_text = match state.session.nav.library.depth {
        LibraryViewDepth::Playlists => " Library ".to_string(),
        LibraryViewDepth::Tracks | LibraryViewDepth::TrackInfo => {
            let p_idx = state.session.nav.library.selected_playlist_idx;
            state
                .library
                .playlists
                .get(p_idx)
                .map(|pl| format!(" {} ", pl.name))
                .unwrap_or_else(|| " Library ".to_string())
        }
    };

    let title = Line::from(vec![
        Span::styled(lib_tab_text, Style::default().fg(Color::DarkGray)),
        Span::styled("|", Style::default().fg(Color::DarkGray)),
        Span::styled(
            " Queue ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let border_color = if is_active_panel {
        Color::Magenta
    } else {
        Color::DarkGray
    };
    let container = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    let inner_area = container.inner(area);
    frame.render_widget(container, area);

    if queue.is_empty() {
        let p = Paragraph::new("Queue is empty")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(p, inner_area);
        return;
    }

    // --- 2. HEADER: NOW PLAYING (Fixed Logic) ---
    // Only show header if we are actually playing from the Queue
    if state.is_playing_from_queue() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Now Playing Header
                Constraint::Min(0),    // The List
            ])
            .split(inner_area);

        // Use the ROBUST Selector, not .front()
        if let Some(current_track) = state.get_playing_track() {
            let time_str = format_duration(current_track.duration_ms);
            let lines = vec![
                Line::from(vec![
                    Span::styled(
                        " ▶ ",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::SLOW_BLINK),
                    ),
                    Span::styled(
                        &current_track.title,
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("    "),
                    Span::styled(
                        current_track.artist.as_deref().unwrap_or("Unknown"),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::ITALIC),
                    ),
                    Span::styled(" • ", Style::default().fg(Color::DarkGray)),
                    Span::styled(time_str, Style::default().fg(Color::DarkGray)),
                ]),
            ];
            frame.render_widget(Paragraph::new(lines), chunks[0]);
        }

        // Render the FULL list below
        draw_queue_table(
            frame,
            chunks[1],
            state,
            queue,
            selected_idx,
            is_active_panel,
        );
    } else {
        // Just the list
        draw_queue_table(
            frame,
            inner_area,
            state,
            queue,
            selected_idx,
            is_active_panel,
        );
    }
}

fn draw_queue_table(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    tracks: &std::collections::VecDeque<melotui_core::library::Track>,
    selected_idx: usize,
    is_active: bool,
) {
    let playing_track_id = if state.is_playing_from_queue() {
        state.get_playing_track().map(|track| track.id)
    } else {
        None
    };

    let header = Row::new(vec![
        Cell::from(""),
        Cell::from("#"),
        Cell::from("Title"),
        Cell::from("Time"),
    ])
    .style(
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = tracks
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let is_selected = selected_idx == i;
            let is_playing = playing_track_id == Some(t.id);

            let mut style = Style::default();
            if is_selected {
                style = if is_active {
                    style.bg(Color::Rgb(40, 40, 40)).fg(Color::White)
                } else {
                    style.bg(Color::Rgb(30, 30, 30))
                };
            } else {
                style = style.fg(Color::Gray);
            }

            // Status Column
            let status = if is_playing { "▶" } else { "" };
            let status_style = if is_playing {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            // Title Column
            let title_style = if is_playing {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(status).style(status_style),
                Cell::from((i + 1).to_string()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(t.title.clone()).style(title_style),
                Cell::from(format_duration(t.duration_ms))
                    .style(Style::default().fg(Color::DarkGray)),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Length(4),
            Constraint::Fill(1),
            Constraint::Length(6),
        ],
    )
    .header(header);

    let mut table_state = TableState::default();
    table_state.select(Some(selected_idx));

    frame.render_stateful_widget(table, area, &mut table_state);
}
