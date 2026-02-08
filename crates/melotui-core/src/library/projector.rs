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

/// visible row count
pub fn get_row_count(snapshot: &LibrarySnapshot, nav: &LibraryNavState) -> usize {
    let mut count = 0;
    traverse_visible_nodes(snapshot, nav, |_| {
        count += 1;
        false
    });
    count
}

/// get visible rows
pub fn get_visible_rows(
    snapshot: &LibrarySnapshot,
    nav: &LibraryNavState,
    skip: usize,
    take: usize,
) -> Vec<SidebarRow> {
    let mut rows = Vec::with_capacity(take);
    let mut current_idx = 0;

    traverse_visible_nodes(snapshot, nav, |row| {
        if current_idx >= skip {
            rows.push(row);
        }
        current_idx += 1;
        rows.len() >= take
    });

    rows
}

/// find node index
pub fn get_index_of(
    snapshot: &LibrarySnapshot,
    nav: &LibraryNavState,
    target: &SidebarNodeId,
) -> Option<usize> {
    let mut found_idx = None;
    let mut current_idx = 0;

    traverse_visible_nodes(snapshot, nav, |row| {
        if &row.id == target {
            found_idx = Some(current_idx);
            return true;
        }
        current_idx += 1;
        false
    });

    found_idx
}

/// get node at index
pub fn get_node_at_index(
    snapshot: &LibrarySnapshot,
    nav: &LibraryNavState,
    target_index: usize,
) -> Option<SidebarNodeId> {
    let mut found_node = None;
    let mut current_idx = 0;

    traverse_visible_nodes(snapshot, nav, |row| {
        if current_idx == target_index {
            found_node = Some(row.id.clone());
            return true;
        }
        current_idx += 1;
        false
    });

    found_node
}

/// traverse visible nodes
fn traverse_visible_nodes<F>(snapshot: &LibrarySnapshot, nav: &LibraryNavState, mut visitor: F)
where
    F: FnMut(SidebarRow) -> bool,
{
    let categories = [
        (CategoryId::Folders, "Folders"),
        (CategoryId::Favorites, "Favorites"),
        (CategoryId::Artists, "Artists"),
        (CategoryId::Albums, "Albums"),
        (CategoryId::Genres, "Genres"),
        (CategoryId::Years, "Years"),
        (CategoryId::Playlists, "Playlists"),
    ];

    for (cat, label) in categories {
        let node_id = SidebarNodeId::Category(cat);
        let expanded = nav.expanded_ids.contains(&node_id);

        if visitor(SidebarRow {
            id: node_id.clone(),
            label: label.to_string(),
            level: 0,
            is_expandable: true,
            is_expanded: expanded,
        }) {
            return;
        }

        if expanded {
            let stop = match cat {
                CategoryId::Folders => visit_directory(
                    &snapshot.folder_tree,
                    &nav.expanded_ids,
                    1,
                    snapshot,
                    &mut visitor,
                ),
                CategoryId::Favorites => {
                    visit_favorites(&snapshot.favorites, &node_id, 1, snapshot, &mut visitor)
                }
                CategoryId::Artists => visit_generic_map(
                    &snapshot.artists,
                    &nav.expanded_ids,
                    1,
                    SidebarNodeId::Artist,
                    snapshot,
                    &mut visitor,
                ),
                CategoryId::Albums => visit_generic_map(
                    &snapshot.albums,
                    &nav.expanded_ids,
                    1,
                    SidebarNodeId::Album,
                    snapshot,
                    &mut visitor,
                ),
                CategoryId::Genres => visit_generic_map(
                    &snapshot.genres,
                    &nav.expanded_ids,
                    1,
                    SidebarNodeId::Genre,
                    snapshot,
                    &mut visitor,
                ),
                CategoryId::Years => visit_generic_map(
                    &snapshot.years,
                    &nav.expanded_ids,
                    1,
                    SidebarNodeId::Year,
                    snapshot,
                    &mut visitor,
                ),
                CategoryId::Playlists => {
                    visit_playlists(snapshot, &nav.expanded_ids, 1, &mut visitor)
                }
            };

            if stop {
                return;
            }
        }
    }
}

fn visit_directory<F>(
    node: &DirectoryNode,
    expanded_ids: &HashSet<SidebarNodeId>,
    level: u8,
    snapshot: &LibrarySnapshot,
    visitor: &mut F,
) -> bool
where
    F: FnMut(SidebarRow) -> bool,
{
    for subdir in &node.subdirs {
        let id = SidebarNodeId::Folder(subdir.path.clone());
        let expanded = expanded_ids.contains(&id);
        let has_children = !subdir.subdirs.is_empty() || !subdir.tracks.is_empty();

        if visitor(SidebarRow {
            id: id.clone(),
            label: subdir.name.clone(),
            level,
            is_expandable: has_children,
            is_expanded: expanded,
        }) {
            return true;
        }

        if expanded && visit_directory(subdir, expanded_ids, level + 1, snapshot, visitor) {
            return true;
        }
    }

    for tid in &node.tracks {
        if let Some(track) = snapshot.tracks.get(tid) {
            let id = SidebarNodeId::Track {
                id: *tid,
                parent: Box::new(SidebarNodeId::Folder(node.path.clone())),
            };
            if visitor(SidebarRow {
                id,
                label: track.title.clone(),
                level: level + 1,
                is_expandable: false,
                is_expanded: false,
            }) {
                return true;
            }
        }
    }
    false
}

fn visit_favorites<F>(
    favorites: &[crate::domain::TrackId],
    parent_id: &SidebarNodeId,
    level: u8,
    snapshot: &LibrarySnapshot,
    visitor: &mut F,
) -> bool
where
    F: FnMut(SidebarRow) -> bool,
{
    for tid in favorites {
        if let Some(track) = snapshot.tracks.get(tid) {
            let id = SidebarNodeId::Track {
                id: *tid,
                parent: Box::new(parent_id.clone()),
            };
            if visitor(SidebarRow {
                id,
                label: track.title.clone(),
                level,
                is_expandable: false,
                is_expanded: false,
            }) {
                return true;
            }
        }
    }
    false
}

fn visit_playlists<F>(
    snapshot: &LibrarySnapshot,
    expanded_ids: &HashSet<SidebarNodeId>,
    level: u8,
    visitor: &mut F,
) -> bool
where
    F: FnMut(SidebarRow) -> bool,
{
    for pid in &snapshot.playlist_order {
        if let Some(playlist) = snapshot.playlists.get(pid) {
            let p_node = SidebarNodeId::Playlist(*pid);
            let expanded = expanded_ids.contains(&p_node);

            if visitor(SidebarRow {
                id: p_node.clone(),
                label: playlist.name.clone(),
                level,
                is_expandable: !playlist.tracks.is_empty(),
                is_expanded: expanded,
            }) {
                return true;
            }

            if expanded {
                for tid in &playlist.tracks {
                    if let Some(track) = snapshot.tracks.get(tid) {
                        let id = SidebarNodeId::Track {
                            id: *tid,
                            parent: Box::new(p_node.clone()),
                        };
                        if visitor(SidebarRow {
                            id,
                            label: track.title.clone(),
                            level: level + 1,
                            is_expandable: false,
                            is_expanded: false,
                        }) {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// generic map visitor
fn visit_generic_map<K, F, IDGen>(
    map: &std::collections::HashMap<K, Vec<crate::domain::TrackId>>,
    expanded_ids: &HashSet<SidebarNodeId>,
    level: u8,
    id_gen: IDGen,
    snapshot: &LibrarySnapshot,
    visitor: &mut F,
) -> bool
where
    K: std::hash::Hash + Eq + Ord + ToString + Clone,
    F: FnMut(SidebarRow) -> bool,
    IDGen: Fn(K) -> SidebarNodeId,
{
    let mut keys: Vec<&K> = map.keys().collect();
    keys.sort();

    for key in keys {
        let id = id_gen(key.clone());
        let expanded = expanded_ids.contains(&id);

        if visitor(SidebarRow {
            id: id.clone(),
            label: key.to_string(),
            level,
            is_expandable: true,
            is_expanded: expanded,
        }) {
            return true;
        }

        if expanded && let Some(tids) = map.get(key) {
            for tid in tids {
                if let Some(track) = snapshot.tracks.get(tid) {
                    let t_id = SidebarNodeId::Track {
                        id: *tid,
                        parent: Box::new(id.clone()),
                    };
                    if visitor(SidebarRow {
                        id: t_id,
                        label: track.title.clone(),
                        level: level + 1,
                        is_expandable: false,
                        is_expanded: false,
                    }) {
                        return true;
                    }
                }
            }
        }
    }
    false
}
