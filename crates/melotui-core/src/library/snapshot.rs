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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibrarySnapshot {
    pub root: PathBuf,
    pub playlists: HashMap<PlaylistId, PlaylistMeta>,
    pub tracks: HashMap<TrackId, TrackMeta>,
    pub playlist_order: Vec<PlaylistId>,

    // browsing indices
    pub folder_tree: DirectoryNode,
    pub artists: HashMap<String, Vec<TrackId>>,
    pub albums: HashMap<String, Vec<TrackId>>,
    pub genres: HashMap<String, Vec<TrackId>>,
    pub years: HashMap<u32, Vec<TrackId>>,
    pub favorites: Vec<TrackId>,
}

impl LibrarySnapshot {
    pub fn new(
        root: PathBuf,
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
        let mut artists: HashMap<String, Vec<TrackId>> = HashMap::new();
        let mut albums: HashMap<String, Vec<TrackId>> = HashMap::new();
        let mut genres: HashMap<String, Vec<TrackId>> = HashMap::new();
        let mut years: HashMap<u32, Vec<TrackId>> = HashMap::new();
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

        // build folder tree
        let folder_tree = Self::build_folder_tree(&root, &tracks);

        Self {
            root,
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

    pub fn add_playlist(&mut self, id: PlaylistId, name: String) {
        let meta = PlaylistMeta {
            id,
            name,
            tracks: Vec::new(),
        };
        self.playlist_order.push(id);
        self.playlists.insert(id, meta);
    }

    pub fn remove_playlist(&mut self, id: PlaylistId) {
        self.playlists.remove(&id);
        self.playlist_order.retain(|pid| *pid != id);
    }

    pub fn add_track_to_playlist(&mut self, playlist_id: PlaylistId, track_id: TrackId) {
        if let Some(p) = self
            .playlists
            .get_mut(&playlist_id)
            .filter(|p| !p.tracks.contains(&track_id))
        {
            p.tracks.push(track_id);
        }
    }

    pub fn remove_track_from_playlist(&mut self, playlist_id: PlaylistId, track_id: TrackId) {
        if let Some(p) = self.playlists.get_mut(&playlist_id) {
            p.tracks.retain(|tid| *tid != track_id);
        }
    }

    /// Find playlist by name (case-insensitive)
    pub fn find_playlist_by_name(&self, name: &str) -> Option<PlaylistId> {
        let lower = name.to_lowercase();
        self.playlists
            .values()
            .find(|p| p.name.to_lowercase() == lower)
            .map(|p| p.id)
    }

    fn build_folder_tree(root: &Path, tracks: &HashMap<TrackId, TrackMeta>) -> DirectoryNode {
        let mut root_node = DirectoryNode {
            name: "Library".to_string(),
            path: root.to_path_buf(),
            subdirs: Vec::new(),
            tracks: Vec::new(),
        };

        // 1. group by parent folder
        let mut path_map: HashMap<PathBuf, Vec<TrackId>> = HashMap::new();
        for (id, meta) in tracks {
            // only include if under library root
            if let Some(parent) = meta.file_path.parent()
                && parent.starts_with(root)
            {
                path_map.entry(parent.to_path_buf()).or_default().push(*id);
            }
        }

        // 2. merge folders into tree
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

        // 3. sort subdirs
        Self::sort_tree_recursive(&mut root_node);

        root_node
    }

    fn sort_tree_recursive(node: &mut DirectoryNode) {
        node.subdirs.sort_by(|a, b| a.name.cmp(&b.name));
        for child in &mut node.subdirs {
            Self::sort_tree_recursive(child);
        }
    }

    pub fn get_track(&self, id: TrackId) -> Option<&TrackMeta> {
        self.tracks.get(&id)
    }

    pub fn get_playlist(&self, id: PlaylistId) -> Option<&PlaylistMeta> {
        self.playlists.get(&id)
    }

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
        if path == self.root {
            return Some(&self.folder_tree);
        }

        if let Ok(relative) = path.strip_prefix(&self.root) {
            let mut current = &self.folder_tree;
            for component in relative.components() {
                let name = component.as_os_str().to_string_lossy();
                if let Some(child) = current.subdirs.iter().find(|d| d.name == name) {
                    current = child;
                } else {
                    return None;
                }
            }
            Some(current)
        } else {
            None
        }
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
