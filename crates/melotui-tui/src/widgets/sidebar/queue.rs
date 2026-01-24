use crate::{format_duration, widgets::sidebar::generate_marquee_delayed};
use melotui_core::AppState;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, TableState},
};

const ACTIVE_BORDER: Color = Color::Cyan;
const INACTIVE_BORDER: Color = Color::Rgb(60, 60, 60);
const SELECTED_BG: Color = Color::Rgb(40, 44, 52);
const PLAYING_TRACK_COLOR: Color = Color::Rgb(166, 227, 161); // Catppuccin Green (#a6e3a1)

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let queue = state.queue();
    let is_active = state.is_queue_focused();

    let border_style = if is_active {
        Style::default().fg(ACTIVE_BORDER)
    } else {
        Style::default().fg(INACTIVE_BORDER)
    };

    let title_text = if is_active {
        Span::styled(
            " Queue ",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(ACTIVE_BORDER),
        )
    } else {
        Span::raw(" Queue ")
    };

    let block = Block::default()
        .title(title_text)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style);

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(inner_area);

    // Draw "Now Playing" header
    let playing_from_queue = state.is_playing_from_queue();
    let current_track = state.get_playing_track();

    if playing_from_queue && current_track.is_some() {
        if let Some(track) = current_track {
            let title_marquee = generate_marquee_delayed(
                &track.title,
                (chunks[0].width as usize).saturating_sub(10),
                state,
            );

            let header_lines = vec![
                Line::from(vec![
                    Span::styled(
                        " ▶ ",
                        Style::default()
                            .fg(PLAYING_TRACK_COLOR)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        title_marquee,
                        Style::default()
                            .fg(PLAYING_TRACK_COLOR)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("    "),
                    Span::styled(
                        track.artist.as_deref().unwrap_or("Unknown"),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::ITALIC),
                    ),
                    Span::styled(" • ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format_duration(track.duration_ms),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
            ];

            frame.render_widget(Paragraph::new(header_lines), chunks[0]);
        }
    } else {
        let no_playing_lines = vec![Line::from(vec![
            Span::styled(
                " ▶ ",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "No song playing",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ),
        ])];
        frame.render_widget(Paragraph::new(no_playing_lines), chunks[0]);
    }

    if queue.is_empty() {
        let empty_para = Paragraph::new("Empty")
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(empty_para, chunks[1]);
    } else {
        let selected_id = state.get_queue_selection_id();
        let selected_idx = selected_id
            .and_then(|id| queue.iter().position(|e| e.id == id))
            .unwrap_or(0);

        draw_queue_table(frame, chunks[1], state, queue, selected_idx, is_active);
    }
}

fn draw_queue_table(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    tracks: &std::collections::VecDeque<melotui_core::playback::state::QueueEntry>,
    selected_idx: usize,
    is_active: bool,
) {
    let playing_id = state.get_playing_track_id();
    let is_playing_queue = state.is_playing_from_queue();

    let rows: Vec<Row> = tracks
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let track = state.library.get_track(entry.track_id);
            let title = track.map(|t| t.title.as_str()).unwrap_or("Unknown");
            let duration = track.map(|t| t.duration_ms).unwrap_or(0);

            let is_selected = selected_idx == i;
            let is_current =
                is_playing_queue && playing_id.map(|p| p == entry.track_id).unwrap_or(false);

            let mut row_style = Style::default();
            if is_selected {
                row_style = row_style.bg(SELECTED_BG);
            }

            let text_color = if is_current {
                PLAYING_TRACK_COLOR
            } else if is_selected {
                Color::White
            } else {
                Color::Gray
            };

            let display_title = if is_selected && is_active {
                let approx_width = area.width.saturating_sub(14) as usize;
                generate_marquee_delayed(title, approx_width, state)
            } else {
                title.to_string()
            };

            let icon_symbol = if is_current { "▶" } else { "" };
            let icon_style = if is_current {
                Style::default().fg(PLAYING_TRACK_COLOR)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            Row::new(vec![
                Cell::from(format!("{:>3}", i + 1)).style(Style::default().fg(Color::DarkGray)),
                Cell::from(icon_symbol).style(icon_style),
                Cell::from(display_title).style(Style::default().fg(text_color).add_modifier(
                    if is_current {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    },
                )),
                Cell::from(format_duration(duration)).style(Style::default().fg(Color::DarkGray)),
            ])
            .style(row_style)
        })
        .collect();

    let header = Row::new(vec!["  #", " ", "Title", "Time"]).style(
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::UNDERLINED),
    );

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Length(2),
            Constraint::Fill(1),
            Constraint::Length(6),
        ],
    )
    .header(header)
    .row_highlight_style(Style::default());

    let mut table_state = TableState::default();
    table_state.select(Some(selected_idx));

    frame.render_stateful_widget(table, area, &mut table_state);
}
