use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use melotui_core::AppState;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let t = crate::theme::get_theme(state);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border_inactive))
        .style(Style::default().bg(t.bg_input));

    let buffer = state.ui_command_buffer();
    let value = buffer.value();
    let cursor = buffer.cursor();

    let mut content_spans = vec![
        Span::styled(
            ":",
            Style::default()
                .fg(t.accent_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ];

    if cursor == value.len() {
        content_spans.push(Span::raw(value));
        content_spans.push(Span::styled("█", Style::default().fg(t.fg_primary)));
    } else {
        // Find the byte length of the character at the cursor
        let char_len = value[cursor..].chars().next().unwrap().len_utf8();
        let before = &value[..cursor];
        let at_cursor = &value[cursor..cursor + char_len];
        let after = &value[cursor + char_len..];

        content_spans.push(Span::raw(before));
        content_spans.push(Span::styled(
            at_cursor,
            Style::default().fg(t.bg_input).bg(t.fg_primary), // Inverted colors for cursor
        ));
        content_spans.push(Span::raw(after));
    }

    let content = Line::from(content_spans);

    let paragraph = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}
