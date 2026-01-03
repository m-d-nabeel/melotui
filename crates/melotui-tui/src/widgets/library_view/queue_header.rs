use melotui_core::AppState;
use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Cell, Row},
};

pub fn make_row<'a>(state: &'a AppState, is_selected: bool) -> Row<'a> {
    let queue_len = state.playback_queue.tracks.len();

    // Check if audio is currently playing from the "Queue" context
    let is_queue_playing = state
        .playback
        .current
        .as_ref()
        .map(|curr| curr.playlist == "Queue")
        .unwrap_or(false);

    // 1. Define Styling Logic
    //    Queue Row has 3 states: Selected, Playing (but not selected), and Idle.
    let (icon, label_style, bg_style) = if is_selected {
        (
            " ", // Icon when selected
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
            Style::default().bg(Color::Rgb(40, 40, 40)),
        )
    } else if is_queue_playing {
        (
            "▶ ", // Icon when playing (active)
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            Style::default(),
        )
    } else {
        (
            " ", // Icon when idle
            Style::default().fg(Color::LightMagenta),
            Style::default(),
        )
    };

    // 2. Build the "Name" Cell
    let name_content = vec![
        Span::styled(format!(" {} ", icon), label_style),
        Span::styled("PLAY QUEUE", label_style),
    ];
    let name_cell = Cell::from(Line::from(name_content));

    // 3. Build the "Count" Cell (Right Aligned)
    let count_style = if is_selected {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let count_cell = Cell::from(
        Line::from(format!("{} items ", queue_len))
            .alignment(Alignment::Right)
            .style(count_style),
    );

    // 4. Return the Row
    Row::new(vec![name_cell, count_cell])
        .style(bg_style)
        .height(1)
}
