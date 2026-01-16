use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear, Gauge, Paragraph, Wrap},
};

use melotui_core::AppState;
use melotui_core::state::NotificationKind;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(notification) = state.current_notification() else {
        return;
    };

    let width = 35;
    let height = 4;
    let popup_area = top_right_rect(width, height, area, 1);

    frame.render_widget(Clear, popup_area);

    let (base_color, title, icon) = match notification.kind {
        NotificationKind::Progress => (Color::Cyan, "Downloading", "⟳"),
        NotificationKind::Info => (Color::Blue, "Info", "ℹ"),
        NotificationKind::Error => (Color::Red, "Error", "✖"),
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

            // 1. Render the outer block first
            frame.render_widget(block, popup_area);

            // 2. Define the area *inside* the borders
            // We use a margin of 1 because Borders::ALL takes up 1 cell
            let inner_area = popup_area.inner(Margin {
                vertical: 1,
                horizontal: 1,
            });

            // 3. Create a layout to vertically center the gauge
            // This forces the gauge to be exactly 1 cell high
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Fill(1),   // Space above
                    Constraint::Length(1), // THE GAUGE (Height 1)
                    Constraint::Fill(1),   // Space below
                ])
                .split(inner_area);

            // 4. Render the Gauge into the thin middle chunk
            let gauge = Gauge::default()
                .gauge_style(Style::default().fg(base_color).add_modifier(Modifier::BOLD))
                .ratio(pct as f64 / 100.0)
                .label(format!("{}%", pct))
                .use_unicode(true);
            // Note: No .block() here, as we rendered it above

            frame.render_widget(gauge, chunks[1]);
        }

        _ => {
            // For text, we can let the Paragraph handle the block normally
            let p = Paragraph::new(notification.message.clone())
                .block(block)
                .style(Style::default().fg(Color::White))
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
    let y = (area.y)
        .saturating_sub(effective_height)
        .saturating_sub(margin);

    Rect {
        x,
        y,
        width: effective_width,
        height: effective_height,
    }
}
