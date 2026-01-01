use melotui_core::{AppState, state_registry::VisualizerMode};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    // 1. Determine Title based on Mode
    let title = match state.visualizer_mode {
        VisualizerMode::Spectrum => " Visualizer ",
        VisualizerMode::AlbumArt => " Album Art ",
    };

    // 2. Create the container block
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // 3. Render Content based on Mode
    match state.visualizer_mode {
        VisualizerMode::Spectrum => {
            // Placeholder for future FFT/Frequency visualization
            let text = "█ ▄ █ ▄ █ ▄ █ ▄\n(Spectrum Mode)";
            let p = Paragraph::new(text)
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Cyan));
            frame.render_widget(p, inner_area);
        }

        VisualizerMode::AlbumArt => {
            if let Some(art) = &state.current_album_art {
                // Render the pre-calculated ASCII art
                let p = Paragraph::new(art.clone()).alignment(Alignment::Center); // Center the art horizontally
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
    }
}
