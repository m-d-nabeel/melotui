use crate::{
    AppMsg, AppState,
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
    state.session.nav.show_track_info = false;
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
        LeftPanelMode::Queue => {}
    }
    vec![]
}

pub fn handle_toggle_track_info(state: &mut AppState) -> Vec<AppMsg> {
    state.last_nav_time = state.time_elapsed;
    if state.session.nav.show_track_info {
        state.session.nav.show_track_info = false;
    } else {
        let has_track = match state.session.nav.active_panel {
            LeftPanelMode::Library => state.get_selected_track_id_in_sidebar().is_some(),
            LeftPanelMode::Queue => state.get_selected_track_id_in_queue().is_some(),
        };
        if has_track {
            let track_id = match state.session.nav.active_panel {
                LeftPanelMode::Library => state.get_selected_track_id_in_sidebar(),
                LeftPanelMode::Queue => state.get_selected_track_id_in_queue(),
            };
            if let Some(tid) = track_id
                && let Some(track) = state.library.get_track(tid).cloned()
            {
                state.load_track_info_art(&track);
            }
            state.session.nav.show_track_info = true;
        }
    }
    vec![]
}

pub fn handle_toggle_tree_expand(state: &mut AppState) -> Vec<AppMsg> {
    state.last_nav_time = state.time_elapsed;
    if let Some(selected) = &state.session.nav.library.selected_id {
        let is_expandable = matches!(
            selected,
            SidebarNodeId::Category(_)
                | SidebarNodeId::Folder(_)
                | SidebarNodeId::Playlist(_)
                | SidebarNodeId::Artist(_)
                | SidebarNodeId::Album(_)
                | SidebarNodeId::Genre(_)
                | SidebarNodeId::Year(_)
        );

        if is_expandable {
            let is_expanded = state.session.nav.library.expanded_ids.contains(selected);
            if is_expanded {
                state.session.nav.library.expanded_ids.remove(selected);
            } else {
                state
                    .session
                    .nav
                    .library
                    .expanded_ids
                    .insert(selected.clone());
            }
        }
    }
    vec![]
}

pub fn handle_move_up(state: &mut AppState) -> Vec<AppMsg> {
    state.last_nav_time = state.time_elapsed;
    match state.session.nav.active_panel {
        LeftPanelMode::Library => {
            let current_sel = state.session.nav.library.selected_id.clone();
            let nav = &state.session.nav.library;

            // If no selection, select last item? Or first?
            // Expensive to find last. Let's select first.
            if current_sel.is_none() {
                if let Some(first) =
                    crate::library::projector::get_node_at_index(&state.library, nav, 0)
                {
                    state.session.nav.library.selected_id = Some(first);
                    state.session.nav.library.scroll_offset = 0;
                }
                return vec![];
            }

            if let Some(target) = current_sel {
                if let Some(idx) =
                    crate::library::projector::get_index_of(&state.library, nav, &target)
                {
                    if idx > 0 {
                        let new_idx = idx - 1;
                        if let Some(new_node) = crate::library::projector::get_node_at_index(
                            &state.library,
                            nav,
                            new_idx,
                        ) {
                            state.session.nav.library.selected_id = Some(new_node);

                            // Scroll Up Logic (Scrolloff = 8)
                            let scrolloff = 8;
                            if new_idx < state.session.nav.library.scroll_offset + scrolloff {
                                state.session.nav.library.scroll_offset =
                                    new_idx.saturating_sub(scrolloff);
                            }
                        }
                    }
                } else {
                    // Selection invalid/collapsed? Reset to 0
                    if let Some(first) =
                        crate::library::projector::get_node_at_index(&state.library, nav, 0)
                    {
                        state.session.nav.library.selected_id = Some(first);
                        state.session.nav.library.scroll_offset = 0;
                    }
                }
            }
        }
        LeftPanelMode::Queue => {
            let queue = state.queue();
            if queue.is_empty() {
                return vec![];
            }

            // Find current queue index
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
            let current_sel = state.session.nav.library.selected_id.clone();
            let nav = &state.session.nav.library;
            let total_rows = crate::library::projector::get_row_count(&state.library, nav);

            if current_sel.is_none() {
                if let Some(first) =
                    crate::library::projector::get_node_at_index(&state.library, nav, 0)
                {
                    state.session.nav.library.selected_id = Some(first);
                    state.session.nav.library.scroll_offset = 0;
                }
                return vec![];
            }

            if let Some(target) = current_sel {
                if let Some(idx) =
                    crate::library::projector::get_index_of(&state.library, nav, &target)
                {
                    if idx + 1 < total_rows {
                        let new_idx = idx + 1;
                        if let Some(new_node) = crate::library::projector::get_node_at_index(
                            &state.library,
                            nav,
                            new_idx,
                        ) {
                            state.session.nav.library.selected_id = Some(new_node);

                            // Scroll Down Logic (Scrolloff = 8, Viewport Estimate = 25)
                            let height = 25;
                            let scrolloff = 8;
                            if new_idx
                                >= state.session.nav.library.scroll_offset + height - scrolloff
                            {
                                state.session.nav.library.scroll_offset =
                                    new_idx + scrolloff + 1 - height;
                            }
                        }
                    }
                } else {
                    // Selection invalid? Select first
                    if let Some(first) =
                        crate::library::projector::get_node_at_index(&state.library, nav, 0)
                    {
                        state.session.nav.library.selected_id = Some(first);
                        state.session.nav.library.scroll_offset = 0;
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
