use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use melotui_core::AppState;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().dark_gray())
        .style(Style::default().bg(Color::Reset));

    let content = Line::from(vec![
        Span::styled(
            ":",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::raw(state.ui_command_buffer()),
        Span::styled("█", Style::default().fg(Color::White)),
    ]);

    let paragraph = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}
