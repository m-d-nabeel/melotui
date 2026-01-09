use crate::format_duration;
use melotui_core::{
    AppState,
    state_registry::{LeftPanelMode, LibraryViewDepth, PlaybackContext},
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, TableState},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let queue = &state.playback_queue.tracks;
    let selected_idx = state.nav.queue.selected_idx;
    let is_active_panel = state.nav.active_panel == LeftPanelMode::Queue;

    // 1. Prepare Main Container Block
    // This creates the single "unified" look the user requested.
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

    // 2. Empty Queue Handling
    if queue.is_empty() {
        let p = Paragraph::new("Queue is empty")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));

        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(1),
                Constraint::Fill(1),
            ])
            .split(inner_area);
        frame.render_widget(p, v_chunks[1]);
        return;
    }

    // 3. Context Check
    let playing_from_queue = state
        .playback
        .current
        .as_ref()
        .is_some_and(|curr| matches!(curr.context, PlaybackContext::Queue));

    if playing_from_queue {
        // --- SPLIT LAYOUT ---
        // Top: Now Playing (Index 0)
        // Mid: Separator
        // Bot: Up Next List (Indices 1..)

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Now Playing Card
                Constraint::Length(2), // Separator (Padding + Text)
                Constraint::Min(0),    // The Table
            ])
            .split(inner_area);

        // A. Now Playing Section
        if let Some(current_track) = queue.front() {
            // Visuals: Green Title, Dimmed Meta. NO Selection highlight logic here.
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

            // Render slightly indented
            let np_area = Rect {
                x: chunks[0].x + 1,
                width: chunks[0].width.saturating_sub(2),
                ..chunks[0]
            };
            frame.render_widget(Paragraph::new(lines), np_area);
        }

        // B. Separator
        let sep_line = Line::from(vec![
            Span::styled(
                " 󰲸 Up Next ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "───────────────────────────────────────────────────",
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        frame.render_widget(Paragraph::new(sep_line), chunks[1]);

        // C. Up Next Table
        if queue.len() > 1 {
            let up_next_slice = queue.iter().skip(1);

            // Selection Logic Fix:
            // Since we skipped 1 item, table row 0 corresponds to Queue Index 1.
            // If selected_idx is 0 (Now Playing), we show NOTHING selected in the table.
            // If selected_idx is 1 (First Up Next), we select table row 0.
            let table_selected_idx = if selected_idx > 0 {
                Some(selected_idx - 1)
            } else {
                None
            };

            draw_simple_table(
                frame,
                chunks[2],
                up_next_slice,
                table_selected_idx,
                is_active_panel,
                1, // Visual offset for # column (Start at 2)
            );
        } else {
            frame.render_widget(
                Paragraph::new("   (End of Queue)").style(Style::default().fg(Color::DarkGray)),
                chunks[2],
            );
        }
    } else {
        // --- PENDING MODE (Not Playing) ---
        // Render whole list normally
        draw_simple_table(
            frame,
            inner_area,
            queue.iter(),
            Some(selected_idx),
            is_active_panel,
            0,
        );
    }
}

fn draw_simple_table<'a>(
    frame: &mut Frame,
    area: Rect,
    tracks: impl Iterator<Item = &'a melotui_core::library::Track>,
    selected_idx: Option<usize>,
    is_active: bool,
    index_offset: usize,
) {
    // Columns: #, Title, Time
    let header = Row::new(vec![
        Cell::from("#"),
        Cell::from("Title"),
        Cell::from("Time"),
    ])
    .style(
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
    .bottom_margin(0);

    let rows: Vec<Row> = tracks
        .enumerate()
        .map(|(i, t)| {
            let is_selected = selected_idx == Some(i);

            let mut style = Style::default();
            if is_selected {
                if is_active {
                    style = style.bg(Color::Rgb(40, 40, 40)).fg(Color::White);
                } else {
                    style = style.bg(Color::Rgb(30, 30, 30));
                }
            } else {
                style = style.fg(Color::Gray);
            }

            let display_num = (i + 1 + index_offset).to_string();

            Row::new(vec![
                Cell::from(display_num).style(Style::default().fg(Color::DarkGray)),
                Cell::from(t.title.clone()).style(if is_selected {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                }),
                // Time in Dark Gray as requested
                Cell::from(format_duration(t.duration_ms))
                    .style(Style::default().fg(Color::DarkGray)),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(4), // #
            Constraint::Fill(1),   // Title
            Constraint::Length(6), // Time
        ],
    )
    .header(header);

    let mut state = TableState::default();
    state.select(selected_idx);

    frame.render_stateful_widget(table, area, &mut state);
}
