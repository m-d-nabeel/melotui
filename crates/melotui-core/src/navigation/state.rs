use crate::{command::SearchField, navigation::ids::SidebarNodeId, playback::state::QueueId};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LeftPanelMode {
    #[default]
    Library,
    Queue,
}

#[derive(Debug, Clone, Default)]
pub struct SearchState {
    pub field: Option<SearchField>,
    pub query: String,
    pub matches: Vec<SidebarNodeId>,
    pub index: usize,
}

#[derive(Debug, Clone, Default)]
pub struct LibraryNavState {
    pub selected_id: Option<SidebarNodeId>,
    pub expanded_ids: HashSet<SidebarNodeId>,
    pub scroll_offset: usize,
    pub search: SearchState,
    pub sort_field: SortField,
    pub sort_direction: SortDirection,
}

#[derive(Debug, Clone, Default)]
pub struct QueueNavState {
    pub selected_queue_id: Option<QueueId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SortField {
    #[default]
    Title,
    Artist,
    Album,
    Year,
    Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
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
