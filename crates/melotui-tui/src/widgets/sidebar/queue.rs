use crate::format_duration;
use melotui_core::{AppState, domain::TrackId};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, TableState},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let queue = state.queue();
    let is_active_panel = state.is_queue_focused();

    // 1. Resolve Selection Index (ID -> Index)
    let selected_id = state.get_queue_selection();
    let selected_idx = selected_id
        .and_then(|id| queue.iter().position(|&x| x == id))
        .unwrap_or(0);

    // 2. CONTAINER & TITLE
    // Adapted legacy logic: If a playlist is selected in the sidebar, show its name.
    let lib_tab_text = if let Some(pl) = state.get_active_playlist() {
        format!(" {} ", pl.name)
    } else {
        " Library ".to_string()
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

    // 3. SHOW "NOW PLAYING" HEADER (If applicable)
    if state.is_playing_from_queue() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Now Playing Header
                Constraint::Min(0),    // The Upcoming List
            ])
            .split(inner_area);

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

        draw_queue_table(
            frame,
            chunks[1],
            state,
            queue,
            selected_idx,
            is_active_panel,
        );
    } else {
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
    tracks: &std::collections::VecDeque<TrackId>,
    selected_idx: usize,
    is_active: bool,
) {
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
        .map(|(i, track_id)| {
            // Resolve TrackId -> TrackMeta using the Library Snapshot
            let track = state.library.get_track(*track_id);
            let title = track.map(|t| t.title.as_str()).unwrap_or("Unknown Track");
            let duration = track.map(|t| t.duration_ms).unwrap_or(0);

            let is_selected = selected_idx == i;

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

            let title_style = if is_selected {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(" "),
                Cell::from((i + 1).to_string()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(title).style(title_style),
                Cell::from(format_duration(duration)).style(Style::default().fg(Color::DarkGray)),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(6),
        ],
    )
    .header(header);

    let mut table_state = TableState::default();
    table_state.select(Some(selected_idx));

    frame.render_stateful_widget(table, area, &mut table_state);
}
