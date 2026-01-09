use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum AudioCommand {
    Play { path: PathBuf },
    Pause,
    Resume,
    Stop,
    Seek { position_ms: u64 },
    SetVisualizer { active: bool },
    SetVolume(f32),
    SetSpeed(f32),
    SetBass(f32),
    SetTreble(f32),
    SetReverb(f32),
    SetEcho(f32),
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum DownloadCommand {
    Request {
        query: String,
        download_path: PathBuf,
    },
    Cancel,
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum DatabaseCommand {
    ScanLibrary { root: PathBuf },
    Query { sql: String },
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum IpcCommand {
    Audio(AudioCommand),
    Download(DownloadCommand),
    Database(DatabaseCommand),
    Shutdown,
}
