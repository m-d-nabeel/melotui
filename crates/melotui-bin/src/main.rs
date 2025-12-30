mod logging;
use melotui_core::{
    AppMsg, AppState, Command, Library, NavMode, UINavState,
    reducer::reduce,
    state::{DownloadState, DownloadStatus, PlaybackState, PlaybackStatus, UIMode, UIState},
};
use melotui_download::spawn_download;
use melotui_tui::Tui;
use std::sync::mpsc;

fn main() -> anyhow::Result<()> {
    let _log_guard = logging::init();

    std::panic::set_hook(Box::new(|panic| {
        tracing::error!("panic: {}", panic);
    }));

    let mut tui = Tui::new()?;
    let music_root = match dirs::audio_dir() {
        Some(music_root) => music_root,
        None => {
            panic!("Root directory for music does not exist");
        }
    };
    let library = Library::scan(music_root)?;
    let playlist_count = library.playlists.len();

    let (tx, rx) = mpsc::channel::<AppMsg>();

    let mut state = AppState {
        app_name: "Melotui".into(),
        status: "Ready".into(),
        nav: UINavState {
            mode: NavMode::Playlists,
            selected_playlist: 0,
            selected_tracks: vec![0; playlist_count],
        },
        library,
        playback: PlaybackState {
            current: None,
            status: PlaybackStatus::Stopped,
        },
        ui: UIState {
            mode: UIMode::Normal,
            command_buffer: String::new(),
        },
        download: DownloadState {
            status: DownloadStatus::Idle,
        },
    };

    let mut running = true;
    while running {
        tui.draw(&state)?;

        if let Some(msg) = tui.poll_event(&state)? {
            if matches!(msg, AppMsg::Quit) {
                break;
            }

            reduce(&mut state, msg.clone());

            if state.status == "Quit" {
                running = false;
            }

            // SIDE EFFECTS BELONG HERE
            if let AppMsg::SubmitCommand(input) = msg {
                if let Command::Download { query } = Command::parse(&input) {
                    tracing::info!("Spawning yt-dlp for query: {}", query);
                    spawn_download(query, state.library.root.clone(), tx.clone());
                }
            }
        }

        // Handle background worker messages
        while let Ok(msg) = rx.try_recv() {
            reduce(&mut state, msg);
        }
    }

    tui.shutdown()?;
    Ok(())
}
