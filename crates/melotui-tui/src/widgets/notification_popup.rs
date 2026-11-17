use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph, Wrap},
};

use melotui_core::AppState;
use melotui_core::state::NotificationKind;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(notification) = state.current_notification() else {
        return;
    };

    let t = crate::theme::get_theme(state);
    let (width, height) = match notification.kind {
        NotificationKind::Progress => (35, 4),
        _ => size_for(&notification.message, area),
    };
    let popup_area = top_right_rect(width, height, area, 1);

    crate::widgets::clear_for_popup(frame, popup_area);

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
        .title_alignment(Alignment::Left)
        .style(Style::default().bg(t.bg_main).fg(t.text()));

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

/// How big a notification saying `message` is: as wide as it needs, from 35
/// columns up to half the screen, and as tall as its words then wrap to, up
/// to six lines. A long one — what went wrong with the config, say — was
/// cut off at the end of its second line.
fn size_for(message: &str, area: Rect) -> (u16, u16) {
    use unicode_width::UnicodeWidthStr;
    const NARROWEST: usize = 35;
    let widest = (usize::from(area.width) / 2).max(NARROWEST);
    let width = (message.width() + 2).clamp(NARROWEST, widest);
    let lines = wrapped_lines(message, width - 2).clamp(2, 6);
    (width as u16, lines as u16 + 2)
}

/// How many lines `text` wraps to at `width`, a word at a time.
fn wrapped_lines(text: &str, width: usize) -> usize {
    use unicode_width::UnicodeWidthStr;
    let width = width.max(1);
    let mut lines = 1;
    let mut used = 0;
    for word in text.split_whitespace() {
        let w = word.width();
        if used > 0 && used + 1 + w <= width {
            used += 1 + w;
            continue;
        }
        if used > 0 {
            lines += 1;
        }
        // A word wider than a line runs on over the lines it needs.
        let more = w.saturating_sub(1) / width;
        lines += more;
        used = w - more * width;
    }
    lines
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_notification_is_as_it_was_and_a_long_one_grows_to_be_read() {
        let screen = Rect::new(0, 0, 160, 40);
        assert_eq!(size_for("Library loaded", screen), (35, 4));
        let long = "Config Errors (1). Check logs for details: Theme cyberpunk: \
                    `acent_primary` is not a colour a theme has";
        let (width, height) = size_for(long, screen);
        assert!(width > 35 && width <= 80, "{width}");
        // Every word fits in the lines it has.
        assert!(wrapped_lines(long, usize::from(width) - 2) <= usize::from(height) - 2);
        // On a narrow screen it is no wider than it ever was, and taller.
        let (width, height) = size_for(long, Rect::new(0, 0, 60, 30));
        assert_eq!(width, 35);
        assert!(height > 4);
    }

    #[test]
    fn words_wrap_as_a_paragraph_wraps_them() {
        assert_eq!(wrapped_lines("one two three", 13), 1);
        assert_eq!(wrapped_lines("one two three", 12), 2);
        assert_eq!(wrapped_lines("", 10), 1);
        // A word longer than a line takes the lines it needs.
        assert_eq!(wrapped_lines("abcdefghijklmnopqrstuvwxy", 10), 3);
    }
}
