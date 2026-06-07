use melotui_core::playback::state::{PlaybackStatus, RepeatMode};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

use crate::theme::Theme;

fn theme() -> &'static Theme {
    static THEME: std::sync::OnceLock<Theme> = std::sync::OnceLock::new();
    THEME.get_or_init(Theme::default)
}

pub fn get_status_icon(status: &PlaybackStatus) -> (&'static str, ratatui::style::Color) {
    let t = theme();
    match status {
        PlaybackStatus::Playing => ("󰐊", t.status_playing),
        PlaybackStatus::Paused => ("󰏤", t.status_paused),
        PlaybackStatus::Stopped => ("󰐌", t.status_stopped),
    }
}

pub fn get_repeat_icon(mode: &RepeatMode) -> Span<'static> {
    let t = theme();
    match mode {
        RepeatMode::Off => Span::styled(
            "󰑗 ",
            Style::default().fg(t.fg_muted).add_modifier(Modifier::DIM),
        ),
        RepeatMode::PlayOnce => Span::styled("󰑗 ", Style::default().fg(t.accent_primary)),
        RepeatMode::Track => Span::styled("󰑘 ", Style::default().fg(t.slider_reverb)),
        RepeatMode::Playlist => Span::styled("󰑖 ", Style::default().fg(t.accent_playing)),
    }
}

pub fn get_shuffle_icon(active: bool) -> Span<'static> {
    let t = theme();
    if active {
        Span::styled("󰒟 ", Style::default().fg(t.accent_playing))
    } else {
        Span::styled(
            "󰒟 ",
            Style::default().fg(t.fg_muted).add_modifier(Modifier::DIM),
        )
    }
}
