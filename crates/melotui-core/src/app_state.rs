use crate::{
    domain::{PlaylistMeta, TrackId, TrackMeta},
    library::projector,
    library::snapshot::Library,
    navigation::{
        ids::SidebarNodeId,
        state::{LeftPanelMode, LibraryNavState},
    },
    playback::state::{PlaybackStatus, QueueEntry, QueueId, RepeatMode},
    state::{
        Notification, NotificationKind, SessionState, SpectrumStyle, UIMode, VisualizerMode,
        ui::PlaylistPickerState,
    },
    utils::{NoOpSpectrum, SpectrumSource},
};
use rand::Rng;
use std::{collections::VecDeque, sync::Arc};

pub struct AppState {
    pub app_name: String,

    // shared data (immutable)
    pub library: Library,

    // user session state
    pub(crate) session: SessionState,

    // runtime buffers
    pub time_elapsed: f32,
    pub last_nav_time: f32,
    pub spectrum_raw: Vec<f32>,
    pub spectrum_smooth: Vec<f32>,
    pub spectrum_peaks: Vec<f32>,
    pub spectrum_source: Box<dyn SpectrumSource>,

    // cached resources
    pub current_cover: Option<Arc<image::DynamicImage>>,
    pub track_info_cover: Option<Arc<image::DynamicImage>>,
    pub track_info_cover_id: Option<TrackId>,
}

// lifecycle
impl AppState {
    pub fn new(library: Library, config: &crate::config::AppConfig) -> Self {
        let mut session = SessionState::default();

        // Apply config to session
        session.ui.sidebar_width = config.ui.sidebar_width;
        session.ui.scrolloff = config.ui.scrolloff;

        // Apply audio defaults from config
        session.audio.speed = config.audio.default_speed;
        session.audio.bass = config.audio.default_bass;
        session.audio.treble = config.audio.default_treble;
        session.audio.reverb = config.audio.default_reverb;
        session.audio.echo = config.audio.default_echo;

        // select first item
        if let Some(first_node) = projector::get_node_at_index(&library, &session.nav.library, 0) {
            session.nav.library.selected_id = Some(first_node);
        }

        Self {
            app_name: "Melotui".into(),
            library,
            session,

            // runtime init
            time_elapsed: 0.0,
            last_nav_time: 0.0,
            spectrum_raw: vec![0.0; 512],
            spectrum_smooth: vec![0.0; 512],
            spectrum_peaks: vec![0.0; 512],
            spectrum_source: Box::new(NoOpSpectrum),
            current_cover: None,
            track_info_cover: None,
            track_info_cover_id: None,
        }
    }
}

// audio system access
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

// playback state access
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

    // progress logic
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

    // track lookups
    pub fn get_playing_track_id(&self) -> Option<TrackId> {
        self.session.playback.current
    }

    pub fn get_playing_node(&self) -> Option<&SidebarNodeId> {
        self.session.playback.active_node.as_ref()
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

    pub fn get_next_context_id(&self) -> Option<TrackId> {
        let ctx = &self.session.playback.context_queue;
        if ctx.is_empty() {
            return None;
        }

        let next = if self.shuffle_active() {
            rand::rng().random_range(0..ctx.len())
        } else {
            let n = self.session.playback.context_idx + 1;
            if n < ctx.len() {
                n
            } else if self.session.repeat_mode == RepeatMode::Playlist {
                0
            } else {
                return None;
            }
        };
        Some(ctx[next])
    }

    pub fn get_prev_context_id(&self) -> Option<TrackId> {
        let ctx = &self.session.playback.context_queue;
        if ctx.is_empty() {
            return None;
        }

        let prev = if self.session.playback.context_idx > 0 {
            self.session.playback.context_idx - 1
        } else if self.session.repeat_mode == RepeatMode::Playlist {
            ctx.len() - 1
        } else {
            return None;
        };
        Some(ctx[prev])
    }
}

// Navigation & view querying
impl AppState {
    /// read-only library nav state
    pub fn library_nav(&self) -> &LibraryNavState {
        &self.session.nav.library
    }

    pub fn search_matches(&self) -> &[SidebarNodeId] {
        &self.session.nav.library.search.matches
    }

    pub fn is_download_active(&self) -> bool {
        matches!(
            self.session.download.status,
            crate::state::DownloadStatus::Running { .. }
        )
    }

    /// selected sidebar node id
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

    pub fn is_showing_track_info(&self) -> bool {
        self.session.nav.show_track_info
    }

    /// viewing playlist or folder?
    pub fn is_viewing_collection(&self) -> bool {
        matches!(
            &self.session.nav.library.selected_id,
            Some(SidebarNodeId::Playlist(_)) | Some(SidebarNodeId::Folder(_))
        )
    }

    /// get selected playlist meta
    pub fn get_active_playlist(&self) -> Option<&PlaylistMeta> {
        match &self.session.nav.library.selected_id {
            Some(SidebarNodeId::Playlist(id)) => self.library.get_playlist(*id),
            Some(SidebarNodeId::Category(_)) => None,
            Some(SidebarNodeId::Folder(_)) => None,
            Some(SidebarNodeId::Track { .. }) => None,
            Some(SidebarNodeId::Artist(_)) => None,
            Some(SidebarNodeId::Album(_)) => None,
            Some(SidebarNodeId::Genre(_)) => None,
            Some(SidebarNodeId::Year(_)) => None,
            Some(SidebarNodeId::Root) => None,
            None => None,
        }
    }

    /// get selected track in sidebar
    pub fn get_selected_track_id_in_sidebar(&self) -> Option<TrackId> {
        match &self.session.nav.library.selected_id {
            Some(SidebarNodeId::Track { id, .. }) => Some(*id),
            Some(SidebarNodeId::Category(_)) => None,
            Some(SidebarNodeId::Folder(_)) => None,
            Some(SidebarNodeId::Playlist(_)) => None,
            Some(SidebarNodeId::Artist(_)) => None,
            Some(SidebarNodeId::Album(_)) => None,
            Some(SidebarNodeId::Genre(_)) => None,
            Some(SidebarNodeId::Year(_)) => None,
            Some(SidebarNodeId::Root) => None,
            None => None,
        }
    }
}

// visualizer access
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

// queue accessors
impl AppState {
    pub fn queue(&self) -> &VecDeque<QueueEntry> {
        &self.session.playback_queue.items
    }

    /// selected queue row id
    pub fn get_queue_selection_id(&self) -> Option<QueueId> {
        self.session.nav.queue.selected_queue_id
    }

    /// selected track in queue
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

// ui & visual state
impl AppState {
    pub fn ui_mode(&self) -> &UIMode {
        &self.session.ui.mode
    }

    pub fn ui_show_help(&self) -> bool {
        self.session.ui.show_help
    }

    pub fn ui_command_buffer(&self) -> &tui_input::Input {
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

    /// clear track info cover
    pub fn clear_track_info_cover(&mut self) {
        self.track_info_cover = None;
        self.track_info_cover_id = None;
    }

    pub fn scrolloff(&self) -> usize {
        self.session.ui.scrolloff
    }

    pub fn terminal_size(&self) -> (u16, u16) {
        self.session.ui.terminal_size
    }

    pub fn playlist_picker(&self) -> &PlaylistPickerState {
        &self.session.ui.playlist_picker
    }

    pub fn confirm_dialog(&self) -> Option<&crate::state::ui::ConfirmDialog> {
        self.session.ui.confirm_dialog.as_ref()
    }

    pub fn sidebar_width(&self) -> u16 {
        self.session.ui.sidebar_width
    }

    pub fn focus_mode(&self) -> crate::state::ui::FocusMode {
        self.session.ui.focus_mode
    }
}

// next song prediction
impl AppState {
    pub fn predict_next_song(&self) -> String {
        // Priority: User queue
        if let Some(qid) = self.session.playback.active_queue_id {
            // scenario a: playing song in queue
            if let Some(idx) = self.queue().iter().position(|e| e.id == qid) {
                if idx + 1 < self.queue().len() {
                    return self.resolve_track_title(self.queue()[idx + 1].track_id);
                }
                // last song, check context
            }
            // scenario b: playing song popped
            else if let Some(first) = self.queue().front() {
                return self.resolve_track_title(first.track_id);
            }
        }
        // scenario c: playing from context
        else if let Some(first) = self.queue().front() {
            return self.resolve_track_title(first.track_id);
        }

        // context (playlist/folder)
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
            return self.resolve_track_title(ctx[next]);
        }

        if self.session.repeat_mode == RepeatMode::Playlist {
            return self.resolve_track_title(ctx[0]);
        }

        "None".into()
    }

    /// resolve track title
    fn resolve_track_title(&self, id: TrackId) -> String {
        self.library
            .get_track(id)
            .map(|t| t.title.clone())
            .unwrap_or("Unknown".into())
    }
}
