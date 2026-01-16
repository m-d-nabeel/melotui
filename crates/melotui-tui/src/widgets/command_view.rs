use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, BorderType, Borders, Paragraph},
};

use melotui_core::AppState;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title("Command")
        .borders(Borders::TOP)
        .border_type(BorderType::Rounded);

    let text = format!(":{}▌", state.ui_command_buffer());

    let paragraph = Paragraph::new(text).block(block);

    frame.render_widget(paragraph, area);
}
