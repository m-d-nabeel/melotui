use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::{Block, Clear},
};

use melotui_core::{AppState, UIMode};

use crate::{
    image_cache::ImageCache,
    widgets::{
        command_view, help_view, notification_popup, playback_view, sidebar, sound_control_view,
        visualizer_view,
    },
};

pub fn draw(frame: &mut Frame, state: &AppState, image_cache: &mut ImageCache) {
    let area = frame.area();

    // main layout
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

    sidebar::draw(frame, horizontal[0], state, image_cache);
    visualizer_view::draw(frame, right_vertical[0], state, image_cache);
    sound_control_view::draw(frame, control_chunks[0], state);
    playback_view::draw(frame, control_chunks[1], state);

    notification_popup::draw(frame, frame.area(), state);

    // command overlay
    if *state.ui_mode() == UIMode::Command {
        frame.render_widget(
            Block::default().style(Style::default().add_modifier(Modifier::DIM)),
            area,
        );
        let overlay = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(area)[1];
        frame.render_widget(Clear, overlay);
        command_view::draw(frame, overlay, state);
    }

    if state.ui_show_help() {
        frame.render_widget(
            Block::default().style(Style::default().add_modifier(Modifier::DIM)),
            area,
        );
        help_view::draw(frame, area, state);
    }
}
