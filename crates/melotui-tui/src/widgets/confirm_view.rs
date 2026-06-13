use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use melotui_core::AppState;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let dialog = match state.confirm_dialog() {
        Some(d) => d,
        None => return,
    };

    let t = crate::theme::get_theme(state);

    // Center a bordered dialog box
    let popup_width = (dialog.message.len() as u16 + 16).min(area.width.saturating_sub(4));
    let popup_height = 3;

    let popup_area = centered_rect(popup_width, popup_height, area);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.accent_warning))
        .style(Style::default().bg(t.bg_overlay));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let line = Line::from(vec![
        Span::styled(
            &dialog.message,
            Style::default()
                .fg(t.accent_warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("[y]", Style::default().fg(t.accent_playing)),
        Span::raw("es / "),
        Span::styled("[n]", Style::default().fg(t.accent_error)),
        Span::raw("o"),
    ]);

    let paragraph = Paragraph::new(line).alignment(Alignment::Center);
    frame.render_widget(paragraph, inner);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(height),
            Constraint::Fill(1),
        ])
        .split(area);

    let horiz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(width),
            Constraint::Fill(1),
        ])
        .split(vert[1]);

    horiz[1]
}
