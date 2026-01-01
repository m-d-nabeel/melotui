use melotui_core::AppState;
use ratatui::{
    Frame,
    layout::Rect,
    style::Color,
    symbols,
    widgets::{
        Block, BorderType, Borders,
        canvas::{Canvas, Line, Rectangle},
    },
};
use std::time::SystemTime;

static SENSITIVITY: f32 = 60.0;

pub fn draw_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(" Visualizer ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();

    let spectrum = &state.spectrum_smooth;
    let peaks = &state.spectrum_peaks;

    if spectrum.is_empty() {
        return;
    }

    let canvas = Canvas::default()
        .marker(symbols::Marker::Braille)
        .x_bounds([0.0, inner_area.width as f64])
        .y_bounds([0.0, inner_area.height as f64])
        .paint(|ctx| {
            let width = inner_area.width as f64;
            let height = inner_area.height as f64;
            let center_x = width / 2.0;

            // We mirror, so we only need to iterate half the width
            let num_bars = (width / 2.0).ceil() as usize;

            // Map 512 bins to `num_bars`.
            // We use a step slightly larger than 1.0 to skip some high-end noise if needed,
            // or exactly mapped to fit all data.
            let step = (spectrum.len() as f64) / (num_bars as f64);

            let bar_width = 1.0;

            for i in 0..num_bars {
                // Sample from the spectrum
                let idx = (i as f64 * step) as usize;
                if idx >= spectrum.len() {
                    break;
                }

                let val: f64 = (spectrum[idx] * SENSITIVITY).into();
                let peak: f64 = (peaks[idx] * SENSITIVITY).into();

                let bar_height = val.clamp(0.0, height);
                let peak_height = peak.clamp(0.0, height);

                // Skip invisible bars
                if bar_height < 0.2 && peak_height < 0.2 {
                    continue;
                }

                // --- COLOR: The "Cyberpunk" Blue/Cyan look ---
                // Low freq = Blue, High freq = Cyan/White
                // Modulate slightly with time for "shimmer"
                let freq_ratio = i as f64 / num_bars as f64;

                let r = (20.0 + freq_ratio * 50.0) as u8;
                let g = (50.0 + freq_ratio * 150.0) as u8;
                let b = (200.0 + (time * 2.0).sin() * 20.0) as u8;

                let color = Color::Rgb(r, g, b);

                // Peak is lighter/whiter version
                let peak_color = Color::Rgb(
                    r.saturating_add(60),
                    g.saturating_add(60),
                    b.saturating_add(60),
                );

                // --- RIGHT SIDE ---
                let right_x = center_x + (i as f64 * bar_width);

                ctx.draw(&Rectangle {
                    x: right_x,
                    y: 0.0,
                    width: bar_width,
                    height: bar_height,
                    color,
                });

                if peak_height > 0.0 {
                    ctx.draw(&Rectangle {
                        x: right_x,
                        y: peak_height,
                        width: bar_width,
                        height: 0.5,
                        color: peak_color,
                    });
                }

                // --- LEFT SIDE (Mirrored) ---
                let left_x = center_x - ((i + 1) as f64 * bar_width);

                ctx.draw(&Rectangle {
                    x: left_x,
                    y: 0.0,
                    width: bar_width,
                    height: bar_height,
                    color,
                });

                if peak_height > 0.0 {
                    ctx.draw(&Rectangle {
                        x: left_x,
                        y: peak_height,
                        width: bar_width,
                        height: 0.5,
                        color: peak_color,
                    });
                }
            }
        });

    frame.render_widget(canvas, inner_area);
}

pub fn draw_default(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(" Visualizer ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let spectrum = &state.spectrum_smooth;
    let peaks = &state.spectrum_peaks;

    if spectrum.is_empty() {
        return;
    }

    let canvas = Canvas::default()
        .marker(symbols::Marker::Braille)
        // 1. HD RESOLUTION SCALING
        // We multiply X by 2 and Y by 4 to map 1:1 with Braille dots.
        // This effectively gives us "Pixel" control.
        .x_bounds([0.0, (inner_area.width * 2) as f64])
        .y_bounds([0.0, (inner_area.height * 4) as f64])
        .paint(|ctx| {
            let hd_width = (inner_area.width * 2) as f64;
            let hd_height = (inner_area.height * 4) as f64;

            // We mirror, so we calculate for half the screen
            let center_x = hd_width / 2.0;
            let num_bars = center_x as usize;

            // Map our 512 spectrum bins to the HD bar count
            let step = (spectrum.len() as f64) / (num_bars as f64);

            for i in 0..num_bars {
                // Sample spectrum
                let idx = (i as f64 * step) as usize;
                if idx >= spectrum.len() {
                    break;
                }

                let val: f64 = (spectrum[idx] * SENSITIVITY * 5.0).into();
                let peak: f64 = (peaks[idx] * SENSITIVITY * 5.0).into();

                let bar_height = val.clamp(0.0, hd_height);
                let peak_height = peak.clamp(0.0, hd_height);

                if bar_height < 1.0 && peak_height < 1.0 {
                    continue;
                }

                // --- COLOR LOGIC (Cyberpunk / Vaporwave) ---
                let freq_ratio = i as f64 / num_bars as f64;

                // Color Logic from your reference:
                // r = Amplitude based (Bright = Loud)
                // g = Low Freqs (Bass is Greenish/Cyan)
                // b = High Freqs (Treble is Blue/Purple)
                let r = 100 + (spectrum[idx] * 800.0).clamp(0.0, 155.0) as u8;
                let g = 100 + ((1.0 - freq_ratio) * 155.0) as u8;
                let b = 150 + (freq_ratio * 105.0) as u8;

                let color = Color::Rgb(r, g, b);
                let peak_color = Color::Rgb(
                    r.saturating_add(50),
                    g.saturating_add(50),
                    b.saturating_add(50),
                );

                // --- MIRRORING: BASS AT EDGES (cddc) ---
                // i=0 is Bass. i=max is Treble.
                // Left Side: Bass starts at x=0, goes right.
                // Right Side: Bass starts at x=width, goes left.

                // 2. DRAWING LINES (Not Rectangles)
                // Drawing a Line on this HD canvas ensures it is exactly 1 dot wide.

                // LEFT BAR (Growing 0 -> Center)
                let left_x = i as f64;
                ctx.draw(&Line {
                    x1: left_x,
                    y1: 0.0,
                    x2: left_x,
                    y2: bar_height,
                    color,
                });
                // Left Peak
                if peak_height > 0.0 {
                    ctx.draw(&Line {
                        x1: left_x,
                        y1: peak_height,
                        x2: left_x,
                        y2: peak_height + 0.5, // 1 dot high
                        color: peak_color,
                    });
                }

                // RIGHT BAR (Growing Width -> Center)
                // We subtract 1.0 to stay within bounds (0-indexed)
                let right_x = hd_width - 1.0 - i as f64;
                ctx.draw(&Line {
                    x1: right_x,
                    y1: 0.0,
                    x2: right_x,
                    y2: bar_height,
                    color,
                });
                // Right Peak
                if peak_height > 0.0 {
                    ctx.draw(&Line {
                        x1: right_x,
                        y1: peak_height,
                        x2: right_x,
                        y2: peak_height + 0.5,
                        color: peak_color,
                    });
                }
            }
        });

    frame.render_widget(canvas, inner_area);
}

pub fn draw_cava(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(" Visualizer (CAVA) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // 1. Data Source (Safe access from State)
    let spectrum = &state.spectrum_smooth; // Smoothed data from Runtime
    let peaks = &state.spectrum_peaks; // Peak data from Runtime

    // If backend isn't sending data yet, render nothing
    if spectrum.is_empty() {
        return;
    }

    // 2. Time for Color Oscillation
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();

    let canvas = Canvas::default()
        .marker(symbols::Marker::Braille)
        .x_bounds([0.0, (inner_area.width * 2) as f64])
        .y_bounds([0.0, (inner_area.height * 4) as f64])
        .paint(|ctx| {
            let width = (inner_area.width * 2) as f64;
            let height = (inner_area.height * 4) as f64;
            // let width = inner_area.width as f64;
            // let height = inner_area.height as f64;

            // 3. Resampling Logic
            // Backend sends fixed bars (e.g. 200), UI might be 100 or 300 wide.
            // We iterate over pixels (x) and sample the spectrum.
            let num_ui_bars = width as usize;

            // Calculate how many backend-bins equal one UI-bar
            // e.g. 200 spectrum items / 100 ui bars = step 2.0
            let step = spectrum.len() as f64 / num_ui_bars as f64;
            let bar_width = 1.0;

            for i in 0..num_ui_bars {
                // Nearest Neighbor sampling
                let idx = (i as f64 * step) as usize;

                // Bounds check
                if idx >= spectrum.len() {
                    break;
                }

                // 4. Scaling
                // CAVA sends normalized 0.0-1.0.
                // We map 1.0 directly to 'height'.
                let val: f64 = spectrum[idx].into();
                let peak: f64 = peaks[idx].into();

                let bar_height = (val * height).clamp(0.0, height);
                let peak_height = (peak * height).clamp(0.0, height);

                // 5. Color Oscillation (From your old code)
                let color_phase = i as f64 / num_ui_bars as f64 + time * 0.1;
                let r = ((color_phase * 2.0).sin() * 0.5 + 0.5) * 255.0;
                let g = ((color_phase * 2.0 + 2.0).sin() * 0.5 + 0.5) * 255.0;
                let b = ((color_phase * 2.0 + 4.0).sin() * 0.5 + 0.5) * 255.0;

                let color = Color::Rgb(r as u8, g as u8, b as u8);

                // Peak Color (Lighter)
                let peak_color = Color::Rgb(
                    (r as u16 + 60).min(255) as u8,
                    (g as u16 + 60).min(255) as u8,
                    (b as u16 + 60).min(255) as u8,
                );

                let x = i as f64;

                // Draw Bar
                if bar_height > 0.5 {
                    ctx.draw(&Rectangle {
                        x,
                        y: 0.0,
                        width: bar_width,
                        height: bar_height,
                        color,
                    });
                }

                // Draw Peak
                if peak_height > 0.5 {
                    ctx.draw(&Rectangle {
                        x,
                        y: peak_height - 1.0, // Floating slightly below top
                        width: bar_width,
                        height: 1.0, // Fixed 1-unit height
                        color: peak_color,
                    });
                }
            }
        });

    frame.render_widget(canvas, inner_area);
}
