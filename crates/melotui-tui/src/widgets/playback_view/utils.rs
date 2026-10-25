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

/// What the music comes out of, crossed through and red while muted;
/// nothing when that is not known and the sound is on.
pub fn get_output_icon(
    kind: melotui_core::state::audio::OutputKind,
    muted: bool,
    t: &Theme,
) -> Option<Span<'static>> {
    use melotui_core::state::audio::OutputKind;
    // Material Design icons, as the shuffle and repeat icons are.
    let (on, off) = match kind {
        OutputKind::Speakers => ("\u{f04c3}", "\u{f04c4}"),
        OutputKind::Headphones => ("\u{f02cb}", "\u{f07ce}"),
        OutputKind::Headset => ("\u{f02ce}", "\u{f02d0}"),
        OutputKind::Display => ("\u{f0f5f}", "\u{f0f60}"),
        OutputKind::Unknown if muted => ("", "\u{f0581}"),
        OutputKind::Unknown => return None,
    };
    Some(if muted {
        Span::styled(format!("{off} "), Style::default().fg(t.accent_error))
    } else {
        Span::styled(format!("{on} "), Style::default().fg(t.fg_secondary))
    })
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

#[cfg(test)]
mod tests {
    use melotui_core::state::audio::OutputKind;

    use super::*;

    #[test]
    fn the_output_shows_crossed_and_red_only_while_muted() {
        let t = Theme::default();
        let icon = |kind, muted| get_output_icon(kind, muted, &t);

        let headphones = icon(OutputKind::Headphones, false).unwrap();
        assert_eq!(headphones.content, "\u{f02cb} ");
        assert_eq!(headphones.style.fg, Some(t.fg_secondary));
        let muted = icon(OutputKind::Headphones, true).unwrap();
        assert_eq!(muted.content, "\u{f07ce} ");
        assert_eq!(muted.style.fg, Some(t.accent_error));

        // Not knowing what it plays through, only the muting is worth showing.
        assert!(icon(OutputKind::Unknown, false).is_none());
        assert_eq!(
            icon(OutputKind::Unknown, true).unwrap().content,
            "\u{f0581} "
        );
    }
}
