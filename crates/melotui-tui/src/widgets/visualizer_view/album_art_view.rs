use melotui_core::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use ratatui_image::StatefulImage;

use crate::image_cache::ImageState;

const BRANDING_ART: &str = r#"
░▒▓██████████████▓▒░░▒▓████████▓▒░▒▓█▓▒░      ░▒▓██████▓▒░▒▓████████▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░ 
░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░     ░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░   ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░ 
░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░     ░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░   ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░ 
░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓██████▓▒░ ░▒▓█▓▒░     ░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░   ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░ 
░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░     ░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░   ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░ 
░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░     ░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░   ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░ 
░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓████████▓▒░▒▓████████▓▒░▒▓██████▓▒░  ░▒▓█▓▒░    ░▒▓██████▓▒░░▒▓█▓▒░ 
"#;
const DESCRIPTION: &str = "The Terminal Music Player";

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, art_state: &mut ImageState) {
    let _ = state;
    let block = Block::default()
        .title(" Album Art ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if let Some(protocol) = &mut art_state.protocol {
        let cell_ratio: f64 = 0.5;

        let area_cells_w = inner_area.width as f64;
        let area_cells_h = inner_area.height as f64;

        let img_px_w = art_state.width as f64;
        let img_px_h = art_state.height as f64;

        if img_px_w > 0.0 && img_px_h > 0.0 {
            let avail_px_w = area_cells_w * cell_ratio;
            let avail_px_h = area_cells_h;

            let scale = (avail_px_w / img_px_w).min(avail_px_h / img_px_h);

            let target_px_w = img_px_w * scale;
            let target_px_h = img_px_h * scale;

            let target_cells_w = (target_px_w / cell_ratio)
                .round()
                .max(1.0)
                .min(area_cells_w) as u16;

            let target_cells_h = target_px_h.round().max(1.0).min(area_cells_h) as u16;

            use ratatui::layout::{Constraint, Flex, Layout};

            let v = Layout::vertical([
                Constraint::Fill(1),
                Constraint::Length(target_cells_h),
                Constraint::Fill(1),
            ])
            .flex(Flex::Center)
            .split(inner_area)[1];

            let mut centered = Layout::horizontal([
                Constraint::Fill(1),
                Constraint::Length(target_cells_w),
                Constraint::Fill(1),
            ])
            .flex(Flex::Center)
            .split(v)[1];

            // A little nudge to right
            let bias_cells: i16 = 1;
            let max_x = inner_area.x + inner_area.width - centered.width;
            centered.x = (centered.x + bias_cells as u16).min(max_x);

            frame.render_stateful_widget(StatefulImage::default(), centered, protocol);
        } else {
            frame.render_stateful_widget(StatefulImage::default(), inner_area, protocol);
        }
    } else {
        let art_clean = BRANDING_ART.trim_matches('\n');
        let content = format!("{}\n\n{}", art_clean, DESCRIPTION);

        let content_height = (art_clean.lines().count() + 2) as u16;
        let y_offset = inner_area.height.saturating_sub(content_height) / 2;

        let centered_rect = Rect {
            x: inner_area.x,
            y: inner_area.y + y_offset,
            width: inner_area.width,
            height: content_height,
        };

        let p = Paragraph::new(content).alignment(Alignment::Center).style(
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

        frame.render_widget(p, centered_rect);
    }
}
