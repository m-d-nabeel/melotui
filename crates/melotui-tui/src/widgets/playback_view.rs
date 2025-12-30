use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
};

use melotui_core::AppState;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default().title("Playback").borders(Borders::ALL);

    let now_playing = state
        .playback
        .current
        .as_ref()
        .and_then(|t| t.path.file_name())
        .map(|n| n.to_string_lossy())
        .unwrap_or_else(|| "—".into());

    let text = format!("Now Playing: {}\nStatus: {}", now_playing, state.status);

    let paragraph = Paragraph::new(text).block(block);

    frame.render_widget(paragraph, area);
}
