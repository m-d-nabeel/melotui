use crate::{
    bytes_to_ascii,
    domain::{PlaylistMeta, TrackId, TrackMeta},
    library::{projector::flatten_tree, snapshot::LibrarySnapshot},
    navigation::{
        ids::SidebarNodeId,
        state::{LeftPanelMode, LibraryNavState},
    },
    playback::state::{PlaybackStatus, QueueEntry, QueueId, RepeatMode},
    state::{Notification, NotificationKind, SessionState, SpectrumStyle, UIMode, VisualizerMode},
    utils::{NoOpSpectrum, SizeMode, SpectrumSource},
};
use std::{collections::VecDeque, sync::Arc};

pub struct AppState {
    pub app_name: String,

    // ─── SHARED CORE DATA (Immutable/Arc) ───
    pub library: Arc<LibrarySnapshot>,

    // ─── USER SESSION STATE (Mutable) ───
    pub(crate) session: SessionState,

    // ─── RUNTIME BUFFERS (Ephemeral) ───
    pub time_elapsed: f32,
    pub last_nav_time: f32,
    pub spectrum_raw: Vec<f32>,
    pub spectrum_smooth: Vec<f32>,
    pub spectrum_peaks: Vec<f32>,
    pub spectrum_source: Box<dyn SpectrumSource>,

    // ─── CACHED RESOURCES ───
    pub current_album_art: Option<ratatui::text::Text<'static>>,
    pub track_info_art: Option<ratatui::text::Text<'static>>,
}

// ─── LIFECYCLE ───────────────────────────────────────────────────────────────
impl AppState {
    pub fn new(library: LibrarySnapshot) -> Self {
        let mut session = SessionState::default();

        let library_arc = Arc::new(library);
        let rows = flatten_tree(&library_arc, &session.nav.library);
        if let Some(row) = rows.first() {
            session.nav.library.selected_id = Some(row.id.clone());
        }

        Self {
            app_name: "Melotui".into(),
            library: library_arc,
            session,

            // Runtime Init
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

// ─── AUDIO SYSTEM ACCESS ─────────────────────────────────────────────────────
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

// ─── PLAYBACK STATE ACCESS ───────────────────────────────────────────────────
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

    // --- Progress Logic ---
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

    // --- Track Lookups ---
    pub fn get_playing_track_id(&self) -> Option<TrackId> {
        self.session.playback.current
    }

    pub fn get_playing_track(&self) -> Option<&TrackMeta> {
        let id = self.get_playing_track_id()?;
        self.library.get_track(id)
    }

    pub fn get_playing_source_name(&self) -> &str {
        if self.session.playback.playing_from_queue {
            "User Queue"
        } else if self.session.playback.source_name.is_empty() {
            "Unknown"
        } else {
            &self.session.playback.source_name
        }
    }

    pub fn context_queue(&self) -> &Vec<TrackId> {
        &self.session.playback.context_queue
    }
}

// ─── NAVIGATION & VIEW QUERYING ──────────────────────────────────────────────
impl AppState {
    /// Returns the Read-Only Library Navigation State.
    /// Required by the `projector::flatten_tree` function.
    pub fn library_nav(&self) -> &LibraryNavState {
        &self.session.nav.library
    }

    /// Returns the ID of the currently selected node in the sidebar.
    pub fn get_library_selection(&self) -> Option<&SidebarNodeId> {
        self.session.nav.library.selected_id.as_ref()
    }

    pub fn is_library_focused(&self) -> bool {
        self.session.nav.active_panel == LeftPanelMode::Library
    }

    pub fn is_queue_focused(&self) -> bool {
        self.session.nav.active_panel == LeftPanelMode::Queue
    }

    pub fn time_elapsed(&self) -> f32 {
        self.time_elapsed
    }

    pub fn is_queue_info_view(&self) -> bool {
        self.session.nav.queue.show_info
    }

    /// Returns true if we are viewing the *contents* of a playlist or folder.
    /// (Replaces old 'is_track_list_view')
    pub fn is_viewing_collection(&self) -> bool {
        matches!(
            &self.session.nav.library.selected_id,
            Some(SidebarNodeId::Playlist(_)) | Some(SidebarNodeId::Folder(_))
        )
    }

    /// Returns the Playlist Metadata if one is currently selected/highlighted.
    /// (ID-Based Replacement for get_selected_playlist)
    pub fn get_active_playlist(&self) -> Option<&PlaylistMeta> {
        match &self.session.nav.library.selected_id {
            Some(SidebarNodeId::Playlist(id)) => self.library.get_playlist(*id),
            _ => None,
        }
    }

    /// Resolves the currently selected track *in the sidebar* (if any).
    /// If a Playlist is selected, this might need to look at `selection_memory` if we implemented that,
    /// but based on strict sidebar-only navigation, checking the node type is sufficient.
    pub fn get_selected_track_id_in_sidebar(&self) -> Option<TrackId> {
        match &self.session.nav.library.selected_id {
            Some(SidebarNodeId::Track { id, .. }) => Some(*id),
            _ => None,
        }
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

// ─── QUEUE ACCESSORS ─────────────────────────────────────────────────────────
impl AppState {
    pub fn queue(&self) -> &VecDeque<QueueEntry> {
        &self.session.playback_queue.items
    }

    /// Returns the Unique ID of the selected row
    pub fn get_queue_selection_id(&self) -> Option<QueueId> {
        self.session.nav.queue.selected_queue_id
    }

    /// Resolves the Queue Selection to a TrackId (for info/art display)
    pub fn get_selected_track_id_in_queue(&self) -> Option<TrackId> {
        let sel_id = self.session.nav.queue.selected_queue_id?;
        self.session
            .playback_queue
            .items
            .iter()
            .find(|e| e.id == sel_id)
            .map(|e| e.track_id)
    }

    pub fn get_active_qid(&self) -> Option<QueueId> {
        self.session.playback.active_queue_id
    }
}

// ─── UI & VISUAL STATE ───────────────────────────────────────────────────────
impl AppState {
    pub fn ui_mode(&self) -> &UIMode {
        &self.session.ui.mode
    }

    pub fn ui_show_help(&self) -> bool {
        self.session.ui.show_help
    }

    pub fn ui_command_buffer(&self) -> &String {
        &self.session.ui.command_buffer
    }

    pub fn current_notification(&self) -> Option<&Notification> {
        self.session.notification.current.as_ref()
    }

    pub fn set_notification(&mut self, kind: NotificationKind, message: impl Into<String>) {
        self.session.notification.current = Some(Notification::new(kind, message));
    }

    pub fn clear_current_notification(&mut self) {
        self.session.notification.current = None;
    }

    /// Loads ASCII art for a specific track into the `track_info_art` buffer.
    pub fn load_track_info_art(&mut self, track: &TrackMeta) {
        self.track_info_art = None;

        if track.has_cover
            && let Some((data, _)) = LibrarySnapshot::get_album_art(&track.file_path)
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

// ─── PREDICTION LOGIC ────────────────────────────────────────────────────────
impl AppState {
    pub fn predict_next_song(&self) -> String {
        // 1. Priority: User Queue
        if let Some(qid) = self.session.playback.active_queue_id {
            if let Some(idx) = self.queue().iter().position(|e| e.id == qid)
                && idx + 1 < self.queue().len()
            {
                return self
                    .library
                    .get_track(self.queue()[idx + 1].track_id)
                    .map(|t| t.title.clone())
                    .unwrap_or("Unknown".into());
            }
        } else if let Some(first) = self.queue().front() {
            return self
                .library
                .get_track(first.track_id)
                .map(|t| t.title.clone())
                .unwrap_or("Unknown".into());
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
                .unwrap_or("Repeating".into());
        }

        let next = idx + 1;
        if next < ctx.len() {
            return self
                .library
                .get_track(ctx[next])
                .map(|t| t.title.clone())
                .unwrap_or("Unknown".into());
        }

        if self.session.repeat_mode == RepeatMode::Playlist {
            return self
                .library
                .get_track(ctx[0])
                .map(|t| t.title.clone())
                .unwrap_or("Unknown".into());
        }

        "None".into()
    }
}
