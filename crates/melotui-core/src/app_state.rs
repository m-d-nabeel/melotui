use std::sync::Arc;

use crate::{
    Library, Playlist,
    library::{LibraryPersistence, Track},
    state::{LeftPanelMode, PlaybackContext, SessionState},
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
}
