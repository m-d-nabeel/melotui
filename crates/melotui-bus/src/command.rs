use std::path::PathBuf;

use melotui_core::{domain::TrackId, message::cover::CoverContext, state::SpectrumStyle};

#[derive(Debug, Clone)]
pub enum AudioCommand {
    Play {
        path: PathBuf,
    },
    /// Make `path` ready to play the moment the track playing ends, with
    /// no gap between them; `None` takes back whatever was made ready.
    Preload {
        path: Option<std::path::PathBuf>,
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
    /// Set one of the effects to a value.
    SetEffect(melotui_core::state::audio::Effect, f32),
    SetEqBand {
        band: usize,
        gain: f32,
    },
    /// Plays through the device with this name, or part of it; `None`
    /// follows the system default.
    SetOutputDevice(Option<String>),
    /// Asks for `IpcEvent::OutputDevices`.
    ListOutputDevices,
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
        album_name: Option<String>,
        duration_secs: Option<u64>,
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
    CreatePlaylist { name: String },
    AddTrackToPlaylist { playlist_id: i64, track_id: i64 },
    DeletePlaylist { playlist_id: i64 },
    RemoveTrackFromPlaylist { playlist_id: i64, track_id: i64 },
    IncrementPlayCount { track_id: i64 },
    SetFavorite { track_id: i64, favorite: bool },
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
