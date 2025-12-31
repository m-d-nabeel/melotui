use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, BorderType, Borders, Paragraph},
};

use melotui_core::AppState;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title("Playback")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let now_playing = state
        .playback
        .current
        .as_ref()
        .and_then(|t| t.path.file_name())
        .map(|n| n.to_string_lossy())
        .unwrap_or_else(|| "—".into());

    let status = match state.playback.status {
        melotui_core::state_registry::PlaybackStatus::Playing => "▶ Playing",
        melotui_core::state_registry::PlaybackStatus::Paused => "⏸ Paused",
        melotui_core::state_registry::PlaybackStatus::Stopped => "⏹ Stopped",
    };

    let text = format!("{}\n{}", status, now_playing);

    let paragraph = Paragraph::new(text).block(block);

    frame.render_widget(paragraph, area);
}
