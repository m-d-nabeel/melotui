use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear, Gauge, Paragraph, Wrap},
};

use melotui_core::AppState;
use melotui_core::state::NotificationKind;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(notification) = state.current_notification() else {
        return;
    };

    let t = crate::theme::get_theme(state);
    let width = 35;
    let height = 4;
    let popup_area = top_right_rect(width, height, area, 1);

    frame.render_widget(Clear, popup_area);

    let (base_color, title, icon) = match notification.kind {
        NotificationKind::Progress => (t.accent_info, "Downloading", "⟳"),
        NotificationKind::Info => (t.accent_info, "Info", "ℹ"),
        NotificationKind::Warning => (t.accent_warning, "Warning", "⚠"),
        NotificationKind::Error => (t.accent_error, "Error", "✖"),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(base_color))
        .title(format!(" {} {} ", icon, title))
        .title_alignment(Alignment::Left);

    match notification.kind {
        NotificationKind::Progress => {
            let pct = notification.progress.unwrap_or(0);

            // outer block
            frame.render_widget(block, popup_area);

            // area inside borders
            // margin 1
            let inner_area = popup_area.inner(Margin {
                vertical: 1,
                horizontal: 1,
            });

            // center gauge layout
            // gauge height 1
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Fill(1),   // space above
                    Constraint::Length(1), // gauge row
                    Constraint::Fill(1),   // space below
                ])
                .split(inner_area);

            // render gauge into chunk
            let gauge = Gauge::default()
                .gauge_style(Style::default().fg(base_color).add_modifier(Modifier::BOLD))
                .ratio(pct as f64 / 100.0)
                .label(format!("{}%", pct))
                .use_unicode(true);
            // no .block() - already rendered above

            frame.render_widget(gauge, chunks[1]);
        }

        _ => {
            // paragraph handles block
            let p = Paragraph::new(notification.message.clone())
                .block(block)
                .style(Style::default().fg(t.fg_primary))
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: true });

            frame.render_widget(p, popup_area);
        }
    }
}

fn top_right_rect(w: u16, h: u16, area: Rect, margin: u16) -> Rect {
    let effective_width = w.min(area.width);
    let effective_height = h.min(area.height);

    let x = (area.x + area.width)
        .saturating_sub(effective_width)
        .saturating_sub(margin);
    // Fixed: position popup at top with margin, not at y=0
    let y = area.y + margin;

    Rect {
        x,
        y,
        width: effective_width,
        height: effective_height,
    }
}
