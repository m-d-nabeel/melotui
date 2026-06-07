use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Clear, Paragraph},
};

use melotui_core::{AppState, UIMode};

use crate::{
    image_cache::ImageCache,
    theme::Theme,
    widgets::{
        command_view, confirm_view, help_view, notification_popup, playback_view,
        playlist_picker_view, sidebar, sound_control_view, visualizer_view,
    },
};

const MIN_WIDTH: u16 = 80;
const MIN_HEIGHT: u16 = 20;

pub fn draw(frame: &mut Frame, state: &AppState, image_cache: &mut ImageCache) {
    let area = frame.area();

    // Minimum terminal size guard
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        draw_too_small(frame, area);
        return;
    }

    // main layout — sidebar uses configurable width
    let sidebar_width = state.sidebar_width();
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(sidebar_width), Constraint::Fill(1)])
        .split(area);

    // Vertical right: visualizer fills remaining, controls get fixed minimum
    let right_vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(14)])
        .split(horizontal[1]);

    // Controls: mixer fills remaining, playback gets priority (essential)
    let control_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(7)])
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

    // playlist picker overlay
    if *state.ui_mode() == UIMode::PlaylistPicker {
        // Dim background
        frame.render_widget(
            Block::default().style(Style::default().add_modifier(Modifier::DIM)),
            area,
        );
        // The picker calculates its own layout relative to the full area
        playlist_picker_view::draw(frame, area, state);
    }

    // confirm dialog bottom bar
    if *state.ui_mode() == UIMode::Confirm {
        frame.render_widget(
            Block::default().style(Style::default().add_modifier(Modifier::DIM)),
            area,
        );
        let overlay = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area)[1];
        confirm_view::draw(frame, overlay, state);
    }

    if state.ui_show_help() {
        frame.render_widget(
            Block::default().style(Style::default().add_modifier(Modifier::DIM)),
            area,
        );
        help_view::draw(frame, area, state);
    }
}

fn draw_too_small(frame: &mut Frame, area: Rect) {
    let theme = Theme::default();
    let msg = Paragraph::new(format!(
        "Terminal too small\nMinimum: {}×{}\nCurrent: {}×{}",
        MIN_WIDTH, MIN_HEIGHT, area.width, area.height
    ))
    .alignment(Alignment::Center)
    .style(Style::default().fg(theme.accent_warning));

    // Center the message
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(4),
            Constraint::Fill(1),
        ])
        .split(area);

    frame.render_widget(msg, popup_layout[1]);
}
