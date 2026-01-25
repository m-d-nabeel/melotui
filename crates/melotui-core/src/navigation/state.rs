use crate::{navigation::ids::SidebarNodeId, playback::state::QueueId};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LeftPanelMode {
    #[default]
    Library,
    Queue,
}

#[derive(Debug, Clone, Default)]
pub struct LibraryNavState {
    pub selected_id: Option<SidebarNodeId>,
    pub expanded_ids: HashSet<SidebarNodeId>,
    pub scroll_offset: usize,
}

#[derive(Debug, Clone, Default)]
pub struct QueueNavState {
    pub selected_queue_id: Option<QueueId>,
}

#[derive(Debug, Clone)]
pub struct NavigationState {
    pub active_panel: LeftPanelMode,
    pub library: LibraryNavState,
    pub queue: QueueNavState,
    pub show_track_info: bool,
}

impl Default for NavigationState {
    fn default() -> Self {
        Self {
            active_panel: LeftPanelMode::Library,
            library: LibraryNavState::default(),
            queue: QueueNavState::default(),
            show_track_info: false,
        }
    }
}
