use std::collections::VecDeque;

use melotui_core::{AppMsg, AppState, Library};
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

    while running {
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
