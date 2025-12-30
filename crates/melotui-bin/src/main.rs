mod logging;
use melotui_core::{
    AppMsg, AppState, Library, NavMode, UINavState,
    reducer::reduce,
    state::{
        DownloadState, DownloadStatus, NotificationState, PlaybackState, PlaybackStatus, UIMode,
        UIState,
    },
};
use melotui_download::spawn_download;
use melotui_tui::Tui;
use std::{collections::VecDeque, sync::mpsc};

fn handle_side_effects(msg: &AppMsg, state: &AppState, tx: &mpsc::Sender<AppMsg>) {
    tracing::debug!("handle_side_effects:msg = {:?}", msg);
    tracing::debug!("handle_side_effects:download_state = {:?}", state.download);

    if let AppMsg::SpawnDownload { query } = msg
        && let DownloadStatus::Running { control, .. } = &state.download.status
    {
        spawn_download(
            query.clone(),
            state.library.root.clone(),
            tx.clone(),
            control.clone(),
        );
    }
}

/// Processes all queued effects until system stabilizes
fn process_effects(state: &mut AppState, queue: &mut VecDeque<AppMsg>, tx: &mpsc::Sender<AppMsg>) {
    while let Some(effect) = queue.pop_front() {
        let new_effects = reduce(state, effect.clone());
        queue.extend(new_effects);
        handle_side_effects(&effect, state, tx);
    }
}

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
            queue: VecDeque::new(),
        },
        notification: NotificationState { current: None },
    };

    let mut running = true;
    let mut effect_queue = VecDeque::<AppMsg>::new();

    while running {
        // --- Draw UI ---
        tui.draw(&state)?;

        // --- UI input ---
        if let Some(msg) = tui.poll_event(&state)? {
            if matches!(msg, AppMsg::Quit) {
                break;
            }

            let effects = reduce(&mut state, msg);
            effect_queue.extend(effects);
        }

        // --- Worker input ---
        while let Ok(msg) = rx.try_recv() {
            let effects = reduce(&mut state, msg);
            effect_queue.extend(effects);
        }

        // --- Apply all effects ---
        process_effects(&mut state, &mut effect_queue, &tx);

        // --- Notification timeout ---
        if let Some(n) = &state.notification.current
            && let Some(timeout) = n.dismiss_after
            && n.created_at.elapsed() >= timeout
        {
            state.notification.current = None;
        }

        // --- Exit condition ---
        if let Some(m) = &state.notification.current
            && m.message == "Quit"
        {
            running = false;
        }
    }

    tui.shutdown()?;
    Ok(())
}
