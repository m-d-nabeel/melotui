use lofty::{file::TaggedFileExt, probe::Probe};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::domain::{PlaylistId, PlaylistMeta, TrackId, TrackMeta};
use crate::navigation::ids::SidebarNodeId;

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

    // Indices for browsing
    pub folder_tree: DirectoryNode,
    pub artists: HashMap<String, Vec<TrackId>>,
    pub albums: HashMap<String, Vec<TrackId>>,
    pub genres: HashMap<String, Vec<TrackId>>,
    pub years: HashMap<u32, Vec<TrackId>>,
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

        // Build Indices
        let mut artists: HashMap<String, Vec<TrackId>> = HashMap::new();
        let mut albums: HashMap<String, Vec<TrackId>> = HashMap::new();
        let mut genres: HashMap<String, Vec<TrackId>> = HashMap::new();
        let mut years: HashMap<u32, Vec<TrackId>> = HashMap::new();

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

        // Sort indices for deterministic ordering
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

        // Build Folder Tree
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
        }
    }

    fn build_folder_tree(root: &Path, tracks: &HashMap<TrackId, TrackMeta>) -> DirectoryNode {
        let mut root_node = DirectoryNode {
            name: "Library".to_string(),
            path: root.to_path_buf(),
            subdirs: Vec::new(),
            tracks: Vec::new(),
        };

        // 1. Group tracks by their direct parent folder
        let mut path_map: HashMap<PathBuf, Vec<TrackId>> = HashMap::new();
        for (id, meta) in tracks {
            // If not top-level, add to parent
            if let Some(parent) = meta.file_path.parent()
                && parent.starts_with(root)
            {
                path_map.entry(parent.to_path_buf()).or_default().push(*id);
            }
        }

        // 2. Iterate all folders that contain tracks and merge into the tree
        for (folder_path, mut tids) in path_map {
            // Sort tracks by track number or filename (using ID for stability now)
            tids.sort();

            // Calculate relative path components
            if let Ok(relative) = folder_path.strip_prefix(root) {
                let mut current_node = &mut root_node;
                let mut current_abs_path = root.to_path_buf();

                // Walk down the tree, creating nodes as needed
                for component in relative.components() {
                    let component_str = component.as_os_str().to_string_lossy().to_string();
                    current_abs_path.push(&component_str);

                    // Find or create child
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

                // We remain at the leaf folder node
                current_node.tracks = tids;
            }
        }

        // 3. Sort subdirectories alphabetically for consistent UI
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
                // Traverse folder tree to find node
                if let Some(dir_node) = self.find_directory_node(path) {
                    // For now, return tracks in this specific folder
                    self.resolve_track_list(&dir_node.tracks)
                } else {
                    vec![]
                }
            }
            _ => vec![],
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
            _ => vec![],
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
