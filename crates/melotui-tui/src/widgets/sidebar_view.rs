use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::widgets::{library_view, queue_view};
use melotui_core::{AppState, state_registry::LeftPanelMode};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    // 1. Generate the Tab Switcher Title
    // Looks like: " Library | Queue " with the active one highlighted
    let (lib_style, queue_style) = match state.nav.active_panel {
        LeftPanelMode::Library => (
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            Style::default().fg(Color::DarkGray),
        ),
        LeftPanelMode::Queue => (
            Style::default().fg(Color::DarkGray),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    };

    let title = Line::from(vec![
        Span::styled(" Library ", lib_style),
        Span::styled("|", Style::default().fg(Color::DarkGray)),
        Span::styled(" Queue ", queue_style),
    ]);

    // 2. Render content into the FULL area (No split!)
    match state.nav.active_panel {
        LeftPanelMode::Library => library_view::draw(frame, area, state, title),
        LeftPanelMode::Queue => queue_view::draw(frame, area, state, title),
    }
}
