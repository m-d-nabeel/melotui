use crate::theme::{Theme, blend};
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

/// The colour `x` of the way through the theme's visualizer colours: 0 its
/// low, a half its middle, 1 its high.
fn gradient(t: &Theme, x: f64) -> Color {
    let x = x.clamp(0.0, 1.0) as f32;
    if x < 0.5 {
        blend(t.visualizer_low, t.visualizer_mid, x * 2.0)
    } else {
        blend(t.visualizer_mid, t.visualizer_high, (x - 0.5) * 2.0)
    }
}

/// `colour` at `level` of its strength: faded toward what it is drawn on,
/// which is black on a dark theme and white on a light one.
fn faded(t: &Theme, colour: Color, level: f64) -> Color {
    blend(
        colour,
        t.palette.ground(),
        (1.0 - level.clamp(0.0, 1.0)) as f32,
    )
}

/// A peak's colour: lighter than its bar, or on a light theme stronger.
fn peak_of(t: &Theme, colour: Color) -> Color {
    blend(colour, t.fg_primary, 0.35)
}

/// The block every style is drawn in.
fn block<'a>(t: &Theme, title: &'a str) -> Block<'a> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(ratatui::style::Style::default().fg(t.border_panel))
}

pub fn draw_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let t = crate::theme::get_theme(state);
    let block = block(t, " Visualizer ");

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let time = state.time_elapsed;

    let spectrum = &state.spectrum_smooth;
    let peaks = &state.spectrum_peaks;

    if spectrum.is_empty() {
        return;
    }

    let canvas = Canvas::default()
        .background_color(t.bg_main)
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

                // Low to high across the bars, shimmering a little.
                let freq_ratio = i as f64 / num_bars as f64;
                let shimmer = 0.06 * (time * 2.0).sin();
                let color = gradient(t, freq_ratio + shimmer);
                let peak_color = peak_of(t, color);

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
    let t = crate::theme::get_theme(state);
    let block = block(t, " Visualizer ");

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let spectrum = &state.spectrum_smooth;
    let peaks = &state.spectrum_peaks;

    if spectrum.is_empty() {
        return;
    }

    let canvas = Canvas::default()
        .background_color(t.bg_main)
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

                // Low to high across the bars, and higher the louder.
                let loudness = f64::from((spectrum[idx] * 800.0).clamp(0.0, 155.0)) / 155.0;
                let color = gradient(t, 0.6 * freq_ratio + 0.4 * loudness);
                let peak_color = peak_of(t, color);

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
    let t = crate::theme::get_theme(state);
    let block = block(t, " Visualizer ");

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
        .background_color(t.bg_main)
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
                let color_phase = (i as f64 / num_ui_bars as f64) + (time * 0.1);
                let color = gradient(t, (color_phase * 2.0).sin() * 0.5 + 0.5);
                let peak_color = peak_of(t, color);

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

/// Particle: droplets thrown up and down from a wavering centre line, the
/// bass in the middle. Where they turn traces the spectrum; their motion is
/// worked out with the rest of the visualizer's physics.
pub fn draw_particle(frame: &mut Frame, area: Rect, state: &AppState) {
    let t = crate::theme::get_theme(state);
    let block = block(t, " Visualizer ");

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let time = state.time_elapsed;
    let drops = &state.spectrum_motion.fountain().drops;

    let canvas = Canvas::default()
        .background_color(t.bg_main)
        .marker(symbols::Marker::Braille)
        .x_bounds([0.0, (inner_area.width * 2) as f64])
        .y_bounds([0.0, (inner_area.height * 4) as f64])
        .paint(move |ctx| {
            let width = (inner_area.width * 2) as f64;
            let height = (inner_area.height * 4) as f64;
            let (center_x, center_y) = (width / 2.0, height / 2.0);
            let reach_y = center_y * 0.95;

            // The centre line, wavering slowly.
            let line = faded(t, t.visualizer_low, 0.67);
            for x_int in 0..(width as usize) {
                let x = x_int as f64;
                let wave = (x * 0.025 + time * 2.0).sin() * 1.5;
                ctx.draw(&Line {
                    x1: x,
                    y1: center_y + wave - 0.5,
                    x2: x,
                    y2: center_y + wave + 0.5,
                    color: line,
                });
            }

            // Droplets in eight shades, from deep blue thrown low to light
            // blue thrown high, a shade dimmer on the way down; drawn a shade
            // at a time, mirrored either side and above and below the line.
            let mut shades: [Vec<(f64, f64)>; 16] = Default::default();
            for d in drops {
                let shade =
                    ((d.peak * 7.0).round() as usize).min(7) + if d.vy < 0.0 { 8 } else { 0 };
                let dx = d.x as f64 * center_x;
                let dy = d.y as f64 * reach_y;
                for (x, y) in [
                    (center_x + dx, center_y + dy),
                    (center_x - dx, center_y + dy),
                    (center_x + dx, center_y - dy),
                    (center_x - dx, center_y - dy),
                ] {
                    shades[shade].push((x, y));
                }
            }
            for (shade, coords) in shades.iter().enumerate() {
                if coords.is_empty() {
                    continue;
                }
                let along = (shade % 8) as f64 / 7.0;
                let dim = if shade >= 8 { 0.75 } else { 1.0 };
                let color = faded(t, gradient(t, along), dim);
                ctx.draw(&ratatui::widgets::canvas::Points { coords, color });
            }
        });

    frame.render_widget(canvas, inner_area);
}

pub fn draw_radial(frame: &mut Frame, area: Rect, state: &AppState) {
    let t = crate::theme::get_theme(state);
    let block = block(t, " Visualizer: Radial (Orbit) ");

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let spectrum = &state.spectrum_smooth;
    let peaks = &state.spectrum_peaks;
    let time = state.time_elapsed;

    if spectrum.is_empty() {
        return;
    }

    let canvas = Canvas::default()
        .background_color(t.bg_main)
        .marker(symbols::Marker::Braille)
        .x_bounds([0.0, (inner_area.width * 2) as f64])
        .y_bounds([0.0, (inner_area.height * 4) as f64])
        .paint(move |ctx| {
            let width = (inner_area.width * 2) as f64;
            let height = (inner_area.height * 4) as f64;
            let cx = width / 2.0;
            let cy = height / 2.0;

            let base_r = (width.min(height) * 0.22).max(5.0);
            let bass = melotui_core::state::nova::kick_level(spectrum) as f64;
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

                // Round the ring through the theme's colours, turning.
                let phase = (i as f64 / num_rays as f64) + time * 0.1;
                let ray = gradient(t, (phase * std::f64::consts::TAU).sin() * 0.5 + 0.5);

                ctx.draw(&Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    color: ray,
                });

                if peak > val + 1.0 {
                    let px = cx + (pulse_r + peak + 1.5) * theta.cos();
                    let py = cy + (pulse_r + peak + 1.5) * theta.sin();
                    ctx.draw(&Rectangle {
                        x: px,
                        y: py,
                        width: 1.0,
                        height: 1.0,
                        color: peak_of(t, t.visualizer_high),
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
                        color: ray,
                    });
                }
            }
        });

    frame.render_widget(canvas, inner_area);
}

/// Nova: particles thrown out of a pulsing core by the music's kicks, with
/// a shockwave ring for each kick. The particles' motion is worked out with
/// the rest of the visualizer's physics; this draws them where they are.
pub fn draw_nova(frame: &mut Frame, area: Rect, state: &AppState) {
    let t = crate::theme::get_theme(state);
    let block = block(t, " Visualizer: Nova ");

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let nova = state.spectrum_motion.nova();

    let canvas = Canvas::default()
        .background_color(t.bg_main)
        .marker(symbols::Marker::Braille)
        .x_bounds([0.0, (inner_area.width * 2) as f64])
        .y_bounds([0.0, (inner_area.height * 4) as f64])
        .paint(move |ctx| {
            let width = (inner_area.width * 2) as f64;
            let height = (inner_area.height * 4) as f64;
            let (cx, cy) = (width / 2.0, height / 2.0);
            // Braille dots are about as tall as they are wide, so one unit
            // serves both ways: the reach from the core to the nearer edge.
            let reach = width.min(height) / 2.0;
            let at = |x: f32, y: f32| (cx + x as f64 * reach, cy + y as f64 * reach);

            // The core, swelling on each kick.
            let core = (melotui_core::state::nova::CORE as f64 * reach).max(3.0)
                * (1.0 + 0.5 * nova.pulse as f64);
            let glow = 0.55 + 0.45 * nova.pulse as f64;
            let steps = 72;
            for s in 0..steps {
                let a1 = s as f64 * std::f64::consts::TAU / steps as f64;
                let a2 = (s + 1) as f64 * std::f64::consts::TAU / steps as f64;
                ctx.draw(&Line {
                    x1: cx + core * a1.cos(),
                    y1: cy + core * a1.sin(),
                    x2: cx + core * a2.cos(),
                    y2: cy + core * a2.sin(),
                    color: faded(t, t.visualizer_high, glow),
                });
            }

            // The last kick's shockwave, spreading out and fading.
            const WAVE_SECS: f32 = 0.6;
            if nova.since_kick < WAVE_SECS {
                let spread = (nova.since_kick / WAVE_SECS) as f64;
                let radius = core + (reach - core) * spread.powf(0.7);
                let fade = (1.0 - spread) * (0.35 + 0.65 * nova.kick_strength as f64);
                let color = faded(t, t.visualizer_mid, fade);
                for s in (0..steps).step_by(2) {
                    let a1 = s as f64 * std::f64::consts::TAU / steps as f64;
                    let a2 = (s + 1) as f64 * std::f64::consts::TAU / steps as f64;
                    ctx.draw(&Line {
                        x1: cx + radius * a1.cos(),
                        y1: cy + radius * a1.sin(),
                        x2: cx + radius * a2.cos(),
                        y2: cy + radius * a2.sin(),
                        color,
                    });
                }
            }

            // The particles, each a streak along its way, dimming as it ages.
            for p in &nova.particles {
                let fade = 0.3 + 0.7 * p.freshness() as f64;
                let streak = faded(t, gradient(t, p.hue as f64), fade);
                let (x1, y1) = at(p.x, p.y);
                let (x2, y2) = at(p.x - p.vx * 0.04, p.y - p.vy * 0.04);
                ctx.draw(&Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    color: streak,
                });
            }
        });

    frame.render_widget(canvas, inner_area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_visualizer_is_drawn_in_the_themes_colours() {
        let t = Theme::built_in("nord").unwrap();
        assert_eq!(
            gradient(&t, 0.0),
            blend(t.visualizer_low, t.visualizer_low, 0.0)
        );
        assert_eq!(
            gradient(&t, 0.5),
            blend(t.visualizer_mid, t.visualizer_mid, 0.0)
        );
        assert_eq!(
            gradient(&t, 1.0),
            blend(t.visualizer_high, t.visualizer_high, 0.0)
        );
        // Past either end, the end.
        assert_eq!(gradient(&t, -3.0), gradient(&t, 0.0));
        assert_eq!(gradient(&t, 9.0), gradient(&t, 1.0));
    }

    #[test]
    fn what_fades_fades_into_the_background_dark_or_light() {
        for name in ["tokyo_night", "catppuccin_latte"] {
            let t = Theme::built_in(name).unwrap();
            let full = t.visualizer_mid;
            assert_eq!(faded(&t, full, 1.0), blend(full, full, 0.0), "{name}");
            assert_eq!(
                faded(&t, full, 0.0),
                blend(t.palette.ground(), full, 0.0),
                "{name}"
            );
            // Half-faded is nearer the background than the colour is.
            let half = faded(&t, full, 0.5);
            let toward = crate::theme::contrast(half, t.palette.ground())
                < crate::theme::contrast(full, t.palette.ground());
            assert!(toward, "{name}: {half:?} is not nearer the background");
        }
    }
}
