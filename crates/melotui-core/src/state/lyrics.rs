use lrc::Lyrics;

#[derive(Debug, Clone, Default)]
pub enum LyricsData {
    Synced(Lyrics),
    Unsynced(String),
    #[default]
    None,
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
        let t = lyrics[0].0.get_timestamp();
    }
}
