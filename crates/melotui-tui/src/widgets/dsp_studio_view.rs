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

/// Draw the DSP Studio view occupying the full screen (no sidebar).
pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, cache: &mut ImageCache) {
    let t = get_theme(state);

    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),    // Spectrum visualizer (gets remaining space)
            Constraint::Length(12), // Horizontal EQ (10 bands + borders)
            Constraint::Length(8),  // Controls grid (3 columns, max 6 inline items + borders)
            Constraint::Length(1),  // Macro legend footer
        ])
        .split(area);

    draw_mini_spectrum(frame, main_layout[0], state, cache);
    draw_horizontal_eq(frame, main_layout[1], state, t, t.icon_category);
    draw_controls_grid(frame, main_layout[2], state, t);

    let muted = Style::default().fg(t.fg_muted);
    let legend = Paragraph::new(Line::from(vec![
        Span::styled(
            "[1] ",
            Style::default()
                .fg(t.accent_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Slowed+Reverb   ", muted),
        Span::styled(
            "[2] ",
            Style::default()
                .fg(t.accent_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Bass Boosted   ", muted),
        Span::styled(
            "[3] ",
            Style::default()
                .fg(t.accent_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("3D Spatial   ", muted),
        Span::styled(
            "[0] ",
            Style::default()
                .fg(t.accent_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Reset All", muted),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(legend, main_layout[3]);
}

// ── Mini Spectrum ────────────────────────────────────────

fn draw_mini_spectrum(frame: &mut Frame, area: Rect, state: &AppState, cache: &mut ImageCache) {
    visualizer_view::draw(frame, area, state, cache);
}

// ── Horizontal EQ ────────────────────────────────────────

fn draw_horizontal_eq(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    t: &Theme,
    border_color: ratatui::style::Color,
) {
    let block = Block::default()
        .title(" ◆ Equalizer ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let eq_bands = state.eq_bands();
    let selected = state.dsp_selected_control();

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
        render_eq_slider(
            frame,
            *row,
            EQ_BAND_LABELS[i],
            gain_db,
            (-12.0, 12.0),
            is_selected,
            t,
        );
    }
}

fn render_eq_slider(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: f32,
    range: (f32, f32),
    is_selected: bool,
    t: &Theme,
) {
    if area.width < 10 {
        return;
    }

    let label_width = 6u16;
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

    let accent = if is_selected {
        t.eq_bar_selected
    } else {
        t.eq_bar_fill
    };
    let label_color = if is_selected {
        t.eq_bar_selected
    } else {
        t.fg_secondary
    };
    let bg = if is_selected {
        t.dsp_selected_bg
    } else {
        ratatui::style::Color::Reset
    };

    // Label
    let label_style = if is_selected {
        Style::default()
            .fg(label_color)
            .bg(bg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(label_color).bg(bg)
    };
    frame.render_widget(
        Paragraph::new(format!("{:>5} ", label)).style(label_style),
        cols[0],
    );

    // Bipolar slider bar (center = zero)
    let (min, max) = range;
    let ratio = ((value - min) / (max - min)).clamp(0.0, 1.0);
    render_bipolar_bar(frame, cols[1], ratio, accent, bg, t, true);

    // Value
    frame.render_widget(
        Paragraph::new(format!(" {:+.0}dB", value))
            .alignment(Alignment::Left)
            .style(Style::default().fg(label_color).bg(bg)),
        cols[2],
    );
}

fn render_bipolar_bar(
    frame: &mut Frame,
    area: Rect,
    ratio: f32, // 0.0 = min, 0.5 = center, 1.0 = max
    accent: ratatui::style::Color,
    bg: ratatui::style::Color,
    t: &Theme,
    show_ticks: bool,
) {
    if area.width < 3 {
        return;
    }

    let width = area.width as usize;
    let center = width / 2;
    let step = (center / 4).max(1);

    let t1 = center.saturating_sub(step * 3);
    let t2 = center.saturating_sub(step * 2);
    let t3 = center.saturating_sub(step);
    let t4 = center;
    let t5 = center + step;
    let t6 = center + step * 2;
    let t7 = center + step * 3;

    let max_idx = width.saturating_sub(1);
    let pos = ((ratio * max_idx as f32).round() as usize).min(max_idx);

    let mut spans = Vec::with_capacity(width);
    for i in 0..width {
        let is_interval = show_ticks
            && i > 0
            && i < max_idx
            && (i == t1 || i == t2 || i == t3 || i == t5 || i == t6 || i == t7);

        let is_center = i == t4;

        let is_active = if pos >= center {
            i >= center && i < pos
        } else {
            i > pos && i <= center
        };

        let (ch, color) = if i == pos {
            ("●", accent)
        } else if is_center {
            ("◆", if is_active { accent } else { t.eq_zero_line })
        } else if is_active {
            ("━", accent)
        } else if is_interval {
            ("▪", t.slider_track)
        } else {
            ("┄", t.slider_track)
        };
        spans.push(Span::styled(ch, Style::default().fg(color).bg(bg)));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

// ── Controls Grid (3 columns) ────────────────────────────

struct ControlItem<'a> {
    id: usize,
    label: &'a str,
    value: f32,
    min: f32,
    max: f32,
    fmt: fn(f32) -> String,
    accent: ratatui::style::Color,
}

impl<'a> ControlItem<'a> {
    fn new(
        id: usize,
        label: &'a str,
        value: f32,
        min: f32,
        max: f32,
        fmt: fn(f32) -> String,
        accent: ratatui::style::Color,
    ) -> Self {
        Self {
            id,
            label,
            value,
            min,
            max,
            fmt,
            accent,
        }
    }
}

fn draw_controls_grid(frame: &mut Frame, area: Rect, state: &AppState, t: &Theme) {
    let selected = state.dsp_selected_control();

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(area);

    draw_control_column(
        frame,
        cols[0],
        " Sound ",
        &[
            ControlItem::new(
                10,
                "Volume",
                state.volume(),
                0.0,
                1.5,
                format_percent as fn(f32) -> String,
                t.slider_volume,
            ),
            ControlItem::new(
                11,
                "Speed",
                state.speed(),
                0.5,
                2.0,
                format_speed,
                t.slider_speed,
            ),
            ControlItem::new(
                12,
                "Bass",
                state.bass(),
                0.1,
                5.0,
                format_mult,
                t.slider_bass,
            ),
            ControlItem::new(
                13,
                "Treble",
                state.treble(),
                0.0,
                5.0,
                format_mult,
                t.slider_treble,
            ),
            ControlItem::new(
                21,
                "Exciter",
                state.exciter(),
                0.0,
                5.0,
                format_exciter,
                t.slider_bass,
            ),
            ControlItem::new(
                22,
                "Bitcrush",
                state.bitcrush(),
                0.0,
                1.0,
                format_percent_off,
                t.slider_treble,
            ),
        ],
        selected,
        t,
        t.slider_volume,
    );

    draw_control_column(
        frame,
        cols[1],
        " Spatial ",
        &[
            ControlItem::new(14, "Pan", state.pan(), -1.0, 1.0, format_pan, t.slider_pan),
            ControlItem::new(
                15,
                "Pan Spd",
                state.auto_pan_rate(),
                0.0,
                2.0,
                format_hz,
                t.slider_pan,
            ),
            ControlItem::new(
                16,
                "Pan Dpt",
                state.auto_pan_depth(),
                0.0,
                1.0,
                format_percent_off,
                t.slider_pan,
            ),
            ControlItem::new(
                23,
                "Width",
                state.width(),
                0.0,
                3.0,
                format_width,
                t.slider_pan,
            ),
            ControlItem::new(
                24,
                "Karaoke",
                state.karaoke(),
                0.0,
                1.0,
                format_percent_off,
                t.slider_pan,
            ),
            ControlItem::new(
                26,
                "PingPong",
                state.ping_pong(),
                0.0,
                1.0,
                format_percent_off,
                t.slider_pan,
            ),
        ],
        selected,
        t,
        t.slider_pan,
    );

    draw_control_column(
        frame,
        cols[2],
        " Effects ",
        &[
            ControlItem::new(
                17,
                "Reverb",
                state.reverb(),
                0.0,
                1.0,
                format_percent_off,
                t.slider_reverb,
            ),
            ControlItem::new(
                18,
                "Echo",
                state.echo(),
                0.0,
                0.9,
                format_percent_off,
                t.slider_echo,
            ),
            ControlItem::new(
                19,
                "Lowpass",
                state.lowpass(),
                200.0,
                20000.0,
                format_lopass,
                t.slider_lowpass,
            ),
            ControlItem::new(
                20,
                "Highpass",
                state.highpass(),
                20.0,
                5000.0,
                format_hipass,
                t.slider_highpass,
            ),
            ControlItem::new(
                25,
                "Wobble",
                state.wobble(),
                0.0,
                1.0,
                format_percent_off,
                t.slider_echo,
            ),
            ControlItem::new(
                27,
                "Phaser",
                state.phaser(),
                0.0,
                1.0,
                format_percent_off,
                t.slider_reverb,
            ),
        ],
        selected,
        t,
        t.slider_reverb,
    );
}

fn draw_control_column(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    items: &[ControlItem<'_>],
    selected: usize,
    t: &Theme,
    border_color: ratatui::style::Color,
) {
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let row_constraints: Vec<Constraint> =
        (0..items.len()).map(|_| Constraint::Length(1)).collect();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(inner);

    for (i, item) in items.iter().enumerate() {
        if i >= rows.len() {
            break;
        }
        let is_sel = selected == item.id;
        render_inline_control(frame, rows[i], item, is_sel, t);
    }
}

fn render_inline_control(
    frame: &mut Frame,
    area: Rect,
    item: &ControlItem<'_>,
    is_selected: bool,
    t: &Theme,
) {
    if area.width < 15 {
        return;
    }

    let label_width = 11u16; // e.g. "▸ Bitcrush "
    let value_width = 7u16; // e.g. "  20.0k" or "   100%"
    let bar_width = area.width.saturating_sub(label_width + value_width);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(label_width),
            Constraint::Length(bar_width),
            Constraint::Length(value_width),
        ])
        .split(area);

    let accent = if is_selected {
        item.accent
    } else {
        t.eq_bar_fill
    };
    let label_color = if is_selected {
        item.accent
    } else {
        t.fg_secondary
    };
    let bg = if is_selected {
        t.dsp_selected_bg
    } else {
        ratatui::style::Color::Reset
    };

    // Label
    let indicator = if is_selected { "▸" } else { " " };
    let label_style = if is_selected {
        Style::default()
            .fg(label_color)
            .bg(bg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(label_color).bg(bg)
    };
    frame.render_widget(
        Paragraph::new(format!("{} {:<8} ", indicator, item.label)).style(label_style),
        cols[0],
    );

    // Slider bar
    let ratio = ((item.value - item.min) / (item.max - item.min)).clamp(0.0, 1.0);
    if item.id == 14 {
        // Pan is bipolar (-1.0 to 1.0)
        render_bipolar_bar(frame, cols[1], ratio, accent, bg, t, false);
    } else {
        render_unipolar_bar(frame, cols[1], ratio, accent, bg, t);
    }

    // Value
    let val_str = (item.fmt)(item.value);
    frame.render_widget(
        Paragraph::new(format!("{:>7}", val_str))
            .alignment(Alignment::Right)
            .style(Style::default().fg(label_color).bg(bg)),
        cols[2],
    );
}

fn render_unipolar_bar(
    frame: &mut Frame,
    area: Rect,
    ratio: f32, // 0.0 = min, 1.0 = max
    accent: ratatui::style::Color,
    bg: ratatui::style::Color,
    t: &Theme,
) {
    if area.width < 3 {
        return;
    }

    let width = area.width as usize;
    let max_idx = width.saturating_sub(1);
    let pos = ((ratio * max_idx as f32).round() as usize).min(max_idx);

    let mut spans = Vec::with_capacity(width);
    for i in 0..width {
        let (ch, color) = if i < pos {
            ("━", accent)
        } else if i == pos {
            ("●", accent)
        } else {
            ("┄", t.slider_track)
        };
        spans.push(Span::styled(ch, Style::default().fg(color).bg(bg)));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
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
        format!("L{}%", (v.abs() * 100.0) as i32)
    } else {
        format!("R{}%", (v * 100.0) as i32)
    }
}

fn format_hz(v: f32) -> String {
    if v < 0.01 {
        "Off".to_string()
    } else {
        format!("{:.2}Hz", v)
    }
}

fn format_percent_off(v: f32) -> String {
    if v < 0.01 {
        "Off".to_string()
    } else {
        format!("{}%", (v * 100.0) as i32)
    }
}

fn format_width(v: f32) -> String {
    if (v - 1.0).abs() < 0.05 {
        "Normal".to_string()
    } else if v < 0.05 {
        "Mono".to_string()
    } else {
        format!("{}%", (v * 100.0) as i32)
    }
}

fn format_lopass(v: f32) -> String {
    if v >= 19900.0 {
        "Open".to_string()
    } else if v >= 1000.0 {
        format!("{:.1}k", v / 1000.0)
    } else {
        format!("{:.0}Hz", v)
    }
}

fn format_hipass(v: f32) -> String {
    if v <= 21.0 {
        "Open".to_string()
    } else if v >= 1000.0 {
        format!("{:.1}k", v / 1000.0)
    } else {
        format!("{:.0}Hz", v)
    }
}

fn format_exciter(v: f32) -> String {
    if v < 0.05 {
        "Off".to_string()
    } else {
        format!("+{:.1}", v)
    }
}
