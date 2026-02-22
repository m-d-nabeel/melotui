use std::fs;

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
        state.session.notification.current = Some(
            Notification::new(
                NotificationKind::Error,
                format!("Failed to create Downloads folder: {e}"),
            )
            .with_duration(4),
        );
        return vec![];
    }

    state.session.download.queue.push_back(DownloadJob {
        query: query.clone(),
        download_path: download_dir,
    });

    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("Queued: {}", query),
    ));

    if matches!(state.session.download.status, DownloadStatus::Idle) {
        return vec![AppMsg::Download(DownloadMsg::BeginNextDownload)];
    }
    vec![]
}

pub fn handle_begin_next_download(state: &mut AppState) -> Vec<AppMsg> {
    let job = match state.session.download.queue.pop_front() {
        Some(job) => job,
        None => return vec![],
    };
    tracing::info!("Next Download Started with: {:?}", job.query);
    state.session.download.status = DownloadStatus::Running { percent: 0 };
    vec![AppMsg::Download(DownloadMsg::DispatchDownload {
        query: job.query,
        download_path: job.download_path,
    })]
}

pub fn handle_download_progress(state: &mut AppState, percent: u8) -> Vec<AppMsg> {
    state.session.notification.current =
        Some(Notification::new(NotificationKind::Progress, "").with_progress(percent));
    if let DownloadStatus::Running { .. } = &state.session.download.status {
        state.session.download.status = DownloadStatus::Running { percent };
    }
    vec![]
}

pub fn handle_download_finished(state: &mut AppState) -> Vec<AppMsg> {
    state.session.notification.current = Some(
        Notification::new(NotificationKind::Info, "Download complete")
            .with_progress(100)
            .with_duration(2),
    );
    state.session.download.status = DownloadStatus::Idle;
    vec![
        AppMsg::Library(LibraryMsg::RequestScanLibrary),
        AppMsg::Download(DownloadMsg::BeginNextDownload),
    ]
}

pub fn handle_download_failed(state: &mut AppState, error: String) -> Vec<AppMsg> {
    state.session.notification.current =
        Some(Notification::new(NotificationKind::Error, error).with_duration(4));
    state.session.download.status = DownloadStatus::Idle;
    vec![AppMsg::Download(DownloadMsg::BeginNextDownload)]
}

pub fn handle_request_cancel_download(state: &mut AppState) -> Vec<AppMsg> {
    if let DownloadStatus::Running { .. } = &state.session.download.status {
        state.session.download.status = DownloadStatus::Cancelling;
        vec![AppMsg::Download(DownloadMsg::RequestCancelDownload)]
    } else {
        vec![]
    }
}

pub fn handle_dispatch_download(
    _state: &mut AppState,
    _query: String,
    _download_path: std::path::PathBuf,
) -> Vec<AppMsg> {
    vec![]
}
