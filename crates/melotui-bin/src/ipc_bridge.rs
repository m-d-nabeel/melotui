use melotui_core::AppMsg;
use melotui_ipc::{command::IpcCommand, event::IpcEvent};

/// UI → Worker
pub fn appmsg_to_ipc(msg: &AppMsg) -> Option<IpcCommand> {
    match msg {
        AppMsg::StartDownload { query } => Some(IpcCommand::Download {
            query: query.clone(),
        }),

        AppMsg::CancelDownload => Some(IpcCommand::CancelDownload),

        AppMsg::Quit => Some(IpcCommand::Shutdown),

        _ => None,
    }
}

/// Worker → UI
pub fn ipc_to_appmsg(evt: IpcEvent) -> AppMsg {
    match evt {
        IpcEvent::DownloadStarted { query } => AppMsg::StartDownload { query },

        IpcEvent::DownloadProgress { percent } => AppMsg::DownloadProgress { percent },

        IpcEvent::DownloadFinished => AppMsg::DownloadFinished,

        IpcEvent::DownloadFailed { error } => AppMsg::DownloadFailed { error },

        IpcEvent::Error { message } => AppMsg::DownloadFailed { error: message },

        _ => AppMsg::Noop,
    }
}
