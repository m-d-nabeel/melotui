use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use melotui_bus::{channel::IpcWorker, command::DownloadCommand, event::IpcEvent};

use crate::{
    progress::parse_progress,
    tags::{REPORT, Reports, retag},
};

enum InternalEvent {
    Progress(u8),
    ItemFinished,
    Finished,
    Failed(String),
    LyricsFetched(PathBuf),
    LyricsFailed {
        save_path: PathBuf,
        error: String,
    },
    LyricsSearchResults {
        lrclib_results: Vec<melotui_core::state::lyrics::LyricsCandidate>,
        ovh_results: Vec<melotui_core::state::lyrics::LyricsCandidate>,
    },
}

pub fn run(worker: impl IpcWorker<DownloadCommand>) {
    tracing::info!("Download worker started");

    // bridge download thread -> ipc
    let (internal_tx, internal_rx) = mpsc::channel::<InternalEvent>();

    // cancellation flag
    let mut cancel_flag: Option<Arc<AtomicBool>> = None;

    loop {
        // Always use timeout so we can periodically check internal_rx for background task events
        let cmd = worker.recv_timeout(Duration::from_millis(100));

        if let Some(cmd) = cmd {
            tracing::debug!("Worker received command: {:?}", cmd);

            match cmd {
                DownloadCommand::Request {
                    query,
                    download_path,
                } => {
                    if cancel_flag.is_some() {
                        tracing::warn!("Download already running, ignoring");
                        continue;
                    }

                    tracing::info!("Starting download for: {}", query);
                    worker.send(IpcEvent::DownloadStarted {
                        query: query.clone(),
                    });

                    let flag = Arc::new(AtomicBool::new(false));
                    cancel_flag = Some(flag.clone());

                    let tx = internal_tx.clone();

                    thread::spawn(move || {
                        download_task(query, download_path, tx, flag);
                    });
                }

                DownloadCommand::FetchLyrics {
                    track_name,
                    artist_name,
                    album_name,
                    duration_secs,
                    save_path,
                    provider,
                    provider_id,
                } => {
                    tracing::info!(
                        "Starting lyrics fetch for: {} - {}",
                        artist_name,
                        track_name
                    );
                    let tx_clone = internal_tx.clone();
                    thread::spawn(move || {
                        fetch_lyrics_task(
                            track_name,
                            artist_name,
                            album_name,
                            duration_secs,
                            save_path,
                            provider,
                            provider_id,
                            tx_clone,
                        );
                    });
                }

                DownloadCommand::SearchLyrics { query } => {
                    tracing::info!("Starting lyrics search for: {}", query);
                    let tx = internal_tx.clone();
                    thread::spawn(move || {
                        search_lyrics_task(query, tx);
                    });
                }

                DownloadCommand::Cancel => {
                    if let Some(flag) = &cancel_flag {
                        tracing::info!("Signaling cancellation...");
                        flag.store(true, Ordering::SeqCst);
                    }
                }

                DownloadCommand::Shutdown => {
                    tracing::info!("Worker: shutdown requested");
                    if let Some(flag) = &cancel_flag {
                        flag.store(true, Ordering::SeqCst);
                    }
                    break;
                }
            }
        }

        // drain internal events from child process
        while let Ok(evt) = internal_rx.try_recv() {
            match evt {
                InternalEvent::Progress(percent) => {
                    worker.send(IpcEvent::DownloadProgress { percent });
                }
                InternalEvent::ItemFinished => {
                    worker.send(IpcEvent::DownloadItemFinished);
                }
                InternalEvent::Finished => {
                    worker.send(IpcEvent::DownloadFinished);
                    cancel_flag = None;
                }
                InternalEvent::Failed(error) => {
                    worker.send(IpcEvent::DownloadFailed { error });
                    cancel_flag = None;
                }
                InternalEvent::LyricsFetched(save_path) => {
                    worker.send(IpcEvent::LyricsFetched { save_path });
                }
                InternalEvent::LyricsFailed { save_path, error } => {
                    worker.send(IpcEvent::LyricsFetchFailed { save_path, error });
                }
                InternalEvent::LyricsSearchResults {
                    lrclib_results,
                    ovh_results,
                } => {
                    worker.send(IpcEvent::LyricsSearchResults {
                        lrclib_results,
                        ovh_results,
                    });
                }
            }
        }
    }

    tracing::info!("Download worker exited");
}

fn download_task(
    query: String,
    dl_path: PathBuf,
    tx: mpsc::Sender<InternalEvent>,
    cancel: Arc<AtomicBool>,
) {
    tracing::debug!("YT-DLP Thread Spawned, root: {:?}", dl_path);

    let target = if query.starts_with("http://") || query.starts_with("https://") {
        query.clone()
    } else {
        format!("ytsearch1:{}", query)
    };

    let mut reports = Reports::new(std::env::temp_dir().join(format!(
        "melotui-download-{}-{}.jsonl",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    )));

    let cmd = Command::new("yt-dlp")
        .arg("-x")
        .arg("-f")
        .arg("bestaudio/best")
        .arg("--extract-audio")
        .arg("--audio-format")
        .arg("flac")
        .arg("--embed-thumbnail")
        .arg("--add-metadata")
        .arg("--output")
        .arg(format!("{}/%(title)s.%(ext)s", dl_path.display()))
        // Reported before the --exec below runs, so each item's report is
        // there by the time it is said to be finished.
        .arg("--print-to-file")
        .arg(REPORT)
        .arg(reports.template())
        .arg("--exec")
        .arg("echo MELOTUI_ITEM_FINISHED")
        .arg(target)
        .arg("--newline") // needed for parsing
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let mut child = match cmd {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(InternalEvent::Failed(format!(
                "Failed to start yt-dlp: {}",
                e
            )));
            return;
        }
    };

    // read stdout/stderr

    let stderr = child.stderr.take().unwrap();
    let stdout = child.stdout.take().unwrap();

    let tx_err = tx.clone();
    let cancel_err = cancel.clone();

    // stderr reader
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            if cancel_err.load(Ordering::SeqCst) {
                return;
            }
            // check progress on stderr
            if let Some(pct) = parse_progress(&line) {
                let _ = tx_err.send(InternalEvent::Progress(pct));
            }
        }
    });

    let tx_out = tx.clone();
    let cancel_out = cancel.clone();

    // stdout reader
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if cancel_out.load(Ordering::SeqCst) {
                return;
            }
            tracing::debug!("yt-dlp: {}", line);
            // check progress on stdout
            if let Some(pct) = parse_progress(&line) {
                let _ = tx_out.send(InternalEvent::Progress(pct));
            }
            if line.contains("MELOTUI_ITEM_FINISHED") {
                for report in reports.take_new() {
                    match retag(&report) {
                        Ok(path) => tracing::info!("Tagged {path:?} with its song"),
                        Err(e) => tracing::warn!("Could not tag a download: {e}"),
                    }
                }
                let _ = tx_out.send(InternalEvent::ItemFinished);
            }
        }
    });

    // monitor process
    loop {
        if cancel.load(Ordering::SeqCst) {
            tracing::info!("Killing yt-dlp process...");
            let _ = child.kill();
            let _ = child.wait();
            let _ = tx.send(InternalEvent::Failed("Cancelled".into()));
            return;
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    let _ = tx.send(InternalEvent::Finished);
                } else {
                    let _ = tx.send(InternalEvent::Failed(format!("Exit code: {}", status)));
                }
                return;
            }
            Ok(None) => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                let _ = tx.send(InternalEvent::Failed(e.to_string()));
                return;
            }
        }
    }
}

const USER_AGENT: &str = concat!(
    "melotui/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/m-d-nabeel/melotui)"
);

/// Writes `lyrics` to `path`, making the folder it goes in if need be.
fn save_lyrics(path: &std::path::Path, lyrics: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, lyrics)
}

/// The lyrics in an LRCLIB record: synced if it has them, plain otherwise,
/// none for an instrumental or a record left empty.
fn lyrics_of(record: &serde_json::Value) -> Option<String> {
    ["syncedLyrics", "plainLyrics"].iter().find_map(|key| {
        record
            .get(*key)
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(str::to_string)
    })
}

/// LRCLIB's search results worth offering: those with lyrics, synced
/// first, each song once — it holds many copies of some, and instrumentals
/// with none, which were offered and then failed to fetch.
fn candidates(
    results: Vec<serde_json::Value>,
) -> Vec<melotui_core::state::lyrics::LyricsCandidate> {
    use melotui_core::state::lyrics::{LyricsApi, LyricsCandidate};
    let field = |r: &serde_json::Value, key: &str| {
        r.get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .trim()
            .to_string()
    };
    let mut offered: Vec<LyricsCandidate> = Vec::new();
    for record in results.iter().filter(|r| lyrics_of(r).is_some()) {
        let candidate = LyricsCandidate {
            api: LyricsApi::Lrclib,
            track_name: field(record, "trackName"),
            artist_name: field(record, "artistName"),
            is_synced: record
                .get("syncedLyrics")
                .and_then(|v| v.as_str())
                .is_some_and(|s| !s.trim().is_empty()),
            provider_id: record
                .get("id")
                .and_then(|v| v.as_i64())
                .map(|id| id.to_string()),
        };
        let same = |c: &LyricsCandidate| {
            c.is_synced == candidate.is_synced
                && c.track_name.to_lowercase() == candidate.track_name.to_lowercase()
                && c.artist_name.to_lowercase() == candidate.artist_name.to_lowercase()
        };
        if !offered.iter().any(same) {
            offered.push(candidate);
        }
    }
    offered.sort_by_key(|c| !c.is_synced);
    offered.truncate(MAX_CANDIDATES);
    offered
}

/// How many results a lyrics search offers from each service.
const MAX_CANDIDATES: usize = 8;

/// The best of LRCLIB's search results for a song `duration` seconds long:
/// one with lyrics, synced before plain, nearest in length first, and none
/// more than `MAX_DRIFT` seconds off — that is another recording, or another
/// song. A music video runs longer than the song for its intro, hence the
/// leeway.
fn best_match(results: Vec<serde_json::Value>, duration: Option<u64>) -> Option<serde_json::Value> {
    const MAX_DRIFT: f64 = 15.0;
    let text = |r: &serde_json::Value, key: &str| {
        r.get(key)
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.trim().is_empty())
    };
    let drift = |r: &serde_json::Value| match duration {
        Some(d) => r
            .get("duration")
            .and_then(|v| v.as_f64())
            .map_or(f64::INFINITY, |len| (len - d as f64).abs()),
        None => 0.0,
    };
    results
        .into_iter()
        .filter(|r| lyrics_of(r).is_some() && drift(r) <= MAX_DRIFT)
        .min_by(|a, b| {
            let rank = |r: &serde_json::Value| (!text(r, "syncedLyrics"), drift(r));
            rank(a)
                .partial_cmp(&rank(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

#[allow(clippy::too_many_arguments)]
fn fetch_lyrics_task(
    track_name: String,
    artist_name: String,
    album_name: Option<String>,
    duration_secs: Option<u64>,
    save_path: PathBuf,
    provider: Option<String>,
    provider_id: Option<String>,
    tx: mpsc::Sender<InternalEvent>,
) {
    tracing::debug!(
        "Fetching lyrics for: {} - {} (provider: {:?}, id: {:?})",
        artist_name,
        track_name,
        provider,
        provider_id
    );

    let try_lrclib = provider.as_deref() == Some("lrclib") || provider.is_none();
    let try_ovh = provider.as_deref() == Some("ovh") || provider.is_none();

    let lrclib_get = |url: &str, query: &[(&str, &str)]| -> Option<serde_json::Value> {
        let mut req = ureq::get(url)
            .set("User-Agent", USER_AGENT)
            .timeout(std::time::Duration::from_secs(30));
        for (key, value) in query {
            req = req.query(key, value);
        }
        let response = req.call().ok()?;
        (response.status() == 200)
            .then(|| response.into_json::<serde_json::Value>().ok())
            .flatten()
    };
    let lrclib_search = |q: &str| -> Option<String> {
        let results = lrclib_get("https://lrclib.net/api/search", &[("q", q)])?;
        let results = serde_json::from_value::<Vec<serde_json::Value>>(results).ok()?;
        best_match(results, duration_secs)
            .as_ref()
            .and_then(lyrics_of)
    };

    if try_lrclib {
        let duration = duration_secs.map(|d| d.to_string());
        let lyrics = if let Some(id) = &provider_id {
            // Chosen from a search: that record, and no other.
            lrclib_get(&format!("https://lrclib.net/api/get/{id}"), &[])
                .as_ref()
                .and_then(lyrics_of)
        } else {
            // The song by its tags; then searched for by artist and title;
            // then, the length known to tell it from others of the name, by
            // title alone, for a song whose "artist" is its label or the
            // channel it was downloaded from.
            let exact = (!artist_name.is_empty())
                .then(|| {
                    let mut query = vec![
                        ("track_name", track_name.as_str()),
                        ("artist_name", artist_name.as_str()),
                    ];
                    if let Some(album) = &album_name {
                        query.push(("album_name", album));
                    }
                    if let Some(duration) = &duration {
                        query.push(("duration", duration));
                    }
                    lrclib_get("https://lrclib.net/api/get", &query)
                })
                .flatten()
                .as_ref()
                .and_then(lyrics_of);
            exact
                .or_else(|| {
                    (!artist_name.is_empty())
                        .then(|| lrclib_search(&format!("{artist_name} {track_name}")))
                        .flatten()
                })
                .or_else(|| {
                    (artist_name.is_empty() || duration_secs.is_some())
                        .then(|| lrclib_search(&track_name))
                        .flatten()
                })
        };

        if let Some(lyrics) = lyrics {
            save_and_tell(&save_path, &lyrics, &tx);
            return;
        }
    }

    // Then lyrics.ovh, which has plain lyrics only.
    if try_ovh && !artist_name.is_empty() {
        let url = format!(
            "https://api.lyrics.ovh/v1/{}/{}",
            encode(&artist_name),
            encode(&track_name)
        );
        let req = ureq::get(&url)
            .set("User-Agent", USER_AGENT)
            .timeout(std::time::Duration::from_secs(30));
        if let Ok(response) = req.call()
            && response.status() == 200
            && let Ok(json) = response.into_json::<serde_json::Value>()
            && let Some(lyrics) = json.get("lyrics").and_then(|v| v.as_str())
            && !lyrics.trim().is_empty()
        {
            save_and_tell(&save_path, lyrics, &tx);
            return;
        }
    }

    let song = if artist_name.is_empty() {
        track_name
    } else {
        format!("{artist_name} – {track_name}")
    };
    tracing::info!("No lyrics found for {song}");
    let _ = tx.send(InternalEvent::LyricsFailed {
        save_path,
        error: format!("no lyrics found for {song}"),
    });
}

/// Saves `lyrics` at `save_path` and says how that went.
fn save_and_tell(save_path: &std::path::Path, lyrics: &str, tx: &mpsc::Sender<InternalEvent>) {
    let event = match save_lyrics(save_path, lyrics) {
        Ok(()) => {
            tracing::info!("Lyrics saved to {save_path:?}");
            InternalEvent::LyricsFetched(save_path.to_path_buf())
        }
        Err(e) => {
            tracing::error!("Failed to write lyrics to {save_path:?}: {e}");
            InternalEvent::LyricsFailed {
                save_path: save_path.to_path_buf(),
                error: format!("could not write the lyrics file: {e}"),
            }
        }
    };
    let _ = tx.send(event);
}

/// `text` as one segment of a URL's path.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn search_lyrics_task(query: String, tx: mpsc::Sender<InternalEvent>) {
    tracing::debug!("Searching lyrics for query: {}", query);

    let mut lrclib_results = Vec::new();
    let mut ovh_results = Vec::new();

    // 1. LRCLIB Search
    let lrclib_req = ureq::get("https://lrclib.net/api/search")
        .query("q", &query)
        .set("User-Agent", USER_AGENT)
        .timeout(std::time::Duration::from_secs(30));

    if let Ok(res) = lrclib_req.call()
        && res.status() == 200
        && let Ok(json_arr) = res.into_json::<Vec<serde_json::Value>>()
    {
        lrclib_results = candidates(json_arr);
    }

    // 2. OVH Suggest
    let ovh_req = ureq::get(&format!(
        "https://api.lyrics.ovh/suggest/{}",
        encode(&query)
    ))
    .set("User-Agent", USER_AGENT)
    .timeout(std::time::Duration::from_secs(30));

    if let Ok(res) = ovh_req.call()
        && res.status() == 200
        && let Ok(json) = res.into_json::<serde_json::Value>()
        && let Some(data) = json.get("data").and_then(|v| v.as_array())
    {
        for item in data.iter().take(MAX_CANDIDATES) {
            let track_name = item
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();
            let artist_name = item
                .get("artist")
                .and_then(|a| a.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();

            ovh_results.push(melotui_core::state::lyrics::LyricsCandidate {
                api: melotui_core::state::lyrics::LyricsApi::Ovh,
                track_name,
                artist_name,
                is_synced: false,
                provider_id: None,
            });
        }
    }

    let _ = tx.send(InternalEvent::LyricsSearchResults {
        lrclib_results,
        ovh_results,
    });
}

#[cfg(test)]
mod lyrics_match_tests {
    use super::best_match;
    use serde_json::json;

    #[test]
    fn a_search_offers_songs_with_lyrics_synced_first_each_once() {
        // As LRCLIB answered "Kuch To Hai": copies, and an instrumental
        // without lyrics where the synced ones should have been.
        let results = vec![
            json!({"id": 6670370, "artistName": "Satyam Anandjee", "trackName": "Kuch To Hai Tere Mere Darmiyaan", "plainLyrics": "words", "syncedLyrics": null}),
            json!({"id": 6670369, "artistName": "Satyam Anandjee", "trackName": "Kuch To Hai Tere Mere Darmiyaan", "plainLyrics": "words", "syncedLyrics": null}),
            json!({"id": 34354413, "artistName": "Amaal Mallik", "trackName": "Kuch To Hai", "instrumental": true, "plainLyrics": null, "syncedLyrics": null}),
            json!({"id": 10879685, "artistName": "Jubin Nautiyal", "trackName": "Kuch to Hai", "plainLyrics": "words", "syncedLyrics": "[00:01.00] words"}),
            json!({"id": 38208616, "artistName": "Armaan Malik", "trackName": "Kuch To Hai", "plainLyrics": "words", "syncedLyrics": "[00:01.00] words"}),
            json!({"id": 39447653, "artistName": "Armaan Malik", "trackName": "Kuch To Hai", "plainLyrics": "words", "syncedLyrics": "[00:01.00] words"}),
            json!({"id": 1, "artistName": "Someone", "trackName": "Blank", "plainLyrics": "  ", "syncedLyrics": ""}),
        ];
        let offered: Vec<(String, bool)> = super::candidates(results)
            .into_iter()
            .map(|c| (format!("{}: {}", c.artist_name, c.track_name), c.is_synced))
            .collect();
        assert_eq!(
            offered,
            vec![
                ("Jubin Nautiyal: Kuch to Hai".to_string(), true),
                ("Armaan Malik: Kuch To Hai".to_string(), true),
                (
                    "Satyam Anandjee: Kuch To Hai Tere Mere Darmiyaan".to_string(),
                    false
                ),
            ]
        );
    }

    #[test]
    fn an_instrumental_record_has_no_lyrics_to_save() {
        assert_eq!(
            super::lyrics_of(&json!({"instrumental": true, "plainLyrics": null})),
            None
        );
        assert_eq!(
            super::lyrics_of(&json!({"plainLyrics": "plain", "syncedLyrics": "[00:01.00] synced"}))
                .as_deref(),
            Some("[00:01.00] synced")
        );
        assert_eq!(super::encode("AC/DC & Co"), "AC%2FDC%20%26%20Co");
    }

    #[test]
    fn synced_lyrics_of_the_right_length_win_and_other_songs_lose() {
        let results = vec![
            json!({"id": 1, "duration": 236.0, "plainLyrics": "plain", "syncedLyrics": null}),
            json!({"id": 2, "duration": 240.0, "plainLyrics": "plain", "syncedLyrics": "[00:01.00] a"}),
            json!({"id": 3, "duration": 410.0, "plainLyrics": "plain", "syncedLyrics": "[00:01.00] b"}),
            json!({"id": 4, "duration": 235.0, "plainLyrics": "", "syncedLyrics": ""}),
        ];
        let best = best_match(results.clone(), Some(235)).unwrap();
        assert_eq!(
            best["id"], 2,
            "synced, five seconds off, before plain and one off"
        );
        let none_close = best_match(results[2..3].to_vec(), Some(235));
        assert!(none_close.is_none(), "175 seconds off is another recording");
        assert_eq!(
            best_match(results, None).unwrap()["id"],
            2,
            "no length known: synced first"
        );
    }
}
