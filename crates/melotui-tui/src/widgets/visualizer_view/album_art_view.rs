use melotui_core::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use ratatui_image::StatefulImage;

use crate::{image_cache::ImageCache, theme::Theme};

fn theme() -> &'static Theme {
    static THEME: std::sync::OnceLock<Theme> = std::sync::OnceLock::new();
    THEME.get_or_init(Theme::default)
}

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

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, cache: &mut ImageCache) {
    let _ = state;
    let t = theme();
    let art_state = &mut cache.album_art;

    let block = Block::default()
        .title(" Album Art ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // shared center cover crop logic
    if art_state.source.is_some() {
        art_state.update_protocol(&cache.picker, inner_area);

        if let Some(protocol) = &mut art_state.protocol {
            let image = StatefulImage::default().resize(ratatui_image::Resize::Fit(None));
            frame.render_stateful_widget(image, inner_area, protocol);
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
                .fg(t.fg_muted)
                .add_modifier(Modifier::BOLD),
        );

        frame.render_widget(p, centered_rect);
    }
}
