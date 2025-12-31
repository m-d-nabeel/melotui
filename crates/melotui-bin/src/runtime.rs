use melotui_core::{AppState, Library};
use melotui_ipc::{IpcReceiver as _, IpcSender as _, channel::inprocess::channel};
use melotui_tui::Tui;

use crate::ipc_bridge::{appmsg_to_ipc, ipc_to_appmsg};

use std::thread;

pub fn run() -> anyhow::Result<()> {
    let music_root = match dirs::audio_dir() {
        Some(music_root) => music_root,
        None => {
            panic!("Root directory for music does not exist");
        }
    };

    let library = Library::scan(music_root)?;
    let mut tui = Tui::new()?;
    let mut state = AppState::new(library); // you should add this ctor soon
    let (ui_ipc, worker_ipc) = channel();

    thread::spawn(move || {
        melotui_download::worker::run(worker_ipc);
    });

    loop {
        tui.draw(&state)?;

        if let Some(msg) = tui.poll_event(&state)? {
            if let Some(cmd) = appmsg_to_ipc(&msg) {
                ui_ipc.send(cmd);
            }

            melotui_core::reducer::reduce(&mut state, msg);
        }

        while let Some(evt) = ui_ipc.try_recv() {
            let app_msg = ipc_to_appmsg(evt);
            melotui_core::reducer::reduce(&mut state, app_msg);
        }
    }

    // tui.shutdown()?;
}
