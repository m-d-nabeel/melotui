mod image_cache;
mod input;
pub mod theme;
mod tui;
mod view;
mod widgets;

pub use tui::{Tui, UIEvent, restore_terminal};

pub fn format_duration(ms: u64) -> String {
    let seconds = ms / 1000;
    let minutes = seconds / 60;
    let rem_seconds = seconds % 60;
    format!("{:02}:{:02}", minutes, rem_seconds)
}
