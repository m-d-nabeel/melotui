use anyhow::Result;
use melotui_core::{
    AppMsg, AppState,
    library::LibrarySnapshot,
    state::{NotificationKind, visualizer::update_visualizer_physics},
};
use melotui_db::load_library_from_db;
use melotui_ipc::{event::IpcEvent, spectrum_bridge};
use melotui_tui::Tui;
use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use crate::ipc_bridge::{IpcRuntime, appmsg_to_ipc, ipc_to_appmsg};

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
        let now = Instant::now();
        let dt = now.duration_since(last_tick).as_secs_f32();
        last_tick = now;

        state.time_elapsed += dt;
        tick_state(&mut state, dt);

        tui.draw(&state)?;

        if let Some(msg) = tui.poll_event(&state)? {
            runtime.effect_queue.push_back(msg);
        }

        if runtime.ipc_alive {
            handle_ipc_events(&mut runtime, &mut state);
        }

        process_effects(&mut runtime, &mut state);
    }

    tui.shutdown()
}

struct RuntimeContext {
    ipc: IpcRuntime,
    ipc_alive: bool,
    running: bool,
    effect_queue: VecDeque<AppMsg>,
    db_path: PathBuf,
    music_root: PathBuf,
}

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

fn load_library_safe(db_path: &Path, music_root: &Path) -> LibrarySnapshot {
    match load_library_from_db(db_path, music_root.to_path_buf()) {
        Ok(lib) => {
            tracing::info!("Loaded {} tracks from database", lib.tracks.len());
            lib
        }
        Err(e) => {
            tracing::warn!("Failed to load from DB: {}. Starting empty.", e);
            LibrarySnapshot::new(music_root.to_path_buf(), vec![], HashMap::new())
        }
    }
}

fn tick_state(state: &mut AppState, dt: f32) {
    update_visualizer_physics(state, dt);
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
                let new_lib = load_library_safe(&ctx.db_path, &ctx.music_root);
                state.library = Arc::new(new_lib);

                state.set_notification(NotificationKind::Info, "Library Updated");
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
