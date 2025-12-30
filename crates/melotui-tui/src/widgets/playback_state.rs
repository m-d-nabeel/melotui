use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
};

use melotui_core::AppState;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default().title("Playback").borders(Borders::ALL);

    let content = Paragraph::new(format!(
        "Now Playing: {}\nStatus: {}",
        "—", // placeholder
        state.status
    ))
    .block(block);

    frame.render_widget(content, area);
}
