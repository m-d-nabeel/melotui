use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, BorderType, Borders, Paragraph},
};

use melotui_core::{AppState, UIMode};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let (title, prefix) = match state.ui_mode() {
        UIMode::Filter => ("Filter", "/"),
        _ => ("Command", ":"),
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::TOP)
        .border_type(BorderType::Rounded);

    let text = format!("{}{}▌", prefix, state.ui_command_buffer());

    let paragraph = Paragraph::new(text).block(block);

    frame.render_widget(paragraph, area);
}
