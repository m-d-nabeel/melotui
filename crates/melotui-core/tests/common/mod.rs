//! Shared fixtures for melotui-core's behaviour suite.
//!
//! Everything here builds state the same way the real app does: through
//! `AppState::new` and `reducer::reduce`, never by hand-assembling a
//! `SessionState`/`PlaybackContext` mid-flight. A test that needs a track
//! playing gets there by dispatching `RequestPlaySelected`, the same message
//! a keypress produces — if that path breaks, the test breaks with it,
//! which is the point.

use melotui_core::{
    AppMsg, AppState,
    domain::{TrackId, TrackMeta},
    library::snapshot::Library,
    navigation::ids::SidebarNodeId,
    reducer,
};
use std::collections::HashMap;
use std::path::PathBuf;

/// One track to seed a library with. Only the fields a given test cares about
/// need setting; everything else takes a neutral default.
pub struct Seed {
    pub id: u64,
    pub title: &'static str,
    pub artist: Option<&'static str>,
    pub album: Option<&'static str>,
    pub genre: Option<&'static str>,
    pub year: Option<u32>,
    pub track_number: Option<u32>,
    pub duration_ms: u64,
}

#[allow(dead_code)] // shared fixture builder; not every test file uses every setter
impl Seed {
    pub fn new(id: u64, title: &'static str) -> Self {
        Self {
            id,
            title,
            artist: None,
            album: None,
            genre: None,
            year: None,
            track_number: None,
            duration_ms: 200_000,
        }
    }

    pub fn artist(mut self, name: &'static str) -> Self {
        self.artist = Some(name);
        self
    }

    pub fn album(mut self, name: &'static str) -> Self {
        self.album = Some(name);
        self
    }

    pub fn genre(mut self, name: &'static str) -> Self {
        self.genre = Some(name);
        self
    }

    pub fn year(mut self, year: u32) -> Self {
        self.year = Some(year);
        self
    }

    pub fn track_number(mut self, n: u32) -> Self {
        self.track_number = Some(n);
        self
    }

    pub fn duration_ms(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }

    fn into_meta(self) -> TrackMeta {
        TrackMeta {
            id: TrackId(self.id),
            file_path: PathBuf::from(format!("/music/{}.flac", self.id)),
            title: self.title.into(),
            artist: self.artist.map(str::to_string),
            album: self.album.map(str::to_string),
            genre: self.genre.map(str::to_string),
            year: self.year,
            is_favorite: false,
            track_number: self.track_number,
            duration_ms: self.duration_ms,
            play_count: 0,
            last_played_at: None,
            bitrate: None,
            sample_rate: None,
            format: "flac".into(),
            has_cover: false,
            file_size_bytes: 0,
        }
    }
}

/// Builds an `AppState` over a library made of `seeds`, exactly as the app
/// looks right after a scan completes and before anything has been played.
pub fn app_state(seeds: impl IntoIterator<Item = Seed>) -> AppState {
    let mut tracks = HashMap::new();
    for seed in seeds {
        let meta = seed.into_meta();
        tracks.insert(meta.id, meta);
    }
    let library = Library::new(vec![PathBuf::from("/music")], vec![], tracks);
    AppState::new(library, &melotui_core::config::AppConfig::default())
}

/// Selects a track in the sidebar as if the user had navigated to it under
/// `parent`, without yet pressing play. Mirrors what `nav_action`'s
/// selection-movement handlers leave behind, which none of these tests need
/// to drive keystroke-by-keystroke to get right.
pub fn select_track(state: &mut AppState, id: u64, parent: SidebarNodeId) {
    state.select_node(SidebarNodeId::Track {
        id: TrackId(id),
        parent: Box::new(parent),
    });
}

/// Dispatches `msg` through the real reducer and drains every effect it
/// returns back through the reducer again, the same loop `runtime.rs` runs in
/// the live app (minus forwarding engine-bound messages to a real audio/db
/// worker, which these tests have none of). This is what makes
/// `AudioFinished` actually result in a play-count increment landing in
/// `state.library`, rather than stopping at the first hop.
pub fn dispatch(state: &mut AppState, msg: AppMsg) {
    let mut queue = std::collections::VecDeque::from([msg]);
    let mut guard = 0;
    while let Some(m) = queue.pop_front() {
        guard += 1;
        assert!(
            guard < 500,
            "effect loop did not terminate — a handler is likely re-queuing itself"
        );
        queue.extend(reducer::reduce(state, m));
    }
}
