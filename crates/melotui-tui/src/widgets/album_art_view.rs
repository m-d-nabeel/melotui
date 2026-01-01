use melotui_core::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(" Album Art ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if let Some(art) = &state.current_album_art {
        // Render the pre-calculated ASCII art
        let p = Paragraph::new(art.clone()).alignment(Alignment::Center);
        frame.render_widget(p, inner_area);
    } else {
        // Fallback if no art is found or loaded
        let text = "\n\nNo Cover Art";
        let p = Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(p, inner_area);
    }
}
