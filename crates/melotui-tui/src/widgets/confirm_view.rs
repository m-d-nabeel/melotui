use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
};

use melotui_core::AppState;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let dialog = match state.confirm_dialog() {
        Some(d) => d,
        None => return,
    };

    frame.render_widget(Clear, area);

    let line = Line::from(vec![
        Span::styled(
            &dialog.message,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled("[y]", Style::default().fg(Color::Green)),
        Span::raw("es / "),
        Span::styled("[n]", Style::default().fg(Color::Red)),
        Span::raw("o"),
    ]);

    let paragraph = Paragraph::new(line).style(Style::default().bg(Color::Rgb(25, 25, 30)));
    frame.render_widget(paragraph, area);
}
