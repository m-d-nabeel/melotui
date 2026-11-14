//! Looking for something to download: what YouTube Music and YouTube have
//! for some words, asked of yt-dlp and told one result at a time.

use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use melotui_core::state::{DownloadCandidate, DownloadSource};

use crate::worker::{
    InternalEvent, error_in, in_a_group_of_its_own, kill_with_its_children, why_it_failed,
};

/// How many songs are asked for. Each is looked up in full, a request and
/// about a second apiece, and past the first few YouTube Music's matches
/// are covers and other songs: these are found in some six seconds.
const SONGS: usize = 5;

/// How many of YouTube's own results are asked for: one request however
/// many, and where a live set or an upload that is not a song is looked for.
const UPLOADS: usize = 10;

/// What yt-dlp is to say of each result: one line of JSON with these fields
/// and no others, where what it says by default runs to hundreds of
/// kilobytes a song.
const FIELDS: &str =
    "%(.{id,title,artists,artist,album,duration,channel,uploader,webpage_url,url})j";

/// Searches both places for `query` at once, telling `tx` each result as it
/// is found and then that the search `id` is over — unless `cancel` is set,
/// when it stops where it is and says nothing more.
pub(crate) fn run(id: u64, query: &str, tx: &mpsc::Sender<InternalEvent>, cancel: &AtomicBool) {
    let errors: Vec<Option<String>> = thread::scope(|scope| {
        let searches = [DownloadSource::Songs, DownloadSource::YouTube]
            .map(|source| scope.spawn(move || search(id, source, query, tx, cancel)));
        searches
            .into_iter()
            .map(|search| search.join().unwrap_or(None))
            .collect()
    });
    if !cancel.load(Ordering::SeqCst) {
        let _ = tx.send(InternalEvent::SearchFinished {
            id,
            error: errors.into_iter().flatten().next(),
        });
    }
}

/// What yt-dlp is run with to search `source` for `query`.
fn yt_dlp_args(source: DownloadSource, query: &str) -> Vec<String> {
    let mut args = vec!["--no-warnings".to_string()];
    let target = match source {
        // Each song is looked up in full, a second or so apiece: the list
        // of them alone has their titles and nothing else, and five songs
        // called "Get Lucky" cannot be told apart.
        DownloadSource::Songs => {
            args.extend(["--ignore-errors", "--playlist-end"].map(String::from));
            args.push(SONGS.to_string());
            format!(
                "https://music.youtube.com/search?q={}#songs",
                percent_encoded(query)
            )
        }
        // YouTube's own list says whose channel each is on and how long
        // it is.
        DownloadSource::YouTube => {
            args.push("--flat-playlist".into());
            format!("ytsearch{UPLOADS}:{query}")
        }
    };
    args.extend([
        "--print".into(),
        FIELDS.into(),
        // After `--`, so that nothing in it is taken for an option.
        "--".into(),
        target,
    ]);
    args
}

/// `text` as it goes in a URL's query.
fn percent_encoded(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

/// Searches one place, and says what went wrong if yt-dlp failed.
fn search(
    id: u64,
    source: DownloadSource,
    query: &str,
    tx: &mpsc::Sender<InternalEvent>,
    cancel: &AtomicBool,
) -> Option<String> {
    let mut command = Command::new("yt-dlp");
    command
        .args(yt_dlp_args(source, query))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    in_a_group_of_its_own(&mut command);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => return Some(format!("Failed to start yt-dlp: {e}")),
    };
    let (stdout, stderr) = (child.stdout.take()?, child.stderr.take()?);

    thread::scope(|scope| {
        let found = scope.spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(candidate) = candidate_in(&line, source) {
                    let _ = tx.send(InternalEvent::SearchResult { id, candidate });
                }
            }
        });
        // The last thing yt-dlp said went wrong.
        let said = scope.spawn(move || {
            BufReader::new(stderr)
                .lines()
                .map_while(Result::ok)
                .filter_map(|line| error_in(&line).map(str::to_string))
                .last()
        });

        let status = loop {
            if cancel.load(Ordering::SeqCst) {
                kill_with_its_children(&mut child);
                break None;
            }
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) => thread::sleep(Duration::from_millis(50)),
                Err(_) => break None,
            }
        };
        let _ = found.join();
        let said = said.join().ok().flatten();
        match status {
            Some(status) if !status.success() => {
                Some(why_it_failed(said.as_deref(), &status.to_string()))
            }
            _ => None,
        }
    })
}

/// The result in a line of what yt-dlp printed, if it is one with a title
/// and somewhere to download it from.
fn candidate_in(line: &str, source: DownloadSource) -> Option<DownloadCandidate> {
    let found: serde_json::Value = serde_json::from_str(line).ok()?;
    let text = |key: &str| {
        found
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let url = text("webpage_url")
        .or_else(|| text("url"))
        .or_else(|| text("id").map(|id| format!("https://www.youtube.com/watch?v={id}")))?;
    let artists = found
        .get("artists")
        .and_then(|v| v.as_array())
        .map(|artists| {
            artists
                .iter()
                .filter_map(|artist| artist.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|artists| !artists.is_empty());
    // A song with no artist named is on a channel YouTube made for the
    // artist, "Daft Punk - Topic".
    let channel = || {
        text("channel")
            .or_else(|| text("uploader"))
            .map(|channel| channel.trim_end_matches(" - Topic").to_string())
    };
    Some(DownloadCandidate {
        source,
        url,
        title: text("title")?,
        by: artists.or_else(|| text("artist")).or_else(channel),
        album: text("album"),
        duration_secs: found
            .get("duration")
            .and_then(|v| v.as_f64())
            .filter(|secs| *secs >= 1.0)
            .map(|secs| secs as u64),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_song_is_offered_with_its_artists_album_and_length() {
        let line = r#"{"id": "4D7u5KF7SP8", "title": "Get Lucky", "artists": ["Daft Punk", "Pharrell Williams"], "artist": "Daft Punk, Pharrell Williams", "album": "Random Access Memories", "duration": 370, "channel": "Daft Punk", "webpage_url": "https://www.youtube.com/watch?v=4D7u5KF7SP8"}"#;
        assert_eq!(
            candidate_in(line, DownloadSource::Songs),
            Some(DownloadCandidate {
                source: DownloadSource::Songs,
                url: "https://www.youtube.com/watch?v=4D7u5KF7SP8".into(),
                title: "Get Lucky".into(),
                by: Some("Daft Punk, Pharrell Williams".into()),
                album: Some("Random Access Memories".into()),
                duration_secs: Some(370),
            })
        );
    }

    #[test]
    fn what_else_youtube_has_is_offered_with_its_channel() {
        let line = r#"{"id": "abc", "title": "Live at Wembley", "duration": 3601.0, "channel": "Queen Official", "uploader": "Queen", "url": "https://www.youtube.com/watch?v=abc"}"#;
        let upload = candidate_in(line, DownloadSource::YouTube).unwrap();
        assert_eq!(upload.by.as_deref(), Some("Queen Official"));
        assert_eq!(upload.album, None);
        assert_eq!(upload.duration_secs, Some(3601));
        assert_eq!(upload.url, "https://www.youtube.com/watch?v=abc");
    }

    #[test]
    fn a_song_with_no_artist_named_is_by_whose_channel_it_is_on() {
        let line = r#"{"id": "abc", "title": "Get Lucky", "channel": "Jayden Anthony - Topic"}"#;
        let song = candidate_in(line, DownloadSource::Songs).unwrap();
        assert_eq!(song.by.as_deref(), Some("Jayden Anthony"));
        assert_eq!(song.url, "https://www.youtube.com/watch?v=abc");
        assert_eq!(song.duration_secs, None);
    }

    #[test]
    fn what_is_not_a_result_is_not_offered() {
        for line in [
            "",
            "[youtube] Extracting URL",
            r#"{"id": "abc"}"#,
            r#"{"title": "No link to it"}"#,
            r#"{"id": "abc", "title": "  "}"#,
        ] {
            assert_eq!(candidate_in(line, DownloadSource::Songs), None, "{line:?}");
        }
    }

    #[test]
    fn the_words_searched_for_cannot_be_taken_for_options_or_break_the_link() {
        let songs = yt_dlp_args(DownloadSource::Songs, "--exec x & y/ñ #1");
        let target = songs.last().unwrap();
        assert_eq!(
            target,
            "https://music.youtube.com/search?q=--exec%20x%20%26%20y%2F%C3%B1%20%231#songs"
        );
        assert_eq!(songs[songs.len() - 2], "--");
        let most = songs
            .iter()
            .position(|arg| arg == "--playlist-end")
            .unwrap();
        assert_eq!(songs[most + 1], "5");

        let youtube = yt_dlp_args(DownloadSource::YouTube, "--exec x");
        assert_eq!(youtube.last().unwrap(), "ytsearch10:--exec x");
        assert_eq!(youtube[youtube.len() - 2], "--");
        assert!(youtube.contains(&"--flat-playlist".to_string()));
    }

    /// Run by hand: asks YouTube.
    #[test]
    #[ignore = "needs yt-dlp and the network"]
    fn a_real_search_finds_songs_and_the_rest_and_says_it_is_over() {
        let (tx, rx) = mpsc::channel();
        run(7, "daft punk get lucky", &tx, &AtomicBool::new(false));
        let events: Vec<InternalEvent> = rx.try_iter().collect();
        let found = |source| {
            events
                .iter()
                .filter(|e| matches!(e, InternalEvent::SearchResult { id: 7, candidate } if candidate.source == source))
                .count()
        };
        assert!(found(DownloadSource::Songs) > 0);
        assert!(found(DownloadSource::YouTube) > 0);
        assert!(matches!(
            events.last(),
            Some(InternalEvent::SearchFinished { id: 7, error: None })
        ));
    }

    /// Run by hand: asks YouTube.
    #[test]
    #[ignore = "needs yt-dlp and the network"]
    fn a_search_cancelled_stops_at_once_and_says_nothing_more() {
        let (tx, rx) = mpsc::channel();
        let cancel = AtomicBool::new(false);
        let started = std::time::Instant::now();
        thread::scope(|scope| {
            scope.spawn(|| run(7, "daft punk get lucky", &tx, &cancel));
            thread::sleep(Duration::from_millis(500));
            cancel.store(true, Ordering::SeqCst);
        });
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(
            !rx.try_iter()
                .any(|e| matches!(e, InternalEvent::SearchFinished { .. }))
        );
    }
}
