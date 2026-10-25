use melotui_core::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::image_cache::ImageCache;

const BRANDING_ART_WIDE: &str = r#"
░▒▓██████████████▓▒░░▒▓████████▓▒░▒▓█▓▒░      ░▒▓██████▓▒░▒▓████████▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░ 
░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░     ░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░   ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░ 
░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░     ░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░   ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░ 
░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓██████▓▒░ ░▒▓█▓▒░     ░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░   ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░ 
░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░     ░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░   ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░ 
░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░     ░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░   ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░ 
░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓████████▓▒░▒▓████████▓▒░▒▓██████▓▒░  ░▒▓█▓▒░    ░▒▓██████▓▒░░▒▓█▓▒░ 
"#;

// Compact branding for narrower terminals (< 92 cols)
const BRANDING_ART_COMPACT: &str = r#"
 ╔╦╗╔═╗╦  ╔═╗╔╦╗╦ ╦╦
 ║║║║╣ ║  ║ ║ ║ ║ ║║
 ╩ ╩╚═╝╩═╝╚═╝ ╩ ╚═╝╩
"#;

const DESCRIPTION: &str = "The Terminal Music Player";

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, cache: &mut ImageCache) {
    let _ = state;
    let t = crate::theme::get_theme(state);
    let art_state = &mut cache.album_art;

    let block = Block::default()
        .title(" Album Art ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if art_state.source.is_some() {
        art_state.draw(frame, &cache.picker, inner_area);
    } else {
        // Select branding art based on available width
        let art = if inner_area.width >= 92 {
            BRANDING_ART_WIDE
        } else {
            BRANDING_ART_COMPACT
        };

        let art_clean = art.trim_matches('\n');
        let content = format!("{}\n\n{}", art_clean, DESCRIPTION);

        let content_height = (art_clean.lines().count() + 2) as u16;
        let y_offset = inner_area.height.saturating_sub(content_height) / 2;

        let centered_rect = Rect {
            x: inner_area.x,
            y: inner_area.y + y_offset,
            width: inner_area.width,
            height: content_height.min(inner_area.height),
        };

        let p = Paragraph::new(content)
            .alignment(Alignment::Center)
            .style(Style::default().fg(t.fg_muted).add_modifier(Modifier::BOLD));

        frame.render_widget(p, centered_rect);
    }
}
