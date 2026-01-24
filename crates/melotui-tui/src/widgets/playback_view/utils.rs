use melotui_core::playback::state::{PlaybackStatus, RepeatMode};
use ratatui::style::{Color, Style};
use ratatui::text::Span;

pub fn get_status_icon(status: &PlaybackStatus) -> (&'static str, Color) {
    match status {
        PlaybackStatus::Playing => ("󰐊", Color::Green),
        PlaybackStatus::Paused => ("󰏤", Color::Yellow),
        PlaybackStatus::Stopped => ("󰐌", Color::Red),
    }
}

pub fn get_repeat_icon(mode: &RepeatMode) -> Span<'static> {
    match mode {
        RepeatMode::Off => Span::styled(" ", Style::default().fg(Color::DarkGray)),
        RepeatMode::PlayOnce => Span::styled("󰑗 ", Style::default().fg(Color::Cyan)),
        RepeatMode::Track => Span::styled("󰑘 ", Style::default().fg(Color::Magenta)),
        RepeatMode::Playlist => Span::styled("󰑖 ", Style::default().fg(Color::Green)),
    }
}

pub fn get_shuffle_icon(active: bool) -> Span<'static> {
    if active {
        Span::styled(" ", Style::default().fg(Color::Green))
    } else {
        Span::styled(" ", Style::default().fg(Color::DarkGray))
    }
}
