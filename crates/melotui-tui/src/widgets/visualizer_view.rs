use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, BorderType, Borders, Paragraph},
};

use melotui_core::AppState;

pub fn draw(frame: &mut Frame, area: Rect, _state: &AppState) {
    let block = Block::default()
        .title("Visualizer")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let content = Paragraph::new("Spectrum").block(block);

    frame.render_widget(content, area);
}
