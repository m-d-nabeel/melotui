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
        ui::{PlaylistPickerState, PresetPickerState},
    },
    utils::{NoOpSpectrum, SpectrumSource},
};
use std::{collections::VecDeque, path::PathBuf, sync::Arc};

pub struct AppState {
    pub app_name: String,

    // shared data (immutable)
    pub library: Library,

    // user session state
    pub(crate) session: SessionState,

    // runtime buffers
    pub time_elapsed: f32,
    pub swirl_phase: f32,
    pub last_nav_time: f32,
    pub spectrum_raw: Vec<f32>,
    pub spectrum_smooth: Vec<f32>,
    pub spectrum_peaks: Vec<f32>,
    pub spectrum_source: Box<dyn SpectrumSource>,

    // cached resources
    pub current_cover: Option<Arc<image::DynamicImage>>,
    pub current_cover_path: Option<PathBuf>,
    pub track_info_cover: Option<Arc<image::DynamicImage>>,
    pub track_info_cover_id: Option<TrackId>,
    pub config: crate::config::AppConfig,
}

// lifecycle
impl AppState {
    pub fn new(library: Library, config: &crate::config::AppConfig) -> Self {
        let mut session = SessionState::default();

        // Apply config to session
        session.ui.sidebar_width = config.ui.sidebar_width;
        session.ui.scrolloff = config.ui.scrolloff;
        session.ui.lyrics_zoom = crate::config::clamp_lyrics_zoom(config.ui.lyrics_zoom, 2.0);
        session.ui.lyrics_zoom_focus =
            crate::config::clamp_lyrics_zoom(config.ui.lyrics_zoom_focus, 3.0);
        session.ui.theme_overrides = config.theme_overrides.clone();

        // Apply audio defaults from config
        session.audio.speed = config.audio.default_speed;
        session.audio.bass = config.audio.default_bass;
        session.audio.treble = config.audio.default_treble;
        session.audio.reverb = config.audio.default_reverb;
        session.audio.echo = config.audio.default_echo;
        session.audio.pan = config.audio.default_pan;
        session.audio.lowpass = config.audio.default_lowpass;
        session.audio.highpass = config.audio.default_highpass;

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
            swirl_phase: 0.0,
            last_nav_time: 0.0,
            spectrum_raw: vec![0.0; 512],
            spectrum_smooth: vec![0.0; 512],
            spectrum_peaks: vec![0.0; 512],
            spectrum_source: Box::new(NoOpSpectrum),
            current_cover: None,
            current_cover_path: None,
            track_info_cover: None,
            track_info_cover_id: None,
            config: config.clone(),
        }
    }
}

// audio system access
impl AppState {
    pub fn audio(&self) -> &crate::state::AudioState {
        &self.session.audio
    }
    pub fn seek_epoch(&self) -> u64 {
        self.session.seek_epoch
    }
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
    pub fn pan(&self) -> f32 {
        self.session.audio.pan
    }
    pub fn lowpass(&self) -> f32 {
        self.session.audio.lowpass
    }
    pub fn highpass(&self) -> f32 {
        self.session.audio.highpass
    }
    pub fn auto_pan_rate(&self) -> f32 {
        self.session.audio.auto_pan_rate
    }
    pub fn auto_pan_depth(&self) -> f32 {
        self.session.audio.auto_pan_depth
    }
    pub fn eq_bands(&self) -> &[f32; 10] {
        &self.session.audio.eq_bands
    }
    pub fn eq_band(&self, idx: usize) -> f32 {
        self.session.audio.eq_bands[idx.min(9)]
    }
    pub fn wobble(&self) -> f32 {
        self.session.audio.wobble
    }
    pub fn karaoke(&self) -> f32 {
        self.session.audio.karaoke
    }
    pub fn width(&self) -> f32 {
        self.session.audio.width
    }
    pub fn bitcrush(&self) -> f32 {
        self.session.audio.bitcrush
    }
    pub fn exciter(&self) -> f32 {
        self.session.audio.exciter
    }
    pub fn ping_pong(&self) -> f32 {
        self.session.audio.ping_pong
    }
    pub fn phaser(&self) -> f32 {
        self.session.audio.phaser
    }

    pub fn theme_overrides(&self) -> &std::collections::HashMap<String, String> {
        &self.session.ui.theme_overrides
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
        self.session.playback.context.shuffle()
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
        } else if self.session.playback.context.source_name().is_empty() {
            "Unknown"
        } else {
            self.session.playback.context.source_name()
        }
    }

    /// Read-only view of the listening context.
    pub fn playback_context(&self) -> &crate::playback::context::PlaybackContext {
        &self.session.playback.context
    }

    pub fn context_queue(&self) -> &[TrackId] {
        self.session.playback.context.tracks()
    }
}

// Navigation & view querying
impl AppState {
    /// read-only library nav state
    pub fn library_nav(&self) -> &LibraryNavState {
        &self.session.nav.library
    }

    /// Selects a sidebar node directly, as if the cursor had landed on it.
    ///
    /// Real keyboard navigation moves the selection one row at a time through
    /// `nav_action`'s handlers; this is the equivalent "land here" primitive
    /// for anything that already knows which node it wants selected —
    /// currently only the behaviour test suite, which needs to put the
    /// cursor on a specific track without walking the whole tree to find it.
    pub fn select_node(&mut self, node: SidebarNodeId) {
        self.session.nav.library.selected_id = Some(node);
    }

    /// Sets the library sort field and direction directly.
    ///
    /// The keyboard-driven path (`LibraryMsg::ToggleSort`) only cycles through
    /// fields one at a time, which is real UI behaviour worth keeping as the
    /// only *interactive* way to change sort — this exists for callers (tests
    /// today) that need a specific field without replaying the cycle.
    pub fn set_sort(
        &mut self,
        field: crate::navigation::state::SortField,
        direction: crate::navigation::state::SortDirection,
    ) {
        self.session.nav.library.sort_field = field;
        self.session.nav.library.sort_direction = direction;
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

    pub fn ensure_library_selection(&mut self) {
        if let Some(active_id) = self.session.playback.current
            && let Some(canon) = crate::state::session_store::canonicalize_track_node(
                &self.library,
                active_id,
                self.session.playback.active_node.as_ref(),
            )
        {
            self.session.playback.active_node = Some(canon.clone());
            let update_sel = match self.session.nav.library.selected_id {
                Some(SidebarNodeId::Track { id, .. }) => id != active_id,
                None => true,
                _ => false,
            };
            if update_sel {
                self.session.nav.library.selected_id = Some(canon.clone());
            }
        }

        if let Some(ref sel) = self.session.nav.library.selected_id.clone() {
            crate::state::session_store::expand_ancestors_of_node(
                &mut self.session.nav.library.expanded_ids,
                sel,
            );
        } else if let Some(first_node) = crate::library::projector::get_node_at_index(
            &self.library,
            &self.session.nav.library,
            0,
        ) {
            self.session.nav.library.selected_id = Some(first_node);
        }
    }

    pub fn get_track_info_target_id(&self) -> Option<TrackId> {
        match self.session.nav.active_panel {
            crate::navigation::state::LeftPanelMode::Library => {
                self.get_selected_track_id_in_sidebar()
            }
            crate::navigation::state::LeftPanelMode::Queue => self.get_selected_track_id_in_queue(),
        }
        .or(self.session.playback.current)
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
    pub fn visualizer_mode(&self) -> &VisualizerMode {
        &self.session.visualizer_mode
    }

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

    pub fn ui_show_lyrics(&self) -> bool {
        self.session.ui.show_lyrics
    }

    pub fn ui_show_playback(&self) -> bool {
        self.session.ui.show_playback
    }

    pub fn lyrics_data(&self) -> &crate::state::LyricsData {
        &self.session.lyrics.data
    }

    pub fn lyrics_search_state(&self) -> Option<&crate::state::lyrics::LyricsSearchState> {
        self.session.lyrics.search_state.as_ref()
    }

    pub fn active_lyrics_line_index(&self) -> Option<usize> {
        self.session.lyrics.active_line_index
    }

    pub fn ui_command_buffer(&self) -> &tui_input::Input {
        &self.session.ui.command_buffer
    }

    pub fn current_notification(&self) -> Option<&Notification> {
        self.session.notification.current.as_ref()
    }

    pub fn set_notification(&mut self, kind: NotificationKind, message: impl Into<String>) {
        if let Some(current) = &self.session.notification.current
            && kind.priority() < current.kind.priority()
        {
            return; // Do not overwrite higher priority notifications
        }
        let duration = self.config.ui.notification_duration_secs;
        self.session.notification.current =
            Some(Notification::new(kind, message).with_duration(duration));
    }

    pub fn set_notification_with_duration(
        &mut self,
        kind: NotificationKind,
        message: impl Into<String>,
        duration_secs: u64,
    ) {
        if let Some(current) = &self.session.notification.current
            && kind.priority() < current.kind.priority()
        {
            return; // Do not overwrite higher priority notifications
        }
        self.session.notification.current =
            Some(Notification::new(kind, message).with_duration(duration_secs));
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

    pub fn preset_picker(&self) -> &PresetPickerState {
        &self.session.ui.preset_picker
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

    /// What was typed before Tab started cycling through completions, and
    /// which completion it is on (counted from 1; 0 before the first Tab).
    pub fn ui_autocomplete(&self) -> (Option<&str>, usize) {
        let ui = &self.session.ui;
        (ui.autocomplete_prefix.as_deref(), ui.autocomplete_cycle)
    }

    /// The playlists' and presets' names, for completing commands.
    pub fn completion_names(&self) -> crate::autocomplete::Names {
        crate::autocomplete::Names {
            playlists: self
                .library
                .playlist_order
                .iter()
                .filter_map(|id| self.library.playlists.get(id))
                .map(|playlist| playlist.name.clone())
                .collect(),
            presets: self.session.ui.command_presets.clone(),
        }
    }

    /// Lyrics zoom in the lyrics panel.
    pub fn lyrics_zoom(&self) -> f32 {
        self.session.ui.lyrics_zoom
    }

    /// Lyrics zoom in the full lyrics view.
    pub fn lyrics_zoom_focus(&self) -> f32 {
        self.session.ui.lyrics_zoom_focus
    }

    pub fn ui_show_dsp_studio(&self) -> bool {
        self.session.ui.show_dsp_studio
    }

    pub fn dsp_selected_control(&self) -> usize {
        self.session.ui.dsp_selected_control
    }
}

// next song prediction
impl AppState {
    pub fn predict_next_song(&self) -> String {
        crate::playback::advance::describe_next(&self.session, |id| self.resolve_track_title(id))
    }

    /// resolve track title
    fn resolve_track_title(&self, id: TrackId) -> String {
        self.library
            .get_track(id)
            .map(|t| t.title.clone())
            .unwrap_or("Unknown".into())
    }
}
