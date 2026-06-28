use melotui_core::{AppState, state::EQ_BAND_LABELS};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::{
    image_cache::ImageCache,
    theme::{Theme, get_theme},
    widgets::visualizer_view,
};

/// Draw the DSP Studio view occupying the full right column (minus playback bar).
pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, cache: &mut ImageCache) {
    let t = get_theme(state);
    let is_wide = area.width >= 72; // enough for vertical EQ bars

    let eq_height = if is_wide { 10 } else { 12 };

    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(6),                // Mini spectrum
            Constraint::Length(eq_height),      // EQ section
            Constraint::Fill(1),               // Sound + Effects
        ])
        .split(area);

    draw_mini_spectrum(frame, main_layout[0], state, cache);

    if is_wide {
        draw_vertical_eq(frame, main_layout[1], state, &t);
    } else {
        draw_horizontal_eq(frame, main_layout[1], state, &t);
    }

    draw_controls_grid(frame, main_layout[2], state, &t);
}

// ── Mini Spectrum ────────────────────────────────────────

fn draw_mini_spectrum(frame: &mut Frame, area: Rect, state: &AppState, _cache: &mut ImageCache) {
    // Reuse existing spectrum visualizer in a smaller area
    visualizer_view::draw(frame, area, state);
}

// ── Vertical EQ (wide terminals) ─────────────────────────

fn draw_vertical_eq(frame: &mut Frame, area: Rect, state: &AppState, t: &Theme) {
    let block = Block::default()
        .title(" Equalizer ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.dsp_section_border));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width < 20 || inner.height < 4 {
        return;
    }

    let eq_bands = state.eq_bands();
    let selected = state.dsp_selected_control();

    // Layout: 10 equal columns for 10 bands
    let band_constraints: Vec<Constraint> = (0..10).map(|_| Constraint::Fill(1)).collect();
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(band_constraints)
        .split(inner);

    let bar_height = inner.height.saturating_sub(1) as f32; // reserve 1 row for label
    let max_db = 12.0_f32;

    for (i, col) in columns.iter().enumerate() {
        let gain_db = eq_bands[i];
        let is_selected = selected == i;

        // Bar area (all rows except last)
        let bar_area = Rect {
            x: col.x,
            y: col.y,
            width: col.width,
            height: col.height.saturating_sub(1),
        };
        // Label area (last row)
        let label_area = Rect {
            x: col.x,
            y: col.y + bar_area.height,
            width: col.width,
            height: 1,
        };

        draw_vertical_bar(frame, bar_area, gain_db, max_db, is_selected, t);

        // Band label
        let label_color = if is_selected {
            t.eq_bar_selected
        } else {
            t.fg_muted
        };
        let label = Paragraph::new(EQ_BAND_LABELS[i])
            .alignment(Alignment::Center)
            .style(Style::default().fg(label_color));
        frame.render_widget(label, label_area);
    }
}

fn draw_vertical_bar(
    frame: &mut Frame,
    area: Rect,
    gain_db: f32,
    max_db: f32,
    is_selected: bool,
    t: &Theme,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let height = area.height as f32;
    let center_y = area.y + (area.height / 2);

    // Normalized gain: -max_db..+max_db maps to 0..height
    let normalized = (gain_db / max_db).clamp(-1.0, 1.0);

    let fill_color = if is_selected {
        t.eq_bar_selected
    } else {
        t.eq_bar_fill
    };

    for row in 0..area.height {
        let y = area.y + row;
        let row_pos = 1.0 - (row as f32 / height) * 2.0; // +1 at top, -1 at bottom

        let is_zero_line = y == center_y;
        let is_filled = if gain_db >= 0.0 {
            row_pos >= 0.0 && row_pos <= normalized
        } else {
            row_pos <= 0.0 && row_pos >= normalized
        };

        let (ch, color) = if is_filled {
            ("█", fill_color)
        } else if is_zero_line {
            ("─", t.eq_zero_line)
        } else {
            ("░", t.eq_bar_empty)
        };

        let bar_width = area.width.min(3); // cap visual width
        let x_offset = (area.width - bar_width) / 2;

        let span = Span::styled(
            ch.repeat(bar_width as usize),
            Style::default().fg(color),
        );
        let para = Paragraph::new(Line::from(span)).alignment(Alignment::Center);
        let cell = Rect {
            x: area.x + x_offset,
            y,
            width: bar_width,
            height: 1,
        };
        frame.render_widget(para, cell);
    }
}

// ── Horizontal EQ (narrow terminals) ─────────────────────

fn draw_horizontal_eq(frame: &mut Frame, area: Rect, state: &AppState, t: &Theme) {
    let block = Block::default()
        .title(" Equalizer ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.dsp_section_border));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let eq_bands = state.eq_bands();
    let selected = state.dsp_selected_control();

    // Show as many bands as we can fit (1 row each)
    let visible = inner.height.min(10) as usize;
    let band_constraints: Vec<Constraint> = (0..visible).map(|_| Constraint::Length(1)).collect();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(band_constraints)
        .split(inner);

    for (i, row) in rows.iter().enumerate() {
        if i >= 10 {
            break;
        }
        let is_selected = selected == i;
        let gain_db = eq_bands[i];
        render_eq_slider(frame, *row, EQ_BAND_LABELS[i], gain_db, -12.0, 12.0, is_selected, t);
    }
}

fn render_eq_slider(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: f32,
    min: f32,
    max: f32,
    is_selected: bool,
    t: &Theme,
) {
    if area.width < 10 {
        return;
    }

    let label_width = 6u16;
    let value_width = 7u16;
    let bar_width = area.width.saturating_sub(label_width + value_width);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(label_width),
            Constraint::Length(bar_width),
            Constraint::Length(value_width),
        ])
        .split(area);

    let label_color = if is_selected { t.eq_bar_selected } else { t.fg_secondary };
    let bg = if is_selected { t.dsp_selected_bg } else { ratatui::style::Color::Reset };

    // Label
    frame.render_widget(
        Paragraph::new(format!("{:>5}", label))
            .style(Style::default().fg(label_color).bg(bg)),
        cols[0],
    );

    // Slider bar
    let ratio = ((value - min) / (max - min)).clamp(0.0, 1.0);
    render_slider_bar(frame, cols[1], ratio, is_selected, bg, t);

    // Value
    let val_str = format!("{:+.0}dB", value);
    frame.render_widget(
        Paragraph::new(val_str)
            .alignment(Alignment::Right)
            .style(Style::default().fg(label_color).bg(bg)),
        cols[2],
    );
}

// ── Controls Grid ────────────────────────────────────────

fn draw_controls_grid(frame: &mut Frame, area: Rect, state: &AppState, t: &Theme) {
    let selected = state.dsp_selected_control();

    // Two-column layout: Sound (left) | Effects (right)
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left column: Sound
    {
        let block = Block::default()
            .title(" Sound ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(t.dsp_section_border));

        let inner = block.inner(cols[0]);
        frame.render_widget(block, cols[0]);

        let items: [(usize, &str, f32, f32, f32, fn(f32) -> String, ratatui::style::Color); 5] = [
            (10, "Volume", state.volume(), 0.0, 1.5, format_percent, t.slider_volume),
            (11, "Speed", state.speed(), 0.5, 2.0, format_speed, t.slider_speed),
            (12, "Pan", state.pan(), -1.0, 1.0, format_pan, t.slider_pan),
            (13, "Bass", state.bass(), 0.1, 5.0, format_mult, t.slider_bass),
            (14, "Treble", state.treble(), 0.0, 5.0, format_mult, t.slider_treble),
        ];

        let row_constraints: Vec<Constraint> = items.iter().map(|_| Constraint::Length(1)).collect();
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(row_constraints)
            .split(inner);

        for (i, (ctrl_idx, label, value, min, max, fmt, color)) in items.iter().enumerate() {
            if i >= rows.len() {
                break;
            }
            let is_sel = selected == *ctrl_idx;
            render_control_slider(frame, rows[i], label, *value, *min, *max, fmt, is_sel, *color, t);
        }
    }

    // Right column: Effects
    {
        let block = Block::default()
            .title(" Effects ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(t.dsp_section_border));

        let inner = block.inner(cols[1]);
        frame.render_widget(block, cols[1]);

        let items: [(usize, &str, f32, f32, f32, fn(f32) -> String, ratatui::style::Color); 4] = [
            (15, "Reverb", state.reverb(), 0.0, 1.0, format_percent, t.slider_reverb),
            (16, "Echo", state.echo(), 0.0, 0.9, format_percent, t.slider_echo),
            (17, "Lo-pass", state.lowpass(), 200.0, 20000.0, format_freq, t.slider_lowpass),
            (18, "Hi-pass", state.highpass(), 20.0, 5000.0, format_freq, t.slider_highpass),
        ];

        let row_constraints: Vec<Constraint> = items.iter().map(|_| Constraint::Length(1)).collect();
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(row_constraints)
            .split(inner);

        for (i, (ctrl_idx, label, value, min, max, fmt, color)) in items.iter().enumerate() {
            if i >= rows.len() {
                break;
            }
            let is_sel = selected == *ctrl_idx;
            render_control_slider(frame, rows[i], label, *value, *min, *max, fmt, is_sel, *color, t);
        }
    }
}

fn render_control_slider(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: f32,
    min: f32,
    max: f32,
    format_fn: &dyn Fn(f32) -> String,
    is_selected: bool,
    accent: ratatui::style::Color,
    t: &Theme,
) {
    if area.width < 15 {
        return;
    }

    let label_width = 8u16;
    let value_width = 8u16;
    let bar_width = area.width.saturating_sub(label_width + value_width);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(label_width),
            Constraint::Length(bar_width),
            Constraint::Length(value_width),
        ])
        .split(area);

    let label_color = if is_selected { accent } else { t.fg_secondary };
    let bg = if is_selected {
        t.dsp_selected_bg
    } else {
        ratatui::style::Color::Reset
    };

    // Label
    let label_style = if is_selected {
        Style::default().fg(label_color).bg(bg).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(label_color).bg(bg)
    };
    frame.render_widget(
        Paragraph::new(format!(" {:<7}", label)).style(label_style),
        cols[0],
    );

    // Slider bar
    let ratio = ((value - min) / (max - min)).clamp(0.0, 1.0);
    render_control_bar(frame, cols[1], ratio, is_selected, accent, bg, t);

    // Value
    let val_str = format_fn(value);
    frame.render_widget(
        Paragraph::new(format!("{:>7} ", val_str))
            .style(Style::default().fg(label_color).bg(bg)),
        cols[2],
    );
}

fn render_slider_bar(
    frame: &mut Frame,
    area: Rect,
    ratio: f32,
    is_selected: bool,
    bg: ratatui::style::Color,
    t: &Theme,
) {
    let color = if is_selected { t.eq_bar_selected } else { t.eq_bar_fill };
    render_control_bar(frame, area, ratio, is_selected, color, bg, t);
}

fn render_control_bar(
    frame: &mut Frame,
    area: Rect,
    ratio: f32,
    _is_selected: bool,
    accent: ratatui::style::Color,
    bg: ratatui::style::Color,
    t: &Theme,
) {
    if area.width < 3 {
        return;
    }

    let width = area.width as usize;
    let filled = ((ratio * width as f32) as usize).min(width);

    let mut bar = String::with_capacity(width);
    for i in 0..width {
        if i == filled && i > 0 && i < width {
            bar.push('●');
        } else if i < filled {
            bar.push('━');
        } else {
            bar.push('─');
        }
    }

    let line = Line::from(vec![
        Span::styled(
            bar[..bar.char_indices().nth(filled.max(1)).map_or(bar.len(), |(idx, _)| idx)].to_string(),
            Style::default().fg(accent).bg(bg),
        ),
        Span::styled(
            bar[bar.char_indices().nth(filled.max(1)).map_or(bar.len(), |(idx, _)| idx)..].to_string(),
            Style::default().fg(t.slider_track).bg(bg),
        ),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

// ── Value Formatters ─────────────────────────────────────

fn format_percent(v: f32) -> String {
    format!("{}%", (v * 100.0) as i32)
}

fn format_speed(v: f32) -> String {
    format!("x{:.1}", v)
}

fn format_mult(v: f32) -> String {
    format!("x{:.1}", v)
}

fn format_pan(v: f32) -> String {
    if v.abs() < 0.05 {
        "Center".to_string()
    } else if v < 0.0 {
        format!("L {}%", (v.abs() * 100.0) as i32)
    } else {
        format!("R {}%", (v * 100.0) as i32)
    }
}

fn format_freq(v: f32) -> String {
    if v >= 1000.0 {
        format!("{:.1}kHz", v / 1000.0)
    } else {
        format!("{:.0}Hz", v)
    }
}
