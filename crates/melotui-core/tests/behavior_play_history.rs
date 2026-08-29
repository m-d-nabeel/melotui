//! Behaviour under test: a user who plays tracks in a running session sees
//! "Most Played" and "Recently Played" reflect that immediately — the app
//! should never need a rescan or restart to notice you just played
//! something. This is deliberately written from that expectation, not from
//! how `Library` happens to implement it: it dispatches the same messages a
//! real playback session produces (`RequestPlaySelected`, `AudioFinished`,
//! `RequestToggleFavorite`) and only ever inspects public, user-facing
//! queries (`Library::most_played`, `favorites`, `resolve_track_ids_for_node`
//! against the actual sidebar categories).

mod common;

use common::{Seed, app_state, dispatch, select_track};
use melotui_core::{
    AppMsg,
    domain::TrackId,
    message::playback::PlaybackMsg,
    navigation::ids::{CategoryId, SidebarNodeId},
    navigation::state::{SortDirection, SortField},
};

#[test]
fn a_track_played_this_session_shows_up_as_most_played_without_any_reload() {
    let mut state = app_state([Seed::new(1, "Only Song").artist("Solo")]);

    select_track(&mut state, 1, SidebarNodeId::Artist("Solo".into()));
    dispatch(
        &mut state,
        AppMsg::Playback(PlaybackMsg::RequestPlaySelected),
    );

    // Nothing has been played yet: Most Played must be empty.
    assert!(
        state.library.most_played().is_empty(),
        "a track that has never finished must not appear in Most Played"
    );

    // Let it finish three times over.
    for _ in 0..3 {
        dispatch(&mut state, AppMsg::Playback(PlaybackMsg::AudioFinished));
    }

    assert_eq!(
        state.library.tracks.get(&TrackId(1)).unwrap().play_count,
        3,
        "three finishes must be three recorded plays"
    );
    assert_eq!(
        state.library.most_played(),
        vec![TrackId(1)],
        "it must appear in Most Played in the same session it was played, with no reload"
    );
    assert_eq!(
        state.library.recently_played(),
        vec![TrackId(1)],
        "and in Recently Played too"
    );
}

#[test]
fn most_played_ranks_by_how_often_a_track_was_actually_played() {
    // Each track gets its own artist, so selecting and playing one never
    // pulls the others into the same playback context — finishing track 1
    // must never, as a side effect of context auto-advance, start counting
    // plays against track 2. Isolating that is the point of this shape, not
    // an accident of it.
    let mut state = app_state([
        Seed::new(1, "Played Twice").artist("Artist 1"),
        Seed::new(2, "Played Once").artist("Artist 2"),
        Seed::new(3, "Never Played").artist("Artist 3"),
    ]);

    for (id, artist, times) in [(1u64, "Artist 1", 2), (2u64, "Artist 2", 1)] {
        select_track(&mut state, id, SidebarNodeId::Artist(artist.into()));
        dispatch(
            &mut state,
            AppMsg::Playback(PlaybackMsg::RequestPlaySelected),
        );
        for _ in 0..times {
            dispatch(&mut state, AppMsg::Playback(PlaybackMsg::AudioFinished));
        }
    }

    assert_eq!(state.library.tracks[&TrackId(1)].play_count, 2);
    assert_eq!(state.library.tracks[&TrackId(2)].play_count, 1);
    assert_eq!(state.library.tracks[&TrackId(3)].play_count, 0);

    let most_played = state.library.most_played();
    assert_eq!(
        most_played,
        vec![TrackId(1), TrackId(2)],
        "ranked by play count, and the untouched track must be absent entirely"
    );
}

#[test]
fn browsing_most_played_in_the_sidebar_shows_exactly_what_was_played() {
    // This is the end-to-end version of the test above: instead of asking
    // Library::most_played() directly, it asks the same function the sidebar
    // widget calls when the user expands that category, at whatever sort
    // field happens to be active — Most Played is still expected to reflect
    // actual play counts, not get silently re-ordered into alphabetical.
    let mut state = app_state([
        Seed::new(1, "Zebra Song").artist("Artist 1"),
        Seed::new(2, "Apple Song").artist("Artist 2"),
    ]);
    state.set_sort(SortField::Title, SortDirection::Ascending);

    select_track(&mut state, 2, SidebarNodeId::Artist("Artist 2".into()));
    dispatch(
        &mut state,
        AppMsg::Playback(PlaybackMsg::RequestPlaySelected),
    );
    dispatch(&mut state, AppMsg::Playback(PlaybackMsg::AudioFinished));
    dispatch(&mut state, AppMsg::Playback(PlaybackMsg::AudioFinished));

    let node = SidebarNodeId::Category(CategoryId::MostPlayed);
    let ids =
        state
            .library
            .resolve_track_ids_for_node(&node, SortField::Title, SortDirection::Ascending);
    assert_eq!(
        ids,
        vec![TrackId(2)],
        "the sidebar's Most Played category must show the track actually played, \
         not be empty because the underlying list was never refreshed"
    );
}

#[test]
fn favoriting_is_visible_in_the_sidebar_the_same_session_it_happens() {
    let mut state = app_state([
        Seed::new(1, "Song A").artist("A"),
        Seed::new(2, "Song B").artist("A"),
    ]);

    let favorites_node = SidebarNodeId::Category(CategoryId::Favorites);
    assert!(
        state
            .library
            .resolve_track_ids_for_node(&favorites_node, SortField::Title, SortDirection::Ascending)
            .is_empty(),
        "nothing is favorited yet"
    );

    dispatch(
        &mut state,
        AppMsg::Library(
            melotui_core::message::library::LibraryMsg::RequestToggleFavorite(TrackId(2)),
        ),
    );

    let favorites = state.library.resolve_track_ids_for_node(
        &favorites_node,
        SortField::Title,
        SortDirection::Ascending,
    );
    assert_eq!(
        favorites,
        vec![TrackId(2)],
        "favoriting must show up under the Favorites category immediately"
    );

    dispatch(
        &mut state,
        AppMsg::Library(
            melotui_core::message::library::LibraryMsg::RequestToggleFavorite(TrackId(2)),
        ),
    );
    assert!(
        state
            .library
            .resolve_track_ids_for_node(&favorites_node, SortField::Title, SortDirection::Ascending)
            .is_empty(),
        "un-favoriting must remove it just as immediately"
    );
}
