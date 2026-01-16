use std::{collections::VecDeque, time::Instant};

use melotui_core::{AppMsg, AppState, state::SpectrumStyle};
use melotui_db::load_library_from_db;
use melotui_ipc::{event::IpcEvent, spectrum_bridge};
use melotui_tui::Tui;

use crate::ipc_bridge::{IpcRuntime, appmsg_to_ipc, ipc_to_appmsg};

pub fn run() -> anyhow::Result<()> {
    tracing::info!("Starting melotui application");

    let db_root = dirs::data_dir()
        .expect("Could not determine data directory")
        .join("melotui");
    std::fs::create_dir_all(&db_root).expect("Failed to create app data directory");

    let db_path = db_root.join("library.db");
    let music_root = dirs::audio_dir().expect("Music directory not found");

    let library = match load_library_from_db(&db_path, music_root.clone()) {
        Ok(lib) => {
            tracing::info!(
                "Loaded {} tracks from database",
                lib.playlists.first().map(|p| p.tracks.len()).unwrap_or(0)
            );
            lib
        }
        Err(e) => {
            tracing::warn!(
                "Failed to load from DB (fresh install?): {}. Starting empty.",
                e
            );
            melotui_core::Library {
                root: music_root.clone(),
                playlists: vec![],
            }
        }
    };

    let mut state = AppState::new(library);
    let mut tui = Tui::new()?;

    let (producer, consumer) = spectrum_bridge::create_bridge();
    let ipc = IpcRuntime::new(producer, db_root.join("library.db"));

    state.spectrum_source = Box::new(consumer);
    let mut ipc_alive = true;
    let mut running = true;
    let mut effect_queue = VecDeque::new();
    let mut last_tick = Instant::now();

    while running {
        let now = Instant::now();
        let dt = now.duration_since(last_tick).as_secs_f32();
        last_tick = now;

        state.time_elapsed += dt;

        update_visualizer_physics(&mut state, dt);
        tui.draw(&state)?;

        // 1. UI Input
        if let Some(msg) = tui.poll_event(&state)? {
            effect_queue.push_back(msg);
        }

        if ipc_alive {
            while let Ok(evt) = ipc.event_rx.try_recv() {
                match evt {
                    // NEW: Hot-reload the library when scan finishes
                    IpcEvent::LibraryRescanned => {
                        tracing::info!("Database updated. Reloading library into memory...");

                        // 1. Load from DB
                        match load_library_from_db(&db_root.join("library.db"), music_root.clone())
                        {
                            Ok(new_lib) => {
                                // 2. Hot-Swap State
                                state.library = std::sync::Arc::new(new_lib);
                                melotui_core::reconcile_nav_state(&mut state);

                                // 3. Notify User
                                state.set_notification(melotui_core::state::Notification {
                                    kind: melotui_core::state::NotificationKind::Info,
                                    message: "Library Updated".into(),
                                    progress: None,
                                    created_at: Instant::now(),
                                    dismiss_after: Some(std::time::Duration::from_secs(3)),
                                });
                            }
                            Err(e) => {
                                tracing::error!("Failed to reload library: {}", e);
                            }
                        }
                    }
                    // Pass other events to the reducer
                    _ => effect_queue.push_back(ipc_to_appmsg(evt)),
                }
            }
        }

        // 3. Process Effects
        while let Some(msg) = effect_queue.pop_front() {
            // Lifecycle Check
            if matches!(msg, AppMsg::RequestQuit) {
                if ipc_alive {
                    ipc.dispatch(melotui_ipc::command::IpcCommand::Shutdown);
                    ipc_alive = false;
                }
                running = false;
                break;
            }

            // A) Dispatch Side Effects (UI -> Runtime)
            if ipc_alive && let Some(cmd) = appmsg_to_ipc(&msg) {
                ipc.dispatch(cmd);
            }

            // B) State Mutation
            let next_effects = melotui_core::reducer::reduce(&mut state, msg);
            effect_queue.extend(next_effects);
        }

        // 4. Cleanup Notifications
        if let Some(n) = state.current_notification()
            && let Some(timeout) = n.dismiss_after
            && n.created_at.elapsed() >= timeout
        {
            state.clear_current_notification();
        }
    }

    tui.shutdown()
}

fn update_visualizer_physics(state: &mut AppState, dt: f32) {
    if !state.should_draw_spectrum() {
        return;
    }

    // 1. PULL Strategy
    // Ensure we have enough capacity for the largest backend (512 is safe)
    // We resize UP to 512 so the buffer is big enough to receive data.
    if state.spectrum_raw.len() < 512 {
        state.spectrum_raw.resize(512, 0.0);
    }

    // Read the data. 'count' is the actual number of bins written (e.g. 200 or 512).
    let count = state.spectrum_source.get_latest(&mut state.spectrum_raw);

    // Resize DOWN to the actual data length.
    // This ensures the visualizer calculates 'step' correctly to fill the screen.
    if count > 0 {
        state.spectrum_raw.truncate(count);
    }

    // 2. Resize Render Buffers
    let len = state.spectrum_raw.len();
    if state.spectrum_smooth.len() != len {
        state.spectrum_smooth.resize(len, 0.0);
    }
    if state.spectrum_peaks.len() != len {
        state.spectrum_peaks.resize(len, 0.0);
    }

    // 3. CAVA Mode: Direct Pass-through
    if *state.spectrum_style() == SpectrumStyle::Cava {
        state.spectrum_smooth.copy_from_slice(&state.spectrum_raw);
        // Copy peaks logic for CAVA if you want "falling peaks" on CAVA too,
        // otherwise just direct copy:
        for i in 0..len {
            if state.spectrum_smooth[i] > state.spectrum_peaks[i] {
                state.spectrum_peaks[i] = state.spectrum_smooth[i];
            } else {
                state.spectrum_peaks[i] -= 2.0 * dt; // Simple gravity
                if state.spectrum_peaks[i] < 0.0 {
                    state.spectrum_peaks[i] = 0.0;
                }
            }
        }
        return;
    }

    // 4. Internal Mode: Apply Physics (Smoothing + Gravity)
    let gravity = 4.0;
    let smoothing_speed = 15.0;

    for i in 0..len {
        let target = state.spectrum_raw[i];

        // Simple Lerp
        let diff = target - state.spectrum_smooth[i];
        state.spectrum_smooth[i] += diff * (smoothing_speed * dt).min(1.0);

        // Peaks
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
