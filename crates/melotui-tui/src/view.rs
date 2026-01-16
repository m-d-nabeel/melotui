use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Clear},
};

use melotui_core::{AppState, UIMode};

use crate::widgets::{
    command_view, help_view, notification_popup, playback_view, sidebar_view, sound_control_view,
    visualizer_view,
};

pub fn draw(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    // --- MAIN LAYOUT (background) ---
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(22), Constraint::Percentage(78)])
        .split(area);

    let right_vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)])
        .split(horizontal[1]);

    let control_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(right_vertical[1]);

    sidebar_view::draw(frame, horizontal[0], state);
    visualizer_view::draw(frame, right_vertical[0], state);
    sound_control_view::draw(frame, control_chunks[0], state);
    playback_view::draw(frame, control_chunks[1], state);

    notification_popup::draw(frame, frame.area(), state);

    // --- COMMAND OVERLAY ---
    if *state.ui_mode() == UIMode::Command {
        // Dim background
        frame.render_widget(
            Block::default().style(Style::default().add_modifier(Modifier::DIM)),
            area,
        );
        // ... draw command UI ...
        let overlay = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(area)[1];
        frame.render_widget(Clear, overlay);
        command_view::draw(frame, overlay, state);
    }

    // --- HELP OVERLAY (Topmost Modal) ---
    if state.ui_show_help() {
        // 1. Dim the whole background first for focus
        let dim_block = Block::default().style(
            Style::default()
                .bg(Color::Reset)
                .add_modifier(Modifier::DIM),
        );
        frame.render_widget(dim_block, area);

        // 2. Draw the help popup
        help_view::draw(frame, area, state);
    }
}
