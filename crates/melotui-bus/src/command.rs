use std::path::PathBuf;

use melotui_core::{domain::TrackId, message::cover::CoverContext, state::SpectrumStyle};

#[derive(Debug, Clone)]
pub enum AudioCommand {
    Play {
        path: PathBuf,
    },
    Pause,
    Resume,
    Stop,
    Seek {
        position_ms: u64,
        epoch: u64,
    },
    SetVisualizer {
        spectrum_style: Option<SpectrumStyle>,
    },
    SetVolume(f32),
    SetSpeed(f32),
    SetBass(f32),
    SetTreble(f32),
    SetReverb(f32),
    SetEcho(f32),
    SetPan(f32),
    SetLowpass(f32),
    SetHighpass(f32),
    SetEqBand { band: usize, gain: f32 },
    SetAutoPanRate(f32),
    SetAutoPanDepth(f32),
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum DownloadCommand {
    Request {
        query: String,
        download_path: PathBuf,
    },
    FetchLyrics {
        track_name: String,
        artist_name: String,
        save_path: PathBuf,
        provider: Option<String>,
        provider_id: Option<String>,
    },
    SearchLyrics {
        query: String,
    },
    Cancel,
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum DatabaseCommand {
    LoadLibrary { roots: Vec<PathBuf> },
    ScanLibrary { roots: Vec<PathBuf> },
    PruneLibrary,
    GetAllTracks,
    SearchTracks { query: String },
    GetPlaylists,
    CreatePlaylist { name: String },
    AddTrackToPlaylist { playlist_id: i64, track_id: i64 },
    DeletePlaylist { playlist_id: i64 },
    RemoveTrackFromPlaylist { playlist_id: i64, track_id: i64 },
    IncrementPlayCount { track_id: i64 },
    ToggleFavorite { track_id: i64 },
    Query { sql: String },
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum CoverCommand {
    Load {
        id: TrackId,
        path: PathBuf,
        context: CoverContext,
    },
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum IpcCommand {
    Audio(AudioCommand),
    Download(DownloadCommand),
    Database(DatabaseCommand),
    Cover(CoverCommand),
    Shutdown,
}
