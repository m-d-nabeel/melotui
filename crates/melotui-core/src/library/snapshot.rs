use lofty::{file::TaggedFileExt, probe::Probe};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::domain::{PlaylistId, PlaylistMeta, TrackId, TrackMeta};
use crate::navigation::ids::{CategoryId, SidebarNodeId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryNode {
    pub name: String,
    pub path: PathBuf,
    pub subdirs: Vec<DirectoryNode>,
    pub tracks: Vec<TrackId>,
}

/// In-memory representation of the music library.
///
/// Contains the canonical track data plus **materialized indices** (artists, albums,
/// genres, years, folder tree, favorites) built at construction time for fast browsing.
///
/// # Mutation categories
///
/// Methods on `Library` fall into three categories:
///
/// 1. **Full Reconstruction** ([`Library::new`]): Builds the entire struct from raw data.
///    Called after structural events (scan completed, library imported, DB reload).
///    Rebuilds all indices from scratch.
///
/// 2. **Incremental Mutations** (`toggle_favorite`, `add_playlist`, etc.): Update only
///    the affected indices in O(1) amortized time. Called optimistically from the reducer
///    before DB confirmation. These do NOT rebuild the folder tree or artist/album/genre/year
///    indices because those are unaffected by playlist or favorite changes.
///
/// 3. **Read-Only Queries** (`get_track`, `resolve_tracks_for_node`, etc.): Pure lookups
///    against the current in-memory state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub roots: Vec<PathBuf>,
    pub playlists: HashMap<PlaylistId, PlaylistMeta>,
    pub tracks: HashMap<TrackId, TrackMeta>,
    pub playlist_order: Vec<PlaylistId>,

    // Materialized indices — rebuilt only during full reconstruction
    pub folder_tree: DirectoryNode,
    pub artists: std::collections::BTreeMap<String, Vec<TrackId>>,
    pub albums: std::collections::BTreeMap<String, Vec<TrackId>>,
    pub genres: std::collections::BTreeMap<String, Vec<TrackId>>,
    pub years: std::collections::BTreeMap<u32, Vec<TrackId>>,
    pub favorites: Vec<TrackId>,
}

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
        let mut favorites = vec![];

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
            if meta.is_favorite {
                favorites.push(*id);
            }
        }

        // sort indices
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
        favorites.sort_by(|a, b| {
            let title_a = tracks.get(a).map(|t| t.title.as_str()).unwrap_or("");
            let title_b = tracks.get(b).map(|t| t.title.as_str()).unwrap_or("");
            title_a.cmp(title_b)
        });

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
            favorites,
        }
    }

    // =========================================================================
    // Incremental Mutations — O(1) amortized, only touch affected indices.
    // Called optimistically from the reducer before DB confirmation.
    // These NEVER rebuild folder_tree/artists/albums/genres/years.
    // =========================================================================

    /// Toggle a track's favorite status. Only updates `tracks[id].is_favorite`
    /// and the `favorites` index. O(n) worst case due to sorted insert.
    pub fn toggle_favorite(&mut self, track_id: TrackId, is_favorite: bool) {
        if let Some(track) = self.tracks.get_mut(&track_id) {
            track.is_favorite = is_favorite;
        }

        if is_favorite {
            if !self.favorites.contains(&track_id) {
                self.favorites.push(track_id);

                let tracks_ref = &self.tracks;
                self.favorites.sort_by(|a, b| {
                    let title_a = tracks_ref.get(a).map(|t| t.title.as_str()).unwrap_or("");
                    let title_b = tracks_ref.get(b).map(|t| t.title.as_str()).unwrap_or("");
                    title_a.cmp(title_b)
                });
            }
        } else if let Some(idx) = self.favorites.iter().position(|&id| id == track_id) {
            self.favorites.remove(idx);
        }
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

    /// Resolve a sidebar node to its list of tracks (by reference).
    pub fn resolve_tracks_for_node(&self, node: &SidebarNodeId) -> Vec<&TrackMeta> {
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
                self.resolve_track_list(&self.favorites)
            }
            SidebarNodeId::Category(_) => vec![],
            SidebarNodeId::Track { .. } => vec![],
            SidebarNodeId::Root => vec![],
        }
    }

    pub fn resolve_track_ids_for_node(&self, node: &SidebarNodeId) -> Vec<TrackId> {
        match node {
            SidebarNodeId::Playlist(pid) => self
                .playlists
                .get(pid)
                .map(|p| p.tracks.clone())
                .unwrap_or_default(),
            SidebarNodeId::Artist(name) => self.artists.get(name).cloned().unwrap_or_default(),
            SidebarNodeId::Album(name) => self.albums.get(name).cloned().unwrap_or_default(),
            SidebarNodeId::Genre(name) => self.genres.get(name).cloned().unwrap_or_default(),
            SidebarNodeId::Year(year) => self.years.get(year).cloned().unwrap_or_default(),
            SidebarNodeId::Folder(path) => self
                .find_directory_node(path)
                .map(|d| d.tracks.clone())
                .unwrap_or_default(),
            SidebarNodeId::Category(CategoryId::Favorites) => self.favorites.clone(),
            SidebarNodeId::Category(_) => vec![],
            SidebarNodeId::Track { .. } => vec![],
            SidebarNodeId::Root => vec![],
        }
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
        assert_eq!(snapshot.favorites, vec![TrackId(1), TrackId(2), TrackId(3)]);

        snapshot.toggle_favorite(TrackId(2), false);
        assert_eq!(snapshot.favorites, vec![TrackId(1), TrackId(3)]);
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
