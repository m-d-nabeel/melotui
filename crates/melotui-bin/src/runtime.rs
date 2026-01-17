use anyhow::Result;
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use melotui_core::{
    AppMsg, AppState, Library,
    state::{Notification, NotificationKind, SpectrumStyle},
};
use melotui_db::load_library_from_db;
use melotui_ipc::{event::IpcEvent, spectrum_bridge};
use melotui_tui::Tui;

use crate::ipc_bridge::{IpcRuntime, appmsg_to_ipc, ipc_to_appmsg};

// --- Main Entry Point ---

pub fn run() -> Result<()> {
    tracing::info!("Starting melotui application");

    let (db_path, music_root) = resolve_paths()?;

    let library = load_library_safe(&db_path, &music_root);

    let mut state = AppState::new(library);
    let mut tui = Tui::new()?;

    let (producer, consumer) = spectrum_bridge::create_bridge();
    state.spectrum_source = Box::new(consumer);

    let ipc = IpcRuntime::new(producer, db_path.clone());

    let mut runtime = RuntimeContext {
        ipc,
        ipc_alive: true,
        running: true,
        effect_queue: VecDeque::new(),
        db_path,
        music_root,
    };

    let mut last_tick = Instant::now();

    while runtime.running {
        // --- Timing ---
        let now = Instant::now();
        let dt = now.duration_since(last_tick).as_secs_f32();
        last_tick = now;

        // --- Update State ---
        state.time_elapsed += dt;
        tick_state(&mut state, dt);

        // --- Draw ---
        tui.draw(&state)?;

        // --- Input ---
        if let Some(msg) = tui.poll_event(&state)? {
            runtime.effect_queue.push_back(msg);
        }

        // --- IPC ---
        if runtime.ipc_alive {
            handle_ipc_events(&mut runtime, &mut state);
        }

        // --- Effects ---
        process_effects(&mut runtime, &mut state);
    }

    tui.shutdown()
}

/// Runtime-specific objects
struct RuntimeContext {
    ipc: IpcRuntime,
    ipc_alive: bool,
    running: bool,
    effect_queue: VecDeque<AppMsg>,
    db_path: PathBuf,
    music_root: PathBuf,
}

// --- Helpers ---
fn resolve_paths() -> Result<(PathBuf, PathBuf)> {
    let db_root = dirs::data_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?
        .join("melotui");

    std::fs::create_dir_all(&db_root)?;
    let db_path = db_root.join("library.db");

    let music_root =
        dirs::audio_dir().ok_or_else(|| anyhow::anyhow!("Music directory not found"))?;

    Ok((db_path, music_root))
}

fn load_library_safe(db_path: &Path, music_root: &Path) -> Library {
    match load_library_from_db(db_path, music_root.to_path_buf()) {
        Ok(lib) => {
            let track_count = lib.playlists.first().map(|p| p.tracks.len()).unwrap_or(0);
            tracing::info!("Loaded {} tracks from database", track_count);
            lib
        }
        Err(e) => {
            tracing::warn!("Failed to load from DB: {}. Starting empty.", e);
            Library::new(music_root.to_path_buf(), vec![])
        }
    }
}

fn tick_state(state: &mut AppState, dt: f32) {
    update_visualizer_physics(state, dt);
    // Cleanup notifications
    if let Some(n) = state.current_notification()
        && let Some(timeout) = n.dismiss_after
        && n.created_at.elapsed() >= timeout
    {
        state.clear_current_notification();
    }
}

fn handle_ipc_events(ctx: &mut RuntimeContext, state: &mut AppState) {
    while let Ok(evt) = ctx.ipc.event_rx.try_recv() {
        match evt {
            IpcEvent::LibraryRescanned => {
                tracing::info!("Database updated. Reloading library...");

                // Use the safe loader helper
                let new_lib = load_library_safe(&ctx.db_path, &ctx.music_root);

                // Hot-swap
                state.library = std::sync::Arc::new(new_lib);
                melotui_core::reconcile_nav_state(state);

                // Notify
                state.set_notification(Notification {
                    kind: NotificationKind::Info,
                    message: "Library Updated".into(),
                    progress: None,
                    created_at: Instant::now(),
                    dismiss_after: Some(Duration::from_secs(3)),
                });
            }
            _ => ctx.effect_queue.push_back(ipc_to_appmsg(evt)),
        }
    }
}

fn process_effects(ctx: &mut RuntimeContext, state: &mut AppState) {
    while let Some(msg) = ctx.effect_queue.pop_front() {
        // Lifecycle
        if matches!(msg, AppMsg::RequestQuit) {
            if ctx.ipc_alive {
                ctx.ipc.dispatch(melotui_ipc::command::IpcCommand::Shutdown);
                ctx.ipc_alive = false;
            }
            ctx.running = false;
            break;
        }

        // Side Effects
        if ctx.ipc_alive
            && let Some(cmd) = appmsg_to_ipc(&msg, state)
        {
            ctx.ipc.dispatch(cmd);
        }

        // State Mutation
        let next_effects = melotui_core::reducer::reduce(state, msg);
        ctx.effect_queue.extend(next_effects);
    }
}

// --- Physics (Legacy/View Logic) ---
// Note: Ideally, this moves to `melotui_core::state::visualizer` in the future.

fn update_visualizer_physics(state: &mut AppState, dt: f32) {
    if !state.should_draw_spectrum() {
        return;
    }

    if state.spectrum_raw.len() < 512 {
        state.spectrum_raw.resize(512, 0.0);
    }

    let count = state.spectrum_source.get_latest(&mut state.spectrum_raw);

    if count > 0 {
        state.spectrum_raw.truncate(count);
    }

    let len = state.spectrum_raw.len();
    if state.spectrum_smooth.len() != len {
        state.spectrum_smooth.resize(len, 0.0);
    }
    if state.spectrum_peaks.len() != len {
        state.spectrum_peaks.resize(len, 0.0);
    }

    if *state.spectrum_style() == SpectrumStyle::Cava {
        state.spectrum_smooth.copy_from_slice(&state.spectrum_raw);
        for i in 0..len {
            if state.spectrum_smooth[i] > state.spectrum_peaks[i] {
                state.spectrum_peaks[i] = state.spectrum_smooth[i];
            } else {
                state.spectrum_peaks[i] -= 2.0 * dt;
                if state.spectrum_peaks[i] < 0.0 {
                    state.spectrum_peaks[i] = 0.0;
                }
            }
        }
        return;
    }

    let gravity = 4.0;
    let smoothing_speed = 15.0;

    for i in 0..len {
        let target = state.spectrum_raw[i];
        let diff = target - state.spectrum_smooth[i];
        state.spectrum_smooth[i] += diff * (smoothing_speed * dt).min(1.0);

        if state.spectrum_smooth[i] > state.spectrum_peaks[i] {
            state.spectrum_peaks[i] = state.spectrum_smooth[i];
        } else {
            state.spectrum_peaks[i] -= gravity * dt;
            if state.spectrum_peaks[i] < 0.0 {
                state.spectrum_peaks[i] = 0.0;
            }
        }
    }
}
