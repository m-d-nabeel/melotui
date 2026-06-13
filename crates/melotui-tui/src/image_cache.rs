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

    /// Resize, crop, and create the rendering protocol for the current terminal area.
    /// This is the ONLY place where a protocol should be created — never in update_state.
    pub fn update_protocol(&mut self, picker: &Picker, area: ratatui::layout::Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let (font_w, font_h) = picker.font_size();
        let target_w = area.width as u32 * font_w as u32;
        let target_h = area.height as u32 * font_h as u32;

        // Skip if already rendered at this size
        if self.width == target_w && self.height == target_h && self.protocol.is_some() {
            return;
        }

        if let Some(source) = &self.source {
            let src_w = source.width();
            let src_h = source.height();

            // cover logic: scale to fill, then center-crop
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

            let off_x = (new_w.saturating_sub(target_w)) / 2;
            let off_y = (new_h.saturating_sub(target_h)) / 2;

            let cropped =
                image::imageops::crop(&mut resized, off_x, off_y, target_w, target_h).to_image();

            // Drop the intermediate resize buffer before creating the protocol
            drop(resized);

            self.protocol =
                Some(picker.new_resize_protocol(image::DynamicImage::ImageRgba8(cropped)));

            self.width = target_w;
            self.height = target_h;

            // Do not set self.source = None here!
            // It's just an Arc pointer, so dropping it doesn't free the 1.4MB image
            // (AppState still holds it). If we set it to None, update_state() will
            // think the source changed, delete the protocol, and force a re-resize
            // on every single frame, causing severe UI stutter!
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

    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            picker: Picker::halfblocks(),
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

        Self::update_state(&mut self.album_art, &state.current_cover, popup_just_closed);

        Self::update_state(
            &mut self.track_info,
            &state.track_info_cover,
            popup_just_closed,
        );
    }

    /// Update the image source reference only.
    /// Does NOT create a protocol — that's done lazily by update_protocol()
    /// when the widget actually renders and knows its area.
    ///
    /// The old code created a protocol here with img.as_ref().clone() which:
    /// 1. Cloned the entire DynamicImage (~1.4 MB) unnecessarily
    /// 2. Created a protocol at the wrong size (source dims, not terminal dims)
    /// 3. That protocol was thrown away on the next frame when update_protocol()
    ///    re-created it at the correct size — double allocation.
    fn update_state(
        local: &mut ImageState,
        core_cover: &Option<Arc<DynamicImage>>,
        force_redraw: bool,
    ) {
        // Fast Arc pointer check — avoids any work if the source hasn't changed
        let same_source = match (&local.source, core_cover) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };

        if same_source && !force_redraw {
            return;
        }

        // Source changed — update reference and invalidate the protocol.
        // The protocol will be created by update_protocol() on the next render.
        local.source = core_cover.clone(); // Arc clone = ref count increment only
        local.width = 0;
        local.height = 0;
        local.protocol = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use melotui_core::library::Library;
    use std::path::PathBuf;

    #[test]
    fn test_image_state_reset() {
        let mut state = ImageState::new();
        state.width = 100;
        state.height = 100;
        state.source = Some(Arc::new(DynamicImage::new_rgb8(10, 10)));

        state.reset();

        assert_eq!(state.width, 0);
        assert_eq!(state.height, 0);
        assert!(state.source.is_none());
        assert!(state.protocol.is_none());
    }

    #[test]
    fn test_image_cache_obscured_reset() {
        let mut cache = ImageCache::empty();
        let mut app_state = AppState::new(
            Library::new(
                vec![PathBuf::from("/")],
                vec![],
                std::collections::HashMap::new(),
            ),
            &melotui_core::config::AppConfig::default(),
        );

        // Setup some fake state
        cache.album_art.width = 50;

        use melotui_core::{AppMsg, message::ui::UIMsg, reducer};

        // When help is shown, it should reset the cache
        reducer::reduce(&mut app_state, AppMsg::UI(UIMsg::OpenHelp));
        cache.update(&app_state);

        assert_eq!(cache.album_art.width, 0);
        assert!(cache.album_art.source.is_none());
    }
}
