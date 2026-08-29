use lofty::{file::TaggedFileExt, probe::Probe};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::domain::{PlaylistId, PlaylistMeta, TrackId, TrackMeta};
use crate::navigation::ids::{CategoryId, SidebarNodeId};
use crate::navigation::state::{SortDirection, SortField};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryNode {
    pub name: String,
    pub path: PathBuf,
    pub subdirs: Vec<DirectoryNode>,
    pub tracks: Vec<TrackId>,
}

/// In-memory representation of the music library.
///
/// Contains the canonical track data (`tracks`, the only place track content
/// lives) plus a handful of **materialized indices** (artists, albums,
/// genres, years, the folder tree) built once at construction for fast
/// browsing — see below for which facts are safe to index this way and which
/// are not.
///
/// # Mutation categories
///
/// 1. **Full Reconstruction** ([`Library::new`]): builds the entire struct
///    from raw data, called after structural events (scan completed, library
///    imported, DB reload). Rebuilds every materialized index from scratch.
///
/// 2. **Incremental Mutations** ([`toggle_favorite`](Self::toggle_favorite),
///    [`record_play`](Self::record_play), `add_playlist`, etc.): each owns
///    exactly one behavioural fact and is the only place that fact changes.
///    Called optimistically from the reducer before DB confirmation.
///
/// 3. **Read-Only Queries** (`get_track`, `resolve_sorted_tracks_for_node`,
///    [`favorites`](Self::favorites), etc.): pure lookups, some of them
///    (favorites/most-played/recently-played) computed fresh from `tracks` on
///    every call rather than cached — see below.
///
/// # Structural indices vs. behavioural facts
///
/// `artists`/`albums`/`genres`/`years`/`folder_tree` are safe to materialize
/// this way because exactly one thing ever writes them — [`Library::new`] —
/// and nothing else in this codebase changes a track's artist, album, genre,
/// year, or path after it is scanned. There is no tag-editing feature; if one
/// is ever added, it must go through a single method the same way
/// [`record_play`](Self::record_play) does below, not by patching these maps
/// from outside.
///
/// `favorites`, `recently_played`, and `most_played` are deliberately **not**
/// struct fields, even though they used to be. Unlike the fields above, the
/// facts they're built from — `is_favorite`, `play_count`, `last_played_at` —
/// *do* change constantly during a session, every time a track is favourited
/// or finishes playing. A cached list needs something to keep it in sync with
/// those changes, and for a while nothing did: play count updated live, but
/// the separately-materialized `most_played`/`recently_played` lists were
/// only ever populated once, at library load, and stayed frozen for the rest
/// of the session. [`favorites`](Self::favorites), [`most_played`](Self::most_played),
/// and [`recently_played`](Self::recently_played) are plain queries over
/// `tracks` instead — there is no second copy of the answer to keep
/// consistent with the first.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub roots: Vec<PathBuf>,
    pub playlists: HashMap<PlaylistId, PlaylistMeta>,
    pub tracks: HashMap<TrackId, TrackMeta>,
    pub playlist_order: Vec<PlaylistId>,

    // Materialized indices — rebuilt only during full reconstruction. See the
    // struct docs above for why only these four (plus the folder tree) are
    // handled this way.
    pub folder_tree: DirectoryNode,
    pub artists: std::collections::BTreeMap<String, Vec<TrackId>>,
    pub albums: std::collections::BTreeMap<String, Vec<TrackId>>,
    pub genres: std::collections::BTreeMap<String, Vec<TrackId>>,
    pub years: std::collections::BTreeMap<u32, Vec<TrackId>>,
}

/// How many entries [`Library::most_played`] and [`Library::recently_played`]
/// return. Matches the `LIMIT 100` the database query used before these
/// became in-memory derivations, so browsing them looks the same as before.
const PLAY_STATS_LIMIT: usize = 100;

impl Library {
    // =========================================================================
    // Full Reconstruction — called after structural events (scan, import, reload)
    // =========================================================================

    /// Build a new Library from raw data, constructing all materialized indices.
    ///
    /// This is the **only** path that rebuilds folder_tree, artists, albums,
    /// genres, and years. Use for structural events only (scan completed,
    /// library imported, metadata refreshed).
    pub fn new(
        roots: Vec<PathBuf>,
        playlists_vec: Vec<PlaylistMeta>,
        tracks: HashMap<TrackId, TrackMeta>,
    ) -> Self {
        let mut playlists = HashMap::new();
        let mut playlist_order = Vec::new();

        for p in playlists_vec {
            playlist_order.push(p.id);
            playlists.insert(p.id, p);
        }

        // build indices
        let mut artists: std::collections::BTreeMap<String, Vec<TrackId>> =
            std::collections::BTreeMap::new();
        let mut albums: std::collections::BTreeMap<String, Vec<TrackId>> =
            std::collections::BTreeMap::new();
        let mut genres: std::collections::BTreeMap<String, Vec<TrackId>> =
            std::collections::BTreeMap::new();
        let mut years: std::collections::BTreeMap<u32, Vec<TrackId>> =
            std::collections::BTreeMap::new();

        // group tracks by various metadata for fast browsing
        for (id, meta) in &tracks {
            if let Some(a) = &meta.artist {
                artists.entry(a.clone()).or_default().push(*id);
            }
            if let Some(al) = &meta.album {
                albums.entry(al.clone()).or_default().push(*id);
            }
            if let Some(g) = &meta.genre {
                genres.entry(g.clone()).or_default().push(*id);
            }
            if let Some(y) = meta.year {
                years.entry(y).or_default().push(*id);
            }
        }

        // Stored order here is otherwise unused: every read goes through
        // `resolve_sorted_tracks_for_node`, which re-sorts by whatever the
        // user's current sort field is. Sorting by id just keeps iteration
        // deterministic for tests and debugging.
        for v in artists.values_mut() {
            v.sort();
        }
        for v in albums.values_mut() {
            v.sort();
        }
        for v in genres.values_mut() {
            v.sort();
        }
        for v in years.values_mut() {
            v.sort();
        }

        // build folder tree from multiple roots
        let folder_tree = Self::build_multi_root_folder_tree(&roots, &tracks);

        Self {
            roots,
            playlists,
            playlist_order,
            tracks,
            folder_tree,
            artists,
            albums,
            genres,
            years,
        }
    }

    // =========================================================================
    // Incremental Mutations — the single writer for each behavioural fact.
    // Called optimistically from the reducer before DB confirmation.
    // These NEVER rebuild folder_tree/artists/albums/genres/years.
    // =========================================================================

    /// Toggle a track's favorite status.
    ///
    /// This is the only place `is_favorite` changes. There is nothing else to
    /// update: [`favorites`](Self::favorites) reads this field directly, so
    /// there is no separate list that could drift from it.
    pub fn toggle_favorite(&mut self, track_id: TrackId, is_favorite: bool) {
        if let Some(track) = self.tracks.get_mut(&track_id) {
            track.is_favorite = is_favorite;
        }
    }

    /// Records that `track_id` finished playing at `played_at` (unix seconds).
    ///
    /// This is the only place `play_count` and `last_played_at` change. It
    /// used to be that the reducer incremented `play_count` directly on
    /// `tracks.get_mut(id)`, bypassing `Library` entirely — which is exactly
    /// how `most_played` ended up with no writer keeping it current: nothing
    /// obligated whoever changed `play_count` to also think about anything
    /// derived from it. Routing the change through one method removes the
    /// possibility of that gap reopening for these two fields, and pairs with
    /// [`most_played`](Self::most_played) / [`recently_played`](Self::recently_played)
    /// having no cached copy to remember to update in the first place.
    ///
    /// Returns `false` if the track is not in the library (e.g. it was
    /// removed by a rescan racing with a still-in-flight play), in which case
    /// nothing is recorded.
    pub fn record_play(&mut self, track_id: TrackId, played_at: u64) -> bool {
        let Some(track) = self.tracks.get_mut(&track_id) else {
            return false;
        };
        track.play_count += 1;
        track.last_played_at = Some(played_at);
        true
    }

    /// Tracks marked favorite, in a stable order (by title).
    ///
    /// A plain query over `tracks`, not a maintained list — see the struct
    /// docs. Callers that display this (the tree) re-sort it again by the
    /// user's chosen sort field regardless, the same as every other browsable
    /// category; the ordering here only has to be deterministic, not
    /// necessarily what ends up on screen.
    pub fn favorites(&self) -> Vec<TrackId> {
        let mut ids: Vec<TrackId> = self
            .tracks
            .values()
            .filter(|t| t.is_favorite)
            .map(|t| t.id)
            .collect();
        ids.sort_by(|a, b| {
            let ta = self.tracks.get(a).map(|t| t.title.as_str()).unwrap_or("");
            let tb = self.tracks.get(b).map(|t| t.title.as_str()).unwrap_or("");
            ta.cmp(tb)
        });
        ids
    }

    /// The [`PLAY_STATS_LIMIT`] most-played tracks, highest count first, ties
    /// broken by title so the result is deterministic.
    ///
    /// Reflects `play_count` as of *this call* — there is no staleness window
    /// like the old cached list had, because there is no cached list.
    pub fn most_played(&self) -> Vec<TrackId> {
        let mut entries: Vec<&TrackMeta> =
            self.tracks.values().filter(|t| t.play_count > 0).collect();
        entries.sort_by(|a, b| {
            b.play_count
                .cmp(&a.play_count)
                .then_with(|| a.title.cmp(&b.title))
        });
        entries.truncate(PLAY_STATS_LIMIT);
        entries.into_iter().map(|t| t.id).collect()
    }

    /// The [`PLAY_STATS_LIMIT`] most recently played tracks, most recent
    /// first, ties broken by title.
    pub fn recently_played(&self) -> Vec<TrackId> {
        let mut entries: Vec<&TrackMeta> = self
            .tracks
            .values()
            .filter(|t| t.last_played_at.is_some())
            .collect();
        entries.sort_by(|a, b| {
            b.last_played_at
                .cmp(&a.last_played_at)
                .then_with(|| a.title.cmp(&b.title))
        });
        entries.truncate(PLAY_STATS_LIMIT);
        entries.into_iter().map(|t| t.id).collect()
    }

    /// Add a playlist. Only touches `playlists` and `playlist_order`. O(1).
    pub fn add_playlist(&mut self, id: PlaylistId, name: String) {
        let meta = PlaylistMeta {
            id,
            name,
            tracks: Vec::new(),
        };
        self.playlist_order.push(id);
        self.playlists.insert(id, meta);
    }

    /// Remove a playlist. Only touches `playlists` and `playlist_order`. O(n).
    pub fn remove_playlist(&mut self, id: PlaylistId) {
        self.playlists.remove(&id);
        self.playlist_order.retain(|pid| *pid != id);
    }

    /// Add a track to a playlist. Only touches `playlist.tracks`. O(n) duplicate check.
    pub fn add_track_to_playlist(&mut self, playlist_id: PlaylistId, track_id: TrackId) {
        if let Some(p) = self
            .playlists
            .get_mut(&playlist_id)
            .filter(|p| !p.tracks.contains(&track_id))
        {
            p.tracks.push(track_id);
        }
    }

    /// Remove a track from a playlist. Only touches `playlist.tracks`. O(n).
    pub fn remove_track_from_playlist(&mut self, playlist_id: PlaylistId, track_id: TrackId) {
        if let Some(p) = self.playlists.get_mut(&playlist_id) {
            p.tracks.retain(|tid| *tid != track_id);
        }
    }

    // =========================================================================
    // Read-Only Queries — pure lookups, no mutations
    // =========================================================================

    /// Find playlist by name (case-insensitive).
    pub fn find_playlist_by_name(&self, name: &str) -> Option<PlaylistId> {
        let lower = name.to_lowercase();
        self.playlists
            .values()
            .find(|p| p.name.to_lowercase() == lower)
            .map(|p| p.id)
    }

    /// Build a folder tree that mounts each root as a child of a virtual "Library" node.
    fn build_multi_root_folder_tree(
        roots: &[PathBuf],
        tracks: &HashMap<TrackId, TrackMeta>,
    ) -> DirectoryNode {
        let mut library_node = DirectoryNode {
            name: "Library".to_string(),
            path: PathBuf::from("/"),
            subdirs: Vec::new(),
            tracks: Vec::new(),
        };

        for root in roots {
            let subtree = Self::build_folder_tree(root, tracks);
            library_node.subdirs.push(subtree);
        }

        // Sort top-level root entries
        library_node.subdirs.sort_by(|a, b| a.name.cmp(&b.name));

        library_node
    }

    fn build_folder_tree(root: &Path, tracks: &HashMap<TrackId, TrackMeta>) -> DirectoryNode {
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Library".to_string());

        let mut root_node = DirectoryNode {
            name,
            path: root.to_path_buf(),
            subdirs: Vec::new(),
            tracks: Vec::new(),
        };

        // group by parent folder
        let mut path_map: HashMap<PathBuf, Vec<TrackId>> = HashMap::new();
        for (id, meta) in tracks {
            // only include if under library root
            if let Some(parent) = meta.file_path.parent()
                && parent.starts_with(root)
            {
                path_map.entry(parent.to_path_buf()).or_default().push(*id);
            }
        }

        // merge folders into tree
        for (folder_path, mut tids) in path_map {
            // sort tracks
            tids.sort();

            // relative path components
            if let Ok(relative) = folder_path.strip_prefix(root) {
                let mut current_node = &mut root_node;
                let mut current_abs_path = root.to_path_buf();

                // walk tree, create nodes for each path component
                for component in relative.components() {
                    let component_str = component.as_os_str().to_string_lossy().to_string();
                    current_abs_path.push(&component_str);

                    // find or create child
                    let idx = if let Some(i) = current_node
                        .subdirs
                        .iter()
                        .position(|d| d.name == component_str)
                    {
                        i
                    } else {
                        let new_node = DirectoryNode {
                            name: component_str.clone(),
                            path: current_abs_path.clone(),
                            subdirs: Vec::new(),
                            tracks: Vec::new(),
                        };
                        current_node.subdirs.push(new_node);
                        current_node.subdirs.len() - 1
                    };

                    current_node = &mut current_node.subdirs[idx];
                }

                // leaf folder node
                current_node.tracks = tids;
            }
        }

        // sort subdirs
        Self::sort_tree_recursive(&mut root_node);

        root_node
    }

    fn sort_tree_recursive(node: &mut DirectoryNode) {
        node.subdirs.sort_by(|a, b| a.name.cmp(&b.name));
        for child in &mut node.subdirs {
            Self::sort_tree_recursive(child);
        }
    }

    /// Look up a track by ID.
    pub fn get_track(&self, id: TrackId) -> Option<&TrackMeta> {
        self.tracks.get(&id)
    }

    /// Look up a playlist by ID.
    pub fn get_playlist(&self, id: PlaylistId) -> Option<&PlaylistMeta> {
        self.playlists.get(&id)
    }

    /// Resolve a sidebar node to its list of tracks (by reference), in no
    /// particular order.
    ///
    /// Deliberately not `pub`: every external caller needs an *ordered*
    /// result, and the only ordering this codebase should ever hand out is
    /// the one [`resolve_sorted_tracks_for_node`](Self::resolve_sorted_tracks_for_node)
    /// produces. Making this private removes the possibility of a future
    /// caller reaching for the unsorted version by mistake, the way
    /// `resolve_track_ids_for_node` itself used to.
    fn resolve_tracks_for_node(&self, node: &SidebarNodeId) -> Vec<&TrackMeta> {
        match node {
            SidebarNodeId::Playlist(pid) => {
                if let Some(p) = self.playlists.get(pid) {
                    self.resolve_track_list(&p.tracks)
                } else {
                    vec![]
                }
            }
            SidebarNodeId::Artist(name) => {
                if let Some(ids) = self.artists.get(name) {
                    self.resolve_track_list(ids)
                } else {
                    vec![]
                }
            }
            SidebarNodeId::Album(name) => {
                if let Some(ids) = self.albums.get(name) {
                    self.resolve_track_list(ids)
                } else {
                    vec![]
                }
            }
            SidebarNodeId::Genre(name) => {
                if let Some(ids) = self.genres.get(name) {
                    self.resolve_track_list(ids)
                } else {
                    vec![]
                }
            }
            SidebarNodeId::Year(year) => {
                if let Some(ids) = self.years.get(year) {
                    self.resolve_track_list(ids)
                } else {
                    vec![]
                }
            }
            SidebarNodeId::Folder(path) => {
                // find node in folder tree
                if let Some(dir_node) = self.find_directory_node(path) {
                    // return tracks in folder
                    self.resolve_track_list(&dir_node.tracks)
                } else {
                    vec![]
                }
            }
            SidebarNodeId::Category(CategoryId::Favorites) => {
                self.resolve_track_list(&self.favorites())
            }
            SidebarNodeId::Category(CategoryId::RecentlyPlayed) => {
                self.resolve_track_list(&self.recently_played())
            }
            SidebarNodeId::Category(CategoryId::MostPlayed) => {
                self.resolve_track_list(&self.most_played())
            }
            SidebarNodeId::Category(_) => vec![],
            SidebarNodeId::Track { .. } => vec![],
            SidebarNodeId::Root => vec![],
        }
    }

    /// Track ids for a sidebar node, in the exact order the tree displays them.
    ///
    /// This has to go through [`resolve_sorted_tracks_for_node`] rather than
    /// reading the raw indices (`self.artists`, `self.albums`, ...) directly.
    /// Those are kept sorted by [`TrackId`] — insertion/scan order — purely so
    /// index rebuilds are cheap; the tree itself is always rendered through
    /// `resolve_sorted_tracks_for_node`, honouring the user's chosen sort field
    /// and direction. Building a playback context from the raw indices instead
    /// made Next/Prev walk a different order than the one on screen — picking
    /// the third track shown would jump to whatever had the third-lowest
    /// internal id, which could be anywhere in the visible list.
    ///
    /// Every consumer of "what should Next/Prev walk" must come through here,
    /// not through `resolve_tracks_for_node` — that name doesn't exist as a
    /// public id-returning method precisely so there is nothing shaped like the
    /// old trap left to reach for by mistake.
    pub fn resolve_track_ids_for_node(
        &self,
        node: &SidebarNodeId,
        sort_field: SortField,
        sort_direction: SortDirection,
    ) -> Vec<TrackId> {
        self.resolve_sorted_tracks_for_node(node, sort_field, sort_direction)
            .into_iter()
            .map(|t| t.id)
            .collect()
    }

    /// Shuffle-declustering group id for each of `ids`, in the same order.
    ///
    /// Groups by whichever metadata dimension actually varies across `ids` and
    /// would feel repetitive if two tracks sharing it played back to back:
    /// normally the artist, but the album once the whole list already shares
    /// one artist — so shuffling one artist's discography spreads across their
    /// albums instead of doing nothing (every track would share one group) or,
    /// worse, still grouping by artist and doing nothing there too.
    ///
    /// This reads the tracks themselves rather than taking the originating
    /// sidebar node, so it needs nothing but the id list: it is exactly as
    /// correct for a context rebuilt from a persisted session, where no node is
    /// available, as for one built live from a tree selection. It also copes
    /// better with the node's *label* lying about its content — an "Album"
    /// node that is actually a various-artists compilation still gets grouped
    /// by artist, because more than one artist is genuinely present.
    pub fn shuffle_group_keys(&self, ids: &[TrackId]) -> Vec<u32> {
        let mut distinct_artists: HashSet<Option<&str>> = HashSet::new();
        for id in ids {
            distinct_artists.insert(self.tracks.get(id).and_then(|t| t.artist.as_deref()));
            if distinct_artists.len() > 1 {
                break;
            }
        }

        if distinct_artists.len() > 1 {
            self.group_ids_by(ids, |t| t.artist.clone())
        } else {
            self.group_ids_by(ids, |t| t.album.clone())
        }
    }

    /// Assigns a small dense id to each distinct key `key_of` returns across
    /// `ids`, preserving `ids`' order. Tracks with no value for the dimension
    /// (e.g. a missing artist tag) share one group rather than each getting a
    /// spurious group of their own.
    fn group_ids_by(
        &self,
        ids: &[TrackId],
        key_of: impl Fn(&TrackMeta) -> Option<String>,
    ) -> Vec<u32> {
        let mut next_id: u32 = 0;
        let mut seen: HashMap<Option<String>, u32> = HashMap::new();
        ids.iter()
            .map(|id| {
                let key = self.tracks.get(id).and_then(&key_of);
                *seen.entry(key).or_insert_with(|| {
                    let g = next_id;
                    next_id += 1;
                    g
                })
            })
            .collect()
    }

    pub fn resolve_sorted_tracks_for_node(
        &self,
        node: &SidebarNodeId,
        sort_field: SortField,
        sort_direction: SortDirection,
    ) -> Vec<&TrackMeta> {
        let mut tracks = self.resolve_tracks_for_node(node);

        tracks.sort_by(|a, b| {
            let cmp = match sort_field {
                SortField::Title => a.title.cmp(&b.title),
                SortField::Artist => a.artist.cmp(&b.artist).then_with(|| a.title.cmp(&b.title)),
                SortField::Album => a
                    .album
                    .cmp(&b.album)
                    .then_with(|| a.track_number.cmp(&b.track_number)),
                SortField::Year => a.year.cmp(&b.year).then_with(|| a.title.cmp(&b.title)),
                SortField::Duration => a.duration_ms.cmp(&b.duration_ms),
            };
            if sort_direction == SortDirection::Descending {
                cmp.reverse()
            } else {
                cmp
            }
        });

        tracks
    }

    fn resolve_track_list(&self, ids: &[TrackId]) -> Vec<&TrackMeta> {
        ids.iter().filter_map(|tid| self.tracks.get(tid)).collect()
    }

    fn find_directory_node(&self, path: &Path) -> Option<&DirectoryNode> {
        // Check each root
        for (i, root) in self.roots.iter().enumerate() {
            if path == root {
                return self.folder_tree.subdirs.get(i);
            }
            if let Ok(relative) = path.strip_prefix(root) {
                let mut current = self.folder_tree.subdirs.get(i)?;
                for component in relative.components() {
                    let name = component.as_os_str().to_string_lossy();
                    if let Some(child) = current.subdirs.iter().find(|d| d.name == name) {
                        current = child;
                    } else {
                        return None;
                    }
                }
                return Some(current);
            }
        }
        None
    }

    pub fn get_album_art(path: &Path) -> Option<(Vec<u8>, String)> {
        let tagged_file = Probe::open(path).ok()?.read().ok()?;
        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag())?;
        let picture = tag.pictures().first()?;
        Some((picture.data().to_vec(), picture.mime_type()?.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_mock_track(
        id: u64,
        path: &str,
        artist: Option<&str>,
        album: Option<&str>,
        genre: Option<&str>,
        fav: bool,
    ) -> TrackMeta {
        TrackMeta {
            id: TrackId(id),
            file_path: PathBuf::from(path),
            title: format!("Track {}", id),
            artist: artist.map(|s| s.to_string()),
            album: album.map(|s| s.to_string()),
            genre: genre.map(|s| s.to_string()),
            year: Some(2020),
            track_number: None,
            play_count: 0,
            last_played_at: None,
            duration_ms: 1000,
            bitrate: None,
            sample_rate: None,
            format: "mp3".to_string(),
            has_cover: false,
            file_size_bytes: 1000,
            is_favorite: fav,
        }
    }

    #[test]
    fn test_build_folder_tree() {
        let mut tracks = HashMap::new();
        tracks.insert(
            TrackId(1),
            create_mock_track(1, "/library/rock/song1.mp3", None, None, None, false),
        );
        tracks.insert(
            TrackId(2),
            create_mock_track(2, "/library/rock/song2.mp3", None, None, None, false),
        );
        tracks.insert(
            TrackId(3),
            create_mock_track(3, "/library/jazz/song3.mp3", None, None, None, false),
        );

        let root = PathBuf::from("/library");
        let snapshot = Library::new(vec![root], vec![], tracks);

        // Virtual root "Library" contains one mounted root
        assert_eq!(snapshot.folder_tree.name, "Library");
        assert_eq!(snapshot.folder_tree.subdirs.len(), 1);

        // The mounted root "/library" contains jazz + rock
        let root_node = &snapshot.folder_tree.subdirs[0];
        assert_eq!(root_node.subdirs.len(), 2);

        let jazz_node = root_node.subdirs.iter().find(|d| d.name == "jazz").unwrap();
        assert_eq!(jazz_node.tracks.len(), 1);
        assert_eq!(jazz_node.tracks[0], TrackId(3));

        let rock_node = root_node.subdirs.iter().find(|d| d.name == "rock").unwrap();
        assert_eq!(rock_node.tracks.len(), 2);
    }

    #[test]
    fn test_toggle_favorite_sorting() {
        let mut tracks = HashMap::new();
        tracks.insert(
            TrackId(1),
            create_mock_track(1, "/lib/A.mp3", None, None, None, false),
        );
        tracks.insert(
            TrackId(2),
            create_mock_track(2, "/lib/B.mp3", None, None, None, false),
        );
        tracks.insert(
            TrackId(3),
            create_mock_track(3, "/lib/Z.mp3", None, None, None, false),
        );

        let mut snapshot = Library::new(vec![PathBuf::from("/lib")], vec![], tracks);

        snapshot.toggle_favorite(TrackId(2), true);
        snapshot.toggle_favorite(TrackId(3), true);
        snapshot.toggle_favorite(TrackId(1), true);

        // Should be sorted alphabetically by title: "Track 1", "Track 2", "Track 3"
        assert_eq!(
            snapshot.favorites(),
            vec![TrackId(1), TrackId(2), TrackId(3)]
        );

        snapshot.toggle_favorite(TrackId(2), false);
        assert_eq!(snapshot.favorites(), vec![TrackId(1), TrackId(3)]);
    }

    #[test]
    fn test_resolve_tracks_for_node() {
        let mut tracks = HashMap::new();
        tracks.insert(
            TrackId(1),
            create_mock_track(
                1,
                "/lib/A.mp3",
                Some("ArtistA"),
                Some("AlbumA"),
                Some("Jazz"),
                false,
            ),
        );
        tracks.insert(
            TrackId(2),
            create_mock_track(
                2,
                "/lib/B.mp3",
                Some("ArtistB"),
                Some("AlbumA"),
                Some("Rock"),
                true,
            ),
        );

        let snapshot = Library::new(vec![PathBuf::from("/lib")], vec![], tracks);

        // Test Artist Node
        let artist_node = SidebarNodeId::Artist("ArtistA".into());
        let resolved = snapshot.resolve_tracks_for_node(&artist_node);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].id, TrackId(1));

        // Test Album Node
        let album_node = SidebarNodeId::Album("AlbumA".into());
        let resolved = snapshot.resolve_tracks_for_node(&album_node);
        assert_eq!(resolved.len(), 2);

        // Test Favorites Node
        let fav_node = SidebarNodeId::Category(CategoryId::Favorites);
        let resolved = snapshot.resolve_tracks_for_node(&fav_node);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].id, TrackId(2));
    }

    #[test]
    fn resolve_track_ids_for_node_matches_the_tree_sort_order() {
        // Scanned as (and so id-ordered) Zebra, Apple, Mango; sorted by title
        // the tree shows Apple, Mango, Zebra. This is the exact shape that made
        // Next/Prev "jump around": building the context from the raw indices
        // walked the id order while the tree displayed the title order.
        fn titled(id: u64, title: &str) -> TrackMeta {
            TrackMeta {
                id: TrackId(id),
                file_path: PathBuf::from(format!("/lib/{title}.mp3")),
                title: title.into(),
                artist: Some("X".into()),
                album: None,
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
                file_size_bytes: 0,
            }
        }
        let mut tracks = HashMap::new();
        tracks.insert(TrackId(1), titled(1, "Zebra"));
        tracks.insert(TrackId(2), titled(2, "Apple"));
        tracks.insert(TrackId(3), titled(3, "Mango"));
        let snapshot = Library::new(vec![PathBuf::from("/lib")], vec![], tracks);
        let node = SidebarNodeId::Artist("X".into());

        let ids =
            snapshot.resolve_track_ids_for_node(&node, SortField::Title, SortDirection::Ascending);
        let titles: Vec<&str> = ids
            .iter()
            .map(|id| snapshot.get_track(*id).unwrap().title.as_str())
            .collect();
        assert_eq!(titles, vec!["Apple", "Mango", "Zebra"]);
    }

    #[test]
    fn shuffle_group_keys_groups_by_artist_when_more_than_one_is_present() {
        let mut tracks = HashMap::new();
        tracks.insert(
            TrackId(1),
            create_mock_track(1, "/lib/A.mp3", Some("Artist A"), None, None, false),
        );
        tracks.insert(
            TrackId(2),
            create_mock_track(2, "/lib/B.mp3", Some("Artist B"), None, None, false),
        );
        tracks.insert(
            TrackId(3),
            create_mock_track(3, "/lib/C.mp3", Some("Artist A"), None, None, false),
        );
        let snapshot = Library::new(vec![PathBuf::from("/lib")], vec![], tracks);

        let ids = vec![TrackId(1), TrackId(2), TrackId(3)];
        let groups = snapshot.shuffle_group_keys(&ids);
        assert_eq!(
            groups[0], groups[2],
            "the two Artist A tracks share a group"
        );
        assert_ne!(groups[0], groups[1], "Artist A and Artist B must differ");
    }

    #[test]
    fn shuffle_group_keys_falls_back_to_album_within_a_single_artist() {
        let mut tracks = HashMap::new();
        tracks.insert(
            TrackId(1),
            create_mock_track(
                1,
                "/lib/A.mp3",
                Some("Solo Artist"),
                Some("Album 1"),
                None,
                false,
            ),
        );
        tracks.insert(
            TrackId(2),
            create_mock_track(
                2,
                "/lib/B.mp3",
                Some("Solo Artist"),
                Some("Album 2"),
                None,
                false,
            ),
        );
        tracks.insert(
            TrackId(3),
            create_mock_track(
                3,
                "/lib/C.mp3",
                Some("Solo Artist"),
                Some("Album 1"),
                None,
                false,
            ),
        );
        let snapshot = Library::new(vec![PathBuf::from("/lib")], vec![], tracks);

        // One artist throughout: grouping by artist would be a single bucket
        // and do nothing, so it must fall through to album instead.
        let ids = vec![TrackId(1), TrackId(2), TrackId(3)];
        let groups = snapshot.shuffle_group_keys(&ids);
        assert_eq!(groups[0], groups[2], "the two Album 1 tracks share a group");
        assert_ne!(groups[0], groups[1], "Album 1 and Album 2 must differ");
    }

    #[test]
    fn shuffle_group_keys_degenerates_to_one_group_for_a_single_album() {
        let mut tracks = HashMap::new();
        tracks.insert(
            TrackId(1),
            create_mock_track(
                1,
                "/lib/A.mp3",
                Some("Solo Artist"),
                Some("The Album"),
                None,
                false,
            ),
        );
        tracks.insert(
            TrackId(2),
            create_mock_track(
                2,
                "/lib/B.mp3",
                Some("Solo Artist"),
                Some("The Album"),
                None,
                false,
            ),
        );
        let snapshot = Library::new(vec![PathBuf::from("/lib")], vec![], tracks);

        let ids = vec![TrackId(1), TrackId(2)];
        let groups = snapshot.shuffle_group_keys(&ids);
        assert_eq!(
            groups[0], groups[1],
            "one artist, one album: nothing to decluster by"
        );
    }

    #[test]
    fn test_playlist_management() {
        let mut snapshot = Library::new(vec![PathBuf::from("/lib")], vec![], HashMap::new());
        let p_id = PlaylistId(10);

        snapshot.add_playlist(p_id, "Workout".into());
        assert!(snapshot.playlists.contains_key(&p_id));

        snapshot.add_track_to_playlist(p_id, TrackId(1));
        assert_eq!(snapshot.playlists.get(&p_id).unwrap().tracks.len(), 1);

        // Prevent duplicates
        snapshot.add_track_to_playlist(p_id, TrackId(1));
        assert_eq!(snapshot.playlists.get(&p_id).unwrap().tracks.len(), 1);

        snapshot.remove_track_from_playlist(p_id, TrackId(1));
        assert_eq!(snapshot.playlists.get(&p_id).unwrap().tracks.len(), 0);

        snapshot.remove_playlist(p_id);
        assert!(!snapshot.playlists.contains_key(&p_id));
        assert!(!snapshot.playlist_order.contains(&p_id));
    }
}
