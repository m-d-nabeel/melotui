use crate::{
    AppMsg, AppState,
    domain::TrackId,
    message::database::DatabaseMsg,
    playback::state::RepeatMode,
    state::{Notification, NotificationKind},
};

pub fn handle_request_scan_library(state: &mut AppState) -> Vec<AppMsg> {
    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        "Scanning Library...",
    ));
    vec![]
}

pub fn handle_add_to_queue(state: &mut AppState, track_id: TrackId) -> Vec<AppMsg> {
    state.session.playback_queue.add_track(track_id);

    let title = state
        .library
        .get_track(track_id)
        .map(|t| t.title.clone())
        .unwrap_or("Track".into());
    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("Added to Queue: {}", title),
    ));
    vec![]
}

pub fn handle_play_next(state: &mut AppState, track_id: TrackId) -> Vec<AppMsg> {
    state.session.playback_queue.add_track_next(track_id);

    let title = state
        .library
        .get_track(track_id)
        .map(|t| t.title.clone())
        .unwrap_or("Track".into());
    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("Will play next: {}", title),
    ));
    vec![]
}

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

pub fn handle_toggle_shuffle(state: &mut AppState) -> Vec<AppMsg> {
    let enabled = !state.session.playback.shuffle;
    state.session.playback.shuffle = enabled;
    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("Shuffle: {}", if enabled { "On" } else { "Off" }),
    ));
    vec![]
}

pub fn handle_remove_from_queue(state: &mut AppState) -> Vec<AppMsg> {
    if !state.is_queue_focused() {
        return vec![];
    }

    let selected_qid = state.session.nav.queue.selected_queue_id;

    if let Some(target_id) = selected_qid {
        let queue = &mut state.session.playback_queue.items;

        if let Some(idx) = queue.iter().position(|e| e.id == target_id) {
            queue.remove(idx);

            if !queue.is_empty() {
                let new_idx = if idx >= queue.len() {
                    queue.len() - 1
                } else {
                    idx
                };
                state.session.nav.queue.selected_queue_id = Some(queue[new_idx].id);
            } else {
                state.session.nav.queue.selected_queue_id = None;
            }

            state.session.notification.current = Some(Notification::new(
                NotificationKind::Info,
                "Removed from Queue",
            ));
        }
    }
    vec![]
}

pub fn handle_clear_queue(state: &mut AppState) -> Vec<AppMsg> {
    if state.queue().is_empty() {
        return vec![];
    }
    state.session.playback_queue.items.clear();
    state.session.nav.queue.selected_queue_id = None;
    state.session.notification.current =
        Some(Notification::new(NotificationKind::Info, "Queue Cleared"));
    vec![]
}

pub fn handle_request_quit(state: &mut AppState) -> Vec<AppMsg> {
    tracing::info!("Application quit requested");
    state.session.notification.current = Some(Notification::new(NotificationKind::Info, "Quit"));
    vec![]
}

pub fn handle_request_toggle_favorite(state: &mut AppState, id: TrackId) -> Vec<AppMsg> {
    let current_status = state
        .library
        .get_track(id)
        .map(|t| t.is_favorite)
        .unwrap_or(false);
    let new_status = !current_status;

    std::sync::Arc::make_mut(&mut state.library).toggle_favorite(id, new_status);

    let title = state
        .library
        .get_track(id)
        .map(|t| t.title.clone())
        .unwrap_or_else(|| "Track".to_string());

    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!(
            "{} {}",
            if new_status {
                "Added to favorites:"
            } else {
                "Removed from favorites:"
            },
            title
        ),
    ));

    // Dispatch to database worker
    vec![AppMsg::Database(DatabaseMsg::DispatchToggleFavorite(id))]
}
