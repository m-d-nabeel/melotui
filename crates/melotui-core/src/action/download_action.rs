use std::{
    fs,
    time::{Duration, Instant},
};

use crate::{
    AppMsg, AppState,
    message::{download::DownloadMsg, library::LibraryMsg},
    state::{DownloadJob, DownloadStatus, Notification, NotificationKind},
};

pub fn handle_request_download(state: &mut AppState, query: String) -> Vec<AppMsg> {
    tracing::info!("RequestDownload called with: {:?}", query);
    enqueue_download(state, query)
}

fn enqueue_download(state: &mut AppState, query: String) -> Vec<AppMsg> {
    let root = &state.library.root;
    let download_dir = root.join("Downloads");

    if !download_dir.exists()
        && let Err(e) = fs::create_dir_all(&download_dir)
    {
        state.notification.current = Some(Notification {
            kind: NotificationKind::Error,
            message: format!("Failed to create Downloads folder: {e}"),
            progress: None,
            created_at: Instant::now(),
            dismiss_after: Some(Duration::from_secs(4)),
        });
        return vec![];
    }

    state.download.queue.push_back(DownloadJob {
        query: query.clone(),
        download_path: download_dir,
    });

    state.notification.current = Some(Notification {
        kind: NotificationKind::Info,
        message: format!("Queued: {}", query),
        progress: None,
        created_at: Instant::now(),
        dismiss_after: Some(Duration::from_secs(2)),
    });

    if matches!(state.download.status, DownloadStatus::Idle) {
        return vec![AppMsg::Download(DownloadMsg::BeginNextDownload)];
    }
    vec![]
}

pub fn handle_begin_next_download(state: &mut AppState) -> Vec<AppMsg> {
    let job = match state.download.queue.pop_front() {
        Some(job) => job,
        None => return vec![],
    };
    tracing::info!("Next Download Started with: {:?}", job.query);
    state.download.status = DownloadStatus::Running { percent: 0 };
    vec![AppMsg::Download(DownloadMsg::DispatchDownload {
        query: job.query,
        download_path: job.download_path,
    })]
}

pub fn handle_download_progress(state: &mut AppState, percent: u8) -> Vec<AppMsg> {
    state.notification.current = Some(Notification {
        kind: NotificationKind::Progress,
        message: "".to_owned(),
        progress: Some(percent),
        created_at: Instant::now(),
        dismiss_after: None,
    });
    if let DownloadStatus::Running { .. } = &state.download.status {
        state.download.status = DownloadStatus::Running { percent };
    }
    vec![]
}

pub fn handle_download_finished(state: &mut AppState) -> Vec<AppMsg> {
    state.notification.current = Some(Notification {
        kind: NotificationKind::Info,
        message: "Download complete".into(),
        progress: Some(100),
        created_at: Instant::now(),
        dismiss_after: Some(Duration::from_secs(2)),
    });
    state.download.status = DownloadStatus::Idle;
    vec![
        AppMsg::Library(LibraryMsg::RequestRescanLibrary),
        AppMsg::Download(DownloadMsg::BeginNextDownload),
    ]
}

pub fn handle_download_failed(state: &mut AppState, error: String) -> Vec<AppMsg> {
    state.notification.current = Some(Notification {
        kind: NotificationKind::Error,
        message: error.clone(),
        progress: None,
        created_at: Instant::now(),
        dismiss_after: Some(Duration::from_secs(4)),
    });
    state.download.status = DownloadStatus::Idle;
    vec![AppMsg::Download(DownloadMsg::BeginNextDownload)]
}

pub fn handle_request_cancel_download(state: &mut AppState) -> Vec<AppMsg> {
    if let DownloadStatus::Running { .. } = &state.download.status {
        state.download.status = DownloadStatus::Cancelling;
    }
    vec![AppMsg::Download(DownloadMsg::RequestCancelDownload)]
}

pub fn handle_dispatch_download(
    _state: &mut AppState,
    _query: String,
    _download_path: std::path::PathBuf,
) -> Vec<AppMsg> {
    vec![]
}
