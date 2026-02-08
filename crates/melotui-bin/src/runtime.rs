use anyhow::Result;
use melotui_core::{
    AppMsg, AppState,
    library::LibrarySnapshot,
    message::ui::UIMsg,
    reducer,
    state::{NotificationKind, visualizer::update_visualizer_physics},
};
use melotui_ipc::{
    command::{DatabaseCommand, IpcCommand},
    event::IpcEvent,
    spectrum_bridge,
};
use melotui_tui::Tui;
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::{
    input_processor::InputProcessor,
    ipc_bridge::{IpcRuntime, appmsg_to_ipc, ipc_to_appmsg},
};

pub fn run() -> Result<()> {
    tracing::info!("Starting melotui application");

    let (db_path, music_root) = resolve_paths()?;

    // Start with an empty library - data loads async
    let library = LibrarySnapshot::new(music_root.clone(), vec![], HashMap::new());

    let mut state = AppState::new(library);
    let mut tui = Tui::new()?;
    let (producer, consumer) = spectrum_bridge::create_bridge();
    state.spectrum_source = Box::new(consumer);

    let ipc = IpcRuntime::new(producer, db_path.clone());

    // Request async library load
    ipc.dispatch(IpcCommand::Database(DatabaseCommand::LoadLibrary {
        root: music_root.clone(),
    }));
    state.set_notification(NotificationKind::Info, "Loading library...");

    let mut runtime = RuntimeContext {
        ipc,
        ipc_alive: true,
        running: true,
        effect_queue: VecDeque::new(),
        input_processor: InputProcessor::new(),
        music_root,
    };

    if let Ok((w, h)) = tui.size() {
        runtime
            .effect_queue
            .push_back(AppMsg::UI(UIMsg::Resize(w, h)));
    }

    let mut last_tick = Instant::now();
    let mut should_render = true;

    while runtime.running {
        let timeout = if state.should_draw_spectrum() {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(100)
        };

        // Event Coalescing Loop
        // Drain up to 50 events from TUI (non-blocking after first)
        let mut events = Vec::with_capacity(10);

        // Blocking poll for first event
        if let Some(msg) = tui.poll_event(&state, timeout)? {
            events.push(msg);

            // Non-blocking poll for subsequent events
            for _ in 0..50 {
                if let Some(msg) = tui.poll_event(&state, Duration::ZERO)? {
                    events.push(msg);
                } else {
                    break;
                }
            }
        }

        if !events.is_empty() {
            let processed = runtime.input_processor.coalesce(events);
            runtime.effect_queue.extend(processed);
            should_render = true;
        }

        if runtime.ipc_alive && handle_ipc_events(&mut runtime, &mut state) {
            should_render = true;
        }

        let now = Instant::now();
        let dt = now.duration_since(last_tick).as_secs_f32();
        last_tick = now;

        state.time_elapsed += dt;
        if tick_state(&mut state, dt) {
            should_render = true;
        }

        if process_effects(&mut runtime, &mut state) {
            should_render = true;
        }

        if should_render {
            tui.draw(&state)?;
            should_render = false;
        }
    }

    tui.shutdown()
}

struct RuntimeContext {
    ipc: IpcRuntime,
    ipc_alive: bool,
    running: bool,
    effect_queue: VecDeque<AppMsg>,
    input_processor: InputProcessor,
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

fn tick_state(state: &mut AppState, dt: f32) -> bool {
    let mut changed = false;

    if state.should_draw_spectrum() {
        update_visualizer_physics(state, dt);
        changed = true;
    }

    if let Some(n) = state.current_notification()
        && let Some(timeout) = n.dismiss_after
        && n.created_at.elapsed() >= timeout
    {
        state.clear_current_notification();
        changed = true;
    }

    changed
}

fn handle_ipc_events(ctx: &mut RuntimeContext, state: &mut AppState) -> bool {
    let mut changed = false;
    while let Ok(evt) = ctx.ipc.event_rx.try_recv() {
        changed = true;
        match evt {
            IpcEvent::LibraryLoaded { snapshot } => {
                tracing::info!("Library loaded with {} tracks", snapshot.tracks.len());
                state.library = *snapshot;
                state.set_notification(NotificationKind::Info, "Library loaded");
            }
            IpcEvent::LibraryRescanned => {
                // Trigger a reload after scan completes
                ctx.ipc
                    .dispatch(IpcCommand::Database(DatabaseCommand::LoadLibrary {
                        root: ctx.music_root.clone(),
                    }));
                state.set_notification(NotificationKind::Info, "Scan complete, reloading...");
            }
            _ => ctx.effect_queue.push_back(ipc_to_appmsg(evt)),
        }
    }
    changed
}

fn process_effects(ctx: &mut RuntimeContext, state: &mut AppState) -> bool {
    let mut changed = false;
    while let Some(msg) = ctx.effect_queue.pop_front() {
        changed = true;
        // Lifecycle
        if matches!(msg, AppMsg::RequestQuit) {
            if ctx.ipc_alive {
                ctx.ipc.dispatch(IpcCommand::Shutdown);
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
        let next_effects = reducer::reduce(state, msg);
        ctx.effect_queue.extend(next_effects);
    }
    changed
}
