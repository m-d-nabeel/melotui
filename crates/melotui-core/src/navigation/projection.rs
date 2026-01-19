use crate::{
    library::snapshot::LibrarySnapshot,
    navigation::{ids::CategoryId, state::LibraryNavState},
};

#[derive(Debug, Clone, PartialEq)]
pub struct SidebarRow {
    pub id: crate::navigation::ids::SidebarNodeId,
    pub parent: Option<crate::navigation::ids::SidebarNodeId>, // Critical for 'h' key (Collapse/Back)
    pub label: String,
    pub level: usize,
    pub is_expandable: bool,
    pub is_expanded: bool,
}

/// Pure function: (Library + State) -> Visual List
pub fn compute_sidebar_rows(library: &LibrarySnapshot, nav: &LibraryNavState) -> Vec<SidebarRow> {
    let mut rows = Vec::new();

    // 1. Root Categories
    // You can re-order these to change the UI
    let categories = [
        (CategoryId::Folders, "Folders"),
        (CategoryId::AllSongs, "All Songs"),
        (CategoryId::Artists, "Artists"),
        (CategoryId::Albums, "Albums"),
        (CategoryId::Playlists, "Playlists"),
    ];

    for (cat_id, label) in categories {
        let node_id = crate::navigation::ids::SidebarNodeId::Category(cat_id);
        let is_expanded = nav.expanded_ids.contains(&node_id);

        rows.push(SidebarRow {
            id: node_id.clone(),
            parent: None,
            label: label.to_string(),
            level: 0,
            is_expandable: true,
            is_expanded,
        });

        // 2. Expand Children
        if is_expanded && cat_id == CategoryId::Playlists {
            for pid in &library.playlist_order {
                if let Some(playlist) = library.playlists.get(pid) {
                    rows.push(SidebarRow {
                        id: crate::navigation::ids::SidebarNodeId::Playlist(*pid),
                        parent: Some(node_id.clone()), // Point back to Category
                        label: playlist.name.clone(),
                        level: 1,
                        is_expandable: false,
                        is_expanded: false,
                    });
                }
            }
        }
    }

    rows
}
