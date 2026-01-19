use crate::{
    AppMsg, AppState,
    library::projector::flatten_tree,
    navigation::{
        ids::{CategoryId, SidebarNodeId},
        state::LeftPanelMode,
    },
};

pub fn handle_toggle_library_queue(state: &mut AppState) -> Vec<AppMsg> {
    state.session.nav.active_panel = match state.session.nav.active_panel {
        LeftPanelMode::Library => {
            if state.session.nav.queue.selected_queue_id.is_none() {
                state.session.nav.queue.selected_queue_id = state.queue().front().map(|e| e.id);
            }
            LeftPanelMode::Queue
        }
        LeftPanelMode::Queue => LeftPanelMode::Library,
    };
    vec![]
}

pub fn handle_switch_to_playlists(state: &mut AppState) -> Vec<AppMsg> {
    state.last_nav_time = state.time_elapsed;
    state.session.nav.active_panel = LeftPanelMode::Library;
    state.session.nav.library.selected_id = Some(SidebarNodeId::Category(CategoryId::Playlists));
    state
        .session
        .nav
        .library
        .expanded_ids
        .insert(SidebarNodeId::Category(CategoryId::Playlists));
    vec![]
}

pub fn handle_switch_to_tracks(state: &mut AppState) -> Vec<AppMsg> {
    state.last_nav_time = state.time_elapsed;
    match state.session.nav.active_panel {
        LeftPanelMode::Library => {
            if let Some(selected) = &state.session.nav.library.selected_id {
                state
                    .session
                    .nav
                    .library
                    .expanded_ids
                    .insert(selected.clone());
            }
        }
        LeftPanelMode::Queue => {
            state.session.nav.queue.show_info = false;
        }
    }
    vec![]
}

pub fn handle_switch_to_track_info(state: &mut AppState) -> Vec<AppMsg> {
    state.last_nav_time = state.time_elapsed;
    match state.session.nav.active_panel {
        LeftPanelMode::Library => {
            if let Some(track_id) = state.get_selected_track_id_in_sidebar() {
                if let Some(track) = state.library.get_track(track_id).cloned() {
                    state.load_track_info_art(&track);
                }
            }
        }
        LeftPanelMode::Queue => {
            state.session.nav.queue.show_info = true;
            // FIXED: Use queue-specific resolution
            if let Some(track_id) = state.get_selected_track_id_in_queue() {
                if let Some(track) = state.library.get_track(track_id).cloned() {
                    state.load_track_info_art(&track);
                }
            }
        }
    };
    vec![]
}

pub fn handle_move_up(state: &mut AppState) -> Vec<AppMsg> {
    state.last_nav_time = state.time_elapsed;
    match state.session.nav.active_panel {
        LeftPanelMode::Library => {
            let rows = flatten_tree(&state.library, &state.session.nav.library);
            let current_idx = state
                .session
                .nav
                .library
                .selected_id
                .as_ref()
                .and_then(|id| rows.iter().position(|r| &r.id == id));

            if let Some(idx) = current_idx {
                if idx > 0 {
                    state.session.nav.library.selected_id = Some(rows[idx - 1].id.clone());
                }
            } else if !rows.is_empty() {
                state.session.nav.library.selected_id = Some(rows.last().unwrap().id.clone());
            }
        }
        LeftPanelMode::Queue => {
            let queue = state.queue();
            if queue.is_empty() {
                return vec![];
            }

            // Find current index using QueueId
            let current_idx = state
                .session
                .nav
                .queue
                .selected_queue_id
                .and_then(|qid| queue.iter().position(|e| e.id == qid));

            if let Some(idx) = current_idx {
                if idx > 0 {
                    state.session.nav.queue.selected_queue_id = Some(queue[idx - 1].id);
                }
            } else {
                state.session.nav.queue.selected_queue_id = Some(queue[0].id);
            }
        }
    }
    vec![]
}

pub fn handle_move_down(state: &mut AppState) -> Vec<AppMsg> {
    state.last_nav_time = state.time_elapsed;
    match state.session.nav.active_panel {
        LeftPanelMode::Library => {
            let rows = flatten_tree(&state.library, &state.session.nav.library);
            let current_idx = state
                .session
                .nav
                .library
                .selected_id
                .as_ref()
                .and_then(|id| rows.iter().position(|r| &r.id == id));

            match current_idx {
                Some(idx) => {
                    if idx + 1 < rows.len() {
                        state.session.nav.library.selected_id = Some(rows[idx + 1].id.clone());
                    }
                }
                None => {
                    if !rows.is_empty() {
                        state.session.nav.library.selected_id = Some(rows[0].id.clone());
                    }
                }
            }
        }
        LeftPanelMode::Queue => {
            let queue = state.queue();
            if queue.is_empty() {
                return vec![];
            }

            let current_idx = state
                .session
                .nav
                .queue
                .selected_queue_id
                .and_then(|qid| queue.iter().position(|e| e.id == qid));

            match current_idx {
                Some(idx) => {
                    if idx + 1 < queue.len() {
                        state.session.nav.queue.selected_queue_id = Some(queue[idx + 1].id);
                    }
                }
                None => {
                    state.session.nav.queue.selected_queue_id = Some(queue[0].id);
                }
            }
        }
    }
    vec![]
}
