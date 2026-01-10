use std::time::{Duration, Instant};

use crate::{
    AppMsg, AppState, Library, reconcile_nav_state,
    state::{Notification, NotificationKind, PlaybackContext, RepeatMode},
};

pub fn handle_request_rescan_library(state: &mut AppState) -> Vec<AppMsg> {
    match Library::scan(state.library.root.clone()) {
        Ok(new_library) => {
            state.library = new_library.into();
            reconcile_nav_state(state);
            tracing::info!("Library rescanned");

            state.session.notification.current = Some(Notification {
                kind: NotificationKind::Info,
                message: "Library rescanned".into(),
                progress: None,
                created_at: Instant::now(),
                dismiss_after: Some(Duration::from_secs(2)),
            });
        }
        Err(err) => {
            tracing::error!("Error rescanning library: {:?}", err);
            state.session.notification.current = Some(Notification {
                kind: NotificationKind::Error,
                message: format!("Rescan failed: {err}"),
                progress: None,
                created_at: Instant::now(),
                dismiss_after: Some(Duration::from_secs(2)),
            });
        }
    }
    vec![]
}

pub fn handle_add_to_queue(state: &mut AppState, track: crate::library::Track) -> Vec<AppMsg> {
    state.session.playback_queue.tracks.push_back(track);
    state.session.notification.current = Some(Notification {
        kind: NotificationKind::Info,
        message: "Added to Queue".into(),
        progress: None,
        created_at: Instant::now(),
        dismiss_after: Some(Duration::from_secs(1)),
    });
    vec![]
}

pub fn handle_play_next(state: &mut AppState, track: crate::library::Track) -> Vec<AppMsg> {
    state
        .session
        .playback_queue
        .tracks
        .push_front(track.clone());

    state.session.notification.current = Some(Notification {
        kind: NotificationKind::Info,
        message: format!("Set to play next: {}", track.title),
        progress: None,
        created_at: Instant::now(),
        dismiss_after: Some(Duration::from_secs(2)),
    });
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

    state.session.notification.current = Some(Notification {
        kind: NotificationKind::Info,
        message: format!("Mode: {}", mode_str),
        progress: None,
        created_at: Instant::now(),
        dismiss_after: Some(Duration::from_secs(1)),
    });

    vec![]
}

pub fn handle_toggle_shuffle(state: &mut AppState) -> Vec<AppMsg> {
    state.session.playback.shuffle = !state.session.playback.shuffle;
    let status = if state.session.playback.shuffle {
        "On"
    } else {
        "Off"
    };
    state.session.notification.current = Some(Notification {
        kind: NotificationKind::Info,
        message: format!("Shuffle: {}", status),
        progress: None,
        created_at: Instant::now(),
        dismiss_after: Some(Duration::from_secs(1)),
    });
    vec![]
}

pub fn handle_remove_from_queue(state: &mut AppState) -> Vec<AppMsg> {
    if state.session.nav.active_panel == crate::state::LeftPanelMode::Queue {
        let idx = state.session.nav.queue.selected_idx;

        let playing_from_queue = state
            .session
            .playback
            .current
            .as_ref()
            .is_some_and(|c| c.context == PlaybackContext::Queue);

        if playing_from_queue && idx == 0 {
            return vec![];
        }

        if idx < state.session.playback_queue.tracks.len() {
            state.session.playback_queue.tracks.remove(idx);
            // reconcile_nav_state(state);

            // Adjust selection after removal
            if idx >= state.session.playback_queue.tracks.len() && idx > 0 {
                state.session.nav.queue.selected_idx = idx - 1;
            }

            // FIX: Ensure selection doesn't fall back to 0 if playing
            if playing_from_queue
                && state.session.nav.queue.selected_idx == 0
                && state.session.playback_queue.tracks.len() > 1
            {
                state.session.nav.queue.selected_idx = 1;
            }
            state.session.notification.current = Some(Notification {
                kind: NotificationKind::Info,
                message: "Removed from Queue".into(),
                progress: None,
                created_at: Instant::now(),
                dismiss_after: Some(Duration::from_secs(1)),
            });
        }
    }
    vec![]
}

pub fn handle_clear_queue(state: &mut AppState) -> Vec<AppMsg> {
    let playing_from_queue = state
        .session
        .playback
        .current
        .as_ref()
        .is_some_and(|c| c.context == PlaybackContext::Queue);

    if playing_from_queue {
        state.session.playback_queue.tracks.truncate(1);
    } else {
        state.session.playback_queue.tracks.clear();
    }

    reconcile_nav_state(state);
    state.session.notification.current = Some(Notification {
        kind: NotificationKind::Info,
        message: "Queue Cleared".into(),
        progress: None,
        created_at: Instant::now(),
        dismiss_after: Some(Duration::from_secs(1)),
    });
    vec![]
}

pub fn handle_request_quit(state: &mut AppState) -> Vec<AppMsg> {
    tracing::info!("Application quit requested");
    state.session.notification.current = Some(Notification {
        kind: NotificationKind::Info,
        message: "Quit".into(),
        progress: None,
        created_at: Instant::now(),
        dismiss_after: None,
    });
    vec![]
}
