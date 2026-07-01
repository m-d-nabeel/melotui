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

            // mirror half width: center bass, highs at edges
            let num_bars = (width / 2.0).ceil() as usize;

            // map bins to bars
            let step = (spectrum.len() as f64) / (num_bars as f64);

            let bar_width = 1.0;

            for i in 0..num_bars {
                // sample spectrum
                let idx = (i as f64 * step) as usize;
                if idx >= spectrum.len() {
                    break;
                }

                let val: f64 = (spectrum[idx] * SENSITIVITY).into();
                let peak: f64 = (peaks[idx] * SENSITIVITY).into();

                let bar_height = val.clamp(0.0, height);
                let peak_height = peak.clamp(0.0, height);

                // skip invisible bars
                if bar_height < 0.2 && peak_height < 0.2 {
                    continue;
                }

                // cyberpunk shimmer
                let freq_ratio = i as f64 / num_bars as f64;

                let r = (20.0 + freq_ratio * 50.0) as u8;
                let g = (50.0 + freq_ratio * 150.0) as u8;
                let b = (200.0 + (time * 2.0).sin() * 20.0) as u8;

                let color = Color::Rgb(r, g, b);

                // peak color
                let peak_color = Color::Rgb(
                    r.saturating_add(60),
                    g.saturating_add(60),
                    b.saturating_add(60),
                );

                // right side
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

                // left side (mirrored)
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
        // HD resolution scaling (2x width, 4x height for braille dots)
        .x_bounds([0.0, (inner_area.width * 2) as f64])
        .y_bounds([0.0, (inner_area.height * 4) as f64])
        .paint(|ctx| {
            let hd_width = (inner_area.width * 2) as f64;
            let hd_height = (inner_area.height * 4) as f64;

            // mirror half screen
            let center_x = hd_width / 2.0;
            let num_bars = center_x as usize;

            // map bins to HD bars
            let step = (spectrum.len() as f64) / (num_bars as f64);

            for i in 0..num_bars {
                // sample spectrum
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

                // color logic
                let freq_ratio = i as f64 / num_bars as f64;

                // freq mapping
                let r = 100 + (spectrum[idx] * 800.0).clamp(0.0, 155.0) as u8;
                let g = 100 + ((1.0 - freq_ratio) * 155.0) as u8;
                let b = 150 + (freq_ratio * 105.0) as u8;

                let color = Color::Rgb(r, g, b);
                let peak_color = Color::Rgb(
                    r.saturating_add(50),
                    g.saturating_add(50),
                    b.saturating_add(50),
                );

                // mirror: bass at edges

                // draw lines

                // left bar
                let left_x = i as f64;
                ctx.draw(&Line {
                    x1: left_x,
                    y1: 0.0,
                    x2: left_x,
                    y2: bar_height,
                    color,
                });
                // left peak
                if peak_height > 0.0 {
                    ctx.draw(&Line {
                        x1: left_x,
                        y1: peak_height,
                        x2: left_x,
                        y2: peak_height + 0.5, // 1 dot high
                        color: peak_color,
                    });
                }

                // right bar
                // subtract 1.0
                let right_x = hd_width - 1.0 - i as f64;
                ctx.draw(&Line {
                    x1: right_x,
                    y1: 0.0,
                    x2: right_x,
                    y2: bar_height,
                    color,
                });
                // right peak
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

    // data source
    let spectrum = &state.spectrum_smooth; // smoothed data
    let peaks = &state.spectrum_peaks; // peak data

    // skip if no data
    if spectrum.is_empty() {
        return;
    }

    // color oscillation
    let time = state.time_elapsed;

    let canvas = Canvas::default()
        .marker(symbols::Marker::Braille)
        .x_bounds([0.0, (inner_area.width * 2) as f64])
        .y_bounds([0.0, (inner_area.height * 4) as f64])
        .paint(|ctx| {
            let width = (inner_area.width * 2) as f64;
            let height = (inner_area.height * 4) as f64;

            // resample for width
            let num_ui_bars = width as usize;

            // calc bins per bar
            let step = spectrum.len() as f64 / num_ui_bars as f64;
            let bar_width = 1.0;

            for i in 0..num_ui_bars {
                // nearest neighbor
                let idx = (i as f64 * step) as usize;

                // bounds check
                if idx >= spectrum.len() {
                    break;
                }

                // scaling (0.0-1.0)
                let val: f64 = spectrum[idx].into();
                let peak: f64 = peaks[idx].into();

                let bar_height = (val * height).clamp(0.0, height);
                let peak_height = (peak * height).clamp(0.0, height);

                // color oscillation: phase shift based on position
                let color_phase = (i as f64 / num_ui_bars as f64) + (time as f64 * 0.1);
                let r = ((color_phase * 2.0).sin() * 0.5 + 0.5) * 255.0;
                let g = ((color_phase * 2.0 + 2.0).sin() * 0.5 + 0.5) * 255.0;
                let b = ((color_phase * 2.0 + 4.0).sin() * 0.5 + 0.5) * 255.0;

                let color = Color::Rgb(r as u8, g as u8, b as u8);

                // peak color
                let peak_color = Color::Rgb(
                    (r as u16 + 60).min(255) as u8,
                    (g as u16 + 60).min(255) as u8,
                    (b as u16 + 60).min(255) as u8,
                );

                let x = i as f64;

                // draw bar
                if bar_height > 0.5 {
                    ctx.draw(&Rectangle {
                        x,
                        y: 0.0,
                        width: bar_width,
                        height: bar_height,
                        color,
                    });
                }

                // draw peak
                if peak_height > 0.5 {
                    ctx.draw(&Rectangle {
                        x,
                        y: peak_height - 1.0, // float below top
                        width: bar_width,
                        height: 1.0, // fixed height
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

    // colors
    let color_wave = Color::LightBlue;
    let color_magenta = Color::Blue;
    let color_accent = Color::DarkGray;
    let color_peak = Color::Gray;

    let canvas = Canvas::default()
        .marker(symbols::Marker::Braille)
        // HD resolution
        .x_bounds([0.0, (inner_area.width * 2) as f64])
        .y_bounds([0.0, (inner_area.height * 4) as f64])
        .paint(move |ctx| {
            // HD dimensions
            let width = (inner_area.width * 2) as f64;
            let height = (inner_area.height * 4) as f64;

            let center_y = height / 2.0;
            let max_height = height / 2.5;

            // center line / wave
            let wave_speed = 2.0;
            // iterate HD width
            for x_int in 0..(width as usize) {
                let x = x_int as f64;
                // slower freq
                let wave = (x * 0.025 + time * wave_speed).sin() * 4.0;

                ctx.draw(&Line {
                    x1: x,
                    y1: center_y + wave - 0.5,
                    x2: x,
                    y2: center_y + wave + 0.5,
                    color: color_wave,
                });
            }

            // mirror logic
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

                // high sensitivity
                let val: f64 = (spectrum[idx] * SENSITIVITY * 2.0).into();
                let peak_val: f64 = (peaks[idx] * SENSITIVITY * 2.0).into();

                let bar_height = val.clamp(0.0, max_height);
                let peak_height = peak_val.clamp(0.0, max_height);

                if bar_height < 1.0 && peak_height < 1.0 {
                    continue;
                }

                let left_x = center_x - 1.0 - i as f64;
                let right_x = center_x + i as f64;

                // wave animation
                let wave_freq = 0.8;
                // adjust x-factor
                let upper_wave = (time * wave_freq + i as f64 * 0.05).sin() * 2.0;
                let lower_wave =
                    (time * wave_freq + i as f64 * 0.05 + std::f64::consts::PI).sin() * 2.0;

                let upper_y = center_y + bar_height + upper_wave;
                let lower_y = center_y - bar_height + lower_wave;

                // draw bars
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

                // draw peaks
                if peak_height > bar_height * 1.05 {
                    let peak_upper_y = center_y + peak_height;
                    let peak_lower_y = center_y - peak_height;

                    // dot size
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

                // draw particles
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

pub fn draw_radial(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(" Visualizer: Radial (Orbit) ")
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

    let canvas = Canvas::default()
        .marker(symbols::Marker::Braille)
        .x_bounds([0.0, (inner_area.width * 2) as f64])
        .y_bounds([0.0, (inner_area.height * 4) as f64])
        .paint(move |ctx| {
            let width = (inner_area.width * 2) as f64;
            let height = (inner_area.height * 4) as f64;
            let cx = width / 2.0;
            let cy = height / 2.0;

            let base_r = (width.min(height) * 0.22).max(5.0);
            let bass = spectrum.first().copied().unwrap_or(0.0) as f64;
            let pulse_r = base_r + bass * SENSITIVITY as f64 * 0.3;

            // Draw central pulsating core circle
            let core_steps = 60;
            for s in 0..core_steps {
                let a1 = s as f64 * 2.0 * std::f64::consts::PI / core_steps as f64;
                let a2 = (s + 1) as f64 * 2.0 * std::f64::consts::PI / core_steps as f64;
                ctx.draw(&Line {
                    x1: cx + pulse_r * 0.6 * a1.cos(),
                    y1: cy + pulse_r * 0.6 * a1.sin(),
                    x2: cx + pulse_r * 0.6 * a2.cos(),
                    y2: cy + pulse_r * 0.6 * a2.sin(),
                    color: Color::DarkGray,
                });
            }

            let num_rays = 90.min(spectrum.len());
            if num_rays == 0 {
                return;
            }

            let step = spectrum.len() as f64 / num_rays as f64;
            for i in 0..num_rays {
                let idx = (i as f64 * step) as usize;
                if idx >= spectrum.len() {
                    break;
                }

                // Smooth slow rotation
                let theta =
                    (i as f64 / num_rays as f64) * 2.0 * std::f64::consts::PI + (time * 0.25);
                let val = (spectrum[idx] * SENSITIVITY * 1.6) as f64;
                let peak = (peaks[idx] * SENSITIVITY * 1.6) as f64;

                let x1 = cx + pulse_r * theta.cos();
                let y1 = cy + pulse_r * theta.sin();
                let x2 = cx + (pulse_r + val) * theta.cos();
                let y2 = cy + (pulse_r + val) * theta.sin();

                // Dynamic neon gradient
                let phase = (i as f64 / num_rays as f64) + time * 0.1;
                let r = ((phase * 2.0 * std::f64::consts::PI).sin() * 100.0 + 155.0)
                    .clamp(0.0, 255.0) as u8;
                let g = ((phase * 2.0 * std::f64::consts::PI + 2.094).sin() * 100.0 + 155.0)
                    .clamp(0.0, 255.0) as u8;
                let b = ((phase * 2.0 * std::f64::consts::PI + 4.188).sin() * 100.0 + 155.0)
                    .clamp(0.0, 255.0) as u8;

                ctx.draw(&Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    color: Color::Rgb(r, g, b),
                });

                if peak > val + 1.0 {
                    let px = cx + (pulse_r + peak + 1.5) * theta.cos();
                    let py = cy + (pulse_r + peak + 1.5) * theta.sin();
                    ctx.draw(&Rectangle {
                        x: px,
                        y: py,
                        width: 1.0,
                        height: 1.0,
                        color: Color::White,
                    });
                }

                // Orbital dust particles
                if val > 5.0 && i % 3 == 0 {
                    let p_dist = pulse_r + val + 8.0 + (time * 2.0 + i as f64).sin() * 4.0;
                    ctx.draw(&Rectangle {
                        x: cx + p_dist * theta.cos(),
                        y: cy + p_dist * theta.sin(),
                        width: 1.0,
                        height: 1.0,
                        color: Color::Rgb(r, g, b),
                    });
                }
            }
        });

    frame.render_widget(canvas, inner_area);
}

pub fn draw_nova(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(" Visualizer: Nova (Supernova Spectrum) ")
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

    let canvas = Canvas::default()
        .marker(symbols::Marker::Braille)
        .x_bounds([0.0, (inner_area.width * 2) as f64])
        .y_bounds([0.0, (inner_area.height * 4) as f64])
        .paint(move |ctx| {
            let width = (inner_area.width * 2) as f64;
            let height = (inner_area.height * 4) as f64;
            let cx = width / 2.0;
            let cy = height / 2.0;

            let base_r = (width.min(height) * 0.22).max(6.0);
            let bass = spectrum.first().copied().unwrap_or(0.0) as f64;
            let pulse_r = base_r + bass * SENSITIVITY as f64 * 0.35;

            // Glowing inner corona circle
            let core_steps = 72;
            for s in 0..core_steps {
                let a1 = s as f64 * 2.0 * std::f64::consts::PI / core_steps as f64;
                let a2 = (s + 1) as f64 * 2.0 * std::f64::consts::PI / core_steps as f64;
                ctx.draw(&Line {
                    x1: cx + pulse_r * a1.cos(),
                    y1: cy + pulse_r * a1.sin(),
                    x2: cx + pulse_r * a2.cos(),
                    y2: cy + pulse_r * a2.sin(),
                    color: Color::Rgb(255, 80, 150),
                });
            }

            // Expanding shockwave ring on high bass energy
            if bass * SENSITIVITY as f64 > 3.5 {
                let shock_r = pulse_r + 10.0 + ((time * 25.0) % 18.0);
                let alpha_ratio = (1.0 - ((shock_r - pulse_r) / 28.0)).clamp(0.1, 1.0);
                for s in (0..core_steps).step_by(2) {
                    let a1 = s as f64 * 2.0 * std::f64::consts::PI / core_steps as f64;
                    let a2 = (s + 1) as f64 * 2.0 * std::f64::consts::PI / core_steps as f64;
                    ctx.draw(&Line {
                        x1: cx + shock_r * a1.cos(),
                        y1: cy + shock_r * a1.sin(),
                        x2: cx + shock_r * a2.cos(),
                        y2: cy + shock_r * a2.sin(),
                        color: Color::Rgb(
                            (80.0 * alpha_ratio) as u8,
                            (240.0 * alpha_ratio) as u8,
                            (255.0 * alpha_ratio) as u8,
                        ),
                    });
                }
            }

            // Mirrored radial spectrum bars (total 120 bars around 360 degrees)
            let total_bars = 120.min(spectrum.len() * 2);
            let half_bars = total_bars / 2;
            if half_bars == 0 {
                return;
            }

            let step = (spectrum.len() as f64 / half_bars as f64).max(1.0);
            for i in 0..total_bars {
                // Mirror left and right from bottom center (theta = PI/2)
                let is_right = i < half_bars;
                let bar_idx = if is_right { i } else { total_bars - 1 - i };
                let freq_idx = ((bar_idx as f64 * step) as usize).min(spectrum.len() - 1);

                let frac = bar_idx as f64 / half_bars as f64; // 0.0 at bass (bottom) to 1.0 at treble (top)
                let angle_offset = frac * std::f64::consts::PI;
                let theta = if is_right {
                    std::f64::consts::PI / 2.0 - angle_offset
                } else {
                    std::f64::consts::PI / 2.0 + angle_offset
                };

                let val = (spectrum[freq_idx] * SENSITIVITY * 1.7) as f64;
                let peak = (peaks[freq_idx] * SENSITIVITY * 1.7) as f64;

                let x1 = cx + pulse_r * theta.cos();
                let y1 = cy + pulse_r * theta.sin();
                let x2 = cx + (pulse_r + val) * theta.cos();
                let y2 = cy + (pulse_r + val) * theta.sin();

                // High-energy spectrum color palette: Hot Magenta (bass) -> Electric Cyan/Blue -> Gold (treble)
                let r = ((1.0 - frac) * 255.0 + frac * 50.0).clamp(0.0, 255.0) as u8;
                let g =
                    ((frac * std::f64::consts::PI).sin() * 230.0 + 30.0).clamp(0.0, 255.0) as u8;
                let b = (frac * 255.0 + (1.0 - frac) * 120.0).clamp(0.0, 255.0) as u8;

                ctx.draw(&Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    color: Color::Rgb(r, g, b),
                });

                // Peak caps hovering above bars
                if peak > val + 0.8 {
                    let px = cx + (pulse_r + peak + 1.2) * theta.cos();
                    let py = cy + (pulse_r + peak + 1.2) * theta.sin();
                    ctx.draw(&Rectangle {
                        x: px,
                        y: py,
                        width: 1.0,
                        height: 1.0,
                        color: Color::White,
                    });
                }
            }
        });

    frame.render_widget(canvas, inner_area);
}
