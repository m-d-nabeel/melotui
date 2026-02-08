use crate::domain::TrackId;

#[derive(Debug, Clone)]
pub enum DatabaseMsg {
    DispatchToggleFavorite(TrackId),
}
