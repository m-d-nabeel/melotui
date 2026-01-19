use crate::{
    AppMsg, AppState,
    library::projector::flatten_tree,
    navigation::{
        ids::{CategoryId, SidebarNodeId},
        state::LeftPanelMode,
    },
};

pub fn handle_toggle_library_queue(state: &mut AppState) -> Vec<AppMsg> {
    tracing::info!("Toggle Library Queue");

    state.session.nav.active_panel = match state.session.nav.active_panel {
        LeftPanelMode::Library => {
            // Select first item in queue if nothing selected
            if state.session.nav.queue.selected_track_id.is_none() {
                state.session.nav.queue.selected_track_id = state.queue().front().copied();
            }
            LeftPanelMode::Queue
        }
        LeftPanelMode::Queue => LeftPanelMode::Library,
    };
    vec![]
}

pub fn handle_switch_to_playlists(state: &mut AppState) -> Vec<AppMsg> {
    tracing::debug!("Switch to Playlist Mode");
    state.last_nav_time = state.time_elapsed;
    state.session.nav.active_panel = LeftPanelMode::Library;

    // Select the "Playlists" category root
    state.session.nav.library.selected_id = Some(SidebarNodeId::Category(CategoryId::Playlists));

    // Optionally expand it
    state
        .session
        .nav
        .library
        .expanded_ids
        .insert(SidebarNodeId::Category(CategoryId::Playlists));

    vec![]
}

pub fn handle_switch_to_tracks(state: &mut AppState) -> Vec<AppMsg> {
    tracing::info!("Expand Selection");
    state.last_nav_time = state.time_elapsed;

    match state.session.nav.active_panel {
        LeftPanelMode::Library => {
            // If we are on a node that can be expanded (like a Playlist), expand it.
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
    tracing::info!("Load Track Info");
    state.last_nav_time = state.time_elapsed;

    match state.session.nav.active_panel {
        LeftPanelMode::Library => {
            if let Some(track_id) = state.get_selected_track_id_in_sidebar() {
                let track = state.library.get_track(track_id).cloned();

                if let Some(t) = track {
                    state.load_track_info_art(&t);
                }
            }
        }
        LeftPanelMode::Queue => {
            state.session.nav.queue.show_info = true;
            if let Some(id) = state.session.nav.queue.selected_track_id {
                let track = state.library.get_track(id).cloned();

                if let Some(t) = track {
                    state.load_track_info_art(&t);
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
            // 1. Calculate the current view (Projection)
            let rows = flatten_tree(&state.library, &state.session.nav.library);

            // 2. Find current selection index
            let current_idx = state
                .session
                .nav
                .library
                .selected_id
                .as_ref()
                .and_then(|id| rows.iter().position(|r| &r.id == id));

            if let Some(idx) = current_idx {
                if idx > 0 {
                    // 3. Move Selection Up
                    state.session.nav.library.selected_id = Some(rows[idx - 1].id.clone());
                }
            } else if !rows.is_empty() {
                // Default to last item if selection invalid/lost
                state.session.nav.library.selected_id = Some(rows.last().unwrap().id.clone());
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
                .selected_track_id
                .and_then(|id| queue.iter().position(|&x| x == id));

            if let Some(idx) = current_idx {
                if idx > 0 {
                    state.session.nav.queue.selected_track_id = queue.get(idx - 1).copied();
                }
            } else {
                state.session.nav.queue.selected_track_id = queue.front().copied();
            }
        }
    }
    vec![]
}

pub fn handle_move_down(state: &mut AppState) -> Vec<AppMsg> {
    state.last_nav_time = state.time_elapsed;

    match state.session.nav.active_panel {
        LeftPanelMode::Library => {
            // 1. Calculate the current view (Projection)
            let rows = flatten_tree(&state.library, &state.session.nav.library);

            // 2. Find current selection index
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
                        // 3. Move Selection Down
                        state.session.nav.library.selected_id = Some(rows[idx + 1].id.clone());
                    }
                }
                None => {
                    // Default to first item
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
                .selected_track_id
                .and_then(|id| queue.iter().position(|&x| x == id));

            match current_idx {
                Some(idx) => {
                    if idx + 1 < queue.len() {
                        state.session.nav.queue.selected_track_id = queue.get(idx + 1).copied();
                    }
                }
                None => {
                    state.session.nav.queue.selected_track_id = queue.front().copied();
                }
            }
        }
    }
    vec![]
}
