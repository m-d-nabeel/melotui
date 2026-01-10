use anyhow::Result;
use image::GenericImageView;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};
use std::io::Cursor;

/// Defines how the image should be resized.
#[derive(Debug, Clone, Copy)]
pub enum SizeMode {
    /// Target number of terminal rows (height). Width is auto-calculated.
    Height(u32),
    /// Target number of terminal columns (width). Height is auto-calculated.
    Width(u32),
    /// Resize to fill the given dimensions, then crop the excess from the center.
    /// (Target Width in chars, Target Height in rows)
    Cover { width: u32, height: u32 },
}

/// Converts raw image bytes into a rectangular Ratatui Text widget.
/// Uses Lanczos3 resampling for high quality and supports alpha transparency.
pub fn bytes_to_ascii(data: &[u8], size_mode: SizeMode) -> Result<Text<'static>> {
    let img = image::load(Cursor::new(data), image::ImageFormat::Jpeg)
        .or_else(|_| image::load(Cursor::new(data), image::ImageFormat::Png))
        .or_else(|_| image::load(Cursor::new(data), image::ImageFormat::WebP))?;

    let (orig_w, orig_h) = img.dimensions();
    let aspect_ratio = orig_w as f64 / orig_h as f64;

    // Terminal cells are approx 1:2 (width:height)
    let (target_w, target_h, do_crop) = match size_mode {
        SizeMode::Height(h) => {
            let internal_h = h * 2;
            let internal_w = (internal_h as f64 * aspect_ratio).round() as u32;
            (internal_w, internal_h, false)
        }
        SizeMode::Width(w) => {
            let internal_w = w;
            let internal_h = (internal_w as f64 / aspect_ratio).round() as u32;
            (internal_w, internal_h, false)
        }
        SizeMode::Cover { width, height } => {
            // Width is 1:1, Height is 1:2 (half-blocks)
            (width, height * 2, true)
        }
    };

    let target_w = target_w.max(1);
    let target_h = target_h.max(1);

    let rgba = if do_crop {
        let mut filled =
            img.resize_to_fill(target_w, target_h, image::imageops::FilterType::Lanczos3);
        let (fw, fh) = filled.dimensions();
        let x = (fw.saturating_sub(target_w)) / 2;
        let y = (fh.saturating_sub(target_h)) / 2;
        filled.crop(x, y, target_w, target_h).to_rgba8()
    } else {
        img.resize_exact(target_w, target_h, image::imageops::FilterType::Lanczos3)
            .to_rgba8()
    };

    let (w, h) = rgba.dimensions();

    let get_color = |x: u32, y: u32| -> Option<Color> {
        if x >= w || y >= h {
            return None;
        }
        let p = rgba.get_pixel(x, y);
        if p[3] < 50 {
            None
        } else {
            Some(Color::Rgb(p[0], p[1], p[2]))
        }
    };

    let mut lines = Vec::new();
    for y in (0..h).step_by(2) {
        let mut spans = Vec::new();
        for x in 0..w {
            let top_color = get_color(x, y);
            let bottom_color = get_color(x, y + 1);

            // High-Res Half-Block Rendering Logic
            let span = match (top_color, bottom_color) {
                // Both pixels have color: Top is BG, Bottom is FG
                (Some(top), Some(bottom)) => Span::styled("▄", Style::default().bg(top).fg(bottom)),
                // Only Top has color: Use Upper Half Block
                (Some(top), None) => Span::styled("▀", Style::default().fg(top)),
                // Only Bottom has color: Use Lower Half Block
                (None, Some(bottom)) => Span::styled("▄", Style::default().fg(bottom)),
                // Both transparent: Empty space
                (None, None) => Span::raw(" "),
            };
            spans.push(span);
        }
        lines.push(Line::from(spans));
    }

    Ok(Text::from(lines))
}
