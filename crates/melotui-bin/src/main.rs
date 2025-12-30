mod logging;
use melotui_core::{
    AppMsg, AppState, Library, NavMode, UINavState,
    reducer::reduce,
    state::{PlaybackState, PlaybackStatus, UIMode, UIState},
};
use melotui_tui::Tui;

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
    };

    let mut running = true;
    while running {
        tui.draw(&state)?;
        if let Some(msg) = tui.poll_event(state.ui.mode.clone())? {
            if matches!(msg, AppMsg::Quit) {
                running = false;
            } else {
                reduce(&mut state, msg);
            }
        }
    }

    tui.shutdown()?;
    Ok(())
}
