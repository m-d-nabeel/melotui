//! Behaviour under test: turning shuffle on should feel like a shuffle — it
//! must never repeat a track before every other track in the list has had a
//! turn, and it should not routinely play the same artist twice in a row
//! just because that's what a coin flip happened to produce. Both are
//! exercised end-to-end through `LibraryMsg::ToggleShuffle` and repeated
//! `PlaybackMsg::RequestPlayNext` dispatches — the same messages the shuffle
//! key and the Next key send — rather than calling `PlaybackContext`
//! directly, so a regression in how the reducer wires shuffle up would show
//! here even if `PlaybackContext` itself were untouched.

mod common;

use common::{Seed, app_state, dispatch, select_track};
use melotui_core::{
    AppMsg,
    message::{library::LibraryMsg, playback::PlaybackMsg},
    navigation::ids::SidebarNodeId,
};

/// Cycles repeat mode (Off -> PlayOnce -> Track -> Playlist -> Off) around to
/// Playlist, the only mode where running off the end wraps instead of
/// stopping — needed to exercise the reshuffle-on-wrap path at all.
fn set_repeat_playlist(state: &mut melotui_core::AppState) {
    for _ in 0..3 {
        dispatch(state, AppMsg::Library(LibraryMsg::ToggleRepeatMode));
    }
    assert_eq!(
        *state.repeat_mode(),
        melotui_core::playback::state::RepeatMode::Playlist
    );
}
use std::collections::HashMap;

const ARTISTS: usize = 6;
const TRACKS_PER_ARTIST: usize = 5;

fn seed_multi_artist_library() -> melotui_core::AppState {
    let mut seeds = Vec::new();
    for a in 0..ARTISTS {
        for t in 0..TRACKS_PER_ARTIST {
            let id = (a * TRACKS_PER_ARTIST + t + 1) as u64;
            let artist: &'static str = Box::leak(format!("Artist {a}").into_boxed_str());
            let title: &'static str = Box::leak(format!("Track {id}").into_boxed_str());
            seeds.push(Seed::new(id, title).artist(artist).genre("Rock"));
        }
    }
    app_state(seeds)
}

fn artist_of(state: &melotui_core::AppState, id: melotui_core::domain::TrackId) -> String {
    state
        .library
        .tracks
        .get(&id)
        .and_then(|t| t.artist.clone())
        .unwrap_or_default()
}

/// Plays every track in one full shuffled pass, returning the sequence.
fn play_one_pass(
    state: &mut melotui_core::AppState,
    total_tracks: usize,
) -> Vec<melotui_core::domain::TrackId> {
    let mut seen = vec![state.get_playing_track_id().expect("something is playing")];
    for _ in 1..total_tracks {
        dispatch(state, AppMsg::Playback(PlaybackMsg::RequestPlayNext));
        seen.push(
            state
                .get_playing_track_id()
                .expect("next must select a track"),
        );
    }
    seen
}

#[test]
fn shuffle_plays_every_track_exactly_once_before_repeating() {
    let total = ARTISTS * TRACKS_PER_ARTIST;
    let mut state = seed_multi_artist_library();

    select_track(&mut state, 1, SidebarNodeId::Genre("Rock".into()));
    dispatch(
        &mut state,
        AppMsg::Playback(PlaybackMsg::RequestPlaySelected),
    );
    dispatch(&mut state, AppMsg::Library(LibraryMsg::ToggleShuffle));
    assert!(state.shuffle_active());

    let pass = play_one_pass(&mut state, total);

    let mut counts: HashMap<_, u32> = HashMap::new();
    for id in &pass {
        *counts.entry(*id).or_default() += 1;
    }
    assert_eq!(
        counts.len(),
        total,
        "every track must be reachable in one pass"
    );
    assert!(
        counts.values().all(|&c| c == 1),
        "no track may repeat within a single pass, got counts {counts:?}"
    );
}

#[test]
fn shuffle_rarely_plays_the_same_artist_twice_running() {
    let total = ARTISTS * TRACKS_PER_ARTIST;
    let mut state = seed_multi_artist_library();

    set_repeat_playlist(&mut state);
    select_track(&mut state, 1, SidebarNodeId::Genre("Rock".into()));
    dispatch(
        &mut state,
        AppMsg::Playback(PlaybackMsg::RequestPlaySelected),
    );
    dispatch(&mut state, AppMsg::Library(LibraryMsg::ToggleShuffle));

    // Two full passes back to back (the second forces a wrap-reshuffle),
    // since the wrap boundary is exactly where a naive implementation is
    // most likely to accidentally repeat a track or an artist.
    let mut sequence = play_one_pass(&mut state, total);
    for _ in 0..total {
        dispatch(&mut state, AppMsg::Playback(PlaybackMsg::RequestPlayNext));
        sequence.push(state.get_playing_track_id().unwrap());
    }

    let artists: Vec<String> = sequence.iter().map(|&id| artist_of(&state, id)).collect();
    let same_artist_repeats = artists.windows(2).filter(|w| w[0] == w[1]).count();

    // With 6 artists of 5 tracks each, a same-artist repeat can only be
    // forced when every other artist has momentarily run out for that pass —
    // it should be rare, not routine. A blind per-track shuffle over this
    // shape would be expected to repeat an artist on a sizeable fraction of
    // steps; declustering should keep it to a small handful across 2 full
    // passes (60 transitions).
    assert!(
        same_artist_repeats <= 4,
        "{same_artist_repeats} same-artist-in-a-row transitions across \
         {} steps is what an artist-blind shuffle would do",
        artists.len() - 1
    );
}

#[test]
fn toggling_shuffle_off_and_back_on_never_interrupts_the_playing_track() {
    let mut state = seed_multi_artist_library();
    select_track(&mut state, 1, SidebarNodeId::Genre("Rock".into()));
    dispatch(
        &mut state,
        AppMsg::Playback(PlaybackMsg::RequestPlaySelected),
    );
    dispatch(&mut state, AppMsg::Library(LibraryMsg::ToggleShuffle));

    dispatch(&mut state, AppMsg::Playback(PlaybackMsg::RequestPlayNext));
    dispatch(&mut state, AppMsg::Playback(PlaybackMsg::RequestPlayNext));
    let playing = state.get_playing_track_id();

    dispatch(&mut state, AppMsg::Library(LibraryMsg::ToggleShuffle)); // off
    assert_eq!(
        state.get_playing_track_id(),
        playing,
        "turning shuffle off must not change what's playing"
    );
    assert!(!state.shuffle_active());

    dispatch(&mut state, AppMsg::Library(LibraryMsg::ToggleShuffle)); // on again
    assert_eq!(
        state.get_playing_track_id(),
        playing,
        "turning shuffle back on must not change what's playing"
    );
    assert!(state.shuffle_active());
}
