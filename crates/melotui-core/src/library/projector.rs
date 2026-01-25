use crate::library::snapshot::{DirectoryNode, LibrarySnapshot};
use crate::navigation::ids::{CategoryId, SidebarNodeId};
use crate::navigation::state::LibraryNavState;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub struct SidebarRow {
    pub id: SidebarNodeId,
    pub label: String,
    pub level: u8,
    pub is_expandable: bool,
    pub is_expanded: bool,
}

/// Calculate total number of visible rows in the tree.
pub fn get_row_count(snapshot: &LibrarySnapshot, nav: &LibraryNavState) -> usize {
    let mut count = 0;

    // Categories are hardcoded
    let categories = [
        CategoryId::Folders,
        CategoryId::Artists,
        CategoryId::Albums,
        CategoryId::Genres,
        CategoryId::Years,
        CategoryId::Playlists,
    ];

    for cat in categories {
        count += 1; // The category itself
        let cat_node = SidebarNodeId::Category(cat);
        if nav.expanded_ids.contains(&cat_node) {
            match cat {
                CategoryId::Folders => {
                    count += count_directory(&snapshot.folder_tree, &nav.expanded_ids)
                }
                CategoryId::Artists => count += count_artists(&snapshot.artists, &nav.expanded_ids),
                CategoryId::Albums => count += count_albums(&snapshot.albums, &nav.expanded_ids),
                CategoryId::Genres => count += count_genres(&snapshot.genres, &nav.expanded_ids),
                CategoryId::Years => count += count_years(&snapshot.years, &nav.expanded_ids),
                CategoryId::Playlists => count += count_playlists(snapshot, &nav.expanded_ids),
            }
        }
    }
    count
}

/// Get a slice of visible rows for the viewport.
pub fn get_visible_rows(
    snapshot: &LibrarySnapshot,
    nav: &LibraryNavState,
    skip: usize,
    take: usize,
) -> Vec<SidebarRow> {
    let mut rows = Vec::new();
    let mut current_idx = 0;

    let categories = [
        (CategoryId::Folders, "Folders"),
        (CategoryId::Artists, "Artists"),
        (CategoryId::Albums, "Albums"),
        (CategoryId::Genres, "Genres"),
        (CategoryId::Years, "Years"),
        (CategoryId::Playlists, "Playlists"),
    ];

    for (cat_id, label) in categories {
        let node_id = SidebarNodeId::Category(cat_id);
        let expanded = nav.expanded_ids.contains(&node_id);

        if current_idx >= skip && rows.len() < take {
            rows.push(SidebarRow {
                id: node_id.clone(),
                label: label.to_string(),
                level: 0,
                is_expandable: true,
                is_expanded: expanded,
            });
        }
        current_idx += 1;
        if rows.len() >= take {
            return rows;
        }

        if expanded {
            match cat_id {
                CategoryId::Folders => visit_directory(
                    &snapshot.folder_tree,
                    &nav.expanded_ids,
                    1,
                    &mut current_idx,
                    skip,
                    take,
                    &mut rows,
                    snapshot,
                ),
                CategoryId::Artists => visit_artists(
                    &snapshot.artists,
                    &nav.expanded_ids,
                    1,
                    &mut current_idx,
                    skip,
                    take,
                    &mut rows,
                    snapshot,
                ),
                CategoryId::Albums => visit_albums(
                    &snapshot.albums,
                    &nav.expanded_ids,
                    1,
                    &mut current_idx,
                    skip,
                    take,
                    &mut rows,
                    snapshot,
                ),
                CategoryId::Genres => visit_genres(
                    &snapshot.genres,
                    &nav.expanded_ids,
                    1,
                    &mut current_idx,
                    skip,
                    take,
                    &mut rows,
                    snapshot,
                ),
                CategoryId::Years => visit_years(
                    &snapshot.years,
                    &nav.expanded_ids,
                    1,
                    &mut current_idx,
                    skip,
                    take,
                    &mut rows,
                    snapshot,
                ),
                CategoryId::Playlists => visit_playlists(
                    snapshot,
                    &nav.expanded_ids,
                    1,
                    &mut current_idx,
                    skip,
                    take,
                    &mut rows,
                ),
            };
        }
        if rows.len() >= take {
            return rows;
        }
    }

    rows
}

/// Find the linear index of a specific node ID in the visible tree.
pub fn get_index_of(
    snapshot: &LibrarySnapshot,
    nav: &LibraryNavState,
    target: &SidebarNodeId,
) -> Option<usize> {
    let mut current_idx = 0;

    let categories = [
        CategoryId::Folders,
        CategoryId::Artists,
        CategoryId::Albums,
        CategoryId::Genres,
        CategoryId::Years,
        CategoryId::Playlists,
    ];

    for cat in categories {
        let node_id = SidebarNodeId::Category(cat);
        if &node_id == target {
            return Some(current_idx);
        }
        current_idx += 1;

        if nav.expanded_ids.contains(&node_id) {
            let found = match cat {
                CategoryId::Folders => search_directory(
                    &snapshot.folder_tree,
                    &nav.expanded_ids,
                    target,
                    &mut current_idx,
                ),
                CategoryId::Artists => search_artists(
                    &snapshot.artists,
                    &nav.expanded_ids,
                    target,
                    &mut current_idx,
                ),
                CategoryId::Albums => search_albums(
                    &snapshot.albums,
                    &nav.expanded_ids,
                    target,
                    &mut current_idx,
                ),
                CategoryId::Genres => search_genres(
                    &snapshot.genres,
                    &nav.expanded_ids,
                    target,
                    &mut current_idx,
                ),
                CategoryId::Years => {
                    search_years(&snapshot.years, &nav.expanded_ids, target, &mut current_idx)
                }
                CategoryId::Playlists => {
                    search_playlists(snapshot, &nav.expanded_ids, target, &mut current_idx)
                }
            };
            if found {
                return Some(current_idx);
            }
        }
    }
    None
}

/// Find the node ID at a specific linear index in the visible tree.
pub fn get_node_at_index(
    snapshot: &LibrarySnapshot,
    nav: &LibraryNavState,
    target_index: usize,
) -> Option<SidebarNodeId> {
    let mut current_idx = 0;

    let categories = [
        CategoryId::Folders,
        CategoryId::Artists,
        CategoryId::Albums,
        CategoryId::Genres,
        CategoryId::Years,
        CategoryId::Playlists,
    ];

    for cat in categories {
        let node_id = SidebarNodeId::Category(cat);
        if current_idx == target_index {
            return Some(node_id);
        }
        current_idx += 1;

        if nav.expanded_ids.contains(&node_id) {
            let found = match cat {
                CategoryId::Folders => find_at_index_directory(
                    &snapshot.folder_tree,
                    &nav.expanded_ids,
                    target_index,
                    &mut current_idx,
                ),
                CategoryId::Artists => find_at_index_artists(
                    &snapshot.artists,
                    &nav.expanded_ids,
                    target_index,
                    &mut current_idx,
                ),
                CategoryId::Albums => find_at_index_albums(
                    &snapshot.albums,
                    &nav.expanded_ids,
                    target_index,
                    &mut current_idx,
                ),
                CategoryId::Genres => find_at_index_genres(
                    &snapshot.genres,
                    &nav.expanded_ids,
                    target_index,
                    &mut current_idx,
                ),
                CategoryId::Years => find_at_index_years(
                    &snapshot.years,
                    &nav.expanded_ids,
                    target_index,
                    &mut current_idx,
                ),
                CategoryId::Playlists => find_at_index_playlists(
                    snapshot,
                    &nav.expanded_ids,
                    target_index,
                    &mut current_idx,
                ),
            };
            if let Some(node) = found {
                return Some(node);
            }
        }
    }
    None
}

// --- Counting Helpers ---

fn count_directory(node: &DirectoryNode, expanded: &HashSet<SidebarNodeId>) -> usize {
    let mut count = 0;
    // Subdirs
    for subdir in &node.subdirs {
        count += 1;
        let id = SidebarNodeId::Folder(subdir.path.clone());
        if expanded.contains(&id) {
            count += count_directory(subdir, expanded);
        }
    }
    // Tracks
    count += node.tracks.len();
    count
}

fn count_artists(
    map: &std::collections::HashMap<String, Vec<crate::domain::TrackId>>,
    expanded: &HashSet<SidebarNodeId>,
) -> usize {
    let mut count = 0;
    for (key, tracks) in map {
        count += 1;
        if expanded.contains(&SidebarNodeId::Artist(key.clone())) {
            count += tracks.len();
        }
    }
    count
}
fn count_albums(
    map: &std::collections::HashMap<String, Vec<crate::domain::TrackId>>,
    expanded: &HashSet<SidebarNodeId>,
) -> usize {
    let mut count = 0;
    for (key, tracks) in map {
        count += 1;
        if expanded.contains(&SidebarNodeId::Album(key.clone())) {
            count += tracks.len();
        }
    }
    count
}
fn count_genres(
    map: &std::collections::HashMap<String, Vec<crate::domain::TrackId>>,
    expanded: &HashSet<SidebarNodeId>,
) -> usize {
    let mut count = 0;
    for (key, tracks) in map {
        count += 1;
        if expanded.contains(&SidebarNodeId::Genre(key.clone())) {
            count += tracks.len();
        }
    }
    count
}
fn count_years(
    map: &std::collections::HashMap<u32, Vec<crate::domain::TrackId>>,
    expanded: &HashSet<SidebarNodeId>,
) -> usize {
    let mut count = 0;
    for (key, tracks) in map {
        count += 1;
        if expanded.contains(&SidebarNodeId::Year(*key)) {
            count += tracks.len();
        }
    }
    count
}

fn count_playlists(snapshot: &LibrarySnapshot, expanded: &HashSet<SidebarNodeId>) -> usize {
    let mut count = 0;
    for pid in &snapshot.playlist_order {
        count += 1;
        let id = SidebarNodeId::Playlist(*pid);
        if expanded.contains(&id)
            && let Some(p) = snapshot.playlists.get(pid)
        {
            count += p.tracks.len();
        }
    }
    count
}

// --- Visitor Helpers ---

#[allow(clippy::too_many_arguments)]
fn visit_directory(
    node: &DirectoryNode,
    expanded: &HashSet<SidebarNodeId>,
    level: u8,
    current_idx: &mut usize,
    skip: usize,
    take: usize,
    rows: &mut Vec<SidebarRow>,
    _snapshot: &LibrarySnapshot,
) {
    for subdir in &node.subdirs {
        let id = SidebarNodeId::Folder(subdir.path.clone());
        let is_exp = expanded.contains(&id);

        if *current_idx >= skip && rows.len() < take {
            rows.push(SidebarRow {
                id: id.clone(),
                label: subdir.name.clone(),
                level,
                is_expandable: !subdir.subdirs.is_empty() || !subdir.tracks.is_empty(),
                is_expanded: is_exp,
            });
        }
        *current_idx += 1;
        if rows.len() >= take {
            return;
        }

        if is_exp {
            visit_directory(
                subdir,
                expanded,
                level + 1,
                current_idx,
                skip,
                take,
                rows,
                _snapshot,
            );
            if rows.len() >= take {
                return;
            }
        }
    }

    // Visit Tracks in this directory
    // We access tracks by ID from snapshot because DirectoryNode lacks title logic
    for tid in &node.tracks {
        if let Some(track) = _snapshot.tracks.get(tid) {
            if *current_idx >= skip && rows.len() < take {
                rows.push(SidebarRow {
                    id: SidebarNodeId::Track {
                        id: *tid,
                        parent: Box::new(SidebarNodeId::Folder(node.path.clone())),
                    },
                    label: track.title.clone(),
                    level: level + 1,
                    is_expandable: false,
                    is_expanded: false,
                });
            }
            *current_idx += 1;
            if rows.len() >= take {
                return;
            }
        }
    }
}

fn visit_playlists(
    snapshot: &LibrarySnapshot,
    expanded: &HashSet<SidebarNodeId>,
    level: u8,
    current_idx: &mut usize,
    skip: usize,
    take: usize,
    rows: &mut Vec<SidebarRow>,
) {
    for pid in &snapshot.playlist_order {
        if let Some(playlist) = snapshot.playlists.get(pid) {
            let p_node = SidebarNodeId::Playlist(*pid);
            let is_exp = expanded.contains(&p_node);

            if *current_idx >= skip && rows.len() < take {
                rows.push(SidebarRow {
                    id: p_node.clone(),
                    label: playlist.name.clone(),
                    level,
                    is_expandable: !playlist.tracks.is_empty(),
                    is_expanded: is_exp,
                });
            }
            *current_idx += 1;
            if rows.len() >= take {
                return;
            }

            if is_exp {
                for tid in &playlist.tracks {
                    if let Some(track) = snapshot.tracks.get(tid) {
                        if *current_idx >= skip && rows.len() < take {
                            rows.push(SidebarRow {
                                id: SidebarNodeId::Track {
                                    id: *tid,
                                    parent: Box::new(p_node.clone()),
                                },
                                label: track.title.clone(),
                                level: level + 1,
                                is_expandable: false,
                                is_expanded: false,
                            });
                        }
                        *current_idx += 1;
                        if rows.len() >= take {
                            return;
                        }
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_artists(
    map: &std::collections::HashMap<String, Vec<crate::domain::TrackId>>,
    expanded: &HashSet<SidebarNodeId>,
    level: u8,
    current_idx: &mut usize,
    skip: usize,
    take: usize,
    rows: &mut Vec<SidebarRow>,
    snapshot: &LibrarySnapshot,
) {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();

    for key in keys {
        let label = key.clone();
        let node_id = SidebarNodeId::Artist(key.clone());
        let is_exp = expanded.contains(&node_id);

        if *current_idx >= skip && rows.len() < take {
            rows.push(SidebarRow {
                id: node_id.clone(),
                label,
                level,
                is_expandable: true,
                is_expanded: is_exp,
            });
        }
        *current_idx += 1;
        if rows.len() >= take {
            return;
        }

        if is_exp {
            visit_track_list(
                map.get(key),
                &node_id,
                level + 1,
                current_idx,
                skip,
                take,
                rows,
                snapshot,
            );
            if rows.len() >= take {
                return;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_albums(
    map: &std::collections::HashMap<String, Vec<crate::domain::TrackId>>,
    expanded: &HashSet<SidebarNodeId>,
    level: u8,
    current_idx: &mut usize,
    skip: usize,
    take: usize,
    rows: &mut Vec<SidebarRow>,
    snapshot: &LibrarySnapshot,
) {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();

    for key in keys {
        let label = key.clone();
        let node_id = SidebarNodeId::Album(key.clone());
        let is_exp = expanded.contains(&node_id);

        if *current_idx >= skip && rows.len() < take {
            rows.push(SidebarRow {
                id: node_id.clone(),
                label,
                level,
                is_expandable: true,
                is_expanded: is_exp,
            });
        }
        *current_idx += 1;
        if rows.len() >= take {
            return;
        }

        if is_exp {
            visit_track_list(
                map.get(key),
                &node_id,
                level + 1,
                current_idx,
                skip,
                take,
                rows,
                snapshot,
            );
            if rows.len() >= take {
                return;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_genres(
    map: &std::collections::HashMap<String, Vec<crate::domain::TrackId>>,
    expanded: &HashSet<SidebarNodeId>,
    level: u8,
    current_idx: &mut usize,
    skip: usize,
    take: usize,
    rows: &mut Vec<SidebarRow>,
    snapshot: &LibrarySnapshot,
) {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();

    for key in keys {
        let label = key.clone();
        let node_id = SidebarNodeId::Genre(key.clone());
        let is_exp = expanded.contains(&node_id);

        if *current_idx >= skip && rows.len() < take {
            rows.push(SidebarRow {
                id: node_id.clone(),
                label,
                level,
                is_expandable: true,
                is_expanded: is_exp,
            });
        }
        *current_idx += 1;
        if rows.len() >= take {
            return;
        }

        if is_exp {
            visit_track_list(
                map.get(key),
                &node_id,
                level + 1,
                current_idx,
                skip,
                take,
                rows,
                snapshot,
            );
            if rows.len() >= take {
                return;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_years(
    map: &std::collections::HashMap<u32, Vec<crate::domain::TrackId>>,
    expanded: &HashSet<SidebarNodeId>,
    level: u8,
    current_idx: &mut usize,
    skip: usize,
    take: usize,
    rows: &mut Vec<SidebarRow>,
    snapshot: &LibrarySnapshot,
) {
    let mut keys: Vec<&u32> = map.keys().collect();
    keys.sort();

    for key in keys {
        let label = key.to_string();
        let node_id = SidebarNodeId::Year(*key);
        let is_exp = expanded.contains(&node_id);

        if *current_idx >= skip && rows.len() < take {
            rows.push(SidebarRow {
                id: node_id.clone(),
                label,
                level,
                is_expandable: true,
                is_expanded: is_exp,
            });
        }
        *current_idx += 1;
        if rows.len() >= take {
            return;
        }

        if is_exp {
            visit_track_list(
                map.get(key),
                &node_id,
                level + 1,
                current_idx,
                skip,
                take,
                rows,
                snapshot,
            );
            if rows.len() >= take {
                return;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_track_list(
    tracks: Option<&Vec<crate::domain::TrackId>>,
    parent_id: &SidebarNodeId,
    level: u8,
    current_idx: &mut usize,
    skip: usize,
    take: usize,
    rows: &mut Vec<SidebarRow>,
    snapshot: &LibrarySnapshot,
) {
    if let Some(tids) = tracks {
        for tid in tids {
            if let Some(track) = snapshot.tracks.get(tid) {
                if *current_idx >= skip && rows.len() < take {
                    rows.push(SidebarRow {
                        id: SidebarNodeId::Track {
                            id: *tid,
                            parent: Box::new(parent_id.clone()),
                        },
                        label: track.title.clone(),
                        level,
                        is_expandable: false,
                        is_expanded: false,
                    });
                }
                *current_idx += 1;
                if rows.len() >= take {
                    return;
                }
            }
        }
    }
}

// --- Search Helpers ---

fn search_directory(
    node: &DirectoryNode,
    expanded: &HashSet<SidebarNodeId>,
    target: &SidebarNodeId,
    current_idx: &mut usize,
) -> bool {
    for subdir in &node.subdirs {
        let id = SidebarNodeId::Folder(subdir.path.clone());
        if &id == target {
            return true;
        }
        *current_idx += 1;

        if expanded.contains(&id) && search_directory(subdir, expanded, target, current_idx) {
            return true;
        }
    }

    // Check tracks in this directory
    for tid in &node.tracks {
        let t_node = SidebarNodeId::Track {
            id: *tid,
            parent: Box::new(SidebarNodeId::Folder(node.path.clone())),
        };
        if &t_node == target {
            return true;
        }
        *current_idx += 1;
    }

    false
}

fn search_playlists(
    snapshot: &LibrarySnapshot,
    expanded: &HashSet<SidebarNodeId>,
    target: &SidebarNodeId,
    current_idx: &mut usize,
) -> bool {
    for pid in &snapshot.playlist_order {
        let p_node = SidebarNodeId::Playlist(*pid);
        if &p_node == target {
            return true;
        }
        *current_idx += 1;

        if expanded.contains(&p_node)
            && let Some(playlist) = snapshot.playlists.get(pid)
        {
            for tid in &playlist.tracks {
                let t_node = SidebarNodeId::Track {
                    id: *tid,
                    parent: Box::new(p_node.clone()),
                };
                if &t_node == target {
                    return true;
                }
                *current_idx += 1;
            }
        }
    }
    false
}

fn search_artists(
    map: &std::collections::HashMap<String, Vec<crate::domain::TrackId>>,
    expanded: &HashSet<SidebarNodeId>,
    target: &SidebarNodeId,
    current_idx: &mut usize,
) -> bool {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();

    for key in keys {
        let node_id = SidebarNodeId::Artist(key.clone());
        if &node_id == target {
            return true;
        }
        *current_idx += 1;

        if expanded.contains(&node_id)
            && let Some(tids) = map.get(key)
        {
            for tid in tids {
                let t_node = SidebarNodeId::Track {
                    id: *tid,
                    parent: Box::new(node_id.clone()),
                };
                if &t_node == target {
                    return true;
                }
                *current_idx += 1;
            }
        }
    }
    false
}

fn search_albums(
    map: &std::collections::HashMap<String, Vec<crate::domain::TrackId>>,
    expanded: &HashSet<SidebarNodeId>,
    target: &SidebarNodeId,
    current_idx: &mut usize,
) -> bool {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();

    for key in keys {
        let node_id = SidebarNodeId::Album(key.clone());
        if &node_id == target {
            return true;
        }
        *current_idx += 1;

        if expanded.contains(&node_id)
            && let Some(tids) = map.get(key)
        {
            for tid in tids {
                let t_node = SidebarNodeId::Track {
                    id: *tid,
                    parent: Box::new(node_id.clone()),
                };
                if &t_node == target {
                    return true;
                }
                *current_idx += 1;
            }
        }
    }
    false
}

fn search_genres(
    map: &std::collections::HashMap<String, Vec<crate::domain::TrackId>>,
    expanded: &HashSet<SidebarNodeId>,
    target: &SidebarNodeId,
    current_idx: &mut usize,
) -> bool {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();

    for key in keys {
        let node_id = SidebarNodeId::Genre(key.clone());
        if &node_id == target {
            return true;
        }
        *current_idx += 1;

        if expanded.contains(&node_id)
            && let Some(tids) = map.get(key)
        {
            for tid in tids {
                let t_node = SidebarNodeId::Track {
                    id: *tid,
                    parent: Box::new(node_id.clone()),
                };
                if &t_node == target {
                    return true;
                }
                *current_idx += 1;
            }
        }
    }
    false
}

fn search_years(
    map: &std::collections::HashMap<u32, Vec<crate::domain::TrackId>>,
    expanded: &HashSet<SidebarNodeId>,
    target: &SidebarNodeId,
    current_idx: &mut usize,
) -> bool {
    let mut keys: Vec<&u32> = map.keys().collect();
    keys.sort();

    for key in keys {
        let node_id = SidebarNodeId::Year(*key);
        if &node_id == target {
            return true;
        }
        *current_idx += 1;

        if expanded.contains(&node_id)
            && let Some(tids) = map.get(key)
        {
            for tid in tids {
                let t_node = SidebarNodeId::Track {
                    id: *tid,
                    parent: Box::new(node_id.clone()),
                };
                if &t_node == target {
                    return true;
                }
                *current_idx += 1;
            }
        }
    }
    false
}

// --- Find By Index Helpers ---

fn find_at_index_directory(
    node: &DirectoryNode,
    expanded: &HashSet<SidebarNodeId>,
    target_index: usize,
    current_idx: &mut usize,
) -> Option<SidebarNodeId> {
    for subdir in &node.subdirs {
        let id = SidebarNodeId::Folder(subdir.path.clone());
        if *current_idx == target_index {
            return Some(id);
        }
        *current_idx += 1;

        if expanded.contains(&id)
            && let Some(res) = find_at_index_directory(subdir, expanded, target_index, current_idx)
        {
            return Some(res);
        }
    }

    // Check tracks in this directory
    for tid in &node.tracks {
        let t_node = SidebarNodeId::Track {
            id: *tid,
            parent: Box::new(SidebarNodeId::Folder(node.path.clone())),
        };
        if *current_idx == target_index {
            return Some(t_node);
        }
        *current_idx += 1;
    }

    None
}

fn find_at_index_playlists(
    snapshot: &LibrarySnapshot,
    expanded: &HashSet<SidebarNodeId>,
    target_index: usize,
    current_idx: &mut usize,
) -> Option<SidebarNodeId> {
    for pid in &snapshot.playlist_order {
        let p_node = SidebarNodeId::Playlist(*pid);
        if *current_idx == target_index {
            return Some(p_node);
        }
        *current_idx += 1;

        if expanded.contains(&p_node)
            && let Some(playlist) = snapshot.playlists.get(pid)
        {
            for tid in &playlist.tracks {
                let t_node = SidebarNodeId::Track {
                    id: *tid,
                    parent: Box::new(p_node.clone()),
                };
                if *current_idx == target_index {
                    return Some(t_node);
                }
                *current_idx += 1;
            }
        }
    }
    None
}

fn find_at_index_artists(
    map: &std::collections::HashMap<String, Vec<crate::domain::TrackId>>,
    expanded: &HashSet<SidebarNodeId>,
    target_index: usize,
    current_idx: &mut usize,
) -> Option<SidebarNodeId> {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();

    for key in keys {
        let node_id = SidebarNodeId::Artist(key.clone());
        if *current_idx == target_index {
            return Some(node_id);
        }
        *current_idx += 1;

        if expanded.contains(&node_id)
            && let Some(tids) = map.get(key)
        {
            for tid in tids {
                let t_node = SidebarNodeId::Track {
                    id: *tid,
                    parent: Box::new(node_id.clone()),
                };
                if *current_idx == target_index {
                    return Some(t_node);
                }
                *current_idx += 1;
            }
        }
    }
    None
}

fn find_at_index_albums(
    map: &std::collections::HashMap<String, Vec<crate::domain::TrackId>>,
    expanded: &HashSet<SidebarNodeId>,
    target_index: usize,
    current_idx: &mut usize,
) -> Option<SidebarNodeId> {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();

    for key in keys {
        let node_id = SidebarNodeId::Album(key.clone());
        if *current_idx == target_index {
            return Some(node_id);
        }
        *current_idx += 1;

        if expanded.contains(&node_id)
            && let Some(tids) = map.get(key)
        {
            for tid in tids {
                let t_node = SidebarNodeId::Track {
                    id: *tid,
                    parent: Box::new(node_id.clone()),
                };
                if *current_idx == target_index {
                    return Some(t_node);
                }
                *current_idx += 1;
            }
        }
    }
    None
}

fn find_at_index_genres(
    map: &std::collections::HashMap<String, Vec<crate::domain::TrackId>>,
    expanded: &HashSet<SidebarNodeId>,
    target_index: usize,
    current_idx: &mut usize,
) -> Option<SidebarNodeId> {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();

    for key in keys {
        let node_id = SidebarNodeId::Genre(key.clone());
        if *current_idx == target_index {
            return Some(node_id);
        }
        *current_idx += 1;

        if expanded.contains(&node_id)
            && let Some(tids) = map.get(key)
        {
            for tid in tids {
                let t_node = SidebarNodeId::Track {
                    id: *tid,
                    parent: Box::new(node_id.clone()),
                };
                if *current_idx == target_index {
                    return Some(t_node);
                }
                *current_idx += 1;
            }
        }
    }
    None
}

fn find_at_index_years(
    map: &std::collections::HashMap<u32, Vec<crate::domain::TrackId>>,
    expanded: &HashSet<SidebarNodeId>,
    target_index: usize,
    current_idx: &mut usize,
) -> Option<SidebarNodeId> {
    let mut keys: Vec<&u32> = map.keys().collect();
    keys.sort();

    for key in keys {
        let node_id = SidebarNodeId::Year(*key);
        if *current_idx == target_index {
            return Some(node_id);
        }
        *current_idx += 1;

        if expanded.contains(&node_id)
            && let Some(tids) = map.get(key)
        {
            for tid in tids {
                let t_node = SidebarNodeId::Track {
                    id: *tid,
                    parent: Box::new(node_id.clone()),
                };
                if *current_idx == target_index {
                    return Some(t_node);
                }
                *current_idx += 1;
            }
        }
    }
    None
}
