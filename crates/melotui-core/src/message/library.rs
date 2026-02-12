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
    Filter(String),
    RequestScanLibrary,
}
