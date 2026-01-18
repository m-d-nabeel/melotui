use std::{collections::VecDeque, sync::Arc};

use crate::{
    SizeMode, bytes_to_ascii,
    domain::{PlaylistMeta, TrackId, TrackMeta},
    library::Library,
    navigation::state::{LeftPanelMode, LibraryNavState, LibraryViewDepth, NavigationState},
    playback::state::{PlaybackStatus, RepeatMode},
    state::{Notification, SessionState, SpectrumStyle, VisualizerMode, ui::UIMode},
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
            nav: NavigationState::new(playlist_count),
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
impl AppState {
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
}

/// ─── Playback & Progress Access ─────────────────────────────────────────────
impl AppState {
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
        self.session.playback.playing_from_queue
    }

    pub fn is_queue_info_view(&self) -> bool {
        self.session.nav.queue.show_info
    }

    // --- Progress Calculations ---
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

    pub fn total_duration_ms(&self) -> u64 {
        self.get_playing_track()
            .map(|t| t.duration_ms)
            .or(self.session.playback.duration_ms)
            .unwrap_or(0)
    }

    // --- Track Info Getters ---
    pub fn get_playing_track_id(&self) -> Option<TrackId> {
        self.session.playback.current
    }

    pub fn get_playing_track(&self) -> Option<&TrackMeta> {
        let id = self.get_playing_track_id()?;
        self.library.get_track(id)
    }

    /// Returns the name of the current source (e.g. "Playlist: Jazz", "User Queue").
    pub fn get_playing_source_name(&self) -> &str {
        if self.session.playback.playing_from_queue {
            "User Queue"
        } else if self.session.playback.source_name.is_empty() {
            "Unknown"
        } else {
            &self.session.playback.source_name
        }
    }

    pub fn get_playing_playlist_name(&self) -> Option<&str> {
        if self.session.playback.source_name.is_empty() {
            None
        } else {
            Some(&self.session.playback.source_name)
        }
    }

    pub fn queue(&self) -> &VecDeque<TrackId> {
        &self.session.playback_queue.items
    }

    pub fn get_queue_track(&self, idx: usize) -> Option<&TrackMeta> {
        let id = *self.session.playback_queue.items.get(idx)?;
        self.library.get_track(id)
    }

    pub fn track_meta(&self, id: TrackId) -> Option<TrackMeta> {
        self.library.get_track(id).cloned()
    }

    pub fn current_track_meta(&self) -> Option<TrackMeta> {
        self.session
            .playback
            .current
            .and_then(|id| self.track_meta(id))
    }
}

/// ─── Navigation & View Access ───────────────────────────────────────────────
impl AppState {
    pub fn is_library_focused(&self) -> bool {
        self.session.nav.active_panel == LeftPanelMode::Library
    }

    pub fn is_queue_focused(&self) -> bool {
        self.session.nav.active_panel == LeftPanelMode::Queue
    }

    pub fn is_playlist_view(&self) -> bool {
        self.is_library_focused() && self.session.nav.library.depth == LibraryViewDepth::Playlists
    }

    pub fn is_track_list_view(&self) -> bool {
        self.is_library_focused() && self.session.nav.library.depth == LibraryViewDepth::Tracks
    }

    pub fn is_track_info_view(&self) -> bool {
        self.is_library_focused() && self.session.nav.library.depth == LibraryViewDepth::TrackInfo
    }

    pub fn get_selected_playlist(&self, nav: &LibraryNavState) -> Option<&PlaylistMeta> {
        self.playlists.get(nav.selected_playlist_idx)
    }
    pub fn get_selected_track(&self) -> Option<TrackId> {
        let pl = self
            .library
            .get_selected_playlist(&self.session.nav.library)?;
        let p_idx = self.selected_playlist_index();
        let t_idx = self.selected_track_index_for_playlist(p_idx);
        pl.tracks.get(t_idx).map(|t| t.id)
    }

    pub fn context_queue(&self) -> &Vec<TrackId> {
        &self.session.playback.context_queue
    }

    pub fn current_context_index(&self) -> usize {
        self.session.playback.context_idx
    }

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
    pub fn set_notification(&mut self, notification: Notification) {
        self.session.notification.current = Some(notification);
    }

    pub fn clear_current_notification(&mut self) {
        self.session.notification.current = None;
    }
}

/// ─── Visualizer Access ──────────────────────────────────────────────────────
impl AppState {
    pub fn should_draw_spectrum(&self) -> bool {
        self.session.visualizer_mode == VisualizerMode::Spectrum
    }

    pub fn spectrum_style(&self) -> &SpectrumStyle {
        &self.session.spectrum_style
    }

    pub fn get_spectrum_data(&self) -> (&Vec<f32>, &Vec<f32>) {
        (&self.spectrum_smooth, &self.spectrum_peaks)
    }
}

/// ─── Prediction Logic ───────────────────────────────────────────────────────
impl AppState {
    pub fn predict_next_song(&self) -> String {
        if let Some(id) = self.session.playback_queue.items.front() {
            return self
                .library
                .get_track(*id)
                .map(|t| t.title.clone())
                .unwrap_or_else(|| "Unknown".into());
        }

        let ctx = &self.session.playback.context_queue;
        let idx = self.session.playback.context_idx;

        if ctx.is_empty() {
            return "None".into();
        }

        if self.session.repeat_mode == RepeatMode::Track {
            return self
                .get_playing_track()
                .map(|t| t.title.clone())
                .unwrap_or_else(|| "Repeating".into());
        }

        let next = idx + 1;
        if next < ctx.len() {
            return self
                .library
                .get_track(ctx[next])
                .map(|t| t.title.clone())
                .unwrap_or_else(|| "Unknown".into());
        }

        if self.session.repeat_mode == RepeatMode::Playlist {
            return self
                .library
                .get_track(ctx[0])
                .map(|t| t.title.clone())
                .unwrap_or_else(|| "Unknown".into());
        }

        "None".into()
    }

    pub fn reconcile_nav(&mut self) {
        let playlist_count = self.library.playlists.len();

        if playlist_count == 0 {
            self.session.nav.library.selected_playlist_idx = 0;
            self.session.nav.library.selected_tracks_curs.clear();
        } else {
            if self.selected_playlist_index() >= playlist_count {
                self.session.nav.library.selected_playlist_idx = playlist_count - 1;
            }

            self.session
                .nav
                .library
                .selected_tracks_curs
                .resize(playlist_count, 0);

            for (i, playlist) in self.library.playlists.iter().enumerate() {
                let track_count = playlist.tracks.len();
                let cursor = self.session.nav.library.selected_tracks_curs[i];

                if track_count == 0 {
                    self.session.nav.library.selected_tracks_curs[i] = 0;
                } else if cursor >= track_count {
                    self.session.nav.library.selected_tracks_curs[i] = track_count - 1;
                }
            }
        }

        let queue_len = self.queue().len();

        if queue_len == 0 {
            self.session.nav.queue.selected_idx = 0;
        } else if self.selected_queue_index() >= queue_len {
            self.session.nav.queue.selected_idx = queue_len - 1;
        }
    }

    pub fn load_track_info_art(&mut self, track: &TrackMeta) {
        self.track_info_art = None;

        if track.has_cover
            && let Some((data, _)) = Library::get_album_art(&track.file_path)
            && let Ok(ascii) = bytes_to_ascii(
                &data,
                SizeMode::Cover {
                    width: 32,
                    height: 15,
                },
            )
        {
            self.track_info_art = Some(ascii);
        }
    }
}
