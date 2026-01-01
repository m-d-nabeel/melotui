use anyhow::Result;
use image::{GenericImageView, Rgba};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};
use std::io::Cursor;

/// Converts raw image bytes into a rectangular Ratatui Text widget.
///
/// `height` is the target number of terminal rows.
/// The width is calculated automatically to preserve aspect ratio.
pub fn bytes_to_ascii(data: &[u8], height: u32) -> Result<Text<'static>> {
    // 1. Load image
    let img = image::load(Cursor::new(data), image::ImageFormat::Jpeg)
        .or_else(|_| image::load(Cursor::new(data), image::ImageFormat::Png))
        .or_else(|_| image::load(Cursor::new(data), image::ImageFormat::WebP))?;

    let (orig_w, orig_h) = img.dimensions();
    let aspect_ratio = orig_w as f64 / orig_h as f64;

    // 2. Calculate Dimensions
    // Since we use half-blocks (2 pixels per char vertically), our internal image
    // height needs to be 2x the terminal rows.
    let resize_h = height * 2;

    // Terminal characters are roughly 1:2 (width:height).
    // To make an image look square, we need 2 horizontal chars for every 1 vertical char visual weight.
    // However, since we are doubling the vertical resolution with half-blocks,
    // the math simplifies: we just resize width based on aspect ratio relative to our new 2x height.
    let resize_w = (resize_h as f64 * aspect_ratio) as u32;

    // 3. Resize with High Quality Filter
    let resized = img.resize_exact(resize_w, resize_h, image::imageops::FilterType::Triangle);
    let rgba = resized.to_rgba8();
    let (w, h) = rgba.dimensions();

    let to_color = |p: &Rgba<u8>| Color::Rgb(p[0], p[1], p[2]);

    let mut lines = Vec::new();

    // 4. Iterate 2 pixels at a time (y, y+1)
    for y in (0..h).step_by(2) {
        let mut spans = Vec::new();
        for x in 0..w {
            let p1 = rgba.get_pixel(x, y);

            // Check if there is a pixel below it (handle odd heights)
            let y2 = y + 1;
            let (p2, has_bottom) = if y2 < h {
                (rgba.get_pixel(x, y2), true)
            } else {
                (p1, false)
            };

            // Half-Block Rendering Logic
            let span = if has_bottom {
                // Top pixel is BG, Bottom pixel is FG (using '▄')
                Span::styled("▄", Style::default().bg(to_color(p1)).fg(to_color(p2)))
            } else {
                // Only top pixel exists (Upper half block)
                Span::styled("▀", Style::default().fg(to_color(p1)))
            };
            spans.push(span);
        }
        lines.push(Line::from(spans));
    }

    Ok(Text::from(lines))
}
