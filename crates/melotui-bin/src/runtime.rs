use anyhow::Result;
use melotui_bus::{event::IpcEvent, spectrum_bridge};
use melotui_core::{
    AppMsg, AppState,
    library::Library,
    message::{library::LibraryMsg, ui::UIMsg},
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
        output_device: config.audio.output_device.clone(),
        // What a worker sends is acted on as it comes, not when the wait
        // for a key next runs out.
        wake: Some(Arc::new(tui.waker())),
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
        last_saved: Instant::now(),
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

    // Initialize MPRIS / system media controls (Linux, macOS, Windows)
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
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
    let mut frames = crate::frame_clock::FrameClock::new(Instant::now());
    let mut spectrum_moving = true;

    while runtime.running {
        let frame_time =
            Duration::from_secs_f64(1.0 / state.config.ui.fps_limit.clamp(1, 1000) as f64);
        // Animated while playing, and after until everything has come to
        // rest; then nothing moves, and nothing is drawn but what changes.
        let animating = state.should_draw_spectrum() && (state.is_playing() || spectrum_moving);
        let timeout = if animating {
            frames.wait(Instant::now())
        } else {
            Duration::from_millis(100)
        };

        if quit_requested.swap(false, Ordering::SeqCst) {
            tracing::info!("Termination signal received, quitting");
            runtime.effect_queue.push_back(AppMsg::RequestQuit);
        }

        // What has been typed, each key read as the state stands once the
        // keys before it have been acted on. A terminal that has gone away
        // quits, session saved.
        let debounce = state.config.ui.search_debounce_ms;
        let taken = take_input(
            |state, wait| tui.poll_event(state, wait),
            |messages, state| {
                let messages = runtime.input_processor.coalesce(messages, debounce);
                runtime.effect_queue.extend(messages);
                process_effects(&mut runtime, state);
                runtime.running
            },
            &mut state,
            timeout,
        );
        match taken {
            Ok(true) => should_render = true,
            Ok(false) => {}
            Err(e) => {
                tracing::warn!("Lost the terminal ({e}), quitting");
                terminal_lost = true;
                runtime.effect_queue.push_back(AppMsg::RequestQuit);
            }
        }
        if !runtime.running {
            break;
        }

        if runtime.engine_alive && handle_ipc_events(&mut runtime, &mut state) {
            if !runtime.running {
                break;
            }
            should_render = true;
        }

        // Poll MPRIS media key events (Linux, macOS, Windows)
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
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
        if tick_state(&mut state) {
            should_render = true;
        }

        let mut frame_due = false;
        if animating {
            if let Some(dt) = frames.tick(now, frame_time) {
                spectrum_moving = update_visualizer_physics(&mut state, dt.min(0.1));
                frame_due = true;
                should_render = true;
            }
        } else if !state.is_playing() {
            frames.rest(now);
        }
        // Hidden while the music plays, the clock runs on: shown again, the
        // spectrum moves on by the time it was away, and flicked in and out
        // it keeps the music's pace rather than moving only while seen.

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
            && state.library_loaded()
        {
            folders_changed_at = None;
            tracing::info!("Music folders changed, scanning");
            runtime
                .effect_queue
                .push_back(AppMsg::Library(LibraryMsg::RequestBackgroundScan));
        }

        if state.library_loaded() && runtime.last_saved.elapsed() >= AUTOSAVE_EVERY {
            save_session(&state);
            runtime.last_saved = Instant::now();
        }

        // While animating, drawn on the frame's time: what changed in between
        // waits at most a frame, and frames stay even.
        if should_render && runtime.running && (frame_due || !animating) {
            if let Err(e) = tui.draw(&state) {
                tracing::warn!("Lost the terminal ({e}), quitting");
                terminal_lost = true;
                runtime.effect_queue.push_back(AppMsg::RequestQuit);
                continue;
            }
            should_render = false;

            // Sync MPRIS metadata/status after render (Linux, macOS, Windows)
            #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
            if let Some(ref mut mgr) = mpris_manager {
                mgr.update_state(&state);
            }
        }
    }

    // Dropping MprisManager (souvlaki DBus) can block for several seconds
    // joining its background thread. The process is about to end, so leak it
    // deliberately: same effect the old `process::exit` had on it, but without
    // taking the log flush and terminal restore down as well.
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    if let Some(mgr) = mpris_manager.take() {
        std::mem::forget(mgr);
    }

    let restored = match tui.shutdown() {
        Err(e) if terminal_lost => {
            tracing::debug!("Not restoring a terminal that is gone: {e}");
            Ok(())
        }
        result => result,
    };

    // With the terminal handed back, so quitting looks as quick as it was:
    // the workers finish what they were sent — the last favorite, the last
    // play count — the database is closed, and a download's yt-dlp stopped.
    if !runtime.engine_alive {
        runtime.engine.join(WORKERS_FINISH_WITHIN);
    }
    restored
}

/// How long quitting waits for the workers to finish.
const WORKERS_FINISH_WITHIN: Duration = Duration::from_secs(3);

/// The most keys taken in one go, so that a held key cannot keep the loop
/// from drawing.
const MOST_INPUT_AT_ONCE: usize = 50;

/// Whether `msg` is one of those that merge with their like when several
/// come together — a held seek key, a held volume key — and so can wait for
/// the rest of the burst.
fn merges(msg: &AppMsg) -> bool {
    use melotui_core::message::{audio::AudioMsg, playback::PlaybackMsg};
    matches!(
        msg,
        AppMsg::Playback(PlaybackMsg::RequestSeekRelative(_) | PlaybackMsg::RequestSeekAbsolute(_))
            | AppMsg::Audio(AudioMsg::AdjustVolume(_))
            | AppMsg::Library(LibraryMsg::AddToQueue(_))
    )
}

/// Takes what has been typed: waits up to `timeout` for the first key, then
/// takes whatever else is waiting. `poll` reads a key as the state stands;
/// `apply` acts on messages, and says whether the app is still running.
/// `true` if anything was typed.
///
/// What a key means depends on the state — `f` is a favorite in the tree
/// and a letter in the command bar — so each is read only once those before
/// it have been acted on. They used to be read all together, as the state
/// stood before any of them: `:find`, arriving at once, opened the command
/// bar and then made a favorite, opened the track's info and stepped the
/// search, leaving `d` typed.
fn take_input(
    mut poll: impl FnMut(&AppState, Duration) -> Result<Option<AppMsg>>,
    mut apply: impl FnMut(Vec<AppMsg>, &mut AppState) -> bool,
    state: &mut AppState,
    timeout: Duration,
) -> Result<bool> {
    let mut any = false;
    // Seeks and volume steps, held back to be merged into one.
    let mut held: Vec<AppMsg> = Vec::new();
    for taken in 0..=MOST_INPUT_AT_ONCE {
        let wait = if taken == 0 { timeout } else { Duration::ZERO };
        let Some(msg) = poll(state, wait)? else {
            break;
        };
        any = true;
        if merges(&msg) {
            held.push(msg);
            continue;
        }
        // In the order typed: what was held goes first.
        let mut now = std::mem::take(&mut held);
        now.push(msg);
        if !apply(now, state) {
            return Ok(true);
        }
    }
    if !held.is_empty() {
        apply(held, state);
    }
    Ok(any)
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
    last_saved: Instant,
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

fn tick_state(state: &mut AppState) -> bool {
    let mut changed = false;

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
            // The session file is read here, at the edge, the first time
            // the library arrives; what is done with it is the reducer's.
            IpcEvent::LibraryLoaded { snapshot } => {
                let mut session = None;
                if !ctx.session_restored && state.config.ui.restore_session {
                    ctx.session_restored = true;
                    match melotui_core::PersistedSession::load() {
                        Ok(saved) => session = Some(Box::new(saved)),
                        Err(e) => {
                            // No file is a first run, and nothing to remark
                            // on; a file that cannot be read is a session
                            // lost, which should at least be said.
                            let there = melotui_core::PersistedSession::file_path()
                                .is_ok_and(|path| path.exists());
                            if there {
                                tracing::warn!("Session not restored: {e:#}");
                            }
                        }
                    }
                }
                ctx.effect_queue
                    .push_back(AppMsg::Library(LibraryMsg::Loaded { snapshot, session }));
            }
            IpcEvent::LibraryGrew { snapshot } => ctx
                .effect_queue
                .push_back(AppMsg::Library(LibraryMsg::Grew(snapshot))),
            IpcEvent::LibraryRescanned { changed } => ctx
                .effect_queue
                .push_back(AppMsg::Library(LibraryMsg::Rescanned { changed })),
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
            if state.library_loaded() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use melotui_core::state::UIMode;
    use melotui_tui::{input::InputHandler, keybinding::KeyRegistry};
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn state() -> AppState {
        let library = Library::new(vec![], vec![], HashMap::new());
        AppState::new(library, &melotui_core::config::AppConfig::default())
    }

    /// Types `keys` all at once, as a paste or a quick hand does, through
    /// the app's own keys and reducer. What the reducer was given, in order.
    fn typed(state: &mut AppState, keys: &str) -> Vec<String> {
        let registry = KeyRegistry::default();
        let mut waiting: VecDeque<char> = keys.chars().collect();
        let mut given = Vec::new();
        take_input(
            |state, _| {
                Ok(waiting.pop_front().and_then(|c| {
                    let key = KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
                    // A key that means nothing here is passed over, as the
                    // terminal's reader passes over it.
                    InputHandler::handle(key, state, &registry).or(Some(AppMsg::Noop))
                }))
            },
            |messages, state| {
                let mut queue: VecDeque<AppMsg> = messages.into();
                while let Some(msg) = queue.pop_front() {
                    given.push(format!("{msg:?}"));
                    queue.extend(reducer::reduce(state, msg));
                }
                true
            },
            state,
            Duration::ZERO,
        )
        .unwrap();
        given
    }

    #[test]
    fn keys_typed_with_the_one_that_opens_the_command_bar_are_typed_into_it() {
        let mut state = state();
        let given = typed(&mut state, ":find");

        assert_eq!(*state.ui_mode(), UIMode::Command);
        assert_eq!(state.ui_command_buffer().value(), "find");
        // None of them was taken for the key it is elsewhere: f a favorite,
        // i the track's info, n the next match.
        for shortcut in ["Favorite", "TrackInfo", "SearchNext"] {
            assert!(
                !given.iter().any(|msg| msg.contains(shortcut)),
                "{shortcut} among {given:?}"
            );
        }
    }

    #[test]
    fn the_search_key_and_what_follows_it_are_one_search() {
        let mut state = state();
        typed(&mut state, "/kuch");
        assert_eq!(state.ui_command_buffer().value(), "find kuch");
    }

    #[test]
    fn held_seek_and_volume_keys_are_still_merged_and_kept_in_order() {
        use melotui_core::message::{audio::AudioMsg, playback::PlaybackMsg};
        let seek = || AppMsg::Playback(PlaybackMsg::RequestSeekRelative(5));
        let mut waiting: VecDeque<AppMsg> = vec![
            seek(),
            seek(),
            seek(),
            AppMsg::Audio(AudioMsg::ToggleMute),
            seek(),
        ]
        .into();
        let mut batches: Vec<usize> = Vec::new();
        let mut state = state();
        let any = take_input(
            |_, _| Ok(waiting.pop_front()),
            |messages, _| {
                batches.push(messages.len());
                true
            },
            &mut state,
            Duration::ZERO,
        )
        .unwrap();
        assert!(any);
        // The three seeks go with the key that ended their run, to be merged
        // ahead of it; the last seek after.
        assert_eq!(batches, [4, 1]);
    }

    #[test]
    fn quitting_stops_the_rest_from_being_read() {
        let mut waiting: VecDeque<AppMsg> =
            vec![AppMsg::RequestQuit, AppMsg::Noop, AppMsg::Noop].into();
        let mut state = state();
        take_input(
            |_, _| Ok(waiting.pop_front()),
            |_, _| false,
            &mut state,
            Duration::ZERO,
        )
        .unwrap();
        assert_eq!(waiting.len(), 2, "read on after quitting");
    }

    #[test]
    fn a_held_key_does_not_keep_the_loop_for_ever() {
        let mut polled = 0;
        let mut state = state();
        take_input(
            |_, _| {
                polled += 1;
                Ok(Some(AppMsg::Noop))
            },
            |_, _| true,
            &mut state,
            Duration::ZERO,
        )
        .unwrap();
        assert_eq!(polled, MOST_INPUT_AT_ONCE + 1);
    }
}
