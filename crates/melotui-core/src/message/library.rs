use crate::library::Track;

#[derive(Debug, Clone)]
pub enum LibraryMsg {
    AddToQueue(Track),
    PlayNext(Track),
    ToggleRepeatMode,
    ToggleShuffle,
    RemoveFromQueue,
    ClearQueue,
    RequestRescanLibrary,
}
