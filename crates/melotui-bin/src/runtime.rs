use std::{collections::VecDeque, time::Instant};

use melotui_core::{
    AppMsg, AppState, Library,
    state::{SpectrumStyle, VisualizerMode},
};
use melotui_ipc::spectrum_bridge;
use melotui_tui::Tui;

use crate::ipc_bridge::{IpcRuntime, appmsg_to_ipc, ipc_to_appmsg};

pub fn run() -> anyhow::Result<()> {
    tracing::info!("Starting melotui application");

    let music_root = dirs::audio_dir().expect("Music directory not found");
    let library = Library::scan(music_root)?;
    let mut state = AppState::new(library);
    let mut tui = Tui::new()?;

    // 1. Create the Bridge
    let (producer, consumer) = spectrum_bridge::create_bridge();

    // 2. Install Consumer into UI State
    state.spectrum_source = Box::new(consumer);

    // 3. Pass Producer to IPC Runtime (which passes it to Audio Worker)
    let ipc = IpcRuntime::new(producer);
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

        // 2. Runtime Input (Single Event Stream)
        if ipc_alive {
            while let Ok(evt) = ipc.event_rx.try_recv() {
                effect_queue.push_back(ipc_to_appmsg(evt));
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
        if let Some(n) = &state.session.notification.current
            && let Some(timeout) = n.dismiss_after
            && n.created_at.elapsed() >= timeout
        {
            state.session.notification.current = None;
        }
    }

    tui.shutdown()
}

fn update_visualizer_physics(state: &mut AppState, dt: f32) {
    if state.session.visualizer_mode != VisualizerMode::Spectrum {
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
    if state.session.spectrum_style == SpectrumStyle::Cava {
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
