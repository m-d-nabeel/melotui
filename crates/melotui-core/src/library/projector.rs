use crate::library::snapshot::LibrarySnapshot;
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

pub fn flatten_tree(snapshot: &LibrarySnapshot, nav: &LibraryNavState) -> Vec<SidebarRow> {
    let mut rows = Vec::new();

    let categories = [
        (CategoryId::Folders, "Folders"),
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

            if expanded {
                for &tid in &playlist.tracks {
                    if let Some(track) = snapshot.tracks.get(&tid) {
                        rows.push(SidebarRow {
                            id: SidebarNodeId::Track {
                                id: tid,
                                parent: Box::new(p_node.clone()),
                            },
                            label: track.title.clone(),
                            level: 2,
                            is_expandable: false,
                            is_expanded: false,
                        });
                    }
                }
            }
        }
    }
}
