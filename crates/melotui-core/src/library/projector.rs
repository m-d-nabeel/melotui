use crate::library::snapshot::{DirectoryNode, Library};
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
pub fn get_row_count(snapshot: &Library, nav: &LibraryNavState) -> usize {
    let mut count = 9; // Categories: RecentlyPlayed, MostPlayed, Folders, Favorites, Artists, Albums, Genres, Years, Playlists

    if nav
        .expanded_ids
        .contains(&SidebarNodeId::Category(CategoryId::Folders))
    {
        count += count_directory(&snapshot.folder_tree, &nav.expanded_ids);
    }
    if nav
        .expanded_ids
        .contains(&SidebarNodeId::Category(CategoryId::Favorites))
    {
        count += snapshot.favorites.len();
    }
    if nav
        .expanded_ids
        .contains(&SidebarNodeId::Category(CategoryId::RecentlyPlayed))
    {
        count += snapshot.recently_played.len();
    }
    if nav
        .expanded_ids
        .contains(&SidebarNodeId::Category(CategoryId::MostPlayed))
    {
        count += snapshot.most_played.len();
    }
    if nav
        .expanded_ids
        .contains(&SidebarNodeId::Category(CategoryId::Artists))
    {
        count += count_generic_map(&snapshot.artists, &nav.expanded_ids, SidebarNodeId::Artist);
    }
    if nav
        .expanded_ids
        .contains(&SidebarNodeId::Category(CategoryId::Albums))
    {
        count += count_generic_map(&snapshot.albums, &nav.expanded_ids, SidebarNodeId::Album);
    }
    if nav
        .expanded_ids
        .contains(&SidebarNodeId::Category(CategoryId::Genres))
    {
        count += count_generic_map(&snapshot.genres, &nav.expanded_ids, SidebarNodeId::Genre);
    }
    if nav
        .expanded_ids
        .contains(&SidebarNodeId::Category(CategoryId::Years))
    {
        count += count_generic_map(&snapshot.years, &nav.expanded_ids, SidebarNodeId::Year);
    }
    if nav
        .expanded_ids
        .contains(&SidebarNodeId::Category(CategoryId::Playlists))
    {
        for pid in &snapshot.playlist_order {
            if let Some(playlist) = snapshot.playlists.get(pid) {
                count += 1;
                if nav.expanded_ids.contains(&SidebarNodeId::Playlist(*pid)) {
                    count += playlist.tracks.len();
                }
            }
        }
    }

    count
}

fn count_directory(node: &DirectoryNode, expanded_ids: &HashSet<SidebarNodeId>) -> usize {
    let mut count = node.subdirs.len() + node.tracks.len();
    for subdir in &node.subdirs {
        let id = SidebarNodeId::Folder(subdir.path.clone());
        if expanded_ids.contains(&id) {
            count += count_directory(subdir, expanded_ids);
        }
    }
    count
}

fn count_generic_map<K, IDGen>(
    map: &std::collections::BTreeMap<K, Vec<crate::domain::TrackId>>,
    expanded_ids: &HashSet<SidebarNodeId>,
    id_gen: IDGen,
) -> usize
where
    K: std::hash::Hash + Eq + Clone,
    IDGen: Fn(K) -> SidebarNodeId,
{
    let mut count = map.len();
    for (key, tids) in map {
        let id = id_gen(key.clone());
        if expanded_ids.contains(&id) {
            count += tids.len();
        }
    }
    count
}

/// get visible rows
pub fn get_visible_rows(
    snapshot: &Library,
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
    snapshot: &Library,
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
    snapshot: &Library,
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
fn traverse_visible_nodes<F>(snapshot: &Library, nav: &LibraryNavState, mut visitor: F)
where
    F: FnMut(SidebarRow) -> bool,
{
    let categories = [
        (CategoryId::RecentlyPlayed, "Recently Played"),
        (CategoryId::MostPlayed, "Most Played"),
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
                CategoryId::Folders => {
                    visit_directory(&snapshot.folder_tree, nav, 1, snapshot, &mut visitor)
                }
                CategoryId::Favorites => visit_favorites(&node_id, nav, 1, snapshot, &mut visitor),
                CategoryId::RecentlyPlayed => {
                    visit_favorites(&node_id, nav, 1, snapshot, &mut visitor)
                }
                CategoryId::MostPlayed => visit_favorites(&node_id, nav, 1, snapshot, &mut visitor),
                CategoryId::Artists => visit_generic_map(
                    &snapshot.artists,
                    nav,
                    1,
                    SidebarNodeId::Artist,
                    snapshot,
                    &mut visitor,
                ),
                CategoryId::Albums => visit_generic_map(
                    &snapshot.albums,
                    nav,
                    1,
                    SidebarNodeId::Album,
                    snapshot,
                    &mut visitor,
                ),
                CategoryId::Genres => visit_generic_map(
                    &snapshot.genres,
                    nav,
                    1,
                    SidebarNodeId::Genre,
                    snapshot,
                    &mut visitor,
                ),
                CategoryId::Years => visit_generic_map(
                    &snapshot.years,
                    nav,
                    1,
                    SidebarNodeId::Year,
                    snapshot,
                    &mut visitor,
                ),
                CategoryId::Playlists => visit_playlists(snapshot, nav, 1, &mut visitor),
            };

            if stop {
                return;
            }
        }
    }
}

fn visit_directory<F>(
    node: &DirectoryNode,
    nav: &LibraryNavState,
    level: u8,
    snapshot: &Library,
    visitor: &mut F,
) -> bool
where
    F: FnMut(SidebarRow) -> bool,
{
    for subdir in &node.subdirs {
        let id = SidebarNodeId::Folder(subdir.path.clone());
        let expanded = nav.expanded_ids.contains(&id);
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

        if expanded && visit_directory(subdir, nav, level + 1, snapshot, visitor) {
            return true;
        }
    }

    let parent_node = SidebarNodeId::Folder(node.path.clone());
    let sorted_tracks =
        snapshot.resolve_sorted_tracks_for_node(&parent_node, nav.sort_field, nav.sort_direction);

    for track in sorted_tracks {
        let id = SidebarNodeId::Track {
            id: track.id,
            parent: Box::new(parent_node.clone()),
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
    false
}

fn visit_favorites<F>(
    parent_id: &SidebarNodeId,
    nav: &LibraryNavState,
    level: u8,
    snapshot: &Library,
    visitor: &mut F,
) -> bool
where
    F: FnMut(SidebarRow) -> bool,
{
    let sorted_tracks =
        snapshot.resolve_sorted_tracks_for_node(parent_id, nav.sort_field, nav.sort_direction);

    for track in sorted_tracks {
        let id = SidebarNodeId::Track {
            id: track.id,
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
    false
}

fn visit_playlists<F>(snapshot: &Library, nav: &LibraryNavState, level: u8, visitor: &mut F) -> bool
where
    F: FnMut(SidebarRow) -> bool,
{
    for pid in &snapshot.playlist_order {
        if let Some(playlist) = snapshot.playlists.get(pid) {
            let p_node = SidebarNodeId::Playlist(*pid);
            let expanded = nav.expanded_ids.contains(&p_node);

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
                let sorted_tracks = snapshot.resolve_sorted_tracks_for_node(
                    &p_node,
                    nav.sort_field,
                    nav.sort_direction,
                );
                for track in sorted_tracks {
                    let id = SidebarNodeId::Track {
                        id: track.id,
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
    false
}

/// generic map visitor
fn visit_generic_map<K, F, IDGen>(
    map: &std::collections::BTreeMap<K, Vec<crate::domain::TrackId>>,
    nav: &LibraryNavState,
    level: u8,
    id_gen: IDGen,
    snapshot: &Library,
    visitor: &mut F,
) -> bool
where
    K: std::hash::Hash + Eq + Ord + ToString + Clone,
    F: FnMut(SidebarRow) -> bool,
    IDGen: Fn(K) -> SidebarNodeId,
{
    for key in map.keys() {
        let id = id_gen(key.clone());
        let expanded = nav.expanded_ids.contains(&id);

        if visitor(SidebarRow {
            id: id.clone(),
            label: key.to_string(),
            level,
            is_expandable: true,
            is_expanded: expanded,
        }) {
            return true;
        }

        if expanded {
            let sorted_tracks =
                snapshot.resolve_sorted_tracks_for_node(&id, nav.sort_field, nav.sort_direction);
            for track in sorted_tracks {
                let t_id = SidebarNodeId::Track {
                    id: track.id,
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
    false
}
