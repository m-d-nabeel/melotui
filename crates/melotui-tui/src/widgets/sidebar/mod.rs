pub mod queue;
pub mod track_info;
pub mod tree;

use melotui_core::AppState;
use ratatui::{Frame, layout::Rect};

use crate::image_cache::ImageCache;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, cache: &mut ImageCache) {
    if state.is_showing_track_info() {
        track_info::draw(frame, area, state, cache);
    } else if state.is_queue_focused() {
        queue::draw(frame, area, state);
    } else {
        tree::draw(frame, area, state);
    }
}

// marquee helper
pub fn generate_marquee_delayed(text: &str, max_width: usize, state: &AppState) -> String {
    use unicode_width::UnicodeWidthStr;

    let text_width = UnicodeWidthStr::width(text);
    if text_width <= max_width {
        return text.to_string();
    }
    let delay = 3.0;
    let time_since_nav = state.time_elapsed() - state.last_nav_time;
    if time_since_nav < delay {
        return text.chars().take(max_width).collect();
    }
    let speed = 2.0;
    let active_scroll_time = time_since_nav - delay;
    let gap = 6;
    let full_len = text.chars().count() + gap;
    let offset = (active_scroll_time * speed) as usize % full_len;
    let display_text = format!("{}{}{}", text, " ".repeat(gap), text);
    display_text.chars().skip(offset).take(max_width).collect()
}
