use std::{collections::VecDeque, sync::Arc};

use crate::{
    Library, Playlist,
    library::{LibraryPersistence, Track},
    state::{
        LeftPanelMode, LibraryViewDepth, PlaybackContext, PlaybackStatus, RepeatMode, SessionState,
        SpectrumStyle, VisualizerMode, notification::Notification, ui::UIMode,
    },
    utils::{NoOpSpectrum, SpectrumSource},
};

pub struct AppState {
    pub app_name: String,

    // Shared core data
    pub library: Arc<Library>,

    // User session state
    pub(crate) session: SessionState,

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
/// ─── Audio Feature Access ───────────────────────────────────────────────────
/// Contains Selectors for UI rendering and Mutators for Reducer actions.
impl AppState {
    // --- Selectors (Public for UI) ---
    pub fn volume(&self) -> f32 {
        self.session.audio.volume
    }

    pub fn speed(&self) -> f32 {
        self.session.audio.speed
    }

    pub fn bass(&self) -> f32 {
        self.session.audio.bass
    }

    pub fn treble(&self) -> f32 {
        self.session.audio.treble
    }

    pub fn reverb(&self) -> f32 {
        self.session.audio.reverb
    }

    pub fn echo(&self) -> f32 {
        self.session.audio.echo
    }

    // --- Mutators (Internal for Reducer) ---
    // Usage: state.set_volume(new_vol) instead of state.session.audio.volume = new_vol

    pub(crate) fn set_volume(&mut self, value: f32) {
        self.session.audio.volume = value.clamp(0.0, 1.5);
    }

    pub(crate) fn set_speed(&mut self, value: f32) {
        self.session.audio.speed = value.clamp(0.5, 2.0);
    }

    pub(crate) fn set_bass(&mut self, value: f32) {
        self.session.audio.bass = value.clamp(0.1, 5.0);
    }

    pub(crate) fn set_treble(&mut self, value: f32) {
        self.session.audio.treble = value.clamp(0.0, 5.0);
    }

    pub(crate) fn set_reverb(&mut self, value: f32) {
        self.session.audio.reverb = value.clamp(0.0, 1.0);
    }

    pub(crate) fn set_echo(&mut self, value: f32) {
        self.session.audio.echo = value.clamp(0.0, 0.9);
    }
}

/// ─── Playback & Progress Access ─────────────────────────────────────────────
/// Encapsulates playback status checks and progress math.
impl AppState {
    // --- Status Checks ---
    pub fn is_playing(&self) -> bool {
        matches!(self.session.playback.status, PlaybackStatus::Playing)
    }

    pub fn is_paused(&self) -> bool {
        matches!(self.session.playback.status, PlaybackStatus::Paused)
    }

    pub fn is_stopped(&self) -> bool {
        matches!(self.session.playback.status, PlaybackStatus::Stopped)
    }

    pub fn playback_status(&self) -> &PlaybackStatus {
        &self.session.playback.status
    }

    pub fn shuffle_active(&self) -> bool {
        self.session.playback.shuffle
    }

    pub fn repeat_mode(&self) -> &RepeatMode {
        &self.session.repeat_mode
    }

    pub fn is_playing_from_queue(&self) -> bool {
        self.session
            .playback
            .current
            .as_ref()
            .is_some_and(|c| matches!(c.context, PlaybackContext::Queue))
    }

    // --- Progress Calculations ---

    /// Returns the current progress ratio (0.0 to 1.0)
    pub fn progress_ratio(&self) -> f64 {
        let current = self.session.playback.progress_ms as f64;
        let total = self.total_duration_ms() as f64;

        if total > 0.0 {
            (current / total).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    pub fn progress_ms(&self) -> u64 {
        self.session.playback.progress_ms
    }

    /// Smart duration fetcher: gets from track, fallback to state, fallback to 0
    pub fn total_duration_ms(&self) -> u64 {
        self.get_playing_track()
            .map(|t| t.duration_ms)
            .or(self.session.playback.duration_ms)
            .unwrap_or(0)
    }

    // --- Track Info Getters ---

    pub fn get_playing_track(&self) -> Option<&Track> {
        let active = self.session.playback.current.as_ref()?;

        match &active.context {
            PlaybackContext::Library { .. } => self.library.get_track(active.id),
            PlaybackContext::Queue => self.session.playback_queue.get_track_by_id(active.id),
        }
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
}

/// ─── Navigation & View Access ───────────────────────────────────────────────
/// Helpers for Sidebar, Input, and Layout logic.
impl AppState {
    // --- Active Panel Checks ---
    pub fn is_library_focused(&self) -> bool {
        matches!(self.session.nav.active_panel, LeftPanelMode::Library)
    }

    pub fn is_queue_focused(&self) -> bool {
        matches!(self.session.nav.active_panel, LeftPanelMode::Queue)
    }

    // --- View Depth Checks ---
    pub fn is_playlist_view(&self) -> bool {
        self.session.nav.library.depth == LibraryViewDepth::Playlists
    }

    pub fn is_track_list_view(&self) -> bool {
        self.session.nav.library.depth == LibraryViewDepth::Tracks
    }

    pub fn is_track_info_view(&self) -> bool {
        self.session.nav.library.depth == LibraryViewDepth::TrackInfo
    }

    // --- Selection Getters ---
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

    // Direct queue access for UI list rendering
    pub fn queue(&self) -> &VecDeque<Track> {
        &self.session.playback_queue.tracks
    }

    // --- Selection Index Getters (for UI rendering/navigation) ---
    pub fn selected_playlist_index(&self) -> usize {
        self.session.nav.library.selected_playlist_idx
    }

    pub fn selected_queue_index(&self) -> usize {
        self.session.nav.queue.selected_idx
    }

    pub fn selected_track_index_for_playlist(&self, playlist_idx: usize) -> usize {
        self.session
            .nav
            .library
            .selected_tracks_curs
            .get(playlist_idx)
            .copied()
            .unwrap_or(0)
    }

    // --- UI State Accessors ---
    pub fn ui_mode(&self) -> &UIMode {
        &self.session.ui.mode
    }

    pub fn ui_show_help(&self) -> bool {
        self.session.ui.show_help
    }

    pub fn ui_command_buffer(&self) -> &String {
        &self.session.ui.command_buffer
    }

    // --- Notification Accessor ---
    pub fn current_notification(&self) -> Option<&Notification> {
        self.session.notification.current.as_ref()
    }

    // --- Notification Mutator ---
    pub fn clear_current_notification(&mut self) {
        self.session.notification.current = None;
    }
}

/// ─── Visualizer Access ──────────────────────────────────────────────────────
/// Helpers for the Visualizer View.
impl AppState {
    pub fn should_draw_spectrum(&self) -> bool {
        self.session.visualizer_mode == VisualizerMode::Spectrum
    }

    pub fn spectrum_style(&self) -> &SpectrumStyle {
        &self.session.spectrum_style
    }

    /// Returns a tuple of (Smooth Data, Peak Data)
    pub fn get_spectrum_data(&self) -> (&Vec<f32>, &Vec<f32>) {
        (&self.spectrum_smooth, &self.spectrum_peaks)
    }
}

/// ─── Prediction Logic ───────────────────────────────────────────────────────
impl AppState {
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
