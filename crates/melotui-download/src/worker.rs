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
    LyricsFailed(String),
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
                InternalEvent::LyricsFailed(error) => {
                    worker.send(IpcEvent::LyricsFetchFailed { error });
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
        .filter(|r| (text(r, "syncedLyrics") || text(r, "plainLyrics")) && drift(r) <= MAX_DRIFT)
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

    let mut json_opt = None;

    let try_lrclib = provider.as_deref() == Some("lrclib") || provider.is_none();
    let try_ovh = provider.as_deref() == Some("ovh") || provider.is_none();

    if try_lrclib {
        if let Some(id) = &provider_id {
            // We have an exact ID, fetch directly!
            let req = ureq::get(&format!("https://lrclib.net/api/get/{}", id))
                .set("User-Agent", USER_AGENT)
                .timeout(std::time::Duration::from_secs(30));
            if let Ok(response) = req.call()
                && response.status() == 200
            {
                json_opt = response.into_json::<serde_json::Value>().ok();
            }
        } else if !artist_name.is_empty() {
            let mut req = ureq::get("https://lrclib.net/api/get")
                .query("track_name", &track_name)
                .query("artist_name", &artist_name);
            if let Some(album) = &album_name {
                req = req.query("album_name", album);
            }
            if let Some(duration) = duration_secs {
                req = req.query("duration", &duration.to_string());
            }
            let req = req
                .set("User-Agent", USER_AGENT)
                .timeout(std::time::Duration::from_secs(30));

            if let Ok(response) = req.call()
                && response.status() == 200
            {
                json_opt = response.into_json::<serde_json::Value>().ok();
            }
        }

        if json_opt.is_none() {
            tracing::debug!("Exact match failed or artist empty, trying search fallback...");
            let query = if artist_name.is_empty() {
                track_name.clone()
            } else {
                format!("{} {}", artist_name, track_name)
            };

            let req = ureq::get("https://lrclib.net/api/search")
                .query("q", &query)
                .set("User-Agent", USER_AGENT)
                .timeout(std::time::Duration::from_secs(30));

            if let Ok(response) = req.call()
                && response.status() == 200
                && let Ok(results) = response.into_json::<Vec<serde_json::Value>>()
            {
                json_opt = best_match(results, duration_secs);
            }
        }

        if let Some(json) = json_opt {
            let mut lyrics = None;
            if let Some(synced) = json.get("syncedLyrics").and_then(|v| v.as_str())
                && !synced.trim().is_empty()
            {
                lyrics = Some(synced.to_string());
            }
            if lyrics.is_none()
                && let Some(plain) = json.get("plainLyrics").and_then(|v| v.as_str())
                && !plain.trim().is_empty()
            {
                lyrics = Some(plain.to_string());
            }

            if let Some(l) = lyrics {
                if let Err(e) = save_lyrics(&save_path, &l) {
                    tracing::error!("Failed to write lyrics to file: {}", e);
                    let _ = tx.send(InternalEvent::LyricsFailed(format!(
                        "Failed to write lyrics file: {}",
                        e
                    )));
                } else {
                    tracing::info!("Lyrics successfully saved to {:?}", save_path);
                    let _ = tx.send(InternalEvent::LyricsFetched(save_path));
                }
                return;
            }
        }
    }

    // Fallback: OVH Lyrics API (Plain lyrics only)
    if try_ovh && !artist_name.is_empty() {
        tracing::debug!("LRCLIB failed or skipped, falling back to api.lyrics.ovh...");

        let ovh_artist = artist_name
            .bytes()
            .map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    (b as char).to_string()
                }
                _ => format!("%{:02X}", b),
            })
            .collect::<String>();

        let ovh_track = track_name
            .bytes()
            .map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    (b as char).to_string()
                }
                _ => format!("%{:02X}", b),
            })
            .collect::<String>();

        let ovh_url = format!("https://api.lyrics.ovh/v1/{}/{}", ovh_artist, ovh_track);
        let req = ureq::get(&ovh_url)
            .set("User-Agent", USER_AGENT)
            .timeout(std::time::Duration::from_secs(30));

        if let Ok(response) = req.call()
            && response.status() == 200
            && let Ok(json) = response.into_json::<serde_json::Value>()
            && let Some(lyrics) = json.get("lyrics").and_then(|v| v.as_str())
            && !lyrics.trim().is_empty()
        {
            if let Err(e) = save_lyrics(&save_path, lyrics) {
                tracing::error!("Failed to write lyrics to file: {}", e);
                let _ = tx.send(InternalEvent::LyricsFailed(format!(
                    "Failed to write lyrics file: {}",
                    e
                )));
            } else {
                tracing::info!("Lyrics successfully saved to {:?}", save_path);
                let _ = tx.send(InternalEvent::LyricsFetched(save_path));
            }
            return;
        }
    }

    tracing::error!("No lyrics found on LRCLIB or OVH API for the given query.");
    let _ = tx.send(InternalEvent::LyricsFailed(
        "No lyrics found or API error".to_string(),
    ));
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
        for item in json_arr.into_iter().take(5) {
            let id = item
                .get("id")
                .and_then(|v| v.as_i64())
                .map(|v| v.to_string());
            let track_name = item
                .get("trackName")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();
            let artist_name = item
                .get("artistName")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();
            let synced = item
                .get("syncedLyrics")
                .and_then(|v| v.as_str())
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);

            lrclib_results.push(melotui_core::state::lyrics::LyricsCandidate {
                api: melotui_core::state::lyrics::LyricsApi::Lrclib,
                track_name,
                artist_name,
                is_synced: synced,
                provider_id: id,
            });
        }
    }

    // 2. OVH Suggest
    let ovh_query = query
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b' ' => "+".to_string(),
            _ => format!("%{:02X}", b),
        })
        .collect::<String>();

    let ovh_req = ureq::get(&format!("https://api.lyrics.ovh/suggest/{}", ovh_query))
        .set("User-Agent", USER_AGENT)
        .timeout(std::time::Duration::from_secs(30));

    if let Ok(res) = ovh_req.call()
        && res.status() == 200
        && let Ok(json) = res.into_json::<serde_json::Value>()
        && let Some(data) = json.get("data").and_then(|v| v.as_array())
    {
        for item in data.iter().take(5) {
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
