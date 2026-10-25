use lrc::Lyrics;

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
        match text.parse::<Lyrics>() {
            Ok(lyrics) if !lyrics.get_timed_lines().is_empty() => LyricsData::Synced(lyrics),
            _ => LyricsData::Unsynced(text),
        }
    }

    /// The lyrics for the audio file at `audio`: from a `.lrc` file beside it
    /// if there is one, or else from the lyrics in its own tags.
    pub fn load_for(audio: &std::path::Path) -> Self {
        if let Ok(text) = std::fs::read_to_string(audio.with_extension("lrc")) {
            let lyrics = Self::from_text(text);
            if !matches!(lyrics, LyricsData::None) {
                return lyrics;
            }
        }
        embedded_lyrics(audio).map_or(LyricsData::None, Self::from_text)
    }
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

#[derive(Debug, Clone, Default)]
pub struct LyricsState {
    pub data: LyricsData,
    pub active_line_index: Option<usize>,
    pub search_state: Option<LyricsSearchState>,
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
        assert!(matches!(LyricsData::load_for(&song), LyricsData::None));

        tag_lyrics(&song, "[00:01.00] From the tag\n[00:03.00] Second line");
        match LyricsData::load_for(&song) {
            LyricsData::Synced(lyrics) => assert_eq!(lyrics.get_timed_lines().len(), 2),
            other => panic!("{other:?}"),
        }

        // A .lrc beside it comes first.
        std::fs::write(song.with_extension("lrc"), "[00:00.50] From the lrc").unwrap();
        match LyricsData::load_for(&song) {
            LyricsData::Synced(lyrics) => {
                assert_eq!(lyrics.get_timed_lines()[0].1.as_ref(), "From the lrc")
            }
            other => panic!("{other:?}"),
        }
    }
}
