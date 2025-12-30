use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use crate::widgets::{library, playback_state, sound_control, visualizer};
use melotui_core::AppState;

pub fn draw(frame: &mut Frame, state: &AppState) {
    // Horizontal split: left navigation + right (visualizer + control)
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(2, 10), Constraint::Ratio(8, 10)])
        .split(frame.area());

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
            Constraint::Percentage(50), // Sound Control
            Constraint::Percentage(50), // Playback State
        ])
        .split(right_vertical[1]);

    let library_area = horizontal[0];
    let visualizer_area = right_vertical[0];
    let sound_control_area = control_chunks[0];
    let playback_state_area = control_chunks[1];

    library::draw(frame, library_area, state);
    visualizer::draw(frame, visualizer_area, state);
    sound_control::draw(frame, sound_control_area, state);
    playback_state::draw(frame, playback_state_area, state);
}
