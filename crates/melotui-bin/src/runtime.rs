use anyhow::Result;
use melotui_bus::{event::IpcEvent, spectrum_bridge};
use melotui_core::{
    AppMsg, AppState,
    library::Library,
    message::{
        cover::{CoverContext, CoverMsg},
        ui::UIMsg,
    },
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
        session_restored: false,
    };

    // Initialize MPRIS / system media controls (Linux/macOS only)
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let (mut mpris_manager, mpris_rx) = {
        match crate::mpris::MprisManager::new() {
            Some((mgr, rx)) => (Some(mgr), Some(rx)),
            None => (None, None),
        }
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
            if !runtime.running {
                break;
            }
            should_render = true;
        }

        // Poll MPRIS media key events (Linux/macOS only)
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        if let Some(ref rx) = mpris_rx {
            while let Ok(evt) = rx.try_recv() {
                match evt {
                    crate::mpris::MprisEvent::MediaKey(msg) => {
                        runtime.effect_queue.push_back(msg);
                        should_render = true;
                    }
                }
            }
        }

        let now = Instant::now();
        let dt = now.duration_since(last_tick).as_secs_f32();
        last_tick = now;

        state.time_elapsed += dt;
        if tick_state(&mut state, dt) {
            should_render = true;
        }

        if process_effects(&mut runtime, &mut state) {
            if !runtime.running {
                break;
            }
            should_render = true;
        }

        if should_render && runtime.running {
            tui.draw(&state)?;
            should_render = false;

            // Sync MPRIS metadata/status after render (Linux/macOS only)
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            if let Some(ref mut mgr) = mpris_manager {
                mgr.update_state(&state);
            }
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
    session_restored: bool,
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
                if !ctx.session_restored && state.config.ui.restore_session {
                    ctx.session_restored = true;
                    if let Ok(session) = melotui_core::PersistedSession::load() {
                        let count = session.apply_to_app_state(state);
                        if let Some(active_id) = state.get_playing_track_id()
                            && let Some(track) = state.library.tracks.get(&active_id)
                        {
                            ctx.engine.play(track.file_path.clone());
                            ctx.engine.pause();
                            if state.progress_ms() > 0 {
                                ctx.engine.seek(state.progress_ms(), state.seek_epoch());
                            }
                            if track.has_cover {
                                ctx.effect_queue
                                    .push_back(AppMsg::Cover(CoverMsg::RequestLoad {
                                        id: active_id,
                                        path: track.file_path.clone(),
                                        context: CoverContext::Playback,
                                    }));
                            }
                        }
                        ctx.engine.sync_dsp(state.audio());

                        if state.should_draw_spectrum() {
                            ctx.effect_queue.push_back(AppMsg::Playback(
                                melotui_core::message::playback::PlaybackMsg::DispatchSetVisualizer {
                                    spectrum_style: Some(state.spectrum_style().clone()),
                                },
                            ));
                        }

                        state.set_notification(
                            NotificationKind::Info,
                            format!("Session restored ({} tracks in queue)", count),
                        );
                    } else {
                        state.set_notification(NotificationKind::Info, "Library loaded");
                    }
                } else {
                    state.set_notification(NotificationKind::Info, "Library loaded");
                }

                state.ensure_library_selection();

                if state.is_showing_track_info()
                    && let Some(tid) = state.get_track_info_target_id()
                    && let Some(info_track) = state.library.get_track(tid).cloned()
                    && info_track.has_cover
                {
                    ctx.effect_queue
                        .push_back(AppMsg::Cover(CoverMsg::RequestLoad {
                            id: tid,
                            path: info_track.file_path,
                            context: CoverContext::Info,
                        }));
                }
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
            let session = melotui_core::PersistedSession::from_app_state(state);
            if let Err(e) = session.save() {
                tracing::warn!("Failed to save session: {}", e);
            } else {
                tracing::info!("Session saved successfully");
            }

            if ctx.engine_alive {
                ctx.engine.shutdown();
                ctx.engine_alive = false;
            }
            ctx.running = false;

            // We must force exit here because dropping MprisManager (souvlaki DBus) at the end of this function
            // can block for several seconds waiting for the DBus thread to join.
            // But we must restore the terminal first!
            let _ = melotui_tui::restore_terminal();
            std::process::exit(0);
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
