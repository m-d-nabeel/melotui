use std::sync::Arc;

use crate::{
    Library, Playlist,
    library::{LibraryPersistence, Track},
    state::{LeftPanelMode, PlaybackContext, RepeatMode, SessionState},
    utils::{NoOpSpectrum, SpectrumSource},
};

pub struct AppState {
    pub app_name: String,

    // Shared core data
    pub library: Arc<Library>,

    // User session state
    pub session: SessionState,

    // Runtime buffers
    pub time_elapsed: f32,
    pub last_nav_time: f32,
    pub spectrum_raw: Vec<f32>,
    pub spectrum_smooth: Vec<f32>,
    pub spectrum_peaks: Vec<f32>,
    pub spectrum_source: Box<dyn SpectrumSource>,

    // Cached resources
    pub current_album_art: Option<ratatui::text::Text<'static>>,
    pub track_info_art: Option<ratatui::text::Text<'static>>,
}

impl AppState {
    pub fn new(library: Library) -> Self {
        let playlist_count = library.playlists.len();
        let session = SessionState {
            nav: crate::state::NavigationState::new(playlist_count),
            ..Default::default()
        };

        Self {
            app_name: "Melotui".into(),
            library: Arc::new(library),
            session,
            time_elapsed: 0.0,
            last_nav_time: 0.0,
            spectrum_raw: vec![0.0; 512],
            spectrum_smooth: vec![0.0; 512],
            spectrum_peaks: vec![0.0; 512],
            spectrum_source: Box::new(NoOpSpectrum),
            current_album_art: None,
            track_info_art: None,
        }
    }
}

impl AppState {
    pub fn get_playing_track(&self) -> Option<&Track> {
        let active = self.session.playback.current.as_ref()?;

        match &active.context {
            PlaybackContext::Library { .. } => self.library.get_track(active.id),
            PlaybackContext::Queue => self.session.playback_queue.get_track_by_id(active.id),
        }
    }

    pub fn get_selected_playlist(&self) -> Option<&Playlist> {
        self.library
            .get_selected_playlist(&self.session.nav.library)
    }

    pub fn get_selected_track(&self) -> Option<&Track> {
        match self.session.nav.active_panel {
            LeftPanelMode::Library => self.library.get_selected_track(&self.session.nav.library),
            LeftPanelMode::Queue => self
                .session
                .playback_queue
                .get_selected_track(&self.session.nav.queue),
        }
    }

    pub fn is_playing_from_queue(&self) -> bool {
        self.session
            .playback
            .current
            .as_ref()
            .is_some_and(|c| matches!(c.context, PlaybackContext::Queue))
    }

    pub fn get_playing_playlist_name(&self) -> Option<&str> {
        self.session.playback.current.as_ref().and_then(|active| {
            if let PlaybackContext::Library { playlist_name } = &active.context {
                Some(playlist_name.as_str())
            } else {
                None
            }
        })
    }

    pub fn predict_next_song(&self) -> String {
        if self.is_playing_from_queue() {
            if self.session.playback_queue.tracks.len() > 1 {
                return self.session.playback_queue.tracks[1].title.clone();
            } else {
                return "None (End of Queue)".to_string();
            }
        }

        if !self.session.playback_queue.tracks.is_empty() {
            return self.session.playback_queue.tracks[0].title.clone();
        }

        if self.session.playback.shuffle {
            return "Random (Shuffle)".to_string();
        }

        if self.session.repeat_mode == RepeatMode::Track && self.session.playback.current.is_some()
        {
            return "Repeating Track".to_string();
        }

        if let Some(current) = &self.session.playback.current
            && let Some(playlist) = self.library.playlists.iter().find(|p| {
                matches!(current.context, PlaybackContext::Library { ref playlist_name }
                    if playlist_name == &p.name)
            })
            && let Some(idx) = playlist.tracks.iter().position(|t| t.id == current.id)
        {
            let next_idx = idx + 1;
            if next_idx < playlist.tracks.len() {
                return playlist.tracks[next_idx].title.clone();
            } else if self.session.repeat_mode == RepeatMode::Playlist
                && let Some(first) = playlist.tracks.first()
            {
                return first.title.clone();
            }
        }

        "None".to_string()
    }
}
