use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::domain::TrackId;
use crate::navigation::{ids::SidebarNodeId, state::LeftPanelMode};
use crate::playback::state::RepeatMode;
use crate::state::{SpectrumStyle, VisualizerMode, audio::AudioState, ui::FocusMode};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistedSession {
    pub queue: Vec<TrackId>,
    pub active_track_id: Option<TrackId>,
    #[serde(default)]
    pub active_node: Option<SidebarNodeId>,
    #[serde(default)]
    pub selected_node: Option<SidebarNodeId>,
    #[serde(default)]
    pub expanded_nodes: Vec<SidebarNodeId>,
    #[serde(default)]
    pub left_panel: LeftPanelMode,
    #[serde(default)]
    pub show_track_info: bool,
    #[serde(default)]
    pub repeat_mode: RepeatMode,
    #[serde(default)]
    pub context_queue: Vec<TrackId>,
    #[serde(default)]
    pub context_idx: usize,
    #[serde(default)]
    pub source_name: String,
    #[serde(default)]
    pub shuffle: bool,
    #[serde(default)]
    pub playing_from_queue: bool,
    #[serde(default)]
    pub scroll_offset: usize,
    pub seek_position_ms: u64,
    pub volume: f32,
    pub speed: f32,
    pub focus_mode: FocusMode,
    pub visualizer_mode: VisualizerMode,
    pub spectrum_style: SpectrumStyle,
    pub dsp: PersistedDspParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedDspParams {
    pub speed: Option<f32>,
    pub bass: f32,
    pub treble: f32,
    pub reverb: f32,
    pub echo: f32,
    pub pan: f32,
    pub lowpass: f32,
    pub highpass: f32,
    pub auto_pan_rate: f32,
    pub auto_pan_depth: f32,
    pub eq_bands: [f32; 10],
    pub exciter: f32,
    pub bitcrush: f32,
    pub width: f32,
    pub karaoke: f32,
    pub wobble: f32,
    pub ping_pong: f32,
    pub phaser: f32,
}

impl Default for PersistedDspParams {
    fn default() -> Self {
        let def = AudioState::default();
        Self::from_audio_state(&def)
    }
}

impl PersistedDspParams {
    pub fn from_audio_state(audio: &AudioState) -> Self {
        Self {
            speed: Some(audio.speed),
            bass: audio.bass,
            treble: audio.treble,
            reverb: audio.reverb,
            echo: audio.echo,
            pan: audio.pan,
            lowpass: audio.lowpass,
            highpass: audio.highpass,
            auto_pan_rate: audio.auto_pan_rate,
            auto_pan_depth: audio.auto_pan_depth,
            eq_bands: audio.eq_bands,
            exciter: audio.exciter,
            bitcrush: audio.bitcrush,
            width: audio.width,
            karaoke: audio.karaoke,
            wobble: audio.wobble,
            ping_pong: audio.ping_pong,
            phaser: audio.phaser,
        }
    }

    pub fn apply_to_audio_state(&self, audio: &mut AudioState) {
        if let Some(s) = self.speed {
            audio.speed = s;
        }
        audio.bass = self.bass;
        audio.treble = self.treble;
        audio.reverb = self.reverb;
        audio.echo = self.echo;
        audio.pan = self.pan;
        audio.lowpass = self.lowpass;
        audio.highpass = self.highpass;
        audio.auto_pan_rate = self.auto_pan_rate;
        audio.auto_pan_depth = self.auto_pan_depth;
        audio.eq_bands = self.eq_bands;
        audio.exciter = self.exciter;
        audio.bitcrush = self.bitcrush;
        audio.width = self.width;
        audio.karaoke = self.karaoke;
        audio.wobble = self.wobble;
        audio.ping_pong = self.ping_pong;
        audio.phaser = self.phaser;
    }
}

pub fn expand_ancestors_of_node(
    expanded_ids: &mut std::collections::HashSet<SidebarNodeId>,
    node: &SidebarNodeId,
) {
    match node {
        SidebarNodeId::Track { parent, .. } => {
            expanded_ids.insert((**parent).clone());
            expand_ancestors_of_node(expanded_ids, parent);
        }
        SidebarNodeId::Folder(path) => {
            expanded_ids.insert(SidebarNodeId::Category(
                crate::navigation::ids::CategoryId::Folders,
            ));
            let mut curr = path.parent();
            while let Some(p) = curr {
                expanded_ids.insert(SidebarNodeId::Folder(p.to_path_buf()));
                curr = p.parent();
            }
        }
        SidebarNodeId::Artist(_) => {
            expanded_ids.insert(SidebarNodeId::Category(
                crate::navigation::ids::CategoryId::Artists,
            ));
        }
        SidebarNodeId::Album(_) => {
            expanded_ids.insert(SidebarNodeId::Category(
                crate::navigation::ids::CategoryId::Albums,
            ));
        }
        SidebarNodeId::Playlist(_) => {
            expanded_ids.insert(SidebarNodeId::Category(
                crate::navigation::ids::CategoryId::Playlists,
            ));
        }
        _ => {}
    }
}

pub fn canonicalize_track_node(
    library: &crate::library::Library,
    track_id: crate::domain::TrackId,
    node: Option<&SidebarNodeId>,
) -> Option<SidebarNodeId> {
    if let Some(SidebarNodeId::Track { id, parent }) = node
        && *id == track_id
    {
        let valid = match **parent {
            SidebarNodeId::Category(_) | SidebarNodeId::Root => false,
            SidebarNodeId::Playlist(pid) => library.playlists.contains_key(&pid),
            SidebarNodeId::Folder(_) => true,
            SidebarNodeId::Artist(ref a) => library.artists.contains_key(a),
            SidebarNodeId::Album(ref a) => library.albums.contains_key(a),
            SidebarNodeId::Genre(ref g) => library.genres.contains_key(g),
            _ => true,
        };
        if valid {
            return Some(SidebarNodeId::Track {
                id: *id,
                parent: parent.clone(),
            });
        }
    }

    if let Some(track) = library.tracks.get(&track_id)
        && let Some(dir) = track.file_path.parent()
    {
        return Some(SidebarNodeId::Track {
            id: track_id,
            parent: Box::new(SidebarNodeId::Folder(dir.to_path_buf())),
        });
    }
    None
}

impl PersistedSession {
    pub fn file_path() -> Result<PathBuf> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?
            .join("melotui");
        std::fs::create_dir_all(&data_dir)?;
        Ok(data_dir.join("last_session.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::file_path()?;
        if !path.exists() {
            anyhow::bail!("No session file exists");
        }
        let content = std::fs::read_to_string(&path)?;
        let session: Self = toml::from_str(&content)?;
        Ok(session)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::file_path()?;
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn from_app_state(state: &crate::AppState) -> Self {
        let queue: Vec<TrackId> = state
            .session
            .playback_queue
            .items
            .iter()
            .map(|item| item.track_id)
            .collect();
        Self {
            queue,
            active_track_id: state.session.playback.current,
            active_node: state.session.playback.active_node.clone(),
            selected_node: state.session.nav.library.selected_id.clone(),
            expanded_nodes: state
                .session
                .nav
                .library
                .expanded_ids
                .iter()
                .cloned()
                .collect(),
            left_panel: state.session.nav.active_panel.clone(),
            show_track_info: state.session.nav.show_track_info,
            repeat_mode: state.session.repeat_mode.clone(),
            context_queue: state.session.playback.context_queue.clone(),
            context_idx: state.session.playback.context_idx,
            source_name: state.session.playback.source_name.clone(),
            shuffle: state.session.playback.shuffle,
            playing_from_queue: state.session.playback.playing_from_queue,
            scroll_offset: state.session.nav.library.scroll_offset,
            seek_position_ms: state.session.playback.progress_ms,
            volume: state.session.audio.volume,
            speed: state.session.audio.speed,
            focus_mode: state.session.ui.focus_mode,
            visualizer_mode: state.session.visualizer_mode.clone(),
            spectrum_style: state.session.spectrum_style.clone(),
            dsp: PersistedDspParams::from_audio_state(&state.session.audio),
        }
    }

    pub fn apply_to_app_state(&self, state: &mut crate::AppState) -> usize {
        let valid_queue: Vec<TrackId> = self
            .queue
            .iter()
            .copied()
            .filter(|id| state.library.tracks.contains_key(id))
            .collect();

        state.session.playback_queue.items.clear();
        state.session.playback_queue.next_id_counter = 0;
        for &track_id in &valid_queue {
            state.session.playback_queue.add_track(track_id);
        }

        state.session.nav.active_panel = self.left_panel.clone();
        state.session.nav.show_track_info = self.show_track_info;
        state.session.repeat_mode = self.repeat_mode.clone();
        state.session.playback.shuffle = self.shuffle;
        state.session.playback.playing_from_queue = self.playing_from_queue;
        if !self.source_name.is_empty() {
            state.session.playback.source_name = self.source_name.clone();
        }

        state.session.nav.library.expanded_ids = self.expanded_nodes.iter().cloned().collect();

        if let Some(active_id) = self.active_track_id {
            if let Some(meta) = state.library.tracks.get(&active_id) {
                state.session.playback.current = Some(active_id);
                state.session.playback.duration_ms = Some(meta.duration_ms);
                state.session.playback.progress_ms = self.seek_position_ms;
                state.session.playback.status = crate::playback::state::PlaybackStatus::Paused;
            } else {
                state.session.playback.current = None;
                state.session.playback.duration_ms = None;
                state.session.playback.progress_ms = 0;
                state.session.playback.status = crate::playback::state::PlaybackStatus::Stopped;
            }
        }

        if let Some(active_id) = state.session.playback.current {
            state.session.playback.active_node =
                canonicalize_track_node(&state.library, active_id, self.active_node.as_ref());
        } else if let Some(ref node) = self.active_node {
            state.session.playback.active_node = Some(node.clone());
        }

        let valid_context: Vec<TrackId> = self
            .context_queue
            .iter()
            .copied()
            .filter(|id| state.library.tracks.contains_key(id))
            .collect();
        if valid_context.is_empty()
            && let Some(SidebarNodeId::Track { ref parent, id }) =
                state.session.playback.active_node
        {
            state.session.playback.context_queue = state.library.resolve_track_ids_for_node(parent);
            if state.session.playback.context_queue.is_empty() {
                state.session.playback.context_queue = vec![id];
            }
            if let Some(idx) = state
                .session
                .playback
                .context_queue
                .iter()
                .position(|&t| t == id)
            {
                state.session.playback.context_idx = idx;
            }
        } else {
            state.session.playback.context_queue = valid_context;
            state.session.playback.context_idx = self
                .context_idx
                .min(state.session.playback.context_queue.len().saturating_sub(1));
        }

        if let Some(active_id) = state.session.playback.current {
            let use_active = match self.selected_node {
                Some(SidebarNodeId::Track { id, .. }) => id != active_id,
                None => true,
                _ => false,
            };
            if use_active {
                state.session.nav.library.selected_id = state.session.playback.active_node.clone();
            } else {
                state.session.nav.library.selected_id = match &self.selected_node {
                    Some(SidebarNodeId::Track { id, parent }) => canonicalize_track_node(
                        &state.library,
                        *id,
                        Some(&SidebarNodeId::Track {
                            id: *id,
                            parent: parent.clone(),
                        }),
                    ),
                    other => other.clone(),
                };
            }
        } else if let Some(ref node) = self.selected_node {
            state.session.nav.library.selected_id = match node {
                SidebarNodeId::Track { id, parent } => canonicalize_track_node(
                    &state.library,
                    *id,
                    Some(&SidebarNodeId::Track {
                        id: *id,
                        parent: parent.clone(),
                    }),
                ),
                other => Some(other.clone()),
            };
        } else if let Some(ref active_node) = state.session.playback.active_node {
            state.session.nav.library.selected_id = Some(active_node.clone());
        } else if state.session.nav.library.selected_id.is_none()
            && let Some(first_node) = crate::library::projector::get_node_at_index(
                &state.library,
                &state.session.nav.library,
                0,
            )
        {
            state.session.nav.library.selected_id = Some(first_node);
        }

        if let Some(ref sel) = state.session.nav.library.selected_id.clone() {
            expand_ancestors_of_node(&mut state.session.nav.library.expanded_ids, sel);
        }

        let total_rows =
            crate::library::projector::get_row_count(&state.library, &state.session.nav.library);
        if let Some(ref sel) = state.session.nav.library.selected_id
            && let Some(idx) = crate::library::projector::get_index_of(
                &state.library,
                &state.session.nav.library,
                sel,
            )
        {
            if self.scroll_offset > 0 && self.scroll_offset < total_rows {
                state.session.nav.library.scroll_offset = self.scroll_offset;
            } else if idx > 15 {
                state.session.nav.library.scroll_offset = idx.saturating_sub(10);
            } else {
                state.session.nav.library.scroll_offset = 0;
            }
        } else if self.scroll_offset > 0 {
            state.session.nav.library.scroll_offset =
                self.scroll_offset.min(total_rows.saturating_sub(1));
        }

        state.session.audio.volume = self.volume;
        state.session.audio.speed = self.speed;
        state.session.ui.focus_mode = self.focus_mode;
        state.session.visualizer_mode = self.visualizer_mode.clone();
        state.session.spectrum_style = self.spectrum_style.clone();
        self.dsp.apply_to_audio_state(&mut state.session.audio);

        valid_queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::TrackId;

    #[test]
    fn test_persisted_session_serialization_roundtrip() {
        let session = PersistedSession {
            queue: vec![TrackId(1), TrackId(2)],
            active_track_id: Some(TrackId(2)),
            seek_position_ms: 12345,
            volume: 0.8,
            speed: 1.25,
            focus_mode: crate::state::ui::FocusMode::Visualizer,
            visualizer_mode: crate::state::VisualizerMode::Spectrum,
            spectrum_style: crate::state::SpectrumStyle::Bar,
            dsp: PersistedDspParams {
                speed: Some(1.25),
                bass: 2.0,
                treble: -1.0,
                reverb: 0.5,
                ..Default::default()
            },
            ..Default::default()
        };

        let toml_str = toml::to_string_pretty(&session).expect("Failed to serialize");
        let deserialized: PersistedSession =
            toml::from_str(&toml_str).expect("Failed to deserialize");

        assert_eq!(deserialized.queue, vec![TrackId(1), TrackId(2)]);
        assert_eq!(deserialized.active_track_id, Some(TrackId(2)));
        assert_eq!(deserialized.seek_position_ms, 12345);
        assert_eq!(deserialized.volume, 0.8);
        assert_eq!(deserialized.speed, 1.25);
        assert_eq!(deserialized.dsp.bass, 2.0);
        assert_eq!(deserialized.dsp.treble, -1.0);
    }

    #[test]
    fn test_persisted_session_full_navigation_roundtrip() {
        use crate::navigation::ids::{CategoryId, SidebarNodeId};
        use crate::navigation::state::LeftPanelMode;

        let selected = SidebarNodeId::Category(CategoryId::Artists);
        let active = SidebarNodeId::Track {
            id: TrackId(42),
            parent: Box::new(SidebarNodeId::Category(CategoryId::Playlists)),
        };

        let session = PersistedSession {
            queue: vec![TrackId(42)],
            active_track_id: Some(TrackId(42)),
            active_node: Some(active.clone()),
            selected_node: Some(selected.clone()),
            expanded_nodes: vec![SidebarNodeId::Category(CategoryId::Artists)],
            left_panel: LeftPanelMode::Queue,
            show_track_info: true,
            repeat_mode: crate::playback::state::RepeatMode::Track,
            context_queue: vec![TrackId(42), TrackId(43)],
            context_idx: 1,
            source_name: "My Source".to_string(),
            shuffle: true,
            playing_from_queue: false,
            scroll_offset: 15,
            seek_position_ms: 5000,
            volume: 1.0,
            speed: 1.0,
            focus_mode: crate::state::ui::FocusMode::Full,
            visualizer_mode: crate::state::VisualizerMode::Spectrum,
            spectrum_style: crate::state::SpectrumStyle::Bar,
            dsp: PersistedDspParams::default(),
        };

        let toml_str = toml::to_string_pretty(&session).expect("Failed to serialize");
        let deserialized: PersistedSession =
            toml::from_str(&toml_str).expect("Failed to deserialize");

        assert_eq!(deserialized.selected_node, Some(selected));
        assert_eq!(deserialized.active_node, Some(active));
        assert_eq!(deserialized.left_panel, LeftPanelMode::Queue);
        assert!(deserialized.show_track_info);
        assert_eq!(
            deserialized.repeat_mode,
            crate::playback::state::RepeatMode::Track
        );
        assert_eq!(deserialized.context_queue, vec![TrackId(42), TrackId(43)]);
        assert_eq!(deserialized.context_idx, 1);
        assert_eq!(deserialized.source_name, "My Source");
        assert!(deserialized.shuffle);
        assert_eq!(deserialized.scroll_offset, 15);
    }

    #[test]
    fn test_apply_to_app_state_canonicalizes_selection_and_expands_ancestors() {
        use crate::domain::TrackMeta;
        use crate::library::Library;
        use crate::navigation::ids::{CategoryId, SidebarNodeId};
        use std::collections::HashMap;
        use std::path::PathBuf;

        let track_id = TrackId(100);
        let mut tracks = HashMap::new();
        tracks.insert(
            track_id,
            TrackMeta {
                id: track_id,
                file_path: PathBuf::from("/home/user/Music/Album/song.mp3"),
                title: "Test Song".to_string(),
                artist: Some("Test Artist".to_string()),
                album: Some("Test Album".to_string()),
                genre: None,
                year: None,
                is_favorite: false,
                track_number: None,
                duration_ms: 180000,
                bitrate: None,
                sample_rate: None,
                format: "mp3".into(),
                has_cover: false,
                file_size_bytes: 1024,
            },
        );

        let library = Library::new(vec![PathBuf::from("/home/user/Music")], vec![], tracks);
        let mut state = crate::AppState::new(library, &crate::config::AppConfig::default());

        let session = PersistedSession {
            active_track_id: Some(track_id),
            active_node: Some(SidebarNodeId::Track {
                id: track_id,
                parent: Box::new(SidebarNodeId::Category(CategoryId::Playlists)),
            }),
            selected_node: Some(SidebarNodeId::Track {
                id: TrackId(999),
                parent: Box::new(SidebarNodeId::Category(CategoryId::Playlists)),
            }),
            ..Default::default()
        };

        session.apply_to_app_state(&mut state);

        let expected_folder = SidebarNodeId::Folder(PathBuf::from("/home/user/Music/Album"));
        let expected_track = SidebarNodeId::Track {
            id: track_id,
            parent: Box::new(expected_folder.clone()),
        };

        assert_eq!(
            state.session.playback.active_node,
            Some(expected_track.clone())
        );
        assert_eq!(state.session.nav.library.selected_id, Some(expected_track));
        assert!(
            state
                .session
                .nav
                .library
                .expanded_ids
                .contains(&expected_folder)
        );
        assert!(
            state
                .session
                .nav
                .library
                .expanded_ids
                .contains(&SidebarNodeId::Category(CategoryId::Folders))
        );
    }
}
