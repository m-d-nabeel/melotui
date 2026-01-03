use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum IpcCommand {
    // ───── Downloads ─────
    Download {
        query: String,
        download_path: PathBuf,
    },
    CancelDownload,

    // ───── Playback (next step) ─────
    Play {
        path: PathBuf,
    },
    Pause,
    Resume,
    Stop,
    Seek {
        position_ms: u64,
    },
    SetVisualizer {
        active: bool,
    },
    // --- Audio Filters ---
    SetVolume(f32),
    SetSpeed(f32),
    SetBass(f32),
    SetTreble(f32),
    SetReverb(f32),
    SetEcho(f32),

    // ───── Lifecycle ─────
    Shutdown,
}
