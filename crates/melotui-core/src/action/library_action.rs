use crate::{
    AppMsg, AppState,
    state::{Notification, NotificationKind, RepeatMode},
};

pub fn handle_request_rescan_library(state: &mut AppState) -> Vec<AppMsg> {
    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        "Scanning Library...",
    ));

    vec![]
}

pub fn handle_add_to_queue(state: &mut AppState, track: crate::library::Track) -> Vec<AppMsg> {
    state.session.playback_queue.items.push_back(track);
    state.session.notification.current =
        Some(Notification::new(NotificationKind::Info, "Added to Queue"));
    vec![]
}

pub fn handle_play_next(state: &mut AppState, track: crate::library::Track) -> Vec<AppMsg> {
    state.session.playback_queue.items.push_front(track.clone());
    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("Set to play next: {}", track.title),
    ));
    vec![]
}

pub fn handle_toggle_repeat_mode(state: &mut AppState) -> Vec<AppMsg> {
    state.session.repeat_mode = state.session.repeat_mode.next();

    let mode_str = match state.session.repeat_mode {
        RepeatMode::Off => "Linear (Default)",
        RepeatMode::PlayOnce => "Play Once",
        RepeatMode::Track => "Repeat Track",
        RepeatMode::Playlist => "Repeat List",
    };

    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("Mode: {}", mode_str),
    ));

    vec![]
}

pub fn handle_toggle_shuffle(state: &mut AppState) -> Vec<AppMsg> {
    let new_shuffle = !state.shuffle_active();
    state.session.playback.shuffle = new_shuffle;
    let status = if new_shuffle { "On" } else { "Off" };
    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("Shuffle: {}", status),
    ));
    vec![]
}

pub fn handle_remove_from_queue(state: &mut AppState) -> Vec<AppMsg> {
    if state.is_queue_focused() {
        let idx = state.selected_queue_index();
        if idx < state.session.playback_queue.items.len() {
            state.session.playback_queue.items.remove(idx);
            if idx >= state.session.playback_queue.items.len() && idx > 0 {
                state.session.nav.queue.selected_idx = idx - 1;
            }
        }
        state.session.notification.current = Some(Notification::new(
            NotificationKind::Info,
            "Removed from Queue",
        ));
    }
    vec![]
}

pub fn handle_clear_queue(state: &mut AppState) -> Vec<AppMsg> {
    state.session.playback_queue.items.clear();
    state.reconcile_nav();
    state.session.notification.current =
        Some(Notification::new(NotificationKind::Info, "Queue Cleared"));
    vec![]
}

pub fn handle_request_quit(state: &mut AppState) -> Vec<AppMsg> {
    tracing::info!("Application quit requested");
    state.session.notification.current = Some(Notification::new(NotificationKind::Info, "Quit"));
    vec![]
}
