use crate::{
    AppMsg, AppState,
    command::SearchField,
    domain::{PlaylistId, TrackId},
    library::projector,
    message::database::DatabaseMsg,
    navigation::ids::{CategoryId, SidebarNodeId},
    navigation::state::SearchState,
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

    // save index if removing selected fav to keep cursor stable
    let mut restore_selection_idx = None;
    if !new_status
        && let Some(SidebarNodeId::Track {
            id: selected_tid,
            parent,
        }) = state.get_library_selection()
        && *selected_tid == id
        && matches!(**parent, SidebarNodeId::Category(CategoryId::Favorites))
    {
        // removing active selection from view
        if let Some(idx) = projector::get_index_of(
            &state.library,
            state.library_nav(),
            &SidebarNodeId::Track {
                id,
                parent: parent.clone(),
            },
        ) {
            restore_selection_idx = Some(idx);
        }
    }

    state.library.toggle_favorite(id, new_status);

    // restore selection idx
    if let Some(old_idx) = restore_selection_idx {
        let nav = &state.session.nav.library;
        let new_count = projector::get_row_count(&state.library, nav);

        if new_count > 0 {
            // clamp
            let target_idx = old_idx.min(new_count - 1);
            if let Some(node) = projector::get_node_at_index(&state.library, nav, target_idx) {
                state.session.nav.library.selected_id = Some(node);
            }
        } else {
            // empty list -> select header
            state.session.nav.library.selected_id =
                Some(SidebarNodeId::Category(CategoryId::Favorites));
        }
    }

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

    // dispatch to DB
    vec![AppMsg::Database(DatabaseMsg::DispatchToggleFavorite(id))]
}

pub fn handle_request_create_playlist(state: &mut AppState, name: String) -> Vec<AppMsg> {
    if state.library.find_playlist_by_name(&name).is_some() {
        state.session.notification.current = Some(Notification::new(
            NotificationKind::Warning,
            format!("Playlist '{}' already exists", name),
        ));
        return vec![];
    }

    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("Creating playlist: {}", name),
    ));

    vec![AppMsg::Database(DatabaseMsg::DispatchCreatePlaylist(name))]
}

pub fn handle_request_delete_playlist(state: &mut AppState, name: String) -> Vec<AppMsg> {
    let pid = match state.library.find_playlist_by_name(&name) {
        Some(id) => id,
        None => {
            state.session.notification.current = Some(Notification::new(
                NotificationKind::Warning,
                format!("Playlist '{}' not found", name),
            ));
            return vec![];
        }
    };

    // adjust selection to next/prev sibling before removing
    if state.session.nav.library.selected_id == Some(SidebarNodeId::Playlist(pid)) {
        let idx = state.library.playlist_order.iter().position(|&p| p == pid);
        let new_sel = idx.and_then(|i| {
            if i + 1 < state.library.playlist_order.len() {
                Some(SidebarNodeId::Playlist(state.library.playlist_order[i + 1]))
            } else if i > 0 {
                Some(SidebarNodeId::Playlist(state.library.playlist_order[i - 1]))
            } else {
                None
            }
        });
        state.session.nav.library.selected_id =
            new_sel.or(Some(SidebarNodeId::Category(CategoryId::Playlists)));
    }

    state.library.remove_playlist(pid);

    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("Deleted playlist: {}", name),
    ));

    vec![AppMsg::Database(DatabaseMsg::DispatchDeletePlaylist(pid))]
}

pub fn handle_request_add_to_playlist(
    state: &mut AppState,
    playlist_id: PlaylistId,
    track_id: TrackId,
) -> Vec<AppMsg> {
    let pl_name = state
        .library
        .playlists
        .get(&playlist_id)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "Playlist".to_string());

    let track_title = state
        .library
        .get_track(track_id)
        .map(|t| t.title.clone())
        .unwrap_or_else(|| "Track".to_string());

    state.library.add_track_to_playlist(playlist_id, track_id);

    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("Added '{}' to '{}'", track_title, pl_name),
    ));

    vec![AppMsg::Database(DatabaseMsg::DispatchAddToPlaylist {
        playlist_id,
        track_id,
    })]
}

pub fn handle_request_remove_from_playlist(
    state: &mut AppState,
    playlist_id: PlaylistId,
    track_id: TrackId,
) -> Vec<AppMsg> {
    let track_title = state
        .library
        .get_track(track_id)
        .map(|t| t.title.clone())
        .unwrap_or_else(|| "Track".to_string());

    // save index if removing selected track to keep cursor stable
    let mut restore_selection_idx = None;
    if let Some(SidebarNodeId::Track {
        id: selected_tid,
        parent,
    }) = state.get_library_selection()
        && *selected_tid == track_id
        && matches!(**parent, SidebarNodeId::Playlist(pid) if pid == playlist_id)
        && let Some(idx) = projector::get_index_of(
            &state.library,
            state.library_nav(),
            &SidebarNodeId::Track {
                id: track_id,
                parent: parent.clone(),
            },
        )
    {
        restore_selection_idx = Some(idx);
    }

    state
        .library
        .remove_track_from_playlist(playlist_id, track_id);

    // restore selection idx
    if let Some(old_idx) = restore_selection_idx {
        let nav = &state.session.nav.library;
        let new_count = projector::get_row_count(&state.library, nav);

        if new_count > 0 {
            // clamp
            let target_idx = old_idx.min(new_count - 1);
            if let Some(node) = projector::get_node_at_index(&state.library, nav, target_idx) {
                state.session.nav.library.selected_id = Some(node);
            }
        } else {
            // empty list -> select header
            state.session.nav.library.selected_id = Some(SidebarNodeId::Playlist(playlist_id));
        }
    }

    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("Removed '{}' from playlist", track_title),
    ));

    vec![AppMsg::Database(DatabaseMsg::DispatchRemoveFromPlaylist {
        playlist_id,
        track_id,
    })]
}

pub fn handle_search(state: &mut AppState, field: SearchField, query: String) -> Vec<AppMsg> {
    let search = &state.session.nav.library.search;
    let is_repeat = search.field.as_ref() == Some(&field) && search.query == query;

    if is_repeat {
        let next = (search.index + 1) % search.matches.len().max(1);
        state.session.nav.library.search.index = next;
    } else {
        let matches = collect_matches(&state.library, &field, &query);
        state.session.nav.library.search = SearchState {
            field: Some(field.clone()),
            query: query.clone(),
            matches,
            index: 0,
        };
    }

    let search = &state.session.nav.library.search;
    if search.matches.is_empty() {
        state.session.notification.current = Some(Notification::new(
            NotificationKind::Warning,
            format!("No results for {} {}", field, query),
        ));
        return vec![];
    }

    navigate_to_current_match(state);
    vec![]
}

pub fn handle_search_next(state: &mut AppState) -> Vec<AppMsg> {
    let search = &state.session.nav.library.search;
    if search.matches.is_empty() {
        return vec![];
    }
    let next = (search.index + 1) % search.matches.len();
    state.session.nav.library.search.index = next;
    navigate_to_current_match(state);
    vec![]
}

pub fn handle_search_prev(state: &mut AppState) -> Vec<AppMsg> {
    let search = &state.session.nav.library.search;
    if search.matches.is_empty() {
        return vec![];
    }
    let len = search.matches.len();
    let prev = (search.index + len - 1) % len;
    state.session.nav.library.search.index = prev;
    navigate_to_current_match(state);
    vec![]
}

fn navigate_to_current_match(state: &mut AppState) {
    let search = &state.session.nav.library.search;
    let node = search.matches[search.index].clone();
    let idx_display = search.index + 1;
    let total = search.matches.len();
    let label = search
        .field
        .as_ref()
        .map(|f| format!("{}", f))
        .unwrap_or_default();
    let query = search.query.clone();

    // expand all ancestors
    let root = state.library.root.clone();
    let mut cur = node.clone();
    while let Some(parent) = cur.parent(&root) {
        state
            .session
            .nav
            .library
            .expanded_ids
            .insert(parent.clone());
        cur = parent;
    }
    state.session.nav.library.selected_id = Some(node.clone());

    // scroll to it
    let nav = &state.session.nav.library;
    if let Some(idx) = projector::get_index_of(&state.library, nav, &node) {
        let (_, h) = state.terminal_size();
        let viewport_height = if h > 3 { (h - 3) as usize } else { 1 };
        state.session.nav.library.scroll_offset = idx.saturating_sub(viewport_height / 2);
    }

    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("[{}/{}] {} {}", idx_display, total, label, query),
    ));
}

fn collect_matches(
    snapshot: &crate::library::Library,
    field: &SearchField,
    query: &str,
) -> Vec<SidebarNodeId> {
    let q = query.to_lowercase();
    match field {
        SearchField::Artist => snapshot
            .artists
            .keys()
            .filter(|k| k.to_lowercase().contains(&q))
            .map(|k| SidebarNodeId::Artist(k.clone()))
            .collect(),

        SearchField::Song => snapshot
            .tracks
            .iter()
            .filter(|(_, meta)| meta.title.to_lowercase().contains(&q))
            .map(|(id, meta)| {
                let parent = meta
                    .artist
                    .as_ref()
                    .map(|a| SidebarNodeId::Artist(a.clone()))
                    .unwrap_or(SidebarNodeId::Root);
                SidebarNodeId::Track {
                    id: *id,
                    parent: Box::new(parent),
                }
            })
            .collect(),

        SearchField::Album => snapshot
            .albums
            .keys()
            .filter(|k| k.to_lowercase().contains(&q))
            .map(|k| SidebarNodeId::Album(k.clone()))
            .collect(),

        SearchField::Genre => snapshot
            .genres
            .keys()
            .filter(|k| k.to_lowercase().contains(&q))
            .map(|k| SidebarNodeId::Genre(k.clone()))
            .collect(),

        SearchField::Year => q
            .parse::<u32>()
            .ok()
            .filter(|y| snapshot.years.contains_key(y))
            .map(SidebarNodeId::Year)
            .into_iter()
            .collect(),
    }
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
                duration_ms: 10000,
                bitrate: None,
                sample_rate: None,
                format: "mp3".into(),
                has_cover: false,
                file_size_bytes: 1024,
            },
        );

        let library = Library::new(std::path::PathBuf::from("/tmp"), vec![], tracks);
        AppState::new(library)
    }

    #[test]
    fn test_add_to_queue() {
        let mut state = test_state();
        let track_id = crate::domain::ids::TrackId(1);
        handle_add_to_queue(&mut state, track_id);

        assert_eq!(state.session.playback_queue.items.len(), 1);
        assert_eq!(state.session.playback_queue.items[0].track_id, track_id);
    }

    #[test]
    fn test_clear_queue() {
        let mut state = test_state();
        let track_id = crate::domain::ids::TrackId(1);
        handle_add_to_queue(&mut state, track_id);
        assert_eq!(state.session.playback_queue.items.len(), 1);

        handle_clear_queue(&mut state);
        assert!(state.session.playback_queue.items.is_empty());
    }

    #[test]
    fn test_toggle_favorite() {
        let mut state = test_state();
        let track_id = crate::domain::ids::TrackId(1);

        assert!(!state.library.get_track(track_id).unwrap().is_favorite);

        let effects = handle_request_toggle_favorite(&mut state, track_id);

        assert!(state.library.get_track(track_id).unwrap().is_favorite);
        assert_eq!(effects.len(), 1);
        assert!(
            matches!(effects[0], AppMsg::Database(DatabaseMsg::DispatchToggleFavorite(id)) if id == track_id)
        );
    }

    #[test]
    fn test_create_playlist() {
        let mut state = test_state();
        let effects = handle_request_create_playlist(&mut state, "My Playlist".into());

        assert_eq!(effects.len(), 1);
        assert!(
            matches!(&effects[0], AppMsg::Database(DatabaseMsg::DispatchCreatePlaylist(name)) if name == "My Playlist")
        );
    }
}
