//! Behaviour under test: whatever order the sidebar tree shows a group of
//! tracks in, Next/Prev must walk that *exact* order — for every sort field
//! and direction the user can choose, not just the default. This is the
//! general form of a very concrete complaint: picking the 2nd track shown and
//! pressing Next played the 4th, because the list backing playback was
//! ordered differently from the list on screen.
//!
//! The comparison is against `get_visible_rows`, the actual function the
//! sidebar widget calls to know what to draw — not a hand-written
//! re-implementation of "what order should this be in", which could just as
//! easily share the same wrong assumption as whatever produced the bug.

mod common;

use common::{Seed, app_state, dispatch, select_track};
use melotui_core::{
    AppMsg,
    domain::TrackId,
    library::projector::get_visible_rows,
    message::playback::PlaybackMsg,
    navigation::ids::{CategoryId, SidebarNodeId},
    navigation::state::{SortDirection, SortField},
};

/// Opens a sidebar node the same way a keypress does: select it, then send
/// the same message a real "activate this row" keypress sends. For a
/// non-track node `handle_request_play_selected` toggles its expansion
/// rather than playing anything.
fn expand(state: &mut melotui_core::AppState, node: SidebarNodeId) {
    state.select_node(node);
    dispatch(state, AppMsg::Playback(PlaybackMsg::RequestPlaySelected));
}

/// The track ids the tree would currently draw under `artist`, in order —
/// reading them straight out of the real row-rendering function.
fn rendered_track_order(state: &melotui_core::AppState, artist: &str) -> Vec<TrackId> {
    get_visible_rows(&state.library, state.library_nav(), 0, 1000)
        .into_iter()
        .filter_map(|row| match row.id {
            SidebarNodeId::Track { id, parent }
                if *parent == SidebarNodeId::Artist(artist.into()) =>
            {
                Some(id)
            }
            _ => None,
        })
        .collect()
}

fn build_state() -> melotui_core::AppState {
    // Four tracks under one artist, deliberately chosen so Title, Album,
    // Year, and Duration order all disagree with each other and with
    // insertion/id order — a sort field that was silently ignored, or an
    // order that leaked from somewhere else, would show up as a mismatch on
    // at least one of these.
    let mut state = app_state([
        Seed::new(1, "Delta")
            .artist("Test Artist")
            .album("B Album")
            .track_number(3)
            .year(2020)
            .duration_ms(300_000),
        Seed::new(2, "Bravo")
            .artist("Test Artist")
            .album("A Album")
            .track_number(1)
            .year(2022)
            .duration_ms(100_000),
        Seed::new(3, "Charlie")
            .artist("Test Artist")
            .album("A Album")
            .track_number(2)
            .year(2021)
            .duration_ms(200_000),
        Seed::new(4, "Alpha")
            .artist("Test Artist")
            .album("B Album")
            .track_number(1)
            .year(2019)
            .duration_ms(400_000),
    ]);
    expand(&mut state, SidebarNodeId::Category(CategoryId::Artists));
    expand(&mut state, SidebarNodeId::Artist("Test Artist".into()));
    state
}

#[test]
fn next_and_prev_walk_exactly_the_order_shown_on_screen_for_every_sort_setting() {
    for field in [
        SortField::Title,
        SortField::Artist,
        SortField::Album,
        SortField::Year,
        SortField::Duration,
    ] {
        for direction in [SortDirection::Ascending, SortDirection::Descending] {
            let mut state = build_state();
            state.set_sort(field, direction);

            let expected = rendered_track_order(&state, "Test Artist");
            assert_eq!(
                expected.len(),
                4,
                "test setup: all four tracks must be visible under the artist \
                 for {field:?}/{direction:?}"
            );

            // Click whichever track the tree currently shows 3rd — not
            // necessarily track id 3, precisely to avoid the test
            // accidentally passing just because id order and click order
            // happened to line up.
            let clicked = expected[2];
            select_track(
                &mut state,
                clicked.0,
                SidebarNodeId::Artist("Test Artist".into()),
            );
            dispatch(
                &mut state,
                AppMsg::Playback(PlaybackMsg::RequestPlaySelected),
            );

            assert_eq!(
                state.get_playing_track_id(),
                Some(clicked),
                "{field:?}/{direction:?}: clicking a track must play that track"
            );
            assert_eq!(
                state.context_queue(),
                expected.as_slice(),
                "{field:?}/{direction:?}: playback context must match the tree's own order"
            );
        }
    }
}

#[test]
fn changing_sort_mid_session_and_reselecting_tracks_the_new_order() {
    // A user is allowed to change sort field while browsing; the very next
    // track they pick must build a context in the *new* order, not a stale
    // one left over from before the sort changed.
    let mut state = build_state();

    state.set_sort(SortField::Title, SortDirection::Ascending);
    select_track(&mut state, 3, SidebarNodeId::Artist("Test Artist".into()));
    dispatch(
        &mut state,
        AppMsg::Playback(PlaybackMsg::RequestPlaySelected),
    );
    assert_eq!(
        state.context_queue(),
        &[TrackId(4), TrackId(2), TrackId(3), TrackId(1)],
        "Title ascending: Alpha, Bravo, Charlie, Delta"
    );

    state.set_sort(SortField::Duration, SortDirection::Ascending);
    select_track(&mut state, 3, SidebarNodeId::Artist("Test Artist".into()));
    dispatch(
        &mut state,
        AppMsg::Playback(PlaybackMsg::RequestPlaySelected),
    );
    assert_eq!(
        state.context_queue(),
        &[TrackId(2), TrackId(3), TrackId(1), TrackId(4)],
        "Duration ascending: 100k, 200k, 300k, 400k"
    );
}
