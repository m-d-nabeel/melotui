use std::{collections::VecDeque, time::Instant};

use melotui_core::{
    AppMsg, AppState, Library,
    state_registry::{SpectrumStyle, VisualizerMode},
};
use melotui_tui::Tui;

use crate::ipc_bridge::{IpcRuntime, appmsg_to_ipc, ipc_to_appmsg};

pub fn run() -> anyhow::Result<()> {
    tracing::info!("Starting melotui application");

    let music_root = dirs::audio_dir().expect("Music directory not found");
    let library = Library::scan(music_root)?;
    let mut state = AppState::new(library);
    let mut tui = Tui::new()?;

    // Initialize the Runtime Supervisor
    let ipc = IpcRuntime::new();
    let mut ipc_alive = true;

    let mut running = true;
    let mut effect_queue = VecDeque::new();
    let mut last_tick = Instant::now();

    while running {
        let now = Instant::now();
        let dt = now.duration_since(last_tick).as_secs_f32();
        last_tick = now;

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
        if let Some(n) = &state.notification.current
            && let Some(timeout) = n.dismiss_after
            && n.created_at.elapsed() >= timeout
        {
            state.notification.current = None;
        }
    }

    tui.shutdown()
}

fn update_visualizer_physics(state: &mut AppState, dt: f32) {
    if state.visualizer_mode != VisualizerMode::Spectrum {
        return;
    }

    // Ensure buffers are the correct size
    let len = state.spectrum_raw.len();
    if state.spectrum_smooth.len() != len {
        state.spectrum_smooth.resize(len, 0.0);
    }
    if state.spectrum_peaks.len() != len {
        state.spectrum_peaks.resize(len, 0.0);
    }

    // 1. CAVA Mode: Direct Pass-through
    if state.spectrum_style == SpectrumStyle::Cava {
        state.spectrum_smooth.copy_from_slice(&state.spectrum_raw);
        state.spectrum_peaks.copy_from_slice(&state.spectrum_raw);
        return;
    }

    // 2. Internal Mode: Apply Physics (Smoothing + Gravity)
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
