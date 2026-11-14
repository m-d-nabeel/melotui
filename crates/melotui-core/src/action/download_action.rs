use std::fs;

use crate::{
    AppMsg, AppState,
    message::{download::DownloadMsg, library::LibraryMsg},
    state::{
        DownloadCandidate, DownloadJob, DownloadSearch, DownloadSource, DownloadStatus,
        Notification, NotificationKind,
    },
};

/// A link is downloaded as it is; anything else is searched for, and what
/// to download picked from what is found.
pub fn handle_request_download(state: &mut AppState, query: String) -> Vec<AppMsg> {
    tracing::info!("RequestDownload called with: {:?}", query);
    let query = query.trim().to_string();
    if is_link(&query) {
        let named = query.clone();
        return enqueue_download(state, query, &named);
    }

    // Said before searching, not after something has been picked.
    if state.config.download_dir(&state.library.roots).is_none() {
        nowhere_to_download_to(state);
        return vec![];
    }

    let download = &mut state.session.download;
    download.searches += 1;
    let id = download.searches;
    // A search already open gives way to this one.
    download.search = Some(DownloadSearch {
        id,
        query: query.clone(),
        searching: true,
        ..Default::default()
    });
    vec![AppMsg::Download(DownloadMsg::DispatchSearchDownloads {
        id,
        query,
    })]
}

fn is_link(query: &str) -> bool {
    query.starts_with("http://") || query.starts_with("https://")
}

/// The search open, if it is the search `id`.
fn search_open(state: &mut AppState, id: u64) -> Option<&mut DownloadSearch> {
    state
        .session
        .download
        .search
        .as_mut()
        .filter(|search| search.id == id)
}

pub fn handle_download_search_result(
    state: &mut AppState,
    id: u64,
    candidate: DownloadCandidate,
) -> Vec<AppMsg> {
    let Some(search) = search_open(state, id) else {
        return vec![];
    };
    let results = match candidate.source {
        DownloadSource::Songs => &mut search.songs,
        DownloadSource::Videos => &mut search.videos,
    };
    if !results.iter().any(|found| found.url == candidate.url) {
        results.push(candidate);
    }
    show_a_tab_with_results(search);
    vec![]
}

/// Songs are shown first, and videos if those are all there is: until a tab
/// or row is chosen by hand, after which nothing moves by itself.
fn show_a_tab_with_results(search: &mut DownloadSearch) {
    if search.moved {
        return;
    }
    search.tab = if search.songs.is_empty() && !search.videos.is_empty() {
        DownloadSource::Videos
    } else {
        DownloadSource::Songs
    };
}

pub fn handle_download_search_finished(
    state: &mut AppState,
    id: u64,
    error: Option<String>,
) -> Vec<AppMsg> {
    let Some(search) = search_open(state, id) else {
        return vec![];
    };
    search.searching = false;
    if !search.songs.is_empty() || !search.videos.is_empty() {
        return vec![];
    }
    // Nothing to pick from: the list closes and says why.
    let said = match error {
        Some(error) => Notification::new(NotificationKind::Error, error),
        None => Notification::new(
            NotificationKind::Warning,
            format!("Nothing found for: {}", search.query),
        ),
    };
    state.session.download.search = None;
    state.session.notification.current = Some(said.with_duration(4));
    vec![]
}

pub fn handle_download_picker_move(state: &mut AppState, by: i64) -> Vec<AppMsg> {
    if let Some(search) = state.session.download.search.as_mut() {
        let last = search.shown().len().saturating_sub(1) as i64;
        search.row = (search.row as i64 + by).clamp(0, last) as usize;
        search.moved = true;
    }
    vec![]
}

pub fn handle_download_picker_switch_tab(state: &mut AppState) -> Vec<AppMsg> {
    if let Some(search) = state.session.download.search.as_mut() {
        search.tab = search.tab.other();
        search.row = 0;
        search.moved = true;
    }
    vec![]
}

/// Downloads the result selected and closes the list. With nothing selected
/// yet — the search still under way — the list stays as it is.
pub fn handle_download_picker_submit(state: &mut AppState) -> Vec<AppMsg> {
    let Some(picked) = state
        .session
        .download
        .search
        .as_ref()
        .and_then(|search| search.selected().cloned())
    else {
        return vec![];
    };
    state.session.download.search = None;
    let named = match &picked.by {
        Some(by) => format!("{} – {}", by, picked.title),
        None => picked.title,
    };
    let mut msgs = vec![AppMsg::Download(DownloadMsg::CancelSearchDownloads)];
    msgs.extend(enqueue_download(state, picked.url, &named));
    msgs
}

pub fn handle_download_picker_cancel(state: &mut AppState) -> Vec<AppMsg> {
    if state.session.download.search.take().is_some() {
        return vec![AppMsg::Download(DownloadMsg::CancelSearchDownloads)];
    }
    vec![]
}

fn nowhere_to_download_to(state: &mut AppState) {
    state.session.notification.current = Some(
        Notification::new(
            NotificationKind::Error,
            "No music directory configured for downloads",
        )
        .with_duration(4),
    );
}

/// Puts `query` in the queue of downloads, saying so by the name `named`.
fn enqueue_download(state: &mut AppState, query: String, named: &str) -> Vec<AppMsg> {
    let download_dir = match state.config.download_dir(&state.library.roots) {
        Some(dir) => dir,
        None => {
            nowhere_to_download_to(state);
            return vec![];
        }
    };
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
        query,
        download_path: download_dir,
    });

    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("Queued: {named}"),
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
        cookies_from_browser: state.config.library.download_cookies_from_browser.clone(),
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

pub fn handle_request_search_lyrics(state: &mut AppState, query: String) -> Vec<AppMsg> {
    tracing::info!("RequestSearchLyrics called with: {:?}", query);
    state.session.notification.current = Some(Notification::new(
        NotificationKind::Info,
        format!("Searching lyrics for: {}", query),
    ));

    // Auto switch to Lyrics View tab (which acts as the selector)
    crate::action::playback_action::lyrics_panel_set_by_hand(state, true);

    // The search itself is asked of the download worker by the engine's
    // bridge, from the message this answers.
    vec![]
}

pub fn handle_lyrics_search_results(
    state: &mut AppState,
    lrclib_results: Vec<crate::state::lyrics::LyricsCandidate>,
    ovh_results: Vec<crate::state::lyrics::LyricsCandidate>,
) -> Vec<AppMsg> {
    tracing::info!(
        "Received lyrics search results: {} lrclib, {} ovh",
        lrclib_results.len(),
        ovh_results.len()
    );

    if lrclib_results.is_empty() && ovh_results.is_empty() {
        state.session.notification.current = Some(
            Notification::new(NotificationKind::Warning, "No lyrics found for your search")
                .with_duration(4),
        );
        state.session.lyrics.search_state = None;
    } else {
        state.session.lyrics.search_state = Some(crate::state::lyrics::LyricsSearchState {
            lrclib_results,
            ovh_results,
            selected_column: 0,
            selected_row: 0,
        });
        state.session.notification.current = Some(
            Notification::new(
                NotificationKind::Info,
                "Search complete. Select lyrics using arrow keys.",
            )
            .with_duration(3),
        );
    }

    vec![]
}

pub fn handle_dispatch_download(
    _state: &mut AppState,
    _query: String,
    _download_path: std::path::PathBuf,
) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_lyrics_fetched(state: &mut AppState, save_path: std::path::PathBuf) -> Vec<AppMsg> {
    // Fetched as a song started, this one or one before: the panel opening
    // says it.
    if !state.session.lyrics.fetching_unasked.remove(&save_path) {
        state.session.notification.current = Some(
            Notification::new(NotificationKind::Info, "Lyrics downloaded successfully!")
                .with_duration(3),
        );
        crate::action::playback_action::lyrics_panel_set_by_hand(state, true);
    }

    // Load them for the song playing, without restarting it.
    if let Some(track) = state.get_playing_track() {
        return crate::action::playback_action::handle_audio_started(
            state,
            track.file_path.clone().to_string_lossy().to_string(),
        );
    }

    vec![]
}

pub fn handle_lyrics_fetch_failed(
    state: &mut AppState,
    save_path: std::path::PathBuf,
    error: String,
) -> Vec<AppMsg> {
    if state.session.lyrics.fetching_unasked.remove(&save_path) {
        // Not every song has lyrics to be found; nobody asked for these.
        tracing::info!("No lyrics found for the song playing: {error}");
        return vec![];
    }
    state.session.notification.current = Some(
        Notification::new(
            NotificationKind::Error,
            format!("Lyrics fetch failed: {}", error),
        )
        .with_duration(5),
    );
    vec![]
}

#[cfg(test)]
mod download_search_tests {
    use super::*;
    use crate::{library::snapshot::Library, message::ui::UIMsg, reducer::reduce};
    use std::collections::HashMap;

    fn state() -> (AppState, tempfile::TempDir) {
        let music = tempfile::tempdir().unwrap();
        let library = Library::new(vec![music.path().to_path_buf()], vec![], HashMap::new());
        let state = AppState::new(library, &crate::config::AppConfig::default());
        (state, music)
    }

    fn ask(state: &mut AppState, query: &str) -> Vec<AppMsg> {
        handle_request_download(state, query.into())
    }

    fn found(source: DownloadSource, name: &str) -> DownloadCandidate {
        DownloadCandidate {
            source,
            url: format!("https://www.youtube.com/watch?v={name}"),
            title: name.into(),
            by: Some("Someone".into()),
            album: None,
            duration_secs: Some(200),
        }
    }

    fn find(state: &mut AppState, id: u64, source: DownloadSource, name: &str) {
        reduce(
            state,
            AppMsg::Download(DownloadMsg::DownloadSearchResult {
                id,
                candidate: found(source, name),
            }),
        );
    }

    fn ui(state: &mut AppState, msg: UIMsg) -> Vec<AppMsg> {
        reduce(state, AppMsg::UI(msg))
    }

    #[test]
    fn a_link_is_downloaded_without_asking_which() {
        let (mut state, _music) = state();
        let msgs = ask(&mut state, " https://youtu.be/abc ");
        assert!(state.session.download.search.is_none());
        assert!(matches!(
            msgs.as_slice(),
            [AppMsg::Download(DownloadMsg::BeginNextDownload)]
        ));
        assert_eq!(
            state.session.download.queue[0].query,
            "https://youtu.be/abc"
        );
    }

    #[test]
    fn words_are_searched_for_and_nothing_downloaded_until_one_is_picked() {
        let (mut state, _music) = state();
        let msgs = ask(&mut state, "get lucky");
        assert!(matches!(
            msgs.as_slice(),
            [AppMsg::Download(DownloadMsg::DispatchSearchDownloads { id: 1, query })]
                if query == "get lucky"
        ));
        assert!(state.session.download.queue.is_empty());
        assert!(state.download_search().unwrap().searching);

        // Asked for before anything is found, nothing is downloaded.
        assert!(ui(&mut state, UIMsg::DownloadPickerSubmit).is_empty());
        assert!(state.download_search().is_some());

        find(&mut state, 1, DownloadSource::Songs, "first");
        find(&mut state, 1, DownloadSource::Songs, "second");
        find(&mut state, 1, DownloadSource::Songs, "second");
        assert_eq!(state.download_search().unwrap().songs.len(), 2, "one each");

        ui(&mut state, UIMsg::DownloadPickerMoveDown);
        ui(&mut state, UIMsg::DownloadPickerMoveDown);
        let msgs = ui(&mut state, UIMsg::DownloadPickerSubmit);
        assert!(matches!(
            msgs.as_slice(),
            [
                AppMsg::Download(DownloadMsg::CancelSearchDownloads),
                AppMsg::Download(DownloadMsg::BeginNextDownload)
            ]
        ));
        assert!(state.download_search().is_none());
        assert_eq!(
            state.session.download.queue[0].query,
            "https://www.youtube.com/watch?v=second"
        );
        let said = state.session.notification.current.as_ref().unwrap();
        assert_eq!(said.message, "Queued: Someone – second");
    }

    #[test]
    fn videos_are_shown_when_they_are_all_there_is_until_a_tab_is_chosen() {
        let (mut state, _music) = state();
        ask(&mut state, "a live set");
        find(&mut state, 1, DownloadSource::Videos, "video");
        assert_eq!(state.download_search().unwrap().tab, DownloadSource::Videos);
        // Songs, once there are some, are what is shown.
        find(&mut state, 1, DownloadSource::Songs, "song");
        assert_eq!(state.download_search().unwrap().tab, DownloadSource::Songs);

        ui(&mut state, UIMsg::DownloadPickerSwitchTab);
        find(&mut state, 1, DownloadSource::Songs, "another");
        let search = state.download_search().unwrap();
        assert_eq!(search.tab, DownloadSource::Videos, "left where it was put");
        assert_eq!(search.selected().unwrap().title, "video");
    }

    #[test]
    fn a_search_closed_stops_and_what_it_still_says_is_not_heard() {
        let (mut state, _music) = state();
        ask(&mut state, "first");
        let msgs = ui(&mut state, UIMsg::DownloadPickerCancel);
        assert!(matches!(
            msgs.as_slice(),
            [AppMsg::Download(DownloadMsg::CancelSearchDownloads)]
        ));
        find(&mut state, 1, DownloadSource::Songs, "late");
        assert!(state.download_search().is_none());

        // Nor is it taken for the next search's.
        ask(&mut state, "second");
        find(&mut state, 1, DownloadSource::Songs, "late");
        reduce(
            &mut state,
            AppMsg::Download(DownloadMsg::DownloadSearchFinished { id: 1, error: None }),
        );
        let search = state.download_search().unwrap();
        assert_eq!(
            (search.id, search.songs.len(), search.searching),
            (2, 0, true)
        );
    }

    #[test]
    fn with_nowhere_to_download_to_nothing_is_searched_for() {
        let library = Library::new(vec![], vec![], HashMap::new());
        let mut state = AppState::new(library, &crate::config::AppConfig::default());
        assert!(ask(&mut state, "get lucky").is_empty());
        assert!(state.download_search().is_none());
        let said = state.session.notification.current.as_ref().unwrap();
        assert_eq!(said.message, "No music directory configured for downloads");
    }

    #[test]
    fn a_search_that_finds_nothing_closes_and_says_why() {
        let (mut state, _music) = state();
        for (error, said) in [
            (None, "Nothing found for: zzqq"),
            (
                Some("Failed to start yt-dlp: not found"),
                "Failed to start yt-dlp: not found",
            ),
        ] {
            ask(&mut state, "zzqq");
            let id = state.download_search().unwrap().id;
            reduce(
                &mut state,
                AppMsg::Download(DownloadMsg::DownloadSearchFinished {
                    id,
                    error: error.map(String::from),
                }),
            );
            assert!(state.download_search().is_none());
            assert_eq!(
                state.session.notification.current.as_ref().unwrap().message,
                said
            );
        }
    }

    #[test]
    fn a_search_over_with_results_stays_open_to_be_picked_from() {
        let (mut state, _music) = state();
        ask(&mut state, "get lucky");
        find(&mut state, 1, DownloadSource::Videos, "video");
        reduce(
            &mut state,
            AppMsg::Download(DownloadMsg::DownloadSearchFinished {
                id: 1,
                error: Some("one video was private".into()),
            }),
        );
        let search = state.download_search().unwrap();
        assert!(!search.searching);
        assert_eq!(search.selected().unwrap().title, "video");
    }
}
