//! Giving each download the song's title and artist in its tags.
//!
//! yt-dlp tags a download with the video's title and the channel's name
//! unless YouTube knows the song, so most music videos came in titled
//! `LiSA『紅蓮華』-MUSiC CLiP-（…）` by `LiSA Official YouTube`. It reports
//! what it knows of each video to a file as it finishes it, and the song is
//! read out of that and written over its tags.

use std::path::{Path, PathBuf};

use lofty::{config::WriteOptions, file::TaggedFileExt, prelude::*};
use melotui_core::domain::song_name::{Video, song_name};

/// What yt-dlp is asked to report of each video once it is in place, a line
/// of JSON each.
pub const REPORT: &str =
    "after_move:%(.{filepath,title,track,artist,artists,album,uploader,channel})j";

/// The lines yt-dlp has reported to `path`, taken as they come.
pub struct Reports {
    path: PathBuf,
    taken: usize,
}

impl Reports {
    pub fn new(path: PathBuf) -> Self {
        Self { path, taken: 0 }
    }

    /// The path as yt-dlp's file argument wants it: an output template, in
    /// which `%` has to be doubled.
    pub fn template(&self) -> String {
        self.path.to_string_lossy().replace('%', "%%")
    }

    /// The lines reported since the last time.
    pub fn take_new(&mut self) -> Vec<String> {
        let Ok(text) = std::fs::read_to_string(&self.path) else {
            return Vec::new();
        };
        let lines: Vec<String> = text
            .lines()
            .skip(self.taken)
            .filter(|l| !l.trim().is_empty())
            .map(str::to_string)
            .collect();
        self.taken = text.lines().count();
        lines
    }
}

impl Drop for Reports {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Tags the file a report is about with the song in it.
pub fn retag(report: &str) -> Result<PathBuf, String> {
    let info: serde_json::Value = serde_json::from_str(report).map_err(|e| e.to_string())?;
    let field = |key: &str| info.get(key).and_then(|v| v.as_str());
    let path = PathBuf::from(field("filepath").ok_or("no file path reported")?);
    let artists = info.get("artists").and_then(|v| v.as_array()).map(|a| {
        a.iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    });
    let song = song_name(&Video {
        title: field("title").unwrap_or_default(),
        track: field("track"),
        artist: field("artist").or(artists.as_deref().filter(|a| !a.is_empty())),
        album: field("album"),
        uploader: field("uploader").or(field("channel")),
    });
    // A tagging library that panics over some odd file must not take the
    // download's progress down with it.
    std::panic::catch_unwind(|| write_tags(&path, &song.title, song.artist.as_deref()))
        .map_err(|_| format!("the tag library failed on {path:?}"))??;
    Ok(path)
}

fn write_tags(path: &Path, title: &str, artist: Option<&str>) -> Result<(), String> {
    let mut file = lofty::read_from_path(path).map_err(|e| e.to_string())?;
    let tag = match file.primary_tag_mut() {
        Some(tag) => tag,
        None => {
            let kind = file.primary_tag_type();
            file.insert_tag(lofty::tag::Tag::new(kind));
            file.primary_tag_mut().ok_or("no tag to write to")?
        }
    };
    tag.set_title(title.to_string());
    if let Some(artist) = artist {
        tag.set_artist(artist.to_string());
    }
    tag.save_to_path(path, WriteOptions::default())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real FLAC, 50 ms of silence, for there to be a file to tag.
    fn empty_flac(path: &Path) {
        std::fs::write(path, include_bytes!("../tests/fixtures/silence.flac")).unwrap();
    }

    fn scratch() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "melotui-tags-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_download_is_tagged_with_the_song_in_its_video_title() {
        let path = scratch().join("LiSA『紅蓮華』-MUSiC CLiP-.flac");
        empty_flac(&path);
        let report = serde_json::json!({
            "filepath": path,
            "title": "LiSA『紅蓮華』-MUSiC CLiP-（アニメ「鬼滅の刃」竈門炭治郎 立志編 オープニングテーマ）",
            "track": null,
            "artist": null,
            "artists": null,
            "album": null,
            "uploader": "LiSA Official YouTube",
            "channel": "LiSA Official YouTube",
        });
        assert_eq!(retag(&report.to_string()).unwrap(), path);
        let file = lofty::read_from_path(&path).unwrap();
        let tag = file.primary_tag().unwrap();
        assert_eq!(tag.title().as_deref(), Some("紅蓮華"));
        assert_eq!(tag.artist().as_deref(), Some("LiSA"));
    }

    #[test]
    fn reports_are_taken_once_each_as_they_come() {
        let dir = scratch();
        let path = dir.join("100%.jsonl");
        let mut reports = Reports::new(path.clone());
        assert!(reports.template().ends_with("100%%.jsonl"));
        assert!(reports.take_new().is_empty(), "nothing reported yet");
        std::fs::write(&path, "{\"a\":1}\n").unwrap();
        assert_eq!(reports.take_new(), vec!["{\"a\":1}"]);
        std::fs::write(&path, "{\"a\":1}\n{\"b\":2}\n").unwrap();
        assert_eq!(reports.take_new(), vec!["{\"b\":2}"]);
        assert!(reports.take_new().is_empty());
        drop(reports);
        assert!(!path.exists(), "the report file goes with it");
    }

    #[test]
    fn a_report_without_a_file_is_an_error_not_a_panic() {
        assert!(retag("{\"title\": \"x\"}").is_err());
        assert!(retag("not json").is_err());
    }
}
