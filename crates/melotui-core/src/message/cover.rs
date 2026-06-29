use crate::domain::TrackId;
use image::DynamicImage;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverContext {
    Playback,
    Info,
}

#[derive(Debug, Clone)]
pub enum CoverMsg {
    RequestLoad {
        id: TrackId,
        path: PathBuf,
        context: CoverContext,
    },
    Loaded {
        id: TrackId,
        cover: Option<Arc<DynamicImage>>,
        context: CoverContext,
        cache_path: Option<PathBuf>,
    },
}
