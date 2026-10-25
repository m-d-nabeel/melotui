use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use lrc::Lyrics;

use crate::domain::TrackMeta;

#[derive(Debug, Clone, Default)]
pub enum LyricsData {
    Synced(Lyrics),
    Unsynced(String),
    #[default]
    None,
}

impl LyricsData {
    /// Lyrics read from `text`: synced if it has LRC timestamps, plain text
    /// otherwise, none if it is empty. Plain text parses as LRC too, with no
    /// timed lines, and was taken as synced lyrics with nothing in them — a
    /// blank lyrics panel.
    pub fn from_text(text: String) -> Self {
        if text.trim().is_empty() {
            return LyricsData::None;
        }
        match parse_lrc(&text) {
            Some(lyrics) => LyricsData::Synced(lyrics),
            None => LyricsData::Unsynced(text),
        }
    }

    /// The lyrics for the audio file at `audio`, the song `track`: from a
    /// `.lrc` file in `store` named after the song or after the file — where
    /// lyrics chosen in a search go, so they win — or one beside it, or else
    /// from the lyrics in its own tags.
    pub fn load_for(audio: &Path, track: Option<&TrackMeta>, store: &Path) -> Self {
        let beside = audio.with_extension("lrc");
        let by_song = track.and_then(|t| stored_path(store, &t.title, t.artist.as_deref()));
        let by_file = audio
            .file_stem()
            .map(|stem| store.join(format!("{}.lrc", stem.to_string_lossy())));
        for path in [by_song, by_file, Some(beside)].into_iter().flatten() {
            if let Ok(text) = std::fs::read_to_string(path) {
                let lyrics = Self::from_text(text);
                if !matches!(lyrics, LyricsData::None) {
                    return lyrics;
                }
            }
        }
        embedded_lyrics(audio).map_or(LyricsData::None, Self::from_text)
    }
}

/// The synced lyrics in `text`, if it has timed lines. A line the parser
/// refuses — words with `[a: b]` in them — is left out, rather than all the
/// others losing their timing and showing as text, timestamps and all. An
/// `[offset:]` is applied as players apply it: a positive one, in
/// milliseconds, brings every line that much sooner.
fn parse_lrc(text: &str) -> Option<Lyrics> {
    let lyrics = text.parse::<Lyrics>().ok().or_else(|| {
        let taken: Vec<&str> = text
            .lines()
            .filter(|line| line.parse::<Lyrics>().is_ok())
            .collect();
        taken.join("\n").parse::<Lyrics>().ok()
    })?;
    if lyrics.get_timed_lines().is_empty() {
        return None;
    }
    let offset = text
        .lines()
        .find_map(|line| {
            let line = line.trim().to_ascii_lowercase();
            let value = line.strip_prefix("[offset:")?.strip_suffix(']')?.trim();
            value.trim_start_matches('+').parse::<i64>().ok()
        })
        .unwrap_or(0);
    if offset == 0 {
        return Some(lyrics);
    }
    let mut shifted = Lyrics::new();
    for (time, line) in lyrics.get_timed_lines() {
        let at = (time.get_timestamp() - offset).max(0);
        shifted
            .add_timed_line(lrc::TimeTag::new(at), line.to_string())
            .ok()?;
    }
    Some(shifted)
}

/// Where lyrics are kept, out of the music folders: those melotui fetches,
/// named after the song, and any put there by hand, named after the song or
/// after the audio file.
pub fn store_dir() -> PathBuf {
    crate::config::AppConfig::data_dir().join("lyrics")
}

/// The file in `store` for `title` by `artist`: `Artist - Title.lrc`.
pub fn stored_path(store: &Path, title: &str, artist: Option<&str>) -> Option<PathBuf> {
    let name = match artist.map(str::trim).filter(|a| !a.is_empty()) {
        Some(artist) => format!("{artist} - {}", title.trim()),
        None => title.trim().to_string(),
    };
    let name: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    // Room for `.lrc` within the usual 255-byte limit on a name.
    let mut end = name.len().min(240);
    while !name.is_char_boundary(end) {
        end -= 1;
    }
    let name = name[..end].trim().trim_matches('.');
    (!name.is_empty() && !title.trim().is_empty()).then(|| store.join(format!("{name}.lrc")))
}

/// The lyrics in an audio file's tags — ID3's USLT, Vorbis and FLAC's
/// LYRICS, MP4's ©lyr — read without its cover art or audio properties, so
/// starting a track stays quick.
fn embedded_lyrics(audio: &std::path::Path) -> Option<String> {
    use lofty::{config::ParseOptions, file::TaggedFileExt, probe::Probe, tag::ItemKey};
    let options = ParseOptions::new()
        .read_properties(false)
        .read_cover_art(false);
    let file = Probe::open(audio).ok()?.options(options).read().ok()?;
    file.tags()
        .iter()
        .find_map(|tag| tag.get_string(&ItemKey::Lyrics))
        .map(str::to_string)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LyricsApi {
    Lrclib,
    Ovh,
}

#[derive(Debug, Clone)]
pub struct LyricsCandidate {
    pub api: LyricsApi,
    pub track_name: String,
    pub artist_name: String,
    pub is_synced: bool,
    pub provider_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct LyricsSearchState {
    pub lrclib_results: Vec<LyricsCandidate>,
    pub ovh_results: Vec<LyricsCandidate>,
    pub selected_column: usize, // 0 for lrclib, 1 for ovh
    pub selected_row: usize,
}

#[derive(Debug, Clone)]
pub struct LyricsState {
    pub data: LyricsData,
    pub active_line_index: Option<usize>,
    pub search_state: Option<LyricsSearchState>,
    /// Where lyrics are kept: [`store_dir`], but for tests.
    pub store: PathBuf,
    /// The songs lyrics have been fetched for this run, found or not, so a
    /// song that has none is not asked about at every play.
    pub fetched: HashSet<PathBuf>,
    /// Where lyrics fetched unasked are going: they come, or don't, quietly.
    /// More than one, songs skipped through before theirs came.
    pub fetching_unasked: HashSet<PathBuf>,
    /// The lyrics panel was closed by hand, and stays closed until opened.
    pub panel_closed: bool,
    /// The lyrics panel was opened for a song's lyrics, and closes again for
    /// a song with none.
    pub panel_opened_for_lyrics: bool,
}

impl Default for LyricsState {
    fn default() -> Self {
        Self {
            data: LyricsData::None,
            active_line_index: None,
            search_state: None,
            store: store_dir(),
            fetched: HashSet::new(),
            fetching_unasked: HashSet::new(),
            panel_closed: false,
            panel_opened_for_lyrics: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lrc() {
        let l = "[00:12.00] Line\n[00:14.00] Line 2"
            .parse::<lrc::Lyrics>()
            .unwrap();
        let lyrics = l.get_timed_lines();
        let _t = lyrics[0].0.get_timestamp();
    }

    #[test]
    fn timestamps_make_lyrics_synced_and_plain_text_stays_plain() {
        assert!(matches!(
            LyricsData::from_text("[00:12.00] Line\n[00:14.00] Line 2".into()),
            LyricsData::Synced(_)
        ));
        match LyricsData::from_text("Just the words\nno times".into()) {
            LyricsData::Unsynced(text) => assert!(text.starts_with("Just the words")),
            other => panic!("plain text read as {other:?}"),
        }
        assert!(matches!(
            LyricsData::from_text("  \n".into()),
            LyricsData::None
        ));
    }

    fn timed(text: &str) -> Vec<(i64, String)> {
        match LyricsData::from_text(text.into()) {
            LyricsData::Synced(lyrics) => lyrics
                .get_timed_lines()
                .iter()
                .map(|(t, l)| (t.get_timestamp(), l.to_string()))
                .collect(),
            other => panic!("not synced: {other:?}"),
        }
    }

    #[test]
    fn one_line_the_parser_refuses_does_not_cost_the_rest_their_timing() {
        let lines = timed("[00:01.00] First\n[00:02.00] Says [x: y] here\n[00:03.00] Third");
        assert_eq!(
            lines,
            vec![(1000, "First".to_string()), (3000, "Third".to_string())]
        );
    }

    #[test]
    fn an_offset_brings_the_lines_sooner_or_later() {
        let sooner = timed("[offset:+500]\n[00:01.00] One\n[00:02.00] Two");
        assert_eq!(sooner[0].0, 500);
        assert_eq!(sooner[1].0, 1500);
        let later = timed("[Offset: -250]\n[00:01.00] One");
        assert_eq!(later[0].0, 1250);
        // Several stamps on a line, each its own moment, in order.
        let chorus = timed("[00:05.00][00:01.00] Chorus\n[00:03.00] Verse");
        assert_eq!(
            chorus.iter().map(|(t, _)| *t).collect::<Vec<_>>(),
            vec![1000, 3000, 5000]
        );
    }

    /// A second of silent mono 8 kHz audio, as a WAV file.
    fn wav(path: &std::path::Path) {
        let data_len: u32 = 16_000;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&8_000u32.to_le_bytes());
        bytes.extend_from_slice(&16_000u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        bytes.resize(bytes.len() + data_len as usize, 0);
        std::fs::write(path, bytes).unwrap();
    }

    /// Writes `lyrics` into the tags of the audio file at `path`.
    fn tag_lyrics(path: &std::path::Path, lyrics: &str) {
        use lofty::{
            config::WriteOptions,
            file::{AudioFile, TaggedFileExt},
            probe::Probe,
            tag::{ItemKey, Tag, TagType},
        };
        let mut file = Probe::open(path).unwrap().read().unwrap();
        let mut tag = Tag::new(TagType::Id3v2);
        tag.insert_text(ItemKey::Lyrics, lyrics.to_string());
        file.insert_tag(tag);
        file.save_to_path(path, WriteOptions::default()).unwrap();
    }

    #[test]
    fn lyrics_in_the_file_itself_are_read_when_there_is_no_lrc() {
        let dir = tempfile::tempdir().unwrap();
        let song = dir.path().join("song.wav");
        wav(&song);
        let store = dir.path().join("store");
        assert!(matches!(
            LyricsData::load_for(&song, None, &store),
            LyricsData::None
        ));

        tag_lyrics(&song, "[00:01.00] From the tag\n[00:03.00] Second line");
        match LyricsData::load_for(&song, None, &store) {
            LyricsData::Synced(lyrics) => assert_eq!(lyrics.get_timed_lines().len(), 2),
            other => panic!("{other:?}"),
        }

        // A .lrc beside it comes first.
        std::fs::write(song.with_extension("lrc"), "[00:00.50] From the lrc").unwrap();
        match LyricsData::load_for(&song, None, &store) {
            LyricsData::Synced(lyrics) => {
                assert_eq!(lyrics.get_timed_lines()[0].1.as_ref(), "From the lrc")
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn lyrics_are_found_in_the_store_by_song_or_by_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("store");
        std::fs::create_dir_all(&store).unwrap();
        let song = dir.path().join("LiSA『紅蓮華』-MUSiC CLiP-.wav");
        wav(&song);
        let track = TrackMeta {
            id: crate::domain::TrackId(1),
            file_path: song.clone(),
            title: "紅蓮華".into(),
            artist: Some("LiSA".into()),
            album: None,
            genre: None,
            year: None,
            is_favorite: false,
            track_number: None,
            play_count: 0,
            last_played_at: None,
            duration_ms: 1000,
            bitrate: None,
            sample_rate: None,
            format: "wav".into(),
            has_cover: false,
            file_size_bytes: 16,
        };

        // Moved over by hand, named as the audio file was.
        std::fs::write(
            store.join("LiSA『紅蓮華』-MUSiC CLiP-.lrc"),
            "[00:01.00] By file",
        )
        .unwrap();
        let line = |l: LyricsData| match l {
            LyricsData::Synced(lyrics) => lyrics.get_timed_lines()[0].1.to_string(),
            other => panic!("{other:?}"),
        };
        assert_eq!(
            line(LyricsData::load_for(&song, Some(&track), &store)),
            "By file"
        );

        // Fetched, named after the song: that comes first.
        std::fs::write(store.join("LiSA - 紅蓮華.lrc"), "[00:01.00] By song").unwrap();
        assert_eq!(
            line(LyricsData::load_for(&song, Some(&track), &store)),
            "By song"
        );
    }

    #[test]
    fn stored_names_are_safe_file_names() {
        let store = Path::new("/store");
        assert_eq!(
            stored_path(store, "AC/DC: Live?", Some("ヨルシカ / n-buna")),
            Some(PathBuf::from("/store/ヨルシカ _ n-buna - AC_DC_ Live_.lrc"))
        );
        assert_eq!(
            stored_path(store, "Title", None),
            Some(PathBuf::from("/store/Title.lrc"))
        );
        assert_eq!(stored_path(store, "  ", Some("Artist")), None);
        let long = "あ".repeat(200);
        let path = stored_path(store, &long, None).unwrap();
        assert!(path.file_name().unwrap().len() <= 255);
    }
}
