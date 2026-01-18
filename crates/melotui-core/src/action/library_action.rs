use crate::{
    AppMsg, AppState,
    domain::TrackId,
    playback::state::RepeatMode,
    state::{Notification, NotificationKind},
};

/// Trigger a DB / FS rescan
pub fn handle_request_rescan_library(state: &mut AppState) -> Vec<AppMsg> {
    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        "Scanning Library...",
    ));
    vec![]
}

/// Add selected track to the end of the user queue
pub fn handle_add_to_queue(state: &mut AppState, track_id: TrackId) -> Vec<AppMsg> {
    state.session.playback_queue.items.push_back(track_id);

    tracing::warn!("Check the to_string usage here.");
    let title = state
        .track_meta(track_id)
        .map(|t| t.title.to_string())
        .unwrap_or("Track".to_string());

    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("Added to Queue: {}", title),
    ));

    vec![]
}

/// Play selected track next (push front of queue)
pub fn handle_play_next(state: &mut AppState, track_id: TrackId) -> Vec<AppMsg> {
    state.session.playback_queue.items.push_front(track_id);

    tracing::warn!("Check the to_string usage here.");
    let title = state
        .track_meta(track_id)
        .map(|t| t.title.to_string())
        .unwrap_or("Track".to_string());

    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("Will play next: {}", title),
    ));

    vec![]
}

/// Cycle repeat mode
pub fn handle_toggle_repeat_mode(state: &mut AppState) -> Vec<AppMsg> {
    state.session.repeat_mode = state.session.repeat_mode.next();

    let mode_str = match state.session.repeat_mode {
        RepeatMode::Off => "Linear",
        RepeatMode::PlayOnce => "Play Once",
        RepeatMode::Track => "Repeat Track",
        RepeatMode::Playlist => "Repeat Playlist",
    };

    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("Repeat Mode: {}", mode_str),
    ));

    vec![]
}

/// Toggle shuffle for context playback
pub fn handle_toggle_shuffle(state: &mut AppState) -> Vec<AppMsg> {
    let enabled = !state.session.playback.shuffle;
    state.session.playback.shuffle = enabled;

    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("Shuffle: {}", if enabled { "On" } else { "Off" }),
    ));

    vec![]
}

/// Remove currently selected queue item
pub fn handle_remove_from_queue(state: &mut AppState) -> Vec<AppMsg> {
    if !state.is_queue_focused() {
        return vec![];
    }

    let idx = state.selected_queue_index();
    let removed = if idx < state.session.playback_queue.items.len() {
        state.session.playback_queue.items.remove(idx)
    } else {
        None
    };

    if removed.is_some() {
        state.reconcile_nav();
        state.session.notification.current = Some(Notification::new(
            NotificationKind::Info,
            "Removed from Queue",
        ));
    }

    vec![]
}

/// Clear entire user queue
pub fn handle_clear_queue(state: &mut AppState) -> Vec<AppMsg> {
    if state.session.playback_queue.items.is_empty() {
        return vec![];
    }

    state.session.playback_queue.items.clear();
    state.reconcile_nav();

    state.session.notification.current =
        Some(Notification::new(NotificationKind::Info, "Queue Cleared"));

    vec![]
}

/// Request application quit
pub fn handle_request_quit(state: &mut AppState) -> Vec<AppMsg> {
    tracing::info!("Application quit requested");

    state.session.notification.current = Some(Notification::new(NotificationKind::Info, "Quit"));

    vec![]
}
