use melotui_core::AppMsg;
use melotui_ipc::{command::IpcCommand, event::IpcEvent};

/// UI → Worker
///
/// Only SIDE-EFFECT TRIGGERS are sent over IPC
pub fn appmsg_to_ipc(msg: &AppMsg) -> Option<IpcCommand> {
    match msg {
        // Side-effect boundary
        AppMsg::DispatchDownload {
            query,
            download_path,
        } => Some(IpcCommand::Download {
            query: query.clone(),
            download_path: download_path.clone(),
        }),

        AppMsg::RequestCancelDownload => Some(IpcCommand::CancelDownload),

        AppMsg::RequestQuit => Some(IpcCommand::Shutdown),

        _ => None,
    }
}

/// Worker → UI
///
/// Only EXTERNAL FACTS come back
pub fn ipc_to_appmsg(evt: IpcEvent) -> AppMsg {
    match evt {
        // Optional – informational only
        IpcEvent::DownloadStarted { .. } => AppMsg::Noop,

        IpcEvent::DownloadProgress { percent } => AppMsg::DownloadProgress { percent },

        IpcEvent::DownloadFinished => AppMsg::DownloadFinished,

        IpcEvent::DownloadFailed { error } => AppMsg::DownloadFailed { error },

        IpcEvent::Error { message } => AppMsg::DownloadFailed { error: message },

        // Playback events will map later
        _ => AppMsg::Noop,
    }
}
