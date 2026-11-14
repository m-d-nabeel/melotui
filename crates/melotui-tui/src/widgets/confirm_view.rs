use crate::keybinding::{KeyAction, KeyContext, KeyRegistry};
use crate::widgets::help_view::key_for;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use melotui_core::AppState;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, registry: &KeyRegistry) {
    let dialog = match state.confirm_dialog() {
        Some(d) => d,
        None => return,
    };

    let t = crate::theme::get_theme(state);

    // Center a bordered dialog box
    let popup_width = (dialog.message.len() as u16 + 16).min(area.width.saturating_sub(4));
    let popup_height = 3;

    let popup_area = centered_rect(popup_width, popup_height, area);

    crate::widgets::clear_for_popup(frame, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.accent_warning))
        .style(Style::default().bg(t.bg_overlay));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // The keys as they are bound, not as they once were.
    let key =
        |action| key_for(registry, KeyContext::Confirm, action).unwrap_or_else(|| "—".to_string());
    let (yes, no) = (key(KeyAction::ConfirmYes), key(KeyAction::ConfirmNo));
    let line = Line::from(vec![
        Span::styled(
            &dialog.message,
            Style::default()
                .fg(t.accent_warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(format!("[{yes}]"), Style::default().fg(t.accent_playing)),
        Span::raw(" yes / "),
        Span::styled(format!("[{no}]"), Style::default().fg(t.accent_error)),
        Span::raw(" no"),
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
