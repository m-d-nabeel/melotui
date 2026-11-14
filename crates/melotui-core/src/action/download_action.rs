use std::fs;

use crate::{
    AppMsg, AppState,
    message::{download::DownloadMsg, library::LibraryMsg},
    state::ui::FocusMode,
    state::{
        DownloadCandidate, DownloadJob, DownloadSearch, DownloadSource, DownloadStatus,
        Notification, NotificationKind, PutAside,
    },
};

/// Downloads `query`: a link as it is, anything else as the first thing
/// YouTube finds for it.
pub fn handle_request_download(state: &mut AppState, query: String) -> Vec<AppMsg> {
    tracing::info!("RequestDownload called with: {:?}", query);
    let query = query.trim().to_string();
    let named = query.clone();
    enqueue_download(state, query, &named)
}

/// Searches for `query` and opens the list to pick from. A link needs no
/// picking, and is downloaded.
pub fn handle_request_search_downloads(state: &mut AppState, query: String) -> Vec<AppMsg> {
    let query = query.trim().to_string();
    if is_link(&query) {
        return handle_request_download(state, query);
    }
    // Said before searching, not after something has been picked.
    if state.config.download_dir(&state.library.roots).is_none() {
        nowhere_to_download_to(state);
        return vec![];
    }

    // The list is shown where lyrics are, as a search for lyrics is; a
    // search already open gives way to this one, and what that one put
    // aside is kept.
    let before = close_search(state).unwrap_or_else(|| {
        let (ui, lyrics) = (&state.session.ui, &state.session.lyrics);
        PutAside {
            show_lyrics: ui.show_lyrics,
            panel_closed: lyrics.panel_closed,
            panel_opened_for_lyrics: lyrics.panel_opened_for_lyrics,
            show_dsp_studio: ui.show_dsp_studio,
            focus_mode: ui.focus_mode,
        }
    });
    crate::action::playback_action::lyrics_panel_set_by_hand(state, true);
    // Where the panel is not on screen, the list would be picked from
    // unseen.
    state.session.ui.show_dsp_studio = false;
    if !matches!(
        state.session.ui.focus_mode,
        FocusMode::Full | FocusMode::LyricsPlayback
    ) {
        state.session.ui.focus_mode = FocusMode::Full;
    }

    let download = &mut state.session.download;
    download.searches += 1;
    let id = download.searches;
    download.search = Some(DownloadSearch {
        id,
        query: query.clone(),
        searching: true,
        before,
        ..Default::default()
    });
    vec![AppMsg::Download(DownloadMsg::DispatchSearchDownloads {
        id,
        query,
    })]
}

/// Closes the list, if it is open, and puts the screen back as it was;
/// what it was, for a search that takes this one's place.
fn close_search(state: &mut AppState) -> Option<PutAside> {
    let before = state.session.download.search.take()?.before;
    let ui = &mut state.session.ui;
    ui.show_lyrics = before.show_lyrics;
    ui.show_dsp_studio = before.show_dsp_studio;
    ui.focus_mode = before.focus_mode;
    state.session.lyrics.panel_closed = before.panel_closed;
    state.session.lyrics.panel_opened_for_lyrics = before.panel_opened_for_lyrics;
    // The song may have changed while the list was open: the panel is
    // open or not for the one playing now.
    crate::action::playback_action::fit_lyrics_panel(state);
    Some(before)
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
        DownloadSource::YouTube => &mut search.youtube,
    };
    if results.iter().any(|found| found.url == candidate.url) {
        return vec![];
    }
    results.push(candidate.clone());
    // A song goes in above the rest of YouTube: a selection made down
    // there stays on the result it was put on.
    if candidate.source == DownloadSource::Songs
        && search.moved
        && search.row >= search.songs.len() - 1
    {
        search.row += 1;
    }
    vec![]
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
    if !search.is_empty() {
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
    close_search(state);
    state.session.notification.current = Some(said.with_duration(4));
    vec![]
}

pub fn handle_download_picker_move(state: &mut AppState, by: i64) -> Vec<AppMsg> {
    if let Some(search) = state.session.download.search.as_mut() {
        let last = search.len().saturating_sub(1) as i64;
        search.row = (search.row as i64 + by).clamp(0, last) as usize;
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
    close_search(state);
    let named = match &picked.by {
        Some(by) => format!("{} – {}", by, picked.title),
        None => picked.title,
    };
    let mut msgs = vec![AppMsg::Download(DownloadMsg::CancelSearchDownloads)];
    msgs.extend(enqueue_download(state, picked.url, &named));
    msgs
}

pub fn handle_download_picker_cancel(state: &mut AppState) -> Vec<AppMsg> {
    if close_search(state).is_some() {
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

    fn search(state: &mut AppState, query: &str) -> Vec<AppMsg> {
        handle_request_search_downloads(state, query.into())
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

    fn selected(state: &AppState) -> &str {
        &state.download_search().unwrap().selected().unwrap().title
    }

    #[test]
    fn dl_downloads_without_asking_which_words_and_links_alike() {
        let (mut state, _music) = state();
        for query in [" https://youtu.be/abc ", "get lucky"] {
            let msgs = handle_request_download(&mut state, query.into());
            assert!(state.download_search().is_none());
            assert_eq!(
                state.session.download.queue.back().unwrap().query,
                query.trim()
            );
            assert!(!msgs.iter().any(|msg| matches!(
                msg,
                AppMsg::Download(DownloadMsg::DispatchSearchDownloads { .. })
            )));
        }
    }

    #[test]
    fn a_link_searched_for_is_downloaded() {
        let (mut state, _music) = state();
        let msgs = search(&mut state, "https://youtu.be/abc");
        assert!(state.download_search().is_none());
        assert!(matches!(
            msgs.as_slice(),
            [AppMsg::Download(DownloadMsg::BeginNextDownload)]
        ));
    }

    #[test]
    fn a_search_downloads_nothing_until_one_is_picked() {
        let (mut state, _music) = state();
        let msgs = search(&mut state, "get lucky");
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
    fn songs_come_first_and_the_selection_stays_on_what_it_was_put_on() {
        let (mut state, _music) = state();
        search(&mut state, "a live set");
        find(&mut state, 1, DownloadSource::YouTube, "upload");
        find(&mut state, 1, DownloadSource::YouTube, "another upload");
        assert_eq!(selected(&state), "upload");
        // Untouched, the selection is on whatever is first.
        find(&mut state, 1, DownloadSource::Songs, "song");
        assert_eq!(selected(&state), "song");

        // Put on something below the songs, it stays there as songs come.
        ui(&mut state, UIMsg::DownloadPickerMoveDown);
        ui(&mut state, UIMsg::DownloadPickerMoveDown);
        assert_eq!(selected(&state), "another upload");
        find(&mut state, 1, DownloadSource::Songs, "second song");
        assert_eq!(selected(&state), "another upload");

        // And on a song, as more come in after it.
        ui(&mut state, UIMsg::DownloadPickerMoveUp);
        ui(&mut state, UIMsg::DownloadPickerMoveUp);
        ui(&mut state, UIMsg::DownloadPickerMoveUp);
        assert_eq!(selected(&state), "song");
        find(&mut state, 1, DownloadSource::Songs, "third song");
        assert_eq!(selected(&state), "song");

        for _ in 0..9 {
            ui(&mut state, UIMsg::DownloadPickerMoveDown);
        }
        assert_eq!(selected(&state), "another upload", "the last there is");
    }

    #[test]
    fn a_search_closed_stops_and_what_it_still_says_is_not_heard() {
        let (mut state, _music) = state();
        search(&mut state, "first");
        let msgs = ui(&mut state, UIMsg::DownloadPickerCancel);
        assert!(matches!(
            msgs.as_slice(),
            [AppMsg::Download(DownloadMsg::CancelSearchDownloads)]
        ));
        find(&mut state, 1, DownloadSource::Songs, "late");
        assert!(state.download_search().is_none());

        // Nor is it taken for the next search's.
        search(&mut state, "second");
        find(&mut state, 1, DownloadSource::Songs, "late");
        reduce(
            &mut state,
            AppMsg::Download(DownloadMsg::DownloadSearchFinished { id: 1, error: None }),
        );
        let search = state.download_search().unwrap();
        assert_eq!((search.id, search.len(), search.searching), (2, 0, true));
    }

    #[test]
    fn the_list_takes_the_lyrics_panels_place_and_gives_the_screen_back() {
        let (mut state, _music) = state();
        state.session.ui.show_dsp_studio = true;
        state.session.ui.focus_mode = FocusMode::Visualizer;
        assert!(!state.session.ui.show_lyrics);

        search(&mut state, "first");
        let ui_now = &state.session.ui;
        assert!(ui_now.show_lyrics);
        assert!(!ui_now.show_dsp_studio, "it would be unseen");
        assert_eq!(ui_now.focus_mode, FocusMode::Full);
        // A second search in its place remembers what the first put aside.
        search(&mut state, "second");
        ui(&mut state, UIMsg::DownloadPickerCancel);
        let ui_now = &state.session.ui;
        assert!(!ui_now.show_lyrics);
        assert!(ui_now.show_dsp_studio);
        assert_eq!(ui_now.focus_mode, FocusMode::Visualizer);

        // Open before, the panel is open after — picked or not found alike.
        crate::action::playback_action::lyrics_panel_set_by_hand(&mut state, true);
        search(&mut state, "third");
        let id = state.download_search().unwrap().id;
        find(&mut state, id, DownloadSource::Songs, "song");
        ui(&mut state, UIMsg::DownloadPickerSubmit);
        assert!(state.session.ui.show_lyrics);
    }

    #[test]
    fn a_song_with_lyrics_that_started_under_the_list_has_its_panel_after() {
        let (mut state, _music) = state();
        assert!(!state.session.ui.show_lyrics);
        search(&mut state, "first");
        // As playback loads them when a song starts.
        state.session.lyrics.data = crate::state::LyricsData::Unsynced("la la".into());
        ui(&mut state, UIMsg::DownloadPickerCancel);
        assert!(state.session.ui.show_lyrics);
        assert!(state.session.lyrics.panel_opened_for_lyrics);
    }

    #[test]
    fn with_nowhere_to_download_to_nothing_is_searched_for() {
        let library = Library::new(vec![], vec![], HashMap::new());
        let mut state = AppState::new(library, &crate::config::AppConfig::default());
        assert!(search(&mut state, "get lucky").is_empty());
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
            search(&mut state, "zzqq");
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
        search(&mut state, "get lucky");
        find(&mut state, 1, DownloadSource::YouTube, "upload");
        reduce(
            &mut state,
            AppMsg::Download(DownloadMsg::DownloadSearchFinished {
                id: 1,
                error: Some("one of them was private".into()),
            }),
        );
        let search = state.download_search().unwrap();
        assert!(!search.searching);
        assert_eq!(search.selected().unwrap().title, "upload");
    }
}
