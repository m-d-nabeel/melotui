use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::domain::TrackId;
use crate::state::{SpectrumStyle, VisualizerMode, audio::AudioState, ui::FocusMode};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistedSession {
    pub queue: Vec<TrackId>,
    pub active_track_id: Option<TrackId>,
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

        if let Some(active_id) = self.active_track_id {
            if state.library.tracks.contains_key(&active_id) {
                state.session.playback.current = Some(active_id);
                state.session.playback.progress_ms = self.seek_position_ms;
                state.session.playback.status = crate::playback::state::PlaybackStatus::Paused;
            } else {
                state.session.playback.current = None;
                state.session.playback.progress_ms = 0;
                state.session.playback.status = crate::playback::state::PlaybackStatus::Stopped;
            }
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
}
