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

static SENSITIVITY: f32 = 60.0;

pub fn draw_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(" Visualizer ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let time = state.time_elapsed;

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

            // Mirror: iterate half width
            let num_bars = (width / 2.0).ceil() as usize;

            // Map spectrum bins to bar count
            let step = (spectrum.len() as f64) / (num_bars as f64);

            let bar_width = 1.0;

            for i in 0..num_bars {
                // Sample spectrum
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

                // Cyberpunk color with shimmer
                let freq_ratio = i as f64 / num_bars as f64;

                let r = (20.0 + freq_ratio * 50.0) as u8;
                let g = (50.0 + freq_ratio * 150.0) as u8;
                let b = (200.0 + (time * 2.0).sin() * 20.0) as u8;

                let color = Color::Rgb(r, g, b);

                // Peak is lighter
                let peak_color = Color::Rgb(
                    r.saturating_add(60),
                    g.saturating_add(60),
                    b.saturating_add(60),
                );

                // Right side
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

                // Left side (mirrored)
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
        // HD resolution scaling (2x width, 4x height)
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

                // Color logic (cyberpunk/vaporwave)
                let freq_ratio = i as f64 / num_bars as f64;

                // Color mapping for frequencies
                let r = 100 + (spectrum[idx] * 800.0).clamp(0.0, 155.0) as u8;
                let g = 100 + ((1.0 - freq_ratio) * 155.0) as u8;
                let b = 150 + (freq_ratio * 105.0) as u8;

                let color = Color::Rgb(r, g, b);
                let peak_color = Color::Rgb(
                    r.saturating_add(50),
                    g.saturating_add(50),
                    b.saturating_add(50),
                );

                // Mirror: bass at edges (cddc)

                // Draw lines (not rectangles) for precision

                // Left bar (grows to center)
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
        .title(" Visualizer ")
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

    // Time for color oscillation
    let time = state.time_elapsed;

    let canvas = Canvas::default()
        .marker(symbols::Marker::Braille)
        .x_bounds([0.0, (inner_area.width * 2) as f64])
        .y_bounds([0.0, (inner_area.height * 4) as f64])
        .paint(|ctx| {
            let width = (inner_area.width * 2) as f64;
            let height = (inner_area.height * 4) as f64;

            // Resampling logic for variable width
            let num_ui_bars = width as usize;

            // Calculate bins per UI bar (e.g. 200/100 = step 2.0)
            let step = spectrum.len() as f64 / num_ui_bars as f64;
            let bar_width = 1.0;

            for i in 0..num_ui_bars {
                // Nearest neighbor sampling
                let idx = (i as f64 * step) as usize;

                // Bounds check
                if idx >= spectrum.len() {
                    break;
                }

                // Scaling (CAVA sends 0.0-1.0)
                let val: f64 = spectrum[idx].into();
                let peak: f64 = peaks[idx].into();

                let bar_height = (val * height).clamp(0.0, height);
                let peak_height = (peak * height).clamp(0.0, height);

                // Color oscillation
                let color_phase = (i as f64 / num_ui_bars as f64) + (time as f64 * 0.1);
                let r = ((color_phase * 2.0).sin() * 0.5 + 0.5) * 255.0;
                let g = ((color_phase * 2.0 + 2.0).sin() * 0.5 + 0.5) * 255.0;
                let b = ((color_phase * 2.0 + 4.0).sin() * 0.5 + 0.5) * 255.0;

                let color = Color::Rgb(r as u8, g as u8, b as u8);

                // Peak color (lighter)
                let peak_color = Color::Rgb(
                    (r as u16 + 60).min(255) as u8,
                    (g as u16 + 60).min(255) as u8,
                    (b as u16 + 60).min(255) as u8,
                );

                let x = i as f64;

                // Draw bar
                if bar_height > 0.5 {
                    ctx.draw(&Rectangle {
                        x,
                        y: 0.0,
                        width: bar_width,
                        height: bar_height,
                        color,
                    });
                }

                // Draw peak
                if peak_height > 0.5 {
                    ctx.draw(&Rectangle {
                        x,
                        y: peak_height - 1.0, // Float slightly below top
                        width: bar_width,
                        height: 1.0, // Fixed height
                        color: peak_color,
                    });
                }
            }
        });

    frame.render_widget(canvas, inner_area);
}

pub fn draw_particle(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(" Visualizer ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let spectrum = &state.spectrum_smooth;
    let peaks = &state.spectrum_peaks;
    let time = state.time_elapsed as f64;

    if spectrum.is_empty() {
        return;
    }

    // Color palette definitions
    let color_wave = Color::LightBlue;
    let color_magenta = Color::Blue;
    let color_accent = Color::DarkGray;
    let color_peak = Color::Gray;

    let canvas = Canvas::default()
        .marker(symbols::Marker::Braille)
        // Enable HD resolution (2x width, 4x height)
        .x_bounds([0.0, (inner_area.width * 2) as f64])
        .y_bounds([0.0, (inner_area.height * 4) as f64])
        .paint(move |ctx| {
            // Update to HD dimensions
            let width = (inner_area.width * 2) as f64;
            let height = (inner_area.height * 4) as f64;

            let center_y = height / 2.0;
            let max_height = height / 2.5;

            // Center line and background wave
            let wave_speed = 2.0;
            // Iterate over HD width now
            for x_int in 0..(width as usize) {
                let x = x_int as f64;
                // Slower frequency (X is 2x larger)
                let wave = (x * 0.025 + time * wave_speed).sin() * 4.0;

                ctx.draw(&Line {
                    x1: x,
                    y1: center_y + wave - 0.5,
                    x2: x,
                    y2: center_y + wave + 0.5,
                    color: color_wave,
                });
            }

            // 3. Mirror Effect Logic
            let center_x = width / 2.0;
            let num_bands = center_x as usize;
            if num_bands == 0 {
                return;
            }

            let step = spectrum.len() as f64 / num_bands as f64;

            for i in 0..num_bands {
                let idx = (i as f64 * step) as usize;
                if idx >= spectrum.len() {
                    break;
                }

                // Increase sensitivity (more vertical pixels)
                let val: f64 = (spectrum[idx] * SENSITIVITY * 2.0).into();
                let peak_val: f64 = (peaks[idx] * SENSITIVITY * 2.0).into();

                let bar_height = val.clamp(0.0, max_height);
                let peak_height = peak_val.clamp(0.0, max_height);

                if bar_height < 1.0 && peak_height < 1.0 {
                    continue;
                }

                let left_x = center_x - 1.0 - i as f64;
                let right_x = center_x + i as f64;

                // Wave animation
                let wave_freq = 0.8;
                // Adjust x-factor (i * 0.05) for higher resolution (0..200 vs 0..100)
                let upper_wave = (time * wave_freq + i as f64 * 0.05).sin() * 2.0;
                let lower_wave =
                    (time * wave_freq + i as f64 * 0.05 + std::f64::consts::PI).sin() * 2.0;

                let upper_y = center_y + bar_height + upper_wave;
                let lower_y = center_y - bar_height + lower_wave;

                // Draw bars (dull) using lines
                for offset in [-1.0, 0.0, 1.0] {
                    // Right
                    ctx.draw(&Line {
                        x1: right_x + offset,
                        y1: center_y,
                        x2: right_x + offset,
                        y2: upper_y,
                        color: color_magenta,
                    });
                    ctx.draw(&Line {
                        x1: right_x + offset,
                        y1: center_y,
                        x2: right_x + offset,
                        y2: lower_y,
                        color: color_magenta,
                    });
                    // Left
                    ctx.draw(&Line {
                        x1: left_x + offset,
                        y1: center_y,
                        x2: left_x + offset,
                        y2: upper_y,
                        color: color_magenta,
                    });
                    ctx.draw(&Line {
                        x1: left_x + offset,
                        y1: center_y,
                        x2: left_x + offset,
                        y2: lower_y,
                        color: color_magenta,
                    });
                }

                // Draw peaks (bright)
                if peak_height > bar_height * 1.05 {
                    let peak_upper_y = center_y + peak_height;
                    let peak_lower_y = center_y - peak_height;

                    // Size 1.0 = 1 dot
                    ctx.draw(&Rectangle {
                        x: right_x,
                        y: peak_upper_y,
                        width: 1.0,
                        height: 1.0,
                        color: color_peak,
                    });
                    ctx.draw(&Rectangle {
                        x: right_x,
                        y: peak_lower_y,
                        width: 1.0,
                        height: 1.0,
                        color: color_peak,
                    });
                    ctx.draw(&Rectangle {
                        x: left_x,
                        y: peak_upper_y,
                        width: 1.0,
                        height: 1.0,
                        color: color_peak,
                    });
                    ctx.draw(&Rectangle {
                        x: left_x,
                        y: peak_lower_y,
                        width: 1.0,
                        height: 1.0,
                        color: color_peak,
                    });
                }

                // --- DRAW PARTICLES (Vibrant Accent) ---
                if bar_height > max_height * 0.6 {
                    let particle_freq = 3.0;
                    let particle_y =
                        upper_y + (time * particle_freq + i as f64 * 0.2).sin().abs() * 8.0;

                    ctx.draw(&Rectangle {
                        x: right_x,
                        y: particle_y,
                        width: 1.0,
                        height: 1.0,
                        color: color_accent,
                    });
                    ctx.draw(&Rectangle {
                        x: left_x,
                        y: particle_y,
                        width: 1.0,
                        height: 1.0,
                        color: color_accent,
                    });
                }
            }
        });

    frame.render_widget(canvas, inner_area);
}
