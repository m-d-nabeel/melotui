use crate::{
    AppMsg, AppState,
    domain::TrackId,
    message::cover::{CoverContext, CoverMsg},
    message::playback::PlaybackMsg,
    navigation::ids::SidebarNodeId,
    playback::state::{PlaybackStatus, RepeatMode},
    state::{Notification, NotificationKind, VisualizerMode},
};

pub fn handle_request_play_selected(state: &mut AppState) -> Vec<AppMsg> {
    let mut target_track_id: Option<TrackId> = None;
    let mut from_queue = false;

    let mut should_update_context = false;

    let mut new_context_queue: Vec<TrackId> = Vec::new();
    let mut new_context_idx: usize = 0;
    let mut new_source_name = String::new();

    // queue selection (rotate)
    if state.is_queue_focused() {
        if let Some(qid) = state.session.nav.queue.selected_queue_id {
            let queue_items = &mut state.session.playback_queue.items;

            if let Some(idx) = queue_items.iter().position(|e| e.id == qid) {
                // rotate to front
                queue_items.rotate_left(idx);

                // play front
                if let Some(entry) = queue_items.front() {
                    target_track_id = Some(entry.track_id);
                    state.session.playback.active_queue_id = Some(entry.id);
                    from_queue = true;
                }
            }
        }
    }
    // tree selection
    else {
        let selected_node = state.session.nav.library.selected_id.clone();
        if let Some(node) = selected_node {
            match node {
                SidebarNodeId::Track { id, parent } => {
                    target_track_id = Some(id);
                    state.session.playback.active_node = Some(SidebarNodeId::Track {
                        id,
                        parent: parent.clone(),
                    });

                    should_update_context = true;

                    // resolve for all parents
                    new_context_queue = state.library.resolve_track_ids_for_node(&parent);

                    if new_context_queue.is_empty() {
                        // fallback if no context support
                        new_context_queue = vec![id];
                        new_source_name = "Library Selection".to_string();
                    } else {
                        match *parent {
                            SidebarNodeId::Playlist(pid) => {
                                if let Some(playlist) = state.library.get_playlist(pid) {
                                    new_source_name = format!("Playlist: {}", playlist.name);
                                }
                            }
                            SidebarNodeId::Folder(_) => {
                                new_source_name = "Folder View".to_string();
                            }
                            SidebarNodeId::Artist(ref name) => {
                                new_source_name = format!("Artist: {}", name);
                            }
                            SidebarNodeId::Album(ref name) => {
                                new_source_name = format!("Album: {}", name);
                            }
                            SidebarNodeId::Genre(ref name) => {
                                new_source_name = format!("Genre: {}", name);
                            }
                            SidebarNodeId::Year(ref year) => {
                                new_source_name = format!("Year: {}", year);
                            }
                            _ => {
                                new_source_name = "Library Selection".to_string();
                            }
                        }
                    }

                    if let Some(idx) = new_context_queue.iter().position(|&x| x == id) {
                        new_context_idx = idx;
                    }
                }

                _ => {
                    if state.session.nav.library.expanded_ids.contains(&node) {
                        state.session.nav.library.expanded_ids.remove(&node);
                    } else {
                        state.session.nav.library.expanded_ids.insert(node);
                    }
                    return vec![];
                }
            }
        }
    }

    // dispatch playback
    if let Some(id) = target_track_id {
        if should_update_context && !from_queue {
            state.session.playback.context_queue = new_context_queue;
            state.session.playback.context_idx = new_context_idx;
            state.session.playback.source_name = new_source_name;
            state.session.playback.active_queue_id = None;
        }

        start_playback(state, id, from_queue)
    } else {
        vec![]
    }
}

pub fn handle_request_next(state: &mut AppState) -> Vec<AppMsg> {
    if let Some(new_id) = state.get_next_context_id() {
        state.session.playback.context_idx = state
            .session
            .playback
            .context_queue
            .iter()
            .position(|&x| x == new_id)
            .unwrap_or(0);
        update_active_node(state, new_id);
        return start_playback(state, new_id, false);
    }
    vec![]
}

pub fn handle_request_prev(state: &mut AppState) -> Vec<AppMsg> {
    if state.session.playback.progress_ms > 3000 {
        return handle_request_seek(state, -(state.total_duration_ms() as i64));
    }

    if let Some(new_id) = state.get_prev_context_id() {
        state.session.playback.context_idx = state
            .session
            .playback
            .context_queue
            .iter()
            .position(|&x| x == new_id)
            .unwrap_or(0);
        update_active_node(state, new_id);
        return start_playback(state, new_id, false);
    }

    state.session.playback.progress_ms = 0;
    state.session.seek_epoch += 1;
    vec![AppMsg::Playback(PlaybackMsg::DispatchSeek {
        position_ms: 0,
        epoch: state.session.seek_epoch,
    })]
}

pub fn handle_audio_finished(state: &mut AppState) -> Vec<AppMsg> {
    state.session.playback.status = PlaybackStatus::Stopped;
    state.session.playback.progress_ms = 0;

    // repeat track
    if state.session.repeat_mode == RepeatMode::Track
        && let Some(id) = state.session.playback.current
    {
        return start_playback(state, id, state.is_playing_from_queue());
    }

    // clean up finished queue item
    if let Some(finished_qid) = state.session.playback.active_queue_id {
        let queue_items = &mut state.session.playback_queue.items;
        if let Some(idx) = queue_items.iter().position(|e| e.id == finished_qid) {
            queue_items.remove(idx);
        }
        state.session.playback.active_queue_id = None;
    }

    // priority: queue vs context

    // check queue first
    if !state.queue().is_empty()
        && let Some(entry) = state.session.playback_queue.items.pop_front()
    {
        state.session.playback.active_queue_id = Some(entry.id);
        // TODO: clear active_node when playing from queue?
        return start_playback(state, entry.track_id, true);
    }

    // context fallback
    if let Some(new_id) = state.get_next_context_id() {
        state.session.playback.context_idx = state
            .session
            .playback
            .context_queue
            .iter()
            .position(|&x| x == new_id)
            .unwrap_or(0);

        // update active node
        update_active_node(state, new_id);

        return start_playback(state, new_id, false);
    }

    state.session.notification.current =
        Some(Notification::new(NotificationKind::Info, "End of Playlist"));

    vec![]
}

pub fn handle_request_toggle_playback(state: &mut AppState) -> Vec<AppMsg> {
    match state.playback_status() {
        PlaybackStatus::Playing => {
            state.session.playback.status = PlaybackStatus::Paused;
            vec![AppMsg::Playback(PlaybackMsg::DispatchPause)]
        }
        PlaybackStatus::Paused => {
            if state.session.playback.current.is_some() {
                state.session.playback.status = PlaybackStatus::Playing;
                vec![AppMsg::Playback(PlaybackMsg::DispatchResume)]
            } else {
                vec![]
            }
        }
        PlaybackStatus::Stopped => {
            if let Some(id) = state.session.playback.current
                && let Some(meta) = state.library.get_track(id)
            {
                state.session.playback.status = PlaybackStatus::Playing;
                return vec![AppMsg::Playback(PlaybackMsg::DispatchPlay {
                    path: meta.file_path.clone(),
                })];
            }
            vec![]
        }
    }
}

pub fn handle_request_stop(state: &mut AppState) -> Vec<AppMsg> {
    state.session.playback.status = PlaybackStatus::Stopped;
    state.session.playback.progress_ms = 0;
    vec![AppMsg::Playback(PlaybackMsg::DispatchStop)]
}

pub fn handle_request_seek(state: &mut AppState, offset_seconds: i64) -> Vec<AppMsg> {
    if !state.is_playing() && !state.is_paused() {
        return vec![];
    }

    let cur = state.progress_ms() as i64;
    let dur = state.total_duration_ms() as i64;
    let new_ms = (cur + offset_seconds * 1000).clamp(0, dur);

    state.session.seek_epoch += 1;
    state.session.playback.progress_ms = new_ms as u64;
    vec![AppMsg::Playback(PlaybackMsg::DispatchSeek {
        position_ms: new_ms as u64,
        epoch: state.session.seek_epoch,
    })]
}

pub fn handle_audio_progress(state: &mut AppState, position_ms: u64, epoch: u64) -> Vec<AppMsg> {
    if state.is_playing() && epoch >= state.session.seek_epoch {
        state.session.playback.progress_ms = position_ms;
    }
    vec![]
}

pub fn handle_audio_error(state: &mut AppState, error: String) -> Vec<AppMsg> {
    state.session.playback.status = PlaybackStatus::Stopped;
    state.session.notification.current = Some(Notification::new(
        NotificationKind::Error,
        format!("Audio Error: {}", error),
    ));
    vec![]
}

pub fn handle_toggle_art(state: &mut AppState) -> Vec<AppMsg> {
    state.session.visualizer_mode = match state.session.visualizer_mode {
        VisualizerMode::Spectrum => VisualizerMode::AlbumArt,
        VisualizerMode::AlbumArt => VisualizerMode::Spectrum,
    };
    let active = state.should_draw_spectrum();
    let style = state.session.spectrum_style.clone();
    vec![AppMsg::Playback(PlaybackMsg::DispatchSetVisualizer {
        spectrum_style: active.then_some(style),
    })]
}

pub fn handle_dispatch_play(_: &mut AppState, _: std::path::PathBuf) -> Vec<AppMsg> {
    vec![]
}
pub fn handle_dispatch_pause(_: &mut AppState) -> Vec<AppMsg> {
    vec![]
}
pub fn handle_dispatch_resume(_: &mut AppState) -> Vec<AppMsg> {
    vec![]
}
pub fn handle_dispatch_stop(_: &mut AppState) -> Vec<AppMsg> {
    vec![]
}
pub fn handle_dispatch_seek(_: &mut AppState, _: u64, _: u64) -> Vec<AppMsg> {
    vec![]
}
pub fn handle_audio_started(_: &mut AppState, _: String) -> Vec<AppMsg> {
    vec![]
}
pub fn handle_audio_paused(_: &mut AppState) -> Vec<AppMsg> {
    vec![]
}
pub fn handle_audio_resumed(_: &mut AppState) -> Vec<AppMsg> {
    vec![]
}

fn start_playback(state: &mut AppState, id: TrackId, from_queue: bool) -> Vec<AppMsg> {
    let meta = match state.library.get_track(id) {
        Some(m) => m,
        None => return vec![],
    };

    state.session.playback.current = Some(id);
    state.session.playback.playing_from_queue = from_queue;
    state.session.playback.status = PlaybackStatus::Playing;
    state.session.playback.progress_ms = 0;
    state.session.playback.duration_ms = Some(meta.duration_ms);

    let title = meta.title.clone();
    let file_path = meta.file_path.clone();

    state.current_cover = None;

    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("Playing {}", title),
    ));

    let mut effects = vec![AppMsg::Playback(PlaybackMsg::DispatchPlay {
        path: file_path.clone(),
    })];

    // request async cover load
    if meta.has_cover {
        effects.push(AppMsg::Cover(CoverMsg::RequestLoad {
            id,
            path: file_path,
            context: CoverContext::Playback,
        }));
    }

    effects
}

fn update_active_node(state: &mut AppState, new_id: TrackId) {
    let parent_box =
        if let Some(SidebarNodeId::Track { parent, .. }) = &state.session.playback.active_node {
            Some(parent.clone())
        } else {
            None
        };

    if let Some(p) = parent_box {
        state.session.playback.active_node = Some(SidebarNodeId::Track {
            id: new_id,
            parent: p,
        });
    }
}
