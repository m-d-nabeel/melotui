use melotui_core::{AppState, UIMode, domain::TrackId};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol};

pub struct ImageState {
    pub protocol: Option<StatefulProtocol>,
    pub last_id: Option<TrackId>,
    pub width: u32,
    pub height: u32,
}

impl ImageState {
    pub fn new() -> Self {
        Self {
            protocol: None,
            last_id: None,
            width: 0,
            height: 0,
        }
    }

    pub fn reset(&mut self) {
        self.protocol = None;
        self.width = 0;
        self.height = 0;
    }
}

pub struct ImageCache {
    picker: Picker,
    pub album_art: ImageState,
    pub track_info: ImageState,
    was_obscured: bool,
}

impl ImageCache {
    pub fn new() -> Self {
        let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());

        Self {
            picker,
            album_art: ImageState::new(),
            track_info: ImageState::new(),
            was_obscured: false,
        }
    }

    pub fn update(&mut self, state: &AppState) {
        let is_obscured = state.current_notification().is_some()
            || *state.ui_mode() == UIMode::Command
            || state.ui_show_help();

        let popup_just_closed = self.was_obscured && !is_obscured;
        self.was_obscured = is_obscured;

        if state.ui_show_help() {
            self.album_art.reset();
            self.track_info.reset();
            return;
        }

        let picker = &self.picker;

        Self::update_state(
            picker,
            &mut self.album_art,
            state.current_album_art_id,
            &state.current_album_art_data,
            popup_just_closed,
        );

        Self::update_state(
            picker,
            &mut self.track_info,
            state.track_info_art_id,
            &state.track_info_art_data,
            popup_just_closed,
        );
    }

    fn update_state(
        picker: &Picker,
        local: &mut ImageState,
        core_id: Option<TrackId>,
        core_data: &Option<Vec<u8>>,
        force_redraw: bool,
    ) {
        let needs_update = local.last_id != core_id || force_redraw || local.protocol.is_none();

        if !needs_update {
            return;
        }

        local.last_id = core_id;

        if force_redraw || local.last_id != core_id {
            local.width = 0;
            local.height = 0;
            local.protocol = None;
        }

        if let Some(data) = core_data {
            if let Ok(img) = image::load_from_memory(data) {
                local.width = img.width();
                local.height = img.height();
                local.protocol = Some(picker.new_resize_protocol(img));
            }
        } else {
            local.protocol = None;
        }
    }
}
