use melotui_core::{AppMsg, AppState, Library};
use melotui_ipc::{IpcReceiver as _, IpcSender as _, channel::inprocess::channel};
use melotui_tui::Tui;

use crate::ipc_bridge::{appmsg_to_ipc, ipc_to_appmsg};

use std::{collections::VecDeque, thread};

pub fn run() -> anyhow::Result<()> {
    tracing::info!("Starting melotui application");

    let music_root = dirs::audio_dir().expect("Music directory not found");

    let library = Library::scan(music_root)?;
    let mut state = AppState::new(library);
    let mut tui = Tui::new()?;

    let (mut ui_ipc, worker_ipc) = channel();

    thread::spawn(move || {
        melotui_download::worker::run(worker_ipc);
    });

    let mut ipc_alive = true;
    let mut running = true;
    let mut effect_queue = VecDeque::new();

    while running {
        tui.draw(&state)?;

        // 1. Gather Input Sources (UI + IPC)
        // We poll UI for a short duration to keep the loop ticking
        if let Some(msg) = tui.poll_event(&state)? {
            effect_queue.push_back(msg);
        }

        // Drain ALL pending IPC events into the queue immediately
        if ipc_alive {
            while let Some(evt) = ui_ipc.try_recv() {
                effect_queue.push_back(ipc_to_appmsg(evt));
            }
        }

        // 2. Process the Effect Queue
        // We keep processing until the queue is empty.
        // This ensures that an event triggering an effect (e.g. Finish -> Next)
        // happens in the same frame or shortly after.
        while let Some(msg) = effect_queue.pop_front() {
            // Check for Quit signal
            if matches!(msg, AppMsg::RequestQuit) {
                tracing::info!("Quit requested by UI");
                if ipc_alive {
                    ui_ipc.send(melotui_ipc::command::IpcCommand::Shutdown);
                    ipc_alive = false; // Prevent sending further commands
                }
                running = false;
                break;
            }

            // A) Side Effects (Network/IPC)
            // If the message translates to an IPC command, send it.
            if ipc_alive && let Some(cmd) = appmsg_to_ipc(&msg) {
                ui_ipc.send(cmd);
            }

            // B) State Mutation (Reducer)
            // Apply the message to the state, getting back new resulting messages
            let next_effects = melotui_core::reducer::reduce(&mut state, msg);

            // C) Re-queue resulting effects
            // This fixes your bug: events returned by reducer are added to the back
            // of the queue to be processed in subsequent iterations of this loop.
            effect_queue.extend(next_effects);
        }

        // 3. Time-based updates (Notifications)
        if let Some(n) = &state.notification.current
            && let Some(timeout) = n.dismiss_after
            && n.created_at.elapsed() >= timeout
        {
            state.notification.current = None;
        }
    }

    tracing::info!("Shutting down TUI");
    tui.shutdown()
}
