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

/// The library has been read from the database, and takes its place in the
/// state. The first time, the session saved at the last quit is put back
/// with it — what was playing, paused where it was, the queue, the effects —
/// and the scan at start is asked for.
///
/// This was done by the app's loop, on the state directly and on the
/// workers directly, where nothing could test it. Here it is a message like
/// any other: what it changes is in the state, and what it asks of the
/// workers is in what it returns.
pub fn handle_library_loaded(
    state: &mut AppState,
    snapshot: crate::library::Library,
    session: Option<crate::state::PersistedSession>,
) -> Vec<AppMsg> {
    use crate::message::{
        audio::AudioMsg,
        cover::{CoverContext, CoverMsg},
        library::LibraryMsg,
        playback::PlaybackMsg,
    };

    tracing::info!("Library loaded with {} tracks", snapshot.tracks.len());
    let first = !state.session.library_loaded;
    state.library = snapshot;
    state.session.library_loaded = true;
    let mut effects = Vec::new();

    if let Some(session) = session {
        let count = session.apply_to_app_state(state);
        // A device chosen with :device last time is chosen again.
        if let Some(device) = session.output_device.clone() {
            effects.push(AppMsg::Audio(AudioMsg::SetOutputDevice(Some(device))));
        }
        // What was playing is made ready, paused where it was.
        if let Some(active_id) = state.get_playing_track_id()
            && let Some(track) = state.library.tracks.get(&active_id)
        {
            effects.push(AppMsg::Playback(PlaybackMsg::DispatchPlay {
                path: track.file_path.clone(),
            }));
            effects.push(AppMsg::Playback(PlaybackMsg::DispatchPause));
            if state.progress_ms() > 0 {
                effects.push(AppMsg::Playback(PlaybackMsg::DispatchSeek {
                    position_ms: state.progress_ms(),
                    epoch: state.seek_epoch(),
                }));
            }
            if track.has_cover {
                effects.push(AppMsg::Cover(CoverMsg::RequestLoad {
                    id: active_id,
                    path: track.file_path.clone(),
                    context: CoverContext::Playback,
                }));
            }
        }
        effects.extend(crate::action::audio_action::sync_audio_thread(state));
        if state.should_draw_spectrum() {
            effects.push(AppMsg::Playback(PlaybackMsg::DispatchSetVisualizer {
                spectrum_style: Some(state.spectrum_style().clone()),
            }));
        }
        state.set_notification(
            NotificationKind::Info,
            format!("Session restored ({count} tracks in queue)"),
        );
    } else if !first && state.take_quiet_scan() {
        // A background scan found something.
        state.set_notification(NotificationKind::Info, "Library updated");
    } else {
        state.set_notification(NotificationKind::Info, "Library loaded");
    }

    // Music added while melotui was closed is picked up at start; an empty
    // library is a first run, so that scan shows.
    if !state.session.startup_scan_requested
        && !state.library.roots.is_empty()
        && state.config.library.scan_on_startup
    {
        state.session.startup_scan_requested = true;
        let scan = if state.library.tracks.is_empty() {
            LibraryMsg::RequestScanLibrary
        } else {
            LibraryMsg::RequestBackgroundScan
        };
        effects.push(AppMsg::Library(scan));
    }

    state.ensure_library_selection();

    if state.is_showing_track_info()
        && let Some(id) = state.get_track_info_target_id()
        && let Some(track) = state.library.get_track(id)
        && track.has_cover
    {
        effects.push(AppMsg::Cover(CoverMsg::RequestLoad {
            id,
            path: track.file_path.clone(),
            context: CoverContext::Info,
        }));
    }
    effects
}

/// A scan under way has more to show. Nothing is said: the scan's own
/// progress is on the screen.
pub fn handle_library_grew(state: &mut AppState, snapshot: crate::library::Library) -> Vec<AppMsg> {
    state.library = snapshot;
    state.ensure_library_selection();
    vec![]
}

/// A scan finished. The library is read again only if the scan found
/// something, so a background scan of an unchanged library disturbs
/// nothing; and a background scan says nothing either way.
pub fn handle_library_rescanned(state: &mut AppState, changed: bool) -> Vec<AppMsg> {
    let quiet = state.is_scanning_quietly();
    if changed {
        if !quiet {
            state.set_notification(NotificationKind::Info, "Scan complete, reloading...");
        }
        return vec![AppMsg::Database(DatabaseMsg::DispatchLoadLibrary)];
    }
    state.take_quiet_scan();
    if !quiet {
        state.set_notification(NotificationKind::Info, "Library is up to date");
    }
    vec![]
}

pub fn handle_request_scan_library(state: &mut AppState) -> Vec<AppMsg> {
    state.session.scan_quietly = false;
    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        "Scanning Library...",
    ));
    vec![]
}

pub fn handle_update_scan_progress(
    state: &mut AppState,
    scanned: u32,
    skipped: u32,
    _total: u32,
) -> Vec<AppMsg> {
    if state.session.scan_quietly {
        return vec![];
    }
    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!(
            "Scanning Library... {} processed ({} skipped)",
            scanned, skipped
        ),
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
    let enabled = !state.session.playback.context.shuffle();
    // Rebuilds the play order around whatever is playing, so toggling never
    // skips the current track and turning it back off returns to the real
    // position in the list.
    state.session.playback.context.set_shuffle(enabled);
    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("Shuffle: {}", if enabled { "On" } else { "Off" }),
    ));
    vec![]
}

pub fn handle_toggle_sort(state: &mut AppState) -> Vec<AppMsg> {
    use crate::navigation::state::{SortDirection, SortField};

    // Toggle logic: Title -> Artist -> Album -> Year -> Duration
    // If we're at Duration, switch direction and go back to Title

    let current_field = state.session.nav.library.sort_field;
    let current_dir = state.session.nav.library.sort_direction;

    let (new_field, new_dir) = match (current_field, current_dir) {
        (SortField::Title, dir) => (SortField::Artist, dir),
        (SortField::Artist, dir) => (SortField::Album, dir),
        (SortField::Album, dir) => (SortField::Year, dir),
        (SortField::Year, dir) => (SortField::Duration, dir),
        (SortField::Duration, SortDirection::Ascending) => {
            (SortField::Title, SortDirection::Descending)
        }
        (SortField::Duration, SortDirection::Descending) => {
            (SortField::Title, SortDirection::Ascending)
        }
    };

    state.session.nav.library.sort_field = new_field;
    state.session.nav.library.sort_direction = new_dir;

    let dir_str = match new_dir {
        SortDirection::Ascending => "Asc",
        SortDirection::Descending => "Desc",
    };

    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("Sorted by {:?} ({})", new_field, dir_str),
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
    vec![AppMsg::Database(DatabaseMsg::DispatchSetFavorite {
        id,
        favorite: new_status,
    })]
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
    let mut cur = node.clone();
    while let Some(parent) = cur.parent(&state.library.roots) {
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

        SearchField::Song => matching_songs(snapshot, &q),

        // Songs first, as what is most often looked for, then the artists
        // and albums of that name.
        SearchField::Any => {
            let mut found = matching_songs(snapshot, &q);
            for field in [SearchField::Artist, SearchField::Album] {
                found.extend(collect_matches(snapshot, &field, query));
            }
            found
        }

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

/// The songs matching `query`, already lowercased: those with every word of
/// it somewhere in their title, artist or album, so that "lisa gurenge" and
/// "gurenge lisa" find the same song and half a word is enough. Those whose
/// title begins with the query come first, then those with it in the title,
/// then the rest, each in order of title.
///
/// Each as it appears in its folder, which every song has: under its artist
/// a song with no artist tagged had nowhere to be shown.
///
/// This used to go to the database's full-text index, which matches whole
/// words only — "fad" did not find "Faded" — and takes a quote or a hyphen
/// in the query for its own syntax, finding nothing and saying nothing.
fn matching_songs(snapshot: &crate::library::Library, query: &str) -> Vec<SidebarNodeId> {
    let words: Vec<&str> = query.split_whitespace().collect();
    if words.is_empty() {
        return vec![];
    }
    let mut found: Vec<(u8, String, SidebarNodeId)> = snapshot
        .tracks
        .values()
        .filter_map(|track| {
            let title = track.title.to_lowercase();
            let rest = format!(
                "{} {}",
                track.artist.as_deref().unwrap_or_default(),
                track.album.as_deref().unwrap_or_default()
            )
            .to_lowercase();
            if !words.iter().all(|w| title.contains(w) || rest.contains(w)) {
                return None;
            }
            let rank = if title.starts_with(query) {
                0
            } else if title.contains(query) {
                1
            } else if words.iter().all(|w| title.contains(w)) {
                2
            } else {
                3
            };
            let dir = track.file_path.parent()?;
            let node = SidebarNodeId::Track {
                id: track.id,
                parent: Box::new(SidebarNodeId::Folder(dir.to_path_buf())),
            };
            Some((rank, title, node))
        })
        .collect();
    found.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    found.into_iter().map(|(_, _, node)| node).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::TrackMeta, library::snapshot::Library, message::library::LibraryMsg};
    use std::collections::HashMap;

    #[test]
    fn a_background_scan_shows_no_progress_and_a_requested_one_does() {
        let mut state = test_state();
        crate::reducer::reduce(
            &mut state,
            AppMsg::Library(LibraryMsg::RequestBackgroundScan),
        );
        assert!(state.is_scanning_quietly());
        handle_update_scan_progress(&mut state, 50, 10, 0);
        assert!(state.session.notification.current.is_none());

        crate::reducer::reduce(&mut state, AppMsg::Library(LibraryMsg::RequestScanLibrary));
        assert!(!state.is_scanning_quietly());
        handle_update_scan_progress(&mut state, 50, 10, 0);
        let message = &state.session.notification.current.as_ref().unwrap().message;
        assert!(message.starts_with("Scanning Library..."), "{message}");
    }

    /// A library of these songs: (id, title, artist, album).
    fn songs(list: &[(u64, &str, Option<&str>, Option<&str>)]) -> AppState {
        let tracks = list
            .iter()
            .map(|&(id, title, artist, album)| {
                let id = TrackId(id);
                let track = TrackMeta {
                    id,
                    file_path: format!("/music/{}/{title}.mp3", artist.unwrap_or("Unknown")).into(),
                    title: title.into(),
                    artist: artist.map(Into::into),
                    album: album.map(Into::into),
                    genre: None,
                    year: None,
                    is_favorite: false,
                    track_number: None,
                    play_count: 0,
                    last_played_at: None,
                    duration_ms: 1000,
                    bitrate: None,
                    sample_rate: None,
                    format: "mp3".into(),
                    has_cover: false,
                    file_size_bytes: 1,
                };
                (id, track)
            })
            .collect::<HashMap<_, _>>();
        let library = Library::new(vec!["/music".into()], vec![], tracks);
        AppState::new(library, &crate::config::AppConfig::default())
    }

    /// The ids of the songs a search finds, in order; `None` for a match
    /// that is not a song.
    fn found(state: &AppState) -> Vec<Option<u64>> {
        state
            .session
            .nav
            .library
            .search
            .matches
            .iter()
            .map(|node| match node {
                SidebarNodeId::Track { id, .. } => Some(id.0),
                _ => None,
            })
            .collect()
    }

    /// The library of `state`, as the database would send it again.
    fn reloaded(state: &AppState) -> Library {
        state.library.clone()
    }

    fn said(state: &AppState) -> String {
        state
            .current_notification()
            .map(|n| n.message.clone())
            .unwrap_or_default()
    }

    /// What the workers are asked for, by the kind of thing asked.
    fn asked(effects: &[AppMsg]) -> Vec<&'static str> {
        use crate::message::{audio::AudioMsg, playback::PlaybackMsg};
        effects
            .iter()
            .filter_map(|e| match e {
                AppMsg::Playback(PlaybackMsg::DispatchPlay { .. }) => Some("play"),
                AppMsg::Playback(PlaybackMsg::DispatchPause) => Some("pause"),
                AppMsg::Playback(PlaybackMsg::DispatchSeek { .. }) => Some("seek"),
                AppMsg::Audio(AudioMsg::DispatchSetVolume(_)) => Some("volume"),
                AppMsg::Library(LibraryMsg::RequestScanLibrary) => Some("scan"),
                AppMsg::Library(LibraryMsg::RequestBackgroundScan) => Some("quiet scan"),
                AppMsg::Database(DatabaseMsg::DispatchLoadLibrary) => Some("load"),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_first_library_brings_the_last_session_back_paused_where_it_was() {
        let mut state = songs(&[(1, "One", Some("A"), None), (2, "Two", Some("A"), None)]);
        let library = reloaded(&state);
        let mut session = crate::state::PersistedSession {
            active_track_id: Some(TrackId(2)),
            seek_position_ms: 700,
            queue: vec![TrackId(1)],
            volume: 0.5,
            ..Default::default()
        };
        session.dsp.bass = 2.0;
        assert!(!state.library_loaded());

        let effects = handle_library_loaded(&mut state, library, Some(session));

        assert!(state.library_loaded());
        assert_eq!(state.get_playing_track_id(), Some(TrackId(2)));
        assert_eq!(state.progress_ms(), 700);
        assert!(!state.is_playing(), "restored playing rather than paused");
        assert_eq!(state.volume(), 0.5);
        assert!(
            said(&state).starts_with("Session restored (1 "),
            "{}",
            said(&state)
        );
        // Made ready, paused, put where it was, and the audio thread told
        // the volume and the effects — in that order — then the scan at start.
        assert_eq!(
            asked(&effects),
            ["play", "pause", "seek", "volume", "quiet scan"]
        );
        let bass_sent = effects.iter().any(|e| {
            matches!(e, AppMsg::Audio(crate::message::audio::AudioMsg::DispatchSet(
                crate::state::audio::Effect::Bass, v)) if *v == 2.0)
        });
        assert!(bass_sent, "the effects were not sent to the audio thread");
    }

    #[test]
    fn a_first_run_with_nothing_saved_scans_for_all_to_see_and_only_once() {
        let mut state = songs(&[]);
        let empty = reloaded(&state);
        let effects = handle_library_loaded(&mut state, empty.clone(), None);
        assert_eq!(asked(&effects), ["scan"], "an empty library scans in view");
        assert_eq!(said(&state), "Library loaded");

        // The reload that scan ends in does not start another.
        let effects = handle_library_loaded(&mut state, empty, None);
        assert!(asked(&effects).is_empty(), "{:?}", asked(&effects));
    }

    #[test]
    fn nothing_is_scanned_at_start_when_told_not_to_or_with_nowhere_to_look() {
        let mut state = songs(&[(1, "One", None, None)]);
        state.config.library.scan_on_startup = false;
        let library = reloaded(&state);
        assert!(asked(&handle_library_loaded(&mut state, library, None)).is_empty());

        let mut state = songs(&[]);
        let mut nowhere = reloaded(&state);
        nowhere.roots.clear();
        assert!(asked(&handle_library_loaded(&mut state, nowhere, None)).is_empty());
    }

    #[test]
    fn a_scan_that_found_something_reloads_and_a_quiet_one_says_so_quietly() {
        let mut state = songs(&[(1, "One", None, None)]);
        let library = reloaded(&state);
        handle_library_loaded(&mut state, library.clone(), None);

        // Asked for by hand: said at each step.
        assert_eq!(asked(&handle_library_rescanned(&mut state, true)), ["load"]);
        assert_eq!(said(&state), "Scan complete, reloading...");
        handle_library_loaded(&mut state, library.clone(), None);
        assert_eq!(said(&state), "Library loaded");
        assert!(handle_library_rescanned(&mut state, false).is_empty());
        assert_eq!(said(&state), "Library is up to date");

        // In the background: reloaded all the same, and said once, after.
        state.clear_current_notification();
        reduce_all(&mut state, LibraryMsg::RequestBackgroundScan);
        assert_eq!(asked(&handle_library_rescanned(&mut state, true)), ["load"]);
        assert_eq!(said(&state), "", "a background scan spoke up");
        handle_library_loaded(&mut state, library, None);
        assert_eq!(said(&state), "Library updated");

        // And one that found nothing says nothing at all.
        state.clear_current_notification();
        reduce_all(&mut state, LibraryMsg::RequestBackgroundScan);
        assert!(handle_library_rescanned(&mut state, false).is_empty());
        assert_eq!(said(&state), "");
        assert!(!state.is_scanning_quietly());
    }

    #[test]
    fn a_library_that_grows_during_a_scan_takes_its_place_without_a_word() {
        let mut state = songs(&[(1, "One", None, None)]);
        state.clear_current_notification();
        let bigger = songs(&[(1, "One", None, None), (2, "Two", None, None)]);
        let effects = handle_library_grew(&mut state, reloaded(&bigger));
        assert!(effects.is_empty());
        assert_eq!(state.library.tracks.len(), 2);
        assert_eq!(said(&state), "");
    }

    fn reduce_all(state: &mut AppState, msg: LibraryMsg) {
        crate::reducer::reduce(state, AppMsg::Library(msg));
    }

    #[test]
    fn a_song_is_found_by_part_of_a_word_and_by_words_in_any_order() {
        let mut state = songs(&[
            (1, "Faded", Some("Alan Walker"), None),
            (2, "Gurenge", Some("LiSA"), Some("LEO-NiNE")),
            (3, "Way Back Home", Some("SHAUN"), None),
            (4, "Untagged", None, None),
        ]);

        // Half a word: the full-text index wanted the whole of it.
        assert!(handle_search(&mut state, SearchField::Song, "fad".into()).is_empty());
        assert_eq!(found(&state), [Some(1)]);

        // By artist and title together, either way round, whatever the case.
        for query in ["lisa gurenge", "GURENGE lisa", "leo gur"] {
            handle_search(&mut state, SearchField::Song, query.into());
            assert_eq!(found(&state), [Some(2)], "{query}");
        }

        // Characters the index took for its own syntax find nothing, quietly.
        for query in ["\"", "way-back", "a\" OR", "*"] {
            handle_search(&mut state, SearchField::Song, query.into());
            assert!(found(&state).is_empty(), "{query}");
        }

        // A song with no artist is found, and has somewhere to be shown.
        handle_search(&mut state, SearchField::Song, "untag".into());
        assert_eq!(found(&state), [Some(4)]);
        assert!(matches!(
            state.session.nav.library.selected_id,
            Some(SidebarNodeId::Track { id: TrackId(4), ref parent }) if matches!(**parent, SidebarNodeId::Folder(_))
        ));
    }

    #[test]
    fn the_closest_titles_come_first_and_find_looks_everywhere() {
        let mut state = songs(&[
            (1, "Home", Some("Someone"), None),
            (2, "Way Back Home", Some("SHAUN"), None),
            (3, "Homecoming", Some("Kanye"), None),
            (4, "Stronger", Some("Homer"), Some("Graduation")),
        ]);
        handle_search(&mut state, SearchField::Song, "home".into());
        // Beginning with it, in order of title; then containing it; then
        // only its artist does.
        assert_eq!(found(&state), [Some(1), Some(3), Some(2), Some(4)]);

        // Everything: the songs, then the artist of that name.
        handle_search(&mut state, SearchField::Any, "homer".into());
        assert_eq!(found(&state), [Some(4), None]);
        assert!(matches!(
            state.session.nav.library.search.matches[1],
            SidebarNodeId::Artist(ref name) if name == "Homer"
        ));

        // The selection goes to the first, and the next match to the next.
        handle_search(&mut state, SearchField::Song, "home".into());
        handle_search_next(&mut state);
        assert!(matches!(
            state.session.nav.library.selected_id,
            Some(SidebarNodeId::Track { id: TrackId(3), .. })
        ));
    }

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
            matches!(effects[0], AppMsg::Database(DatabaseMsg::DispatchSetFavorite { id, favorite: true }) if id == track_id)
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
