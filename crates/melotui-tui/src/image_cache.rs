use image::DynamicImage;
use melotui_core::{AppState, UIMode};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol};
use std::sync::Arc;

pub struct ImageState {
    pub protocol: Option<StatefulProtocol>,
    pub source: Option<Arc<DynamicImage>>,
    pub width: u32,
    pub height: u32,
}

impl ImageState {
    pub fn new() -> Self {
        Self {
            protocol: None,
            source: None,
            width: 0,
            height: 0,
        }
    }

    pub fn reset(&mut self) {
        self.protocol = None;
        self.source = None;
        self.width = 0;
        self.height = 0;
    }

    /// Resize and crop the source image to cover the target area, determining protocol on demand.
    pub fn update_protocol(&mut self, picker: &Picker, area: ratatui::layout::Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let (font_w, font_h) = picker.font_size();
        let target_w = area.width as u32 * font_w as u32;
        let target_h = area.height as u32 * font_h as u32;

        // Skip if dimensions match and protocol exists
        if self.width == target_w && self.height == target_h && self.protocol.is_some() {
            return;
        }

        if let Some(source) = &self.source {
            let src_w = source.width();
            let src_h = source.height();

            // Cover Logic: Scale to fill
            let scale_w = target_w as f32 / src_w as f32;
            let scale_h = target_h as f32 / src_h as f32;
            let scale = scale_w.max(scale_h);

            let new_w = (src_w as f32 * scale).ceil() as u32;
            let new_h = (src_h as f32 * scale).ceil() as u32;

            let mut resized = image::imageops::resize(
                source.as_ref(),
                new_w,
                new_h,
                image::imageops::FilterType::Triangle,
            );

            // Center Crop Logic
            let off_x = (new_w.saturating_sub(target_w)) / 2;
            let off_y = (new_h.saturating_sub(target_h)) / 2;

            let cropped =
                image::imageops::crop(&mut resized, off_x, off_y, target_w, target_h).to_image();

            // Cache the new resize protocol
            self.protocol =
                Some(picker.new_resize_protocol(image::DynamicImage::ImageRgba8(cropped)));

            self.width = target_w;
            self.height = target_h;
        }
    }
}

pub struct ImageCache {
    pub picker: Picker,
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
            &state.current_cover,
            popup_just_closed,
        );

        Self::update_state(
            picker,
            &mut self.track_info,
            &state.track_info_cover,
            popup_just_closed,
        );
    }

    fn update_state(
        picker: &Picker,
        local: &mut ImageState,
        core_cover: &Option<Arc<DynamicImage>>,
        force_redraw: bool,
    ) {
        // Simple pointer check for equality is fast since we use Arc
        let same_source = match (&local.source, core_cover) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };

        if same_source && !force_redraw && local.protocol.is_some() {
            return;
        }

        if !same_source {
            local.source = core_cover.clone();
            local.width = 0;
            local.height = 0;
            local.protocol = None;
        }

        if let Some(img) = core_cover {
            local.width = img.width();
            local.height = img.height();
            local.protocol = Some(picker.new_resize_protocol(img.as_ref().clone()));
        } else {
            local.protocol = None;
        }
    }
}
