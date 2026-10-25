use crate::{
    AppMsg, AppState,
    library::projector,
    message::cover::CoverMsg,
    navigation::{
        ids::{CategoryId, SidebarNodeId},
        state::LeftPanelMode,
    },
};

/// Tab: the tree, then the queue, then the mixer when it is on screen, and
/// round again.
pub fn handle_toggle_library_queue(state: &mut AppState) -> Vec<AppMsg> {
    let from_mixer = state.is_mixer_focused();
    state.session.ui.mixer_focused = false;
    state.session.nav.active_panel = match state.session.nav.active_panel {
        _ if from_mixer => LeftPanelMode::Library,
        LeftPanelMode::Library => {
            if state.session.nav.queue.selected_queue_id.is_none() {
                state.session.nav.queue.selected_queue_id = state.queue().front().map(|e| e.id);
            }
            LeftPanelMode::Queue
        }
        LeftPanelMode::Queue if state.mixer_on_screen() => {
            state.session.ui.mixer_focused = true;
            crate::action::ui_action::select_in_mixer(state);
            LeftPanelMode::Queue
        }
        LeftPanelMode::Queue => LeftPanelMode::Library,
    };
    vec![]
}

/// Esc in the mixer: back to the panel it was moved on to from.
pub fn handle_leave_mixer(state: &mut AppState) -> Vec<AppMsg> {
    state.session.ui.mixer_focused = false;
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
            state.session.nav.show_track_info = true;
            if let Some(tid) = track_id
                && let Some(track) = state.library.get_track(tid).cloned()
                && state.track_info_cover_id != Some(tid)
            {
                state.clear_track_info_cover();
                if track.has_cover {
                    return vec![AppMsg::Cover(CoverMsg::RequestLoad {
                        id: tid,
                        path: track.file_path,
                        context: crate::message::cover::CoverContext::Info,
                    })];
                }
            }
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
    state.session.nav.show_track_info = false;
    let effects = vec![];

    match state.session.nav.active_panel {
        LeftPanelMode::Library => {
            let current_sel = state.get_library_selection();
            let nav = &state.session.nav.library;

            // If no selection, select first
            if current_sel.is_none() {
                if let Some(first) = projector::get_node_at_index(&state.library, nav, 0) {
                    state.session.nav.library.selected_id = Some(first);
                    state.session.nav.library.scroll_offset = 0;
                }
                return effects;
            }

            if let Some(target) = current_sel {
                if let Some(idx) = projector::get_index_of(&state.library, nav, target) {
                    if idx > 0 {
                        let new_idx = idx - 1;
                        if let Some(new_node) = crate::library::projector::get_node_at_index(
                            &state.library,
                            nav,
                            new_idx,
                        ) {
                            state.session.nav.library.selected_id = Some(new_node);

                            // Figure out how much of the list we can see
                            let (_, h) = state.terminal_size();
                            if h <= 3 {
                                return effects;
                            }
                            let viewport_height = (h - 3) as usize;

                            // Shrink scrolloff on tiny screens
                            let base_scrolloff = state.scrolloff();
                            let scrolloff = base_scrolloff.min(viewport_height / 2);

                            if new_idx < state.session.nav.library.scroll_offset + scrolloff {
                                state.session.nav.library.scroll_offset =
                                    new_idx.saturating_sub(scrolloff);
                            }
                        }
                    }
                } else {
                    // Selection broken, just reset to 0
                    if let Some(first) = projector::get_node_at_index(&state.library, nav, 0) {
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
                .get_queue_selection_id()
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
    effects
}

/// Moves a screenful down the tree or the queue, or up, a row at a time so
/// the view scrolls as it does for one.
pub fn handle_move_page(state: &mut AppState, down: bool) -> Vec<AppMsg> {
    let (_, h) = state.terminal_size();
    let rows = (h.saturating_sub(3) as usize).max(1);
    for _ in 0..rows {
        if down {
            handle_move_down(state);
        } else {
            handle_move_up(state);
        }
    }
    vec![]
}

/// Selects the first row of the tree or the queue, or with `last` the last,
/// the tree scrolled to show it.
pub fn handle_move_to_end(state: &mut AppState, last: bool) -> Vec<AppMsg> {
    state.last_nav_time = state.time_elapsed;
    state.session.nav.show_track_info = false;
    match state.session.nav.active_panel {
        LeftPanelMode::Library => {
            let nav = &state.session.nav.library;
            let total_rows = projector::get_row_count(&state.library, nav);
            if total_rows == 0 {
                return vec![];
            }
            let index = if last { total_rows - 1 } else { 0 };
            if let Some(node) = projector::get_node_at_index(&state.library, nav, index) {
                state.session.nav.library.selected_id = Some(node);
                let (_, h) = state.terminal_size();
                let viewport_height = (h.saturating_sub(3) as usize).max(1);
                state.session.nav.library.scroll_offset = if last {
                    total_rows.saturating_sub(viewport_height)
                } else {
                    0
                };
            }
        }
        LeftPanelMode::Queue => {
            let queue = state.queue();
            let entry = if last { queue.back() } else { queue.front() };
            if let Some(entry) = entry {
                state.session.nav.queue.selected_queue_id = Some(entry.id);
            }
        }
    }
    vec![]
}

pub fn handle_move_down(state: &mut AppState) -> Vec<AppMsg> {
    state.last_nav_time = state.time_elapsed;
    state.session.nav.show_track_info = false;
    let effects = vec![];

    match state.session.nav.active_panel {
        LeftPanelMode::Library => {
            let current_sel = state.get_library_selection();
            let nav = &state.session.nav.library;
            let total_rows = projector::get_row_count(&state.library, nav);

            if current_sel.is_none() {
                if let Some(first) = projector::get_node_at_index(&state.library, nav, 0) {
                    state.session.nav.library.selected_id = Some(first);
                    state.session.nav.library.scroll_offset = 0;
                }
                return effects;
            }

            if let Some(target) = current_sel {
                if let Some(idx) = projector::get_index_of(&state.library, nav, target) {
                    if idx + 1 < total_rows {
                        let new_idx = idx + 1;
                        if let Some(new_node) = crate::library::projector::get_node_at_index(
                            &state.library,
                            nav,
                            new_idx,
                        ) {
                            state.session.nav.library.selected_id = Some(new_node);

                            // Figure out how much of the list we can see
                            let (_, h) = state.terminal_size();
                            if h <= 3 {
                                return effects;
                            }
                            let viewport_height = (h - 3) as usize;

                            // Shrink scrolloff on tiny screens
                            let base_scrolloff = state.scrolloff();
                            let scrolloff = base_scrolloff.min(viewport_height / 2);

                            // Fix scroll offset if it somehow got ahead
                            if state.session.nav.library.scroll_offset > new_idx {
                                state.session.nav.library.scroll_offset =
                                    new_idx.saturating_sub(scrolloff);
                            }

                            // Scroll down when we reach the bottom edge
                            if new_idx
                                >= state.session.nav.library.scroll_offset + viewport_height
                                    - scrolloff
                            {
                                // Where we want the offset to be
                                let target_offset = new_idx + scrolloff + 1;
                                let raw_offset = target_offset.saturating_sub(viewport_height);

                                // Don't scroll past the end
                                let max_offset = total_rows.saturating_sub(viewport_height);
                                state.session.nav.library.scroll_offset =
                                    raw_offset.min(max_offset);
                            }
                        }
                    }
                } else {
                    // Selection invalid -> Select first
                    if let Some(first) = projector::get_node_at_index(&state.library, nav, 0) {
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
                .get_queue_selection_id()
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
    effects
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::TrackMeta, library::snapshot::Library};
    use std::collections::HashMap;

    fn test_state() -> AppState {
        let mut tracks = HashMap::new();
        let track_id = crate::domain::ids::TrackId(1);
        tracks.insert(
            track_id,
            TrackMeta {
                id: track_id,
                file_path: std::path::PathBuf::from("/tmp/test.mp3"),
                title: "Test Track".into(),
                artist: None,
                album: None,
                genre: None,
                year: None,
                is_favorite: false,
                track_number: None,
                play_count: 0,
                last_played_at: None,
                duration_ms: 10000,
                bitrate: None,
                sample_rate: None,
                format: "mp3".into(),
                has_cover: false,
                file_size_bytes: 1024,
            },
        );

        let library = Library::new(vec![std::path::PathBuf::from("/tmp")], vec![], tracks);
        AppState::new(library, &crate::config::AppConfig::default())
    }

    #[test]
    fn tab_goes_round_the_tree_the_queue_and_the_mixer() {
        let mut state = test_state();
        assert_eq!(state.session.nav.active_panel, LeftPanelMode::Library);

        handle_toggle_library_queue(&mut state);
        assert_eq!(state.session.nav.active_panel, LeftPanelMode::Queue);
        assert!(!state.is_mixer_focused());

        // The queue stays in the sidebar while the mixer has the keys.
        handle_toggle_library_queue(&mut state);
        assert!(state.is_mixer_focused());
        assert_eq!(state.session.nav.active_panel, LeftPanelMode::Queue);

        handle_toggle_library_queue(&mut state);
        assert!(!state.is_mixer_focused());
        assert_eq!(state.session.nav.active_panel, LeftPanelMode::Library);

        // Esc steps back out to the queue.
        handle_toggle_library_queue(&mut state);
        handle_toggle_library_queue(&mut state);
        handle_leave_mixer(&mut state);
        assert!(!state.is_mixer_focused());
        assert_eq!(state.session.nav.active_panel, LeftPanelMode::Queue);
    }

    #[test]
    fn tab_passes_the_mixer_by_where_it_is_not_shown() {
        let mut state = test_state();
        state.session.ui.show_lyrics = true;
        handle_toggle_library_queue(&mut state);
        handle_toggle_library_queue(&mut state);
        assert!(!state.is_mixer_focused());
        assert_eq!(state.session.nav.active_panel, LeftPanelMode::Library);

        // Focused, then hidden by the lyrics: the keys go to the queue.
        state.session.ui.show_lyrics = false;
        handle_toggle_library_queue(&mut state);
        handle_toggle_library_queue(&mut state);
        assert!(state.is_mixer_focused());
        state.session.ui.show_lyrics = true;
        assert!(!state.is_mixer_focused());
        assert!(state.is_queue_focused());
    }

    #[test]
    fn test_switch_to_playlists() {
        let mut state = test_state();
        handle_switch_to_playlists(&mut state);

        assert_eq!(state.session.nav.active_panel, LeftPanelMode::Library);
        assert_eq!(
            state.session.nav.library.selected_id,
            Some(SidebarNodeId::Category(CategoryId::Playlists))
        );
        assert!(
            state
                .session
                .nav
                .library
                .expanded_ids
                .contains(&SidebarNodeId::Category(CategoryId::Playlists))
        );
    }

    #[test]
    fn test_switch_to_tracks() {
        let mut state = test_state();
        state.session.nav.show_track_info = true;

        handle_switch_to_tracks(&mut state);

        assert!(!state.session.nav.show_track_info);
    }

    #[test]
    fn test_toggle_tree_expand() {
        let mut state = test_state();
        let node = SidebarNodeId::Category(CategoryId::Artists);
        state.session.nav.library.selected_id = Some(node.clone());

        assert!(!state.session.nav.library.expanded_ids.contains(&node));

        handle_toggle_tree_expand(&mut state);
        assert!(state.session.nav.library.expanded_ids.contains(&node));

        handle_toggle_tree_expand(&mut state);
        assert!(!state.session.nav.library.expanded_ids.contains(&node));
    }

    #[test]
    fn the_top_and_bottom_of_the_tree_are_a_key_away() {
        let mut state = test_state();
        state.session.ui.terminal_size = (80, 6);
        let rows = projector::get_row_count(&state.library, &state.session.nav.library);
        assert!(rows > 3, "{rows} rows");

        handle_move_to_end(&mut state, true);
        let nav = &state.session.nav.library;
        let at = projector::get_index_of(&state.library, nav, nav.selected_id.as_ref().unwrap());
        assert_eq!(at, Some(rows - 1));
        assert_eq!(
            nav.scroll_offset,
            rows - 3,
            "scrolled to show it at the bottom"
        );

        handle_move_to_end(&mut state, false);
        let nav = &state.session.nav.library;
        let at = projector::get_index_of(&state.library, nav, nav.selected_id.as_ref().unwrap());
        assert_eq!(at, Some(0));
        assert_eq!(nav.scroll_offset, 0);
    }

    #[test]
    fn the_first_and_last_of_the_queue_are_a_key_away() {
        use crate::{message::library::LibraryMsg, reducer::reduce};
        let mut state = test_state();
        let track = crate::domain::ids::TrackId(1);
        for _ in 0..3 {
            reduce(&mut state, AppMsg::Library(LibraryMsg::AddToQueue(track)));
        }
        state.session.nav.active_panel = LeftPanelMode::Queue;
        let ids: Vec<_> = state.queue().iter().map(|e| e.id).collect();
        assert_eq!(ids.len(), 3);

        handle_move_to_end(&mut state, true);
        assert_eq!(state.session.nav.queue.selected_queue_id, Some(ids[2]));
        handle_move_to_end(&mut state, false);
        assert_eq!(state.session.nav.queue.selected_queue_id, Some(ids[0]));
    }
}
