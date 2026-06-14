use anyhow::Result;
use melotui_bus::{event::IpcEvent, spectrum_bridge};
use melotui_core::{
    AppMsg, AppState,
    library::Library,
    message::ui::UIMsg,
    reducer,
    state::{NotificationKind, visualizer::update_visualizer_physics},
};
use melotui_engine::{Engine, EngineConfig, bridge};
use melotui_tui::Tui;
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::input_processor::InputProcessor;

pub fn run() -> Result<()> {
    tracing::info!("Starting melotui application");

    let config = melotui_core::config::AppConfig::load();
    let (db_path, music_roots) = resolve_paths(&config)?;

    // start empty library - data loads async
    let library = Library::new(music_roots.clone(), vec![], HashMap::new());

    let mut state = AppState::new(library, &config);
    let (mut tui, keymap_errors) = Tui::new(&config)?;
    let (producer, consumer) = spectrum_bridge::create_bridge();
    state.spectrum_source = Box::new(consumer);

    let engine = Engine::new(EngineConfig {
        db_path,
        spectrum_producer: producer,
        fft_size: config.audio.fft_size,
        max_cover_resolution: config.library.max_cover_resolution,
    });

    // request async library load
    engine.load_library(music_roots.clone());

    // Aggregate startup errors
    let mut startup_errors = config.startup_errors.clone();
    startup_errors.extend(keymap_errors);

    if !startup_errors.is_empty() {
        let msg = format!(
            "Config Errors ({}). Check logs for details: {}",
            startup_errors.len(),
            startup_errors[0]
        );
        state.set_notification_with_duration(NotificationKind::Error, msg, 7);
    } else {
        state.set_notification(NotificationKind::Info, "Loading library...");
    }

    let mut runtime = RuntimeContext {
        engine,
        engine_alive: true,
        running: true,
        effect_queue: VecDeque::new(),
        input_processor: InputProcessor::new(),
        music_roots,
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
            let limit = state.config.ui.fps_limit.max(1); // prevent division by zero
            Duration::from_millis(1000 / limit)
        } else {
            Duration::from_millis(100)
        };

        // event coalescing loop
        // drain up to 50 events - non-blocking after first
        let mut events = Vec::with_capacity(10);

        // blocking poll for first event
        if let Some(msg) = tui.poll_event(&state, timeout)? {
            events.push(msg);

            // non-blocking poll for next events
            for _ in 0..50 {
                if let Some(msg) = tui.poll_event(&state, Duration::ZERO)? {
                    events.push(msg);
                } else {
                    break;
                }
            }
        }

        if !events.is_empty() {
            let debounce = state.config.ui.search_debounce_ms;
            let processed = runtime.input_processor.coalesce(events, debounce);
            runtime.effect_queue.extend(processed);
            should_render = true;
        }

        if runtime.engine_alive && handle_ipc_events(&mut runtime, &mut state) {
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
    engine: Engine,
    engine_alive: bool,
    running: bool,
    effect_queue: VecDeque<AppMsg>,
    input_processor: InputProcessor,
    music_roots: Vec<PathBuf>,
}

fn resolve_paths(config: &melotui_core::config::AppConfig) -> Result<(PathBuf, Vec<PathBuf>)> {
    let db_path = config.resolve_db_path()?;
    let music_roots = config.resolve_music_dirs();

    if music_roots.is_empty() {
        return Err(anyhow::anyhow!(
            "No music directories configured and OS audio directory not found. \
             Set music_directories in ~/.config/melotui/config.toml"
        ));
    }

    Ok((db_path, music_roots))
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
    while let Ok(evt) = ctx.engine.event_rx.try_recv() {
        changed = true;
        match evt {
            IpcEvent::LibraryLoaded { snapshot } => {
                tracing::info!("Library loaded with {} tracks", snapshot.tracks.len());
                state.library = *snapshot;
                state.set_notification(NotificationKind::Info, "Library loaded");
            }
            IpcEvent::LibraryRescanned => {
                // reload after scan
                ctx.engine.load_library(ctx.music_roots.clone());
                state.set_notification(NotificationKind::Info, "Scan complete, reloading...");
            }
            _ => ctx.effect_queue.push_back(bridge::ipc_to_appmsg(evt)),
        }
    }
    changed
}

fn process_effects(ctx: &mut RuntimeContext, state: &mut AppState) -> bool {
    let mut changed = false;
    while let Some(msg) = ctx.effect_queue.pop_front() {
        changed = true;
        // lifecycle
        if matches!(msg, AppMsg::RequestQuit) {
            if ctx.engine_alive {
                ctx.engine.shutdown();
                ctx.engine_alive = false;
            }
            ctx.running = false;
            break;
        }

        // side effects
        if ctx.engine_alive
            && let Some(cmd) = bridge::appmsg_to_ipc(&msg, &state.library.roots)
        {
            ctx.engine.dispatch(cmd);
        }

        // state mutation
        let next_effects = reducer::reduce(state, msg);
        ctx.effect_queue.extend(next_effects);
    }
    changed
}
