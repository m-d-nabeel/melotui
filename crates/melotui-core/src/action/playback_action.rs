use crate::{
    AppMsg, AppState,
    domain::TrackId,
    message::cover::{CoverContext, CoverMsg},
    message::playback::PlaybackMsg,
    navigation::ids::SidebarNodeId,
    playback::state::{PlaybackStatus, RepeatMode},
    playback::{advance, context::PlaybackContext},
    state::{Notification, NotificationKind, VisualizerMode},
};

pub fn handle_request_play_selected(state: &mut AppState) -> Vec<AppMsg> {
    let mut target_track_id: Option<TrackId> = None;
    let mut from_queue = false;

    let mut should_update_context = false;

    let mut new_context_tracks: Vec<TrackId> = Vec::new();
    let mut new_context_groups: Vec<u32> = Vec::new();
    let mut new_context_start: usize = 0;
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
                    new_context_tracks = state.library.resolve_track_ids_for_node(
                        &parent,
                        state.session.nav.library.sort_field,
                        state.session.nav.library.sort_direction,
                    );
                    // Computed here, before the empty-context fallback can
                    // replace `new_context_tracks`, keeping the two in
                    // lockstep rather than re-deriving groups wherever the
                    // context is eventually built.
                    new_context_groups = state.library.shuffle_group_keys(&new_context_tracks);

                    if new_context_tracks.is_empty() {
                        // fallback if no context support
                        new_context_tracks = vec![id];
                        new_context_groups = vec![0];
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

                    // Only used to position the cursor at build time; from then
                    // on the cursor moves by arithmetic, never by lookup.
                    if let Some(idx) = new_context_tracks.iter().position(|&x| x == id) {
                        new_context_start = idx;
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
            let shuffle = state.session.playback.context.shuffle();
            state.session.playback.context = PlaybackContext::with_groups(
                new_context_tracks,
                new_context_groups,
                new_context_start,
                new_source_name,
                shuffle,
            );
            state.session.playback.active_queue_id = None;
        }

        start_playback(state, id, from_queue)
    } else {
        vec![]
    }
}

/// Starts whatever `advance::peek` chose, or reports the end of the run.
fn play_next_up(state: &mut AppState) -> Vec<AppMsg> {
    let decision = advance::peek(&state.session);
    let from_queue = matches!(decision, advance::NextUp::Queued { .. });

    match advance::commit(&mut state.session, &decision) {
        Some(track) => {
            if !from_queue {
                update_active_node(state, track);
            } else {
                state.session.playback.active_node = None;
            }
            start_playback(state, track, from_queue)
        }
        None => {
            // Say so rather than silently doing nothing, which read as a
            // broken button.
            let source = state.get_playing_source_name().to_string();
            state.session.playback.status = PlaybackStatus::Stopped;
            state.session.notification.current = Some(Notification::new(
                NotificationKind::Info,
                format!("End of {source}"),
            ));
            vec![AppMsg::Playback(PlaybackMsg::DispatchStop)]
        }
    }
}

pub fn handle_request_next(state: &mut AppState) -> Vec<AppMsg> {
    // Pressing Next is a deliberate move past this track, so the queue entry it
    // came from is spent — exactly as if the track had played out. Repeat modes
    // that would replay or stop are deliberately not consulted here: an
    // explicit Next means "the next thing", whatever repeat is set to.
    advance::retire_active_queue_entry(&mut state.session);
    play_next_up(state)
}

pub fn handle_request_prev(state: &mut AppState) -> Vec<AppMsg> {
    // Standard transport behaviour: once a track is properly under way,
    // Previous restarts it rather than leaving the track.
    if state.session.playback.progress_ms > 3000 {
        return handle_request_seek_absolute(state, 0);
    }

    advance::retire_active_queue_entry(&mut state.session);

    if let Some(track) = advance::retreat(&mut state.session) {
        update_active_node(state, track);
        return start_playback(state, track, false);
    }

    // Nowhere further back: restart the current track.
    state.session.playback.progress_ms = 0;
    state.session.seek_epoch += 1;
    vec![AppMsg::Playback(PlaybackMsg::DispatchSeek {
        position_ms: 0,
        epoch: state.session.seek_epoch,
    })]
}

pub fn handle_audio_finished(state: &mut AppState) -> Vec<AppMsg> {
    let mut effects = vec![];
    if let Some(id) = state.session.playback.current {
        effects.push(AppMsg::Database(
            crate::message::database::DatabaseMsg::DispatchIncrementPlayCount(id),
        ));
    }

    state.session.playback.status = PlaybackStatus::Stopped;
    state.session.playback.progress_ms = 0;

    match state.session.repeat_mode {
        // Repeat-one: play the same track again, queue untouched.
        RepeatMode::Track => {
            if let Some(id) = state.session.playback.current {
                let from_queue = state.is_playing_from_queue();
                effects.extend(start_playback(state, id, from_queue));
                return effects;
            }
        }
        // "End After Track": stop here. The mode is left as the user set it,
        // so it keeps meaning what its label says on the next track too.
        RepeatMode::PlayOnce => {
            advance::retire_active_queue_entry(&mut state.session);
            state.session.notification.current = Some(Notification::new(
                NotificationKind::Info,
                "Stopped — end after track",
            ));
            effects.push(AppMsg::Playback(PlaybackMsg::DispatchStop));
            return effects;
        }
        RepeatMode::Off | RepeatMode::Playlist => {}
    }

    advance::retire_active_queue_entry(&mut state.session);
    effects.extend(play_next_up(state));
    effects
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

pub fn handle_request_seek_relative(state: &mut AppState, offset_seconds: i64) -> Vec<AppMsg> {
    if !state.is_playing() && !state.is_paused() {
        return vec![];
    }

    let cur = state.progress_ms() as i64;
    let dur = state.total_duration_ms() as i64;

    // if duration is 0 (e.g. unknown or stream), we just allow seeking from current progress,
    // though realistically a stream shouldn't be seekable. We clamp at 0.
    let max_dur = if dur == 0 { i64::MAX } else { dur };

    let new_ms = (cur + offset_seconds * 1000).clamp(0, max_dur);

    state.session.seek_epoch += 1;
    state.session.playback.progress_ms = new_ms as u64;
    sync_lyrics_line(state);
    vec![AppMsg::Playback(PlaybackMsg::DispatchSeek {
        position_ms: new_ms as u64,
        epoch: state.session.seek_epoch,
    })]
}

pub fn handle_request_seek_absolute(state: &mut AppState, position_seconds: u64) -> Vec<AppMsg> {
    if !state.is_playing() && !state.is_paused() {
        return vec![];
    }

    let dur = state.total_duration_ms();
    let max_dur = if dur == 0 { u64::MAX } else { dur };

    let new_ms = (position_seconds * 1000).clamp(0, max_dur);

    state.session.seek_epoch += 1;
    state.session.playback.progress_ms = new_ms;
    sync_lyrics_line(state);
    vec![AppMsg::Playback(PlaybackMsg::DispatchSeek {
        position_ms: new_ms,
        epoch: state.session.seek_epoch,
    })]
}

pub fn handle_audio_progress(state: &mut AppState, position_ms: u64, epoch: u64) -> Vec<AppMsg> {
    if state.is_playing() && epoch >= state.session.seek_epoch {
        state.session.playback.progress_ms = position_ms;
        sync_lyrics_line(state);
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

pub fn handle_cycle_visualizer_style(state: &mut AppState) -> Vec<AppMsg> {
    state.session.visualizer_mode = VisualizerMode::Spectrum;
    state.session.spectrum_style = state.session.spectrum_style.next();
    let style = state.session.spectrum_style.clone();
    state.session.notification.current = Some(
        Notification::new(
            NotificationKind::Info,
            format!("Visualizer Style: {}", style.as_str()),
        )
        .with_duration(2),
    );
    vec![AppMsg::Playback(PlaybackMsg::DispatchSetVisualizer {
        spectrum_style: Some(style),
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
pub fn handle_audio_started(state: &mut AppState, path: String) -> Vec<AppMsg> {
    state.session.lyrics.data =
        crate::state::lyrics::LyricsData::load_for(std::path::Path::new(&path));
    sync_lyrics_line(state);
    vec![]
}

/// Points the lyrics at the line for where playback is — after progress, and
/// after a seek, which while paused brings no progress to do it.
fn sync_lyrics_line(state: &mut AppState) {
    let position = state.session.playback.progress_ms as i64;
    state.session.lyrics.active_line_index = match &state.session.lyrics.data {
        crate::state::lyrics::LyricsData::Synced(lyrics) => lyrics.find_timed_line_index(position),
        _ => None,
    };
}
pub fn handle_audio_paused(_: &mut AppState) -> Vec<AppMsg> {
    vec![]
}
pub fn handle_audio_resumed(_: &mut AppState) -> Vec<AppMsg> {
    vec![]
}

fn start_playback(state: &mut AppState, id: TrackId, from_queue: bool) -> Vec<AppMsg> {
    // throttle if same track requested within 1s
    let now = std::time::Instant::now();
    if state.session.playback.current == Some(id)
        && state
            .session
            .last_play_time
            .is_some_and(|last| now.duration_since(last) < std::time::Duration::from_secs(1))
    {
        return vec![];
    }

    state.session.last_play_time = Some(now);

    let meta = match state.library.get_track(id) {
        Some(m) => m,
        None => return vec![],
    };

    // Extract all needed values before mutating state (borrow checker)
    let title = meta.title.clone();
    let file_path = meta.file_path.clone();
    let has_cover = meta.has_cover;
    let duration_ms = meta.duration_ms;

    state.session.playback.current = Some(id);
    state.session.playback.playing_from_queue = from_queue;
    state.session.playback.status = PlaybackStatus::Playing;
    state.session.playback.progress_ms = 0;
    state.session.playback.duration_ms = Some(duration_ms);

    state.current_cover = None;

    // Clear stale track_info cover — if the info panel is open, it should
    // show the new track's art, not the previous track's.
    if state.session.nav.show_track_info {
        state.clear_track_info_cover();
    }

    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("Playing {}", title),
    ));

    let active = state.should_draw_spectrum();
    let style = state.session.spectrum_style.clone();
    let mut effects = vec![
        AppMsg::Playback(PlaybackMsg::DispatchPlay {
            path: file_path.clone(),
        }),
        AppMsg::Playback(PlaybackMsg::DispatchSetVisualizer {
            spectrum_style: active.then_some(style),
        }),
    ];

    // request async cover load
    if has_cover {
        effects.push(AppMsg::Cover(CoverMsg::RequestLoad {
            id,
            path: file_path.clone(),
            context: CoverContext::Playback,
        }));

        // Also load for track_info panel if it's visible
        if state.session.nav.show_track_info {
            effects.push(AppMsg::Cover(CoverMsg::RequestLoad {
                id,
                path: file_path,
                context: CoverContext::Info,
            }));
        }
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
        let new_node = SidebarNodeId::Track {
            id: new_id,
            parent: p.clone(),
        };
        state.session.playback.active_node = Some(new_node.clone());

        if !state.is_playing_from_queue()
            && let Some(SidebarNodeId::Track {
                parent: ref sel_p, ..
            }) = state.session.nav.library.selected_id
            && *sel_p == p
        {
            state.session.nav.library.selected_id = Some(new_node.clone());

            let nav = &state.session.nav.library;
            let total_rows = crate::library::projector::get_row_count(&state.library, nav);
            if let Some(idx) =
                crate::library::projector::get_index_of(&state.library, nav, &new_node)
            {
                let (_, h) = state.terminal_size();
                if h > 3 {
                    let viewport_height = (h - 3) as usize;
                    let scrolloff = state.scrolloff().min(viewport_height / 2);

                    if state.session.nav.library.scroll_offset > idx {
                        state.session.nav.library.scroll_offset = idx.saturating_sub(scrolloff);
                    } else if idx
                        >= state.session.nav.library.scroll_offset + viewport_height - scrolloff
                    {
                        let target_offset = idx + scrolloff + 1;
                        let raw_offset = target_offset.saturating_sub(viewport_height);
                        let max_offset = total_rows.saturating_sub(viewport_height);
                        state.session.nav.library.scroll_offset = raw_offset.min(max_offset);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::TrackMeta, library::snapshot::Library};
    use std::collections::HashMap;

    /// Builds a state playing `tracks[start]` from a context of all of them.
    fn playing_context(ids: &[u64], start: usize) -> AppState {
        let mut tracks = HashMap::new();
        for &id in ids {
            let tid = TrackId(id);
            tracks.insert(
                tid,
                TrackMeta {
                    id: tid,
                    file_path: format!("/tmp/{id}.mp3").into(),
                    title: format!("T{id}"),
                    artist: None,
                    album: None,
                    genre: None,
                    year: None,
                    is_favorite: false,
                    track_number: None,
                    play_count: 0,
                    last_played_at: None,
                    duration_ms: 10_000,
                    bitrate: None,
                    sample_rate: None,
                    format: "mp3".into(),
                    has_cover: false,
                    file_size_bytes: 0,
                },
            );
        }
        let library = Library::new(vec![std::path::PathBuf::from("/tmp")], vec![], tracks);
        let mut state = AppState::new(library, &crate::config::AppConfig::default());
        let order: Vec<TrackId> = ids.iter().copied().map(TrackId).collect();
        state.session.playback.current = Some(order[start]);
        state.session.playback.context =
            PlaybackContext::new(order, start, "Album: Test".into(), false);
        state
    }

    /// Bypasses the "same track within 1s" throttle in `start_playback`.
    fn untick(state: &mut AppState) {
        state.session.last_play_time =
            Some(std::time::Instant::now() - std::time::Duration::from_secs(2));
    }

    #[test]
    fn seeking_while_paused_moves_the_lyrics_to_the_line_for_there() {
        let mut state = test_state_with_track();
        state.session.lyrics.data = crate::state::lyrics::LyricsData::from_text(
            "[00:01.00] One\n[00:10.00] Two\n[00:20.00] Three".into(),
        );
        state.session.playback.current = Some(TrackId(1));
        state.session.playback.status = PlaybackStatus::Paused;
        handle_request_seek_absolute(&mut state, 12);
        assert_eq!(state.session.lyrics.active_line_index, Some(1));
        // The test track is ten seconds long: from its end, back eight.
        handle_request_seek_relative(&mut state, -8);
        assert_eq!(state.session.lyrics.active_line_index, Some(0));
    }

    #[test]
    fn next_walks_the_same_order_the_tree_displays() {
        // Scanned (and so id-assigned) as Zebra, Apple, Mango; the tree, sorted
        // by title (the default), shows Apple, Mango, Zebra. Next used to walk
        // the id order instead, so picking the first track shown (Apple) and
        // pressing Next played Mango — right by the tree, wrong reason — while
        // picking the *second* track shown (Mango) and pressing Next jumped to
        // Zebra, skipping over Apple, which is the "jumps around" symptom.
        let mut tracks = HashMap::new();
        for (id, title) in [(1, "Zebra Song"), (2, "Apple Song"), (3, "Mango Song")] {
            tracks.insert(
                TrackId(id),
                TrackMeta {
                    id: TrackId(id),
                    file_path: format!("/tmp/{title}.flac").into(),
                    title: title.into(),
                    artist: Some("Artist X".into()),
                    album: None,
                    genre: None,
                    year: None,
                    is_favorite: false,
                    track_number: None,
                    play_count: 0,
                    last_played_at: None,
                    duration_ms: 10_000,
                    bitrate: None,
                    sample_rate: None,
                    format: "flac".into(),
                    has_cover: false,
                    file_size_bytes: 0,
                },
            );
        }
        let library = Library::new(vec![std::path::PathBuf::from("/tmp")], vec![], tracks);
        let mut state = AppState::new(library, &crate::config::AppConfig::default());

        // Select "Mango Song" (TrackId 3) — the second track the tree shows.
        state.session.nav.library.selected_id = Some(SidebarNodeId::Track {
            id: TrackId(3),
            parent: Box::new(SidebarNodeId::Artist("Artist X".into())),
        });
        handle_request_play_selected(&mut state);
        assert_eq!(
            state.session.playback.context.tracks(),
            &[TrackId(2), TrackId(3), TrackId(1)],
            "context must hold tracks in title order, matching the tree"
        );
        assert_eq!(state.get_playing_track().unwrap().title, "Mango Song");

        untick(&mut state);
        handle_request_next(&mut state);
        assert_eq!(
            state.get_playing_track().unwrap().title,
            "Zebra Song",
            "Next after the tree's 2nd track must play its 3rd, not skip around"
        );
    }

    #[test]
    fn shuffling_a_multi_artist_selection_declusters_by_artist() {
        // Two artists, 6 tracks each, selected via a Genre node (so the context
        // spans more than one artist). Enabling shuffle end-to-end through
        // handle_request_play_selected + handle_toggle_shuffle must produce a
        // context that actually avoids playing the same artist back to back —
        // proving the group keys reach PlaybackContext, not just that
        // shuffle_group_keys works in isolation.
        let mut tracks = HashMap::new();
        for n in 1..=6u64 {
            tracks.insert(
                TrackId(n),
                TrackMeta {
                    id: TrackId(n),
                    file_path: format!("/tmp/a{n}.mp3").into(),
                    title: format!("A{n}"),
                    artist: Some("Artist A".into()),
                    album: None,
                    genre: Some("Rock".into()),
                    year: None,
                    is_favorite: false,
                    track_number: None,
                    play_count: 0,
                    last_played_at: None,
                    duration_ms: 10_000,
                    bitrate: None,
                    sample_rate: None,
                    format: "mp3".into(),
                    has_cover: false,
                    file_size_bytes: 0,
                },
            );
            tracks.insert(
                TrackId(100 + n),
                TrackMeta {
                    id: TrackId(100 + n),
                    file_path: format!("/tmp/b{n}.mp3").into(),
                    title: format!("B{n}"),
                    artist: Some("Artist B".into()),
                    album: None,
                    genre: Some("Rock".into()),
                    year: None,
                    is_favorite: false,
                    track_number: None,
                    play_count: 0,
                    last_played_at: None,
                    duration_ms: 10_000,
                    bitrate: None,
                    sample_rate: None,
                    format: "mp3".into(),
                    has_cover: false,
                    file_size_bytes: 0,
                },
            );
        }
        let library = Library::new(vec![std::path::PathBuf::from("/tmp")], vec![], tracks);
        let mut state = AppState::new(library, &crate::config::AppConfig::default());

        state.session.nav.library.selected_id = Some(SidebarNodeId::Track {
            id: TrackId(1),
            parent: Box::new(SidebarNodeId::Genre("Rock".into())),
        });
        handle_request_play_selected(&mut state);
        crate::action::library_action::handle_toggle_shuffle(&mut state);
        assert!(state.shuffle_active());

        let mut prev_artist = state.get_playing_track().unwrap().artist.clone();
        let mut forced_repeats = 0;
        for _ in 0..11 {
            untick(&mut state);
            handle_request_next(&mut state);
            let artist = state.get_playing_track().unwrap().artist.clone();
            if artist == prev_artist {
                forced_repeats += 1;
            }
            prev_artist = artist;
        }

        assert!(
            forced_repeats <= 1,
            "{forced_repeats} same-artist repeats across 12 plays of 2 artists x 6 tracks \
             is what an artist-blind shuffle would do, not a declustered one"
        );
    }

    #[test]
    fn next_and_natural_end_choose_the_same_track() {
        // A queued track must be honoured whichever way the current one ends.
        // These used to disagree: finishing played the queued track, pressing
        // Next skipped it and played the next context track instead.
        for press_next in [false, true] {
            let mut state = playing_context(&[1, 2, 3], 0);
            state.session.playback_queue.add_track(TrackId(9));
            state.library.tracks.insert(
                TrackId(9),
                state.library.tracks.get(&TrackId(1)).unwrap().clone(),
            );
            state.session.playback_queue.items[0].track_id = TrackId(9);
            untick(&mut state);

            if press_next {
                handle_request_next(&mut state);
            } else {
                handle_audio_finished(&mut state);
            }
            assert_eq!(
                state.session.playback.current,
                Some(TrackId(9)),
                "queued track must win (press_next = {press_next})"
            );
            assert!(state.is_playing_from_queue());
        }
    }

    #[test]
    fn next_walks_a_context_containing_the_same_track_twice() {
        // The cursor used to be recovered by searching for the track id, so the
        // second copy of a track was unreachable and playback looped forever
        // over the first few entries.
        let mut state = playing_context(&[1, 2, 3], 0);
        state.session.playback.context = PlaybackContext::new(
            vec![TrackId(1), TrackId(2), TrackId(1)],
            0,
            "P".into(),
            false,
        );
        state.session.repeat_mode = RepeatMode::Off;

        let mut visited = vec![state.session.playback.context.natural_position()];
        for _ in 0..2 {
            untick(&mut state);
            handle_request_next(&mut state);
            visited.push(state.session.playback.context.natural_position());
        }
        assert_eq!(
            visited,
            vec![0, 1, 2],
            "must reach the repeated entry at index 2"
        );
    }

    #[test]
    fn next_past_the_last_track_stops_and_says_so() {
        let mut state = playing_context(&[1, 2, 3], 2);
        state.session.repeat_mode = RepeatMode::Off;
        untick(&mut state);

        let effects = handle_request_next(&mut state);

        assert!(
            matches!(state.playback_status(), PlaybackStatus::Stopped),
            "running off the end must stop rather than silently do nothing"
        );
        assert!(
            state
                .current_notification()
                .is_some_and(|n| n.message.starts_with("End of")),
            "the user must be told the run is over"
        );
        assert!(
            effects
                .iter()
                .any(|e| matches!(e, AppMsg::Playback(PlaybackMsg::DispatchStop)))
        );
    }

    #[test]
    fn play_once_ends_playback_after_the_current_track() {
        // The UI labels this mode "End After Track"; nothing used to read it.
        let mut state = playing_context(&[1, 2, 3], 0);
        state.session.repeat_mode = RepeatMode::PlayOnce;
        untick(&mut state);

        let effects = handle_audio_finished(&mut state);

        assert_eq!(
            state.session.playback.current,
            Some(TrackId(1)),
            "playback must not move on to the next track"
        );
        assert!(matches!(state.playback_status(), PlaybackStatus::Stopped));
        assert!(
            effects
                .iter()
                .any(|e| matches!(e, AppMsg::Playback(PlaybackMsg::DispatchStop)))
        );
    }

    #[test]
    fn repeat_one_replays_but_an_explicit_next_still_moves_on() {
        let mut state = playing_context(&[1, 2, 3], 0);
        state.session.repeat_mode = RepeatMode::Track;

        untick(&mut state);
        handle_audio_finished(&mut state);
        assert_eq!(
            state.session.playback.current,
            Some(TrackId(1)),
            "repeat-one replays"
        );

        untick(&mut state);
        handle_request_next(&mut state);
        assert_eq!(
            state.session.playback.current,
            Some(TrackId(2)),
            "an explicit Next overrides repeat-one"
        );
    }

    #[test]
    fn prev_restarts_the_track_once_it_is_under_way() {
        let mut state = playing_context(&[1, 2, 3], 1);
        state.session.playback.progress_ms = 8_000;
        untick(&mut state);

        handle_request_prev(&mut state);
        assert_eq!(
            state.session.playback.current,
            Some(TrackId(2)),
            "past the grace period, Previous restarts rather than leaves the track"
        );

        state.session.playback.progress_ms = 500;
        untick(&mut state);
        handle_request_prev(&mut state);
        assert_eq!(state.session.playback.current, Some(TrackId(1)));
    }

    #[test]
    fn up_next_matches_what_next_actually_plays() {
        // The prediction and the transport share one decision point, so this
        // holds by construction; the test guards the wiring.
        let mut state = playing_context(&[1, 2, 3], 0);
        state.session.playback_queue.add_track(TrackId(3));

        let predicted = state.predict_next_song();
        untick(&mut state);
        handle_request_next(&mut state);
        let actual = state
            .get_playing_track()
            .map(|t| t.title.clone())
            .unwrap_or_default();

        assert_eq!(predicted, actual);
    }

    fn test_state_with_track() -> AppState {
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
        let mut state = AppState::new(library, &crate::config::AppConfig::default());

        state.session.playback.current = Some(track_id);
        state.session.playback.duration_ms = Some(10000);
        state
    }

    #[test]
    fn test_toggle_play_pause_cycle() {
        let mut state = test_state_with_track();
        state.session.playback.status = PlaybackStatus::Playing;

        // Playing -> Paused
        let effects = handle_request_toggle_playback(&mut state);
        assert_eq!(state.session.playback.status, PlaybackStatus::Paused);
        assert!(matches!(
            effects[0],
            AppMsg::Playback(PlaybackMsg::DispatchPause)
        ));

        // Paused -> Playing
        let effects = handle_request_toggle_playback(&mut state);
        assert_eq!(state.session.playback.status, PlaybackStatus::Playing);
        assert!(matches!(
            effects[0],
            AppMsg::Playback(PlaybackMsg::DispatchResume)
        ));
    }

    #[test]
    fn test_seek_absolute_clamps_to_duration() {
        let mut state = test_state_with_track();
        state.session.playback.status = PlaybackStatus::Playing;
        state.session.playback.duration_ms = Some(10000);
        state.session.playback.progress_ms = 5000;

        let effects = handle_request_seek_absolute(&mut state, 20); // Absolute 20s (20000ms) > 10000ms

        assert_eq!(state.session.playback.progress_ms, 10000);
        match effects.first().unwrap() {
            AppMsg::Playback(PlaybackMsg::DispatchSeek { position_ms, .. }) => {
                assert_eq!(*position_ms, 10000);
            }
            _ => panic!("Expected DispatchSeek"),
        }
    }

    #[test]
    fn test_seek_relative_clamps_to_duration() {
        let mut state = test_state_with_track();
        state.session.playback.status = PlaybackStatus::Playing;
        state.session.playback.duration_ms = Some(10000);
        state.session.playback.progress_ms = 5000;

        let effects = handle_request_seek_relative(&mut state, 15); // +15s -> 20000ms > 10000ms

        assert_eq!(state.session.playback.progress_ms, 10000);
        match effects.first().unwrap() {
            AppMsg::Playback(PlaybackMsg::DispatchSeek { position_ms, .. }) => {
                assert_eq!(*position_ms, 10000);
            }
            _ => panic!("Expected DispatchSeek"),
        }
    }

    #[test]
    fn test_seek_relative_clamps_to_zero() {
        let mut state = test_state_with_track();
        state.session.playback.status = PlaybackStatus::Playing;
        state.session.playback.duration_ms = Some(10000);
        state.session.playback.progress_ms = 5000;

        let effects = handle_request_seek_relative(&mut state, -10); // -10s -> -5000ms < 0

        assert_eq!(state.session.playback.progress_ms, 0);
        match effects.first().unwrap() {
            AppMsg::Playback(PlaybackMsg::DispatchSeek { position_ms, .. }) => {
                assert_eq!(*position_ms, 0);
            }
            _ => panic!("Expected DispatchSeek"),
        }
    }

    #[test]
    fn test_seek_ignored_when_stopped() {
        let mut state = test_state_with_track();
        state.session.playback.status = PlaybackStatus::Stopped;
        state.session.playback.duration_ms = Some(10000);
        state.session.playback.progress_ms = 0;

        let effects = handle_request_seek_relative(&mut state, 5);
        assert!(effects.is_empty(), "Seek should be ignored when stopped");

        let effects_abs = handle_request_seek_absolute(&mut state, 5);
        assert!(
            effects_abs.is_empty(),
            "Absolute seek should be ignored when stopped"
        );
    }

    #[test]
    fn test_stop_resets_progress() {
        let mut state = test_state_with_track();
        state.session.playback.status = PlaybackStatus::Playing;
        state.session.playback.progress_ms = 5000;

        let effects = handle_request_stop(&mut state);
        assert_eq!(state.session.playback.status, PlaybackStatus::Stopped);
        assert_eq!(state.session.playback.progress_ms, 0);
        assert!(matches!(
            effects[0],
            AppMsg::Playback(PlaybackMsg::DispatchStop)
        ));
    }

    #[test]
    fn test_skip_next_and_prev_after_session_restore() {
        use crate::navigation::ids::SidebarNodeId;
        use crate::state::session_store::PersistedSession;
        use std::path::PathBuf;

        let mut tracks = HashMap::new();
        for id in 1..=3 {
            let track_id = TrackId(id);
            tracks.insert(
                track_id,
                TrackMeta {
                    id: track_id,
                    file_path: PathBuf::from(format!("/home/user/Music/Album/track{}.mp3", id)),
                    title: format!("Track {}", id),
                    artist: Some("Artist".into()),
                    album: Some("Album".into()),
                    genre: None,
                    year: None,
                    is_favorite: false,
                    track_number: Some(id as u32),
                    play_count: 0,
                    last_played_at: None,
                    duration_ms: 180000,
                    bitrate: None,
                    sample_rate: None,
                    format: "mp3".into(),
                    has_cover: false,
                    file_size_bytes: 1024,
                },
            );
        }

        let library = Library::new(vec![PathBuf::from("/home/user/Music")], vec![], tracks);

        // --- Step 1: Normal App Session ---
        // Start app, select Track 2 in sidebar, and play it.
        let mut original_state =
            AppState::new(library.clone(), &crate::config::AppConfig::default());
        let folder_node = SidebarNodeId::Folder(PathBuf::from("/home/user/Music/Album"));
        original_state.session.nav.library.selected_id = Some(SidebarNodeId::Track {
            id: TrackId(2),
            parent: Box::new(folder_node.clone()),
        });

        // Trigger normal playback selection action
        handle_request_play_selected(&mut original_state);
        assert_eq!(original_state.session.playback.current, Some(TrackId(2)));
        assert_eq!(
            original_state.session.playback.context.tracks(),
            &[TrackId(1), TrackId(2), TrackId(3)]
        );
        assert_eq!(
            original_state.session.playback.context.natural_position(),
            1
        );

        // --- Step 2: Simulate App Shutdown & Session Persistence ---
        // PersistedSession::from_app_state serializes the state exactly as saved to last_session.toml
        let persisted = PersistedSession::from_app_state(&original_state);
        let toml_str = toml::to_string(&persisted).expect("Serialization failed");

        // --- Step 3: Simulate App Relaunch (runtime.rs LibraryLoaded flow) ---
        let mut restored_state = AppState::new(library, &crate::config::AppConfig::default());
        let loaded_session: PersistedSession =
            toml::from_str(&toml_str).expect("Deserialization failed");

        // Execute exact runtime restoration sequence from runtime.rs
        loaded_session.apply_to_app_state(&mut restored_state);
        restored_state.ensure_library_selection();

        // Verify state after full restoration
        assert_eq!(restored_state.session.playback.current, Some(TrackId(2)));
        assert_eq!(
            restored_state.session.playback.context.tracks(),
            &[TrackId(1), TrackId(2), TrackId(3)]
        );
        assert_eq!(
            restored_state.session.playback.context.natural_position(),
            1
        );

        // --- Step 4: Verify Skip Next works identically after session restore ---
        // Advance time to bypass 1s same-track play throttle
        restored_state.session.last_play_time =
            Some(std::time::Instant::now() - std::time::Duration::from_secs(2));
        let next_effects = handle_request_next(&mut restored_state);
        assert!(
            !next_effects.is_empty(),
            "Skip next should produce playback actions"
        );
        assert_eq!(restored_state.session.playback.current, Some(TrackId(3)));
        assert_eq!(
            restored_state.session.playback.context.natural_position(),
            2
        );

        // --- Step 5: Verify Skip Prev works identically after session restore ---
        restored_state.session.playback.progress_ms = 0; // Ensure progress is at start so it skips to previous track
        restored_state.session.last_play_time =
            Some(std::time::Instant::now() - std::time::Duration::from_secs(2));
        let prev_effects = handle_request_prev(&mut restored_state);
        assert!(
            !prev_effects.is_empty(),
            "Skip prev should produce playback actions"
        );
        assert_eq!(restored_state.session.playback.current, Some(TrackId(2)));
        assert_eq!(
            restored_state.session.playback.context.natural_position(),
            1
        );
    }
}
