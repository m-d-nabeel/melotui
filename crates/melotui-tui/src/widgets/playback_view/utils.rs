use melotui_core::{
    AppState,
    state_registry::{PlaybackContext, PlaybackStatus, RepeatMode},
};
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

pub fn predict_next_song(state: &AppState) -> String {
    // 1. Queue Priority
    // Check if we are currently playing from the Queue
    let playing_from_queue = state
        .playback
        .current
        .as_ref()
        .is_some_and(|c| matches!(c.context, PlaybackContext::Queue));

    // If playing from Queue, the active song is Index 0. Next is Index 1.
    if playing_from_queue {
        if state.playback_queue.tracks.len() > 1 {
            return state.playback_queue.tracks[1].title.clone();
        } else {
            // Queue ends after this song
            return "None (End of Queue)".to_string();
        }
    }

    // If playing from Library, but Queue has items, Queue is NEXT (Index 0).
    if !state.playback_queue.tracks.is_empty() {
        return state.playback_queue.tracks[0].title.clone();
    }

    // 2. Shuffle Mode (Unpredictable without pre-shuffled list)
    if state.playback.shuffle {
        return "Random (Shuffle)".to_string();
    }

    // 3. Repeat Track
    if state.repeat_mode == RepeatMode::Track
        && let Some(_current) = &state.playback.current
    {
        return "Repeating Track".to_string();
    }

    // 4. Sequential Library Playback
    // We need to look up the current playlist and find the next index
    if let Some(current_id) = &state.playback.current
        && let Some(playlist) = state
            .library
            .playlists
            .iter()
            .find(|p| matches!(current_id.context, PlaybackContext::Library { ref playlist_name } if playlist_name == &p.name))
        && let Some(idx) = playlist
            .tracks
            .iter()
            .position(|t| t.file_path == current_id.path)
    {
        let next_idx = idx + 1;
        if next_idx < playlist.tracks.len() {
            return playlist.tracks[next_idx].title.clone();
        } else if state.repeat_mode == RepeatMode::Playlist
            && let Some(first) = playlist.tracks.first()
        {
            return first.title.clone();
        }
    }

    "None".to_string()
}
