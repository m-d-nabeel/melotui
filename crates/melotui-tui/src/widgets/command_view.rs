use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::theme::Theme;
use melotui_core::AppState;

fn theme() -> &'static Theme {
    static THEME: std::sync::OnceLock<Theme> = std::sync::OnceLock::new();
    THEME.get_or_init(Theme::default)
}

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let t = theme();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border_inactive))
        .style(Style::default().bg(t.bg_input));

    let content = Line::from(vec![
        Span::styled(
            ":",
            Style::default()
                .fg(t.accent_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::raw(state.ui_command_buffer()),
        Span::styled("█", Style::default().fg(t.fg_primary)),
    ]);

    let paragraph = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}
