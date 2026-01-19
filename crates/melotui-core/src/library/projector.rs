use crate::library::snapshot::LibrarySnapshot;
use crate::navigation::ids::{CategoryId, SidebarNodeId};
use crate::navigation::state::LibraryNavState;

#[derive(Debug, Clone, PartialEq)]
pub struct SidebarRow {
    pub id: SidebarNodeId,
    /// Visual text to display (e.g. "  ▼ Playlists")
    pub label: String,
    /// Indentation level (0, 1, 2...)
    pub level: u8,
    /// Can this node be expanded? (Shows ▶ / ▼)
    pub is_expandable: bool,
    /// Is it currently expanded?
    pub is_expanded: bool,
}

/// The "Projection Engine".
/// Converts the tree structure into a flat list of rows for rendering.
pub fn flatten_tree(snapshot: &LibrarySnapshot, nav: &LibraryNavState) -> Vec<SidebarRow> {
    let mut rows = Vec::new();

    // 1. Define Root Categories
    // These are the top-level items that always exist.
    let categories = [
        (CategoryId::Folders, "Folders"),
        (CategoryId::AllSongs, "All Songs"),
        (CategoryId::Artists, "Artists"),
        (CategoryId::Albums, "Albums"),
        (CategoryId::Playlists, "Playlists"),
    ];

    for (cat_id, label) in categories {
        let node_id = SidebarNodeId::Category(cat_id);
        let expanded = nav.expanded_ids.contains(&node_id);

        rows.push(SidebarRow {
            id: node_id.clone(),
            label: label.to_string(),
            level: 0,
            is_expandable: true,
            is_expanded: expanded,
        });

        // 2. Expand Categories if open
        if expanded && cat_id == CategoryId::Playlists {
            flatten_playlists(snapshot, nav, &mut rows)
        }
    }

    rows
}

fn flatten_playlists(
    snapshot: &LibrarySnapshot,
    nav: &LibraryNavState,
    rows: &mut Vec<SidebarRow>,
) {
    // Iterate over playlists in the defined order
    for pid in &snapshot.playlist_order {
        if let Some(playlist) = snapshot.playlists.get(pid) {
            let p_node = SidebarNodeId::Playlist(*pid);
            let expanded = nav.expanded_ids.contains(&p_node);
            let track_count = playlist.tracks.len();

            rows.push(SidebarRow {
                id: p_node.clone(),
                label: playlist.name.clone(),
                level: 1,
                is_expandable: track_count > 0,
                is_expanded: expanded,
            });

            // 3. Expand Playlist Content (Tracks)
            if expanded {
                // Look up track metadata for every ID in the playlist
                for &tid in &playlist.tracks {
                    if let Some(track) = snapshot.tracks.get(&tid) {
                        rows.push(SidebarRow {
                            id: SidebarNodeId::Track(tid),
                            label: track.title.clone(),
                            level: 2,
                            is_expandable: false, // Tracks are leaves
                            is_expanded: false,
                        });
                    }
                }
            }
        }
    }
}
