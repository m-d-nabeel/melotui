use anyhow::Result;
use melotui_bus::{event::IpcEvent, spectrum_bridge};
use melotui_core::{
    AppMsg, AppState,
    library::Library,
    message::{
        cover::{CoverContext, CoverMsg},
        library::LibraryMsg,
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
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

/// How often the session is saved while running, so a crash or a killed
/// terminal loses at most this much.
const AUTOSAVE_EVERY: Duration = Duration::from_secs(30);

use crate::input_processor::InputProcessor;

pub fn run(config: melotui_core::config::AppConfig) -> Result<()> {
    tracing::info!("Starting melotui application");

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
        sample_rate: config.audio.sample_rate,
        buffer_size: config.audio.buffer_size,
    });

    // request async library load
    engine.load_library(music_roots.clone());

    // Aggregate startup errors
    let mut startup_errors = config.startup_errors.clone();
    startup_errors.extend(keymap_errors);

    if music_roots.is_empty() {
        state.set_notification_with_duration(
            NotificationKind::Warning,
            format!(
                "No music folder found. Set music_directories in {}",
                melotui_core::config::AppConfig::config_path().display()
            ),
            15,
        );
    } else if !startup_errors.is_empty() {
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
        library_loaded: false,
        last_saved: Instant::now(),
        startup_scan_requested: false,
    };

    // Changes in the music folders while running are scanned for a few
    // seconds after they settle.
    let (folder_tx, folder_rx) = std::sync::mpsc::channel();
    let _watcher = if config.library.watch && !runtime.music_roots.is_empty() {
        match watch_folders(&runtime.music_roots, folder_tx) {
            Ok(watcher) => Some(watcher),
            Err(e) => {
                tracing::warn!("Cannot watch the music folders: {e}");
                state.set_notification_with_duration(
                    NotificationKind::Warning,
                    format!("Not watching the music folders ({e}); use :rescan after adding music"),
                    8,
                );
                None
            }
        }
    } else {
        None
    };
    let mut folders_changed_at: Option<Instant> = None;

    // Closing the terminal (SIGHUP), `kill` (SIGTERM) or Ctrl+C outside raw
    // mode (SIGINT), and closing the console on Windows, quit the way `:q`
    // does — session saved — instead of killing the process where it stands.
    let quit_requested = Arc::new(AtomicBool::new(false));
    {
        let quit_requested = Arc::clone(&quit_requested);
        if let Err(e) = ctrlc::set_handler(move || quit_requested.store(true, Ordering::SeqCst)) {
            tracing::warn!("Cannot handle termination signals: {e}");
        }
    }
    // Set once the terminal itself has gone, so shutting down does not fail
    // trying to restore it.
    let mut terminal_lost = false;

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

        if quit_requested.swap(false, Ordering::SeqCst) {
            tracing::info!("Termination signal received, quitting");
            runtime.effect_queue.push_back(AppMsg::RequestQuit);
        }

        // blocking poll for first event, then whatever else is waiting. A
        // terminal that has gone away quits, session saved.
        match tui.poll_event(&state, timeout) {
            Ok(Some(msg)) => {
                events.push(msg);
                for _ in 0..50 {
                    match tui.poll_event(&state, Duration::ZERO) {
                        Ok(Some(msg)) => events.push(msg),
                        _ => break,
                    }
                }
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!("Lost the terminal ({e}), quitting");
                terminal_lost = true;
                runtime.effect_queue.push_back(AppMsg::RequestQuit);
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

        while folder_rx.try_recv().is_ok() {
            folders_changed_at = Some(Instant::now());
        }
        if let Some(at) = folders_changed_at
            && at.elapsed() >= FOLDERS_SETTLE
            && runtime.library_loaded
        {
            folders_changed_at = None;
            tracing::info!("Music folders changed, scanning");
            runtime
                .effect_queue
                .push_back(AppMsg::Library(LibraryMsg::RequestBackgroundScan));
        }

        if runtime.library_loaded && runtime.last_saved.elapsed() >= AUTOSAVE_EVERY {
            save_session(&state);
            runtime.last_saved = Instant::now();
        }

        if should_render && runtime.running {
            if let Err(e) = tui.draw(&state) {
                tracing::warn!("Lost the terminal ({e}), quitting");
                terminal_lost = true;
                runtime.effect_queue.push_back(AppMsg::RequestQuit);
                continue;
            }
            should_render = false;

            // Sync MPRIS metadata/status after render (Linux/macOS only)
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            if let Some(ref mut mgr) = mpris_manager {
                mgr.update_state(&state);
            }
        }
    }

    // Dropping MprisManager (souvlaki DBus) can block for several seconds
    // joining its background thread. The process is about to end, so leak it
    // deliberately: same effect the old `process::exit` had on it, but without
    // taking the log flush and terminal restore down as well.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    if let Some(mgr) = mpris_manager.take() {
        std::mem::forget(mgr);
    }

    match tui.shutdown() {
        Err(e) if terminal_lost => {
            tracing::debug!("Not restoring a terminal that is gone: {e}");
            Ok(())
        }
        result => result,
    }
}

/// Saves the session, logging rather than failing if it cannot.
fn save_session(state: &AppState) {
    let session = melotui_core::PersistedSession::from_app_state(state);
    match session.save() {
        Ok(()) => tracing::debug!("Session saved"),
        Err(e) => tracing::warn!("Failed to save session: {}", e),
    }
}

struct RuntimeContext {
    engine: Engine,
    engine_alive: bool,
    running: bool,
    effect_queue: VecDeque<AppMsg>,
    input_processor: InputProcessor,
    music_roots: Vec<PathBuf>,
    session_restored: bool,
    /// Whether the library is in. Until it is, the state has none of the
    /// saved session in it, and saving would overwrite that with nothing.
    library_loaded: bool,
    last_saved: Instant,
    /// Whether the scan at start has been asked for.
    startup_scan_requested: bool,
}

/// How long the music folders have to be still after a change before they
/// are scanned: a copy in progress keeps changing them.
const FOLDERS_SETTLE: Duration = Duration::from_secs(3);

/// Watches `roots` for files added, removed, renamed or rewritten, sending on
/// `changed` for each.
fn watch_folders(
    roots: &[PathBuf],
    changed: std::sync::mpsc::Sender<()>,
) -> notify::Result<notify::RecommendedWatcher> {
    use notify::{EventKind, RecursiveMode, Watcher, event::ModifyKind};
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if let Ok(event) = event
            && matches!(
                event.kind,
                EventKind::Create(_)
                    | EventKind::Remove(_)
                    | EventKind::Modify(ModifyKind::Name(_) | ModifyKind::Data(_))
            )
        {
            let _ = changed.send(());
        }
    })?;
    for root in roots.iter().filter(|root| root.exists()) {
        watcher.watch(root, RecursiveMode::Recursive)?;
    }
    Ok(watcher)
}

/// The library database, and the music directories — which may be none, in
/// which case the app starts with an empty library and says how to set them.
fn resolve_paths(config: &melotui_core::config::AppConfig) -> Result<(PathBuf, Vec<PathBuf>)> {
    let db_path = config.resolve_db_path()?;
    let music_roots = config.resolve_music_dirs();
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
                ctx.library_loaded = true;
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
                } else if state.take_quiet_scan() {
                    // A background scan found something.
                    state.set_notification(NotificationKind::Info, "Library updated");
                } else {
                    state.set_notification(NotificationKind::Info, "Library loaded");
                }

                // Music added while melotui was closed is picked up at start;
                // an empty library is a first run, so that scan shows.
                if !ctx.startup_scan_requested
                    && !ctx.music_roots.is_empty()
                    && state.config.library.scan_on_startup
                {
                    ctx.startup_scan_requested = true;
                    let scan = if state.library.tracks.is_empty() {
                        LibraryMsg::RequestScanLibrary
                    } else {
                        LibraryMsg::RequestBackgroundScan
                    };
                    ctx.effect_queue.push_back(AppMsg::Library(scan));
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
            IpcEvent::LibraryRescanned { changed } => {
                let quiet = state.is_scanning_quietly();
                if changed {
                    // Reloaded only when the scan found something, so a
                    // background scan of an unchanged library disturbs nothing.
                    ctx.engine.load_library(ctx.music_roots.clone());
                    if !quiet {
                        state.set_notification(
                            NotificationKind::Info,
                            "Scan complete, reloading...",
                        );
                    }
                } else {
                    state.take_quiet_scan();
                    if !quiet {
                        state.set_notification(NotificationKind::Info, "Library is up to date");
                    }
                }
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
            if ctx.library_loaded {
                save_session(state);
            }

            if ctx.engine_alive {
                ctx.engine.shutdown();
                ctx.engine_alive = false;
            }
            ctx.running = false;

            // Stop draining effects; the main loop breaks on `!running` and the
            // shutdown path at the end of `run` takes over. This used to call
            // `std::process::exit(0)` directly to dodge a slow MprisManager
            // drop, but that also skipped the tracing WorkerGuard, so the last
            // buffered log lines — the ones you actually want when diagnosing a
            // dropout — never reached disk. `run` now neutralises the slow drop
            // explicitly instead.
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
