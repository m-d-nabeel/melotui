use melotui_core::playback::state::{PlaybackStatus, RepeatMode};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

use crate::theme::Theme;

pub fn get_status_icon(status: &PlaybackStatus, t: &Theme) -> (&'static str, Color) {
    match status {
        PlaybackStatus::Playing => ("󰐊", t.status_playing),
        PlaybackStatus::Paused => ("󰏤", t.status_paused),
        PlaybackStatus::Stopped => ("󰐌", t.status_stopped),
    }
}

pub fn get_repeat_icon(mode: &RepeatMode, t: &Theme) -> Span<'static> {
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

pub fn get_shuffle_icon(active: bool, t: &Theme) -> Span<'static> {
    if active {
        Span::styled("󰒟 ", Style::default().fg(t.accent_playing))
    } else {
        Span::styled(
            "󰒟 ",
            Style::default().fg(t.fg_muted).add_modifier(Modifier::DIM),
        )
    }
}
