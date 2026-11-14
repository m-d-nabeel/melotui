use crate::library::snapshot::{DirectoryNode, Library};
use crate::navigation::ids::{CategoryId, SidebarNodeId};
use crate::navigation::state::LibraryNavState;

#[derive(Debug, Clone, PartialEq)]
pub struct SidebarRow {
    pub id: SidebarNodeId,
    pub label: String,
    pub level: u8,
    pub is_expandable: bool,
    pub is_expanded: bool,
}

/// What the laid-out tree depends on besides the library: which nodes are
/// open, and how tracks are sorted.
fn layout_key(nav: &LibraryNavState) -> u64 {
    use std::hash::{Hash, Hasher};
    let hash = |value: &dyn Fn(&mut std::collections::hash_map::DefaultHasher)| {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        value(&mut hasher);
        hasher.finish()
    };
    // The open nodes in whatever order the set gives them: summed, so the
    // order does not matter.
    let open = nav
        .expanded_ids
        .iter()
        .map(|id| hash(&|h| id.hash(h)))
        .fold(0u64, u64::wrapping_add);
    hash(&|h| {
        open.hash(h);
        nav.expanded_ids.len().hash(h);
        (nav.sort_field as u8).hash(h);
        (nav.sort_direction as u8).hash(h);
    })
}

/// Every row of the tree as it stands, top to bottom: laid out once, and
/// kept until the library, the open nodes or the sorting change.
fn rows(snapshot: &Library, nav: &LibraryNavState) -> std::sync::Arc<Vec<SidebarRow>> {
    let key = layout_key(nav);
    if let Some(rows) = snapshot.projection.get(key) {
        return rows;
    }
    let mut rows = Vec::new();
    traverse_visible_nodes(snapshot, nav, |row| {
        rows.push(row);
        false
    });
    let rows = std::sync::Arc::new(rows);
    snapshot.projection.put(key, rows.clone());
    rows
}

/// visible row count
pub fn get_row_count(snapshot: &Library, nav: &LibraryNavState) -> usize {
    rows(snapshot, nav).len()
}

pub fn get_visible_rows(
    snapshot: &Library,
    nav: &LibraryNavState,
    skip: usize,
    take: usize,
) -> Vec<SidebarRow> {
    rows(snapshot, nav)
        .iter()
        .skip(skip)
        .take(take)
        .cloned()
        .collect()
}

/// find node index
pub fn get_index_of(
    snapshot: &Library,
    nav: &LibraryNavState,
    target: &SidebarNodeId,
) -> Option<usize> {
    rows(snapshot, nav).iter().position(|row| &row.id == target)
}

/// get node at index
pub fn get_node_at_index(
    snapshot: &Library,
    nav: &LibraryNavState,
    target_index: usize,
) -> Option<SidebarNodeId> {
    rows(snapshot, nav)
        .get(target_index)
        .map(|row| row.id.clone())
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

#[cfg(test)]
mod layout_tests {
    use super::*;
    use crate::domain::{PlaylistId, PlaylistMeta, TrackId, TrackMeta};
    use crate::navigation::state::{SortDirection, SortField};
    use std::collections::HashMap;

    fn library() -> Library {
        let tracks: HashMap<TrackId, TrackMeta> = (1..=6u64)
            .map(|n| {
                let id = TrackId(n);
                let track = TrackMeta {
                    id,
                    file_path: format!("/music/album {}/song {n}.mp3", n % 2).into(),
                    title: format!("Song {}", 7 - n),
                    artist: Some(format!("Artist {}", n % 3)),
                    album: Some(format!("Album {}", n % 2)),
                    genre: None,
                    year: Some(2000 + n as u32),
                    is_favorite: n == 1,
                    track_number: Some(n as u32),
                    play_count: 0,
                    last_played_at: None,
                    duration_ms: 1000 * n,
                    bitrate: None,
                    sample_rate: None,
                    format: "mp3".into(),
                    has_cover: false,
                    file_size_bytes: 1,
                };
                (id, track)
            })
            .collect();
        let playlist = PlaylistMeta {
            id: PlaylistId(1),
            name: "Mix".into(),
            tracks: vec![TrackId(2)],
        };
        Library::new(vec!["/music".into()], vec![playlist], tracks)
    }

    /// Everything open, so that every kind of row is laid out.
    fn all_open(library: &Library) -> LibraryNavState {
        let mut nav = LibraryNavState::default();
        // Opening shows more to open: until nothing new appears.
        loop {
            let before = nav.expanded_ids.len();
            let mut found = Vec::new();
            traverse_visible_nodes(library, &nav, |row| {
                if row.is_expandable {
                    found.push(row.id);
                }
                false
            });
            nav.expanded_ids.extend(found);
            if nav.expanded_ids.len() == before {
                return nav;
            }
        }
    }

    /// The tree laid out afresh, whatever is kept.
    fn afresh(library: &Library, nav: &LibraryNavState) -> Vec<SidebarRow> {
        let mut rows = Vec::new();
        traverse_visible_nodes(library, nav, |row| {
            rows.push(row);
            false
        });
        rows
    }

    /// That what the tree's readers are given is the tree as it stands.
    fn check(library: &Library, nav: &LibraryNavState, after: &str) {
        let now = afresh(library, nav);
        assert_eq!(
            get_row_count(library, nav),
            now.len(),
            "count after {after}"
        );
        assert_eq!(
            get_visible_rows(library, nav, 0, usize::MAX),
            now,
            "rows after {after}"
        );
        for (index, row) in now.iter().enumerate().step_by(3) {
            assert_eq!(get_index_of(library, nav, &row.id), Some(index), "{after}");
            assert_eq!(
                get_node_at_index(library, nav, index).as_ref(),
                Some(&row.id),
                "{after}"
            );
        }
        assert_eq!(get_node_at_index(library, nav, now.len()), None);
    }

    #[test]
    fn the_tree_kept_is_the_tree_as_it_stands_after_every_kind_of_change() {
        let mut library = library();
        let mut nav = all_open(&library);
        check(&library, &nav, "nothing");
        let rows = get_row_count(&library, &nav);
        assert!(rows > 30, "only {rows} rows: not everything was opened");

        // Each way the library changes.
        library.toggle_favorite(TrackId(3), true);
        check(&library, &nav, "a favorite made");
        library.toggle_favorite(TrackId(1), false);
        check(&library, &nav, "a favorite unmade");
        library.record_play(TrackId(4), 100);
        check(&library, &nav, "a play");
        library.record_play(TrackId(5), 200);
        check(&library, &nav, "another play");
        library.add_playlist(PlaylistId(2), "Another".into());
        nav.expanded_ids
            .insert(SidebarNodeId::Playlist(PlaylistId(2)));
        check(&library, &nav, "a playlist made");
        library.add_track_to_playlist(PlaylistId(2), TrackId(6));
        check(&library, &nav, "a track added to a playlist");
        library.remove_track_from_playlist(PlaylistId(1), TrackId(2));
        check(&library, &nav, "a track taken from a playlist");
        library.remove_playlist(PlaylistId(1));
        check(&library, &nav, "a playlist deleted");

        // Each way the view of it changes.
        for field in [
            SortField::Artist,
            SortField::Album,
            SortField::Year,
            SortField::Duration,
            SortField::Title,
        ] {
            nav.sort_field = field;
            check(&library, &nav, "a change of sort field");
        }
        nav.sort_direction = SortDirection::Descending;
        check(&library, &nav, "a change of direction");
        nav.expanded_ids.remove(&SidebarNodeId::Category(
            crate::navigation::ids::CategoryId::Artists,
        ));
        check(&library, &nav, "a node closed");
        nav.expanded_ids.insert(SidebarNodeId::Category(
            crate::navigation::ids::CategoryId::Artists,
        ));
        check(&library, &nav, "a node opened");

        // And a library that takes this one's place starts with nothing kept.
        let reloaded = library.clone();
        check(&reloaded, &LibraryNavState::default(), "a reload");
    }

    #[test]
    fn the_tree_is_laid_out_once_until_something_changes() {
        let library = library();
        let nav = all_open(&library);
        let first = rows(&library, &nav);
        let again = rows(&library, &nav);
        assert!(std::sync::Arc::ptr_eq(&first, &again), "laid out again");

        // Moving the selection or scrolling is not a change to the layout.
        let mut moved = nav.clone();
        moved.scroll_offset = 9;
        moved.selected_id = Some(SidebarNodeId::Root);
        assert!(std::sync::Arc::ptr_eq(&first, &rows(&library, &moved)));
    }
}
