use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Clear},
};

use melotui_core::{AppState, UIMode};

use crate::widgets::{
    command_view, library_view, notification_popup, playback_view, sound_control_view,
    visualizer_view,
};

pub fn draw(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    // --- MAIN LAYOUT (always the same) ---
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(22), Constraint::Percentage(78)])
        .split(area);

    let right_vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Ratio(2, 3), // Visualizer
            Constraint::Ratio(1, 3), // Controls
        ])
        .split(horizontal[1]);

    let control_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // Sound control
            Constraint::Percentage(50), // Playback
        ])
        .split(right_vertical[1]);

    library_view::draw(frame, horizontal[0], state);
    visualizer_view::draw(frame, right_vertical[0], state);
    sound_control_view::draw(frame, control_chunks[0], state);
    playback_view::draw(frame, control_chunks[1], state);
    notification_popup::draw(frame, frame.area(), state);

    // --- COMMAND OVERLAY ---
    if state.ui.mode == UIMode::Command {
        let backdrop = Block::default().style(Style::default().add_modifier(Modifier::DIM));
        frame.render_widget(backdrop, area);
        let overlay = command_overlay_rect(area);
        frame.render_widget(Clear, overlay);
        command_view::draw(frame, overlay, state);
    }
}

fn command_overlay_rect(area: Rect) -> Rect {
    let height = 3;
    Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(height),
        width: area.width,
        height,
    }
}

pub fn format_duration(ms: u64) -> String {
    let seconds = ms / 1000;
    let minutes = seconds / 60;
    let rem_seconds = seconds % 60;
    format!("{:02}:{:02}", minutes, rem_seconds)
}
