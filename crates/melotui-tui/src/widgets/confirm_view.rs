use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
};

use crate::theme::Theme;
use melotui_core::AppState;

fn theme() -> &'static Theme {
    static THEME: std::sync::OnceLock<Theme> = std::sync::OnceLock::new();
    THEME.get_or_init(Theme::default)
}

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let dialog = match state.confirm_dialog() {
        Some(d) => d,
        None => return,
    };

    let t = theme();
    frame.render_widget(Clear, area);

    let line = Line::from(vec![
        Span::styled(
            &dialog.message,
            Style::default()
                .fg(t.accent_warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled("[y]", Style::default().fg(t.accent_playing)),
        Span::raw("es / "),
        Span::styled("[n]", Style::default().fg(t.accent_error)),
        Span::raw("o"),
    ]);

    let paragraph = Paragraph::new(line).style(Style::default().bg(t.bg_input));
    frame.render_widget(paragraph, area);
}
