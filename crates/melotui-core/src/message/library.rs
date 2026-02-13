use crate::command::SearchField;
use crate::domain::TrackId;

#[derive(Debug, Clone)]
pub enum LibraryMsg {
    AddToQueue(TrackId),
    PlayNext(TrackId),
    ToggleRepeatMode,
    ToggleShuffle,
    RequestToggleFavorite(TrackId),
    RemoveFromQueue,
    ClearQueue,
    Search { field: SearchField, query: String },
    SearchNext,
    SearchPrev,
    ClearSearch,
    RequestScanLibrary,
}
