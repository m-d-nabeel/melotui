use crate::domain::TrackId;

#[derive(Debug, Clone)]
pub enum LibraryMsg {
    AddToQueue(TrackId),
    PlayNext(TrackId),
    ToggleRepeatMode,
    ToggleShuffle,
    RemoveFromQueue,
    ClearQueue,
    RequestRescanLibrary,
}
