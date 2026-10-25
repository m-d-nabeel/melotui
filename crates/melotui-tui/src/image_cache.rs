use crate::lyrics::LyricsRenderer;
use crate::lyrics::kitty::KittyImage;
use image::DynamicImage;
use melotui_core::{AppState, UIMode};
use ratatui_image::{
    picker::{Picker, ProtocolType},
    protocol::StatefulProtocol,
};
use std::sync::Arc;

pub struct ImageState {
    pub protocol: Option<StatefulProtocol>,
    /// The image, in kitty: sent as it is and scaled by kitty to its cells.
    pub kitty: Option<KittyImage>,
    pub source: Option<Arc<DynamicImage>>,
    /// The cells it was made for.
    pub width: u16,
    pub height: u16,
    kitty_id: Option<u32>,
    next_id: u32,
    /// kitty images no longer shown, freed once the frame is drawn.
    retired: Vec<u32>,
}

impl ImageState {
    pub fn new() -> Self {
        // Ids are shared with the lyrics', which start somewhere arbitrary;
        // these start somewhere else, below 2^24 so a placeholder's colour
        // carries the whole id.
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos())
            .rotate_left(7)
            ^ std::process::id();
        Self {
            protocol: None,
            kitty: None,
            source: None,
            width: 0,
            height: 0,
            kitty_id: None,
            next_id: (seed & 0xFF_FFFF).max(1),
            retired: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.forget_image();
        self.source = None;
    }

    /// Drops the image made from the source, to be made again.
    fn forget_image(&mut self) {
        self.protocol = None;
        self.kitty = None;
        self.retired.extend(self.kitty_id.take());
        self.width = 0;
        self.height = 0;
    }

    /// Makes the image for `area` from the source, unless it is made for it
    /// already.
    ///
    /// The source is cropped to the area's shape and left at its own
    /// resolution: the terminal, or ratatui-image for terminals that cannot
    /// scale, enlarges it. It used to be enlarged here to the area's full
    /// size in pixels first: a 512-pixel cover filling a large panel became
    /// some 7 MB of pixels, kitty's copy of them as many again, and preparing
    /// them took 45 MB at the peak.
    pub fn update_protocol(&mut self, picker: &Picker, area: ratatui::layout::Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        if self.width == area.width
            && self.height == area.height
            && (self.protocol.is_some() || self.kitty.is_some())
        {
            return;
        }
        let Some(source) = &self.source else {
            return;
        };

        let (font_w, font_h) = picker.font_size();
        let aspect = (area.width as f32 * font_w as f32) / (area.height as f32 * font_h as f32);
        let (src_w, src_h) = (source.width().max(1), source.height().max(1));
        let (crop_w, crop_h) = if src_w as f32 / src_h as f32 > aspect {
            (
                ((src_h as f32 * aspect).round() as u32).clamp(1, src_w),
                src_h,
            )
        } else {
            (
                src_w,
                ((src_w as f32 / aspect).round() as u32).clamp(1, src_h),
            )
        };
        let cropped = source
            .crop_imm((src_w - crop_w) / 2, (src_h - crop_h) / 2, crop_w, crop_h)
            .to_rgba8();

        self.forget_image();
        if picker.protocol_type() == ProtocolType::Kitty {
            let id = self.next_id;
            self.next_id = self.next_id % 0xFF_FFFF + 1;
            let is_tmux = std::env::var_os("TMUX").is_some();
            self.kitty = KittyImage::new(&cropped, id, area.width, area.height, is_tmux);
            self.kitty_id = self.kitty.as_ref().map(|_| id);
        } else {
            self.protocol = Some(picker.new_resize_protocol(DynamicImage::ImageRgba8(cropped)));
        }
        self.width = area.width;
        self.height = area.height;
    }

    /// Draws the image into `area`, made for it if need be.
    pub fn draw(
        &mut self,
        frame: &mut ratatui::Frame,
        picker: &Picker,
        area: ratatui::layout::Rect,
    ) {
        self.update_protocol(picker, area);
        if let Some(kitty) = &self.kitty {
            frame.render_widget(kitty, area);
        } else if let Some(protocol) = &mut self.protocol {
            // The crop fits the area's shape; scaled up as well as down, it
            // fills the area.
            let image = ratatui_image::StatefulImage::default().resize(
                ratatui_image::Resize::Scale(Some(image::imageops::FilterType::Triangle)),
            );
            frame.render_stateful_widget(image, area, protocol);
        }
    }

    /// Frees, in kitty, the images no longer shown.
    pub fn flush_retired(&mut self, out: &mut impl std::io::Write) -> std::io::Result<()> {
        if self.retired.is_empty() {
            return Ok(());
        }
        let is_tmux = std::env::var_os("TMUX").is_some();
        for id in self.retired.drain(..) {
            out.write_all(crate::lyrics::kitty::delete(id, is_tmux).as_bytes())?;
        }
        out.flush()
    }
}

pub struct ImageCache {
    pub picker: Picker,
    pub album_art: ImageState,
    pub track_info: ImageState,
    pub lyrics: LyricsRenderer,
    was_obscured: bool,
    /// What was drawn over the panels last frame: any notification, the UI
    /// mode, the help screen.
    overlay: Option<(bool, UIMode, bool)>,
}

impl ImageCache {
    pub fn new(lyrics_shaping: bool) -> Self {
        let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());

        let mut lyrics = LyricsRenderer::new(lyrics_shaping);
        // Sixel has no transparency, so lyrics drawn through it are blended
        // onto the terminal's background colour — if the terminal will say
        // what that is. Only asked where it is needed.
        if lyrics_shaping && picker.protocol_type() == ratatui_image::picker::ProtocolType::Sixel {
            let options = terminal_colorsaurus::QueryOptions::default();
            let background = terminal_colorsaurus::background_color(options)
                .ok()
                .map(|color| {
                    let (r, g, b) = color.scale_to_8bit();
                    [r, g, b]
                });
            lyrics.set_background(background);
        }

        Self {
            picker,
            album_art: ImageState::new(),
            track_info: ImageState::new(),
            lyrics,
            was_obscured: false,
            overlay: None,
        }
    }

    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            picker: Picker::halfblocks(),
            album_art: ImageState::new(),
            track_info: ImageState::new(),
            lyrics: LyricsRenderer::new(false),
            was_obscured: false,
            overlay: None,
        }
    }

    pub fn update(&mut self, state: &AppState) {
        let is_obscured = state.current_notification().is_some()
            || *state.ui_mode() == UIMode::Command
            || state.ui_show_help();

        // Images drawn as pixels over the cells — Sixel, iTerm2 — stay over
        // whatever is later drawn on those cells, and are drawn afresh when
        // something that covered them goes. kitty's placeholders and half
        // blocks are cells like any other, and need nothing done: making
        // them again cost a full image's work at every passing notification.
        let paints_over = matches!(
            self.picker.protocol_type(),
            ProtocolType::Sixel | ProtocolType::Iterm2
        );
        let popup_just_closed = paints_over && self.was_obscured && !is_obscured;
        self.was_obscured = is_obscured;

        let overlay = (
            state.current_notification().is_some(),
            state.ui_mode().clone(),
            state.ui_show_help(),
        );
        if let Some(previous) = self.overlay.replace(overlay.clone())
            && paints_over
            && previous != overlay
            && (previous.0 || previous.1 != UIMode::Normal || previous.2)
        {
            self.lyrics.clear();
        }

        if paints_over && state.ui_show_help() {
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

    /// Frees, in kitty, the images that left the screen this frame.
    pub fn flush_retired(&mut self, out: &mut impl std::io::Write) -> std::io::Result<()> {
        self.lyrics.flush_retired(out)?;
        self.album_art.flush_retired(out)?;
        self.track_info.flush_retired(out)
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
        local.forget_image();
        local.source = core_cover.clone(); // Arc clone = ref count increment only
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
    fn lyrics_are_drawn_afresh_after_an_overlay_only_where_they_paint_over_it() {
        use melotui_core::{AppMsg, message::ui::UIMsg, reducer};
        use ratatui::style::Color;

        for (protocol, redrawn) in [(ProtocolType::Kitty, false), (ProtocolType::Iterm2, true)] {
            let mut cache = ImageCache::empty();
            cache.picker.set_protocol_type(protocol);
            cache.lyrics = LyricsRenderer::new(true);
            cache.lyrics.wait_for_fonts();
            let mut state = AppState::new(
                Library::new(
                    vec![PathBuf::from("/")],
                    vec![],
                    std::collections::HashMap::new(),
                ),
                &melotui_core::config::AppConfig::default(),
            );
            let draw = |cache: &mut ImageCache| {
                let mut terminal =
                    ratatui::Terminal::new(ratatui::backend::TestBackend::new(40, 4)).unwrap();
                terminal
                    .draw(|f| {
                        let lines = [("तुम ही हो", Color::Rgb(137, 180, 250))];
                        assert!(
                            cache
                                .lyrics
                                .draw(f, f.area(), &cache.picker, &lines, None, 1.0)
                        );
                    })
                    .unwrap();
            };
            cache.update(&state);
            draw(&mut cache);
            assert_eq!(cache.lyrics.shown_lines(), Some(1));

            // Help opening draws over the lyrics; nothing needs redrawing.
            reducer::reduce(&mut state, AppMsg::UI(UIMsg::OpenHelp));
            cache.update(&state);
            assert_eq!(cache.lyrics.shown_lines(), Some(1), "{protocol:?}: opening");

            // Help closing: pixels painted over the cells would stay over
            // them, so those are drawn afresh; kitty's placeholder cells are
            // redrawn like any others, and nothing needs doing.
            reducer::reduce(&mut state, AppMsg::UI(UIMsg::CloseHelp));
            cache.update(&state);
            assert_eq!(
                cache.lyrics.shown_lines().is_none(),
                redrawn,
                "{protocol:?}: closing"
            );
        }
    }

    #[test]
    fn art_is_made_again_for_help_only_where_images_paint_over_it() {
        use melotui_core::{AppMsg, message::ui::UIMsg, reducer};
        for (protocol, made_again) in [
            (ProtocolType::Iterm2, true),
            (ProtocolType::Kitty, false),
            (ProtocolType::Halfblocks, false),
        ] {
            let mut cache = ImageCache::empty();
            cache.picker.set_protocol_type(protocol);
            let mut app_state = AppState::new(
                Library::new(
                    vec![PathBuf::from("/")],
                    vec![],
                    std::collections::HashMap::new(),
                ),
                &melotui_core::config::AppConfig::default(),
            );
            cache.album_art.width = 50;
            reducer::reduce(&mut app_state, AppMsg::UI(UIMsg::OpenHelp));
            cache.update(&app_state);
            assert_eq!(cache.album_art.width == 0, made_again, "{protocol:?}");
        }
    }
}
