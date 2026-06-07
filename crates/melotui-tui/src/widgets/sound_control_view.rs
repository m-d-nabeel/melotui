use crate::theme::Theme;
use melotui_core::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
};

fn theme() -> &'static Theme {
    static THEME: std::sync::OnceLock<Theme> = std::sync::OnceLock::new();
    THEME.get_or_init(Theme::default)
}

/// How to format the slider value display.
/// Using an enum instead of string-matching the label name.
#[derive(Clone, Copy)]
enum SliderFormat {
    Percent,    // "75%"
    Multiplier, // "x1.5"
}

struct SliderConfig<'a> {
    label: &'a str,
    val: f32,
    min: f32,
    max: f32,
    color: Color,
    format: SliderFormat,
}

impl SliderConfig<'_> {
    fn format_value(&self) -> String {
        match self.format {
            SliderFormat::Percent => format!("{:.0}%", self.val * 100.0),
            SliderFormat::Multiplier => format!("x{:.1}", self.val),
        }
    }
}

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let t = theme();
    let block = Block::default()
        .title(" Mixer ")
        .title_style(Style::default().add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border_inactive))
        .padding(Padding::horizontal(2));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // 3 column grid
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),   // Col 1
            Constraint::Length(4), // Gap
            Constraint::Fill(1),   // Col 2
            Constraint::Length(4), // Gap
            Constraint::Fill(1),   // Col 3
        ])
        .split(inner);

    let col1 = layout[0];
    let col2 = layout[2];
    let col3 = layout[4];

    let split_rows = |rect| {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(1),
                Constraint::Fill(1),
            ])
            .split(rect)
    };

    let c1 = split_rows(col1);
    let c2 = split_rows(col2);
    let c3 = split_rows(col3);

    // Col 1: Volume + Speed
    render_sleek_slider(
        frame,
        c1[0],
        &SliderConfig {
            label: "Volume",
            val: state.volume(),
            min: 0.0,
            max: 1.5,
            color: t.slider_volume,
            format: SliderFormat::Percent,
        },
    );
    render_sleek_slider(
        frame,
        c1[2],
        &SliderConfig {
            label: "Speed",
            val: state.speed(),
            min: 0.5,
            max: 2.0,
            color: t.slider_speed,
            format: SliderFormat::Multiplier,
        },
    );

    // Col 2: Bass + Treble
    render_sleek_slider(
        frame,
        c2[0],
        &SliderConfig {
            label: "Bass",
            val: state.bass(),
            min: 0.0,
            max: 5.0,
            color: t.slider_bass,
            format: SliderFormat::Multiplier,
        },
    );
    render_sleek_slider(
        frame,
        c2[2],
        &SliderConfig {
            label: "Treble",
            val: state.treble(),
            min: 0.0,
            max: 5.0,
            color: t.slider_treble,
            format: SliderFormat::Multiplier,
        },
    );

    // Col 3: Reverb + Echo
    render_sleek_slider(
        frame,
        c3[0],
        &SliderConfig {
            label: "Reverb",
            val: state.reverb(),
            min: 0.0,
            max: 1.0,
            color: t.slider_reverb,
            format: SliderFormat::Percent,
        },
    );
    render_sleek_slider(
        frame,
        c3[2],
        &SliderConfig {
            label: "Echo",
            val: state.echo(),
            min: 0.0,
            max: 0.9,
            color: t.slider_echo,
            format: SliderFormat::Percent,
        },
    );
}

fn render_sleek_slider(frame: &mut Frame, area: Rect, config: &SliderConfig) {
    let t = theme();
    let ratio = ((config.val - config.min) / (config.max - config.min)).clamp(0.0, 1.0);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

    let label_area = layout[1];
    let slider_area = layout[2];

    // Row 1: label + value
    let header = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Fill(1), Constraint::Fill(1)])
        .split(label_area);

    let label_span = Span::styled(
        config.label,
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(t.fg_muted),
    );
    frame.render_widget(Paragraph::new(label_span), header[0]);

    let val_str = config.format_value();
    frame.render_widget(
        Paragraph::new(val_str)
            .style(Style::default().fg(t.fg_muted))
            .alignment(Alignment::Right),
        header[1],
    );

    // Row 2: slider
    let width = slider_area.width as usize;
    if width < 1 {
        return;
    }

    let track_width = width.saturating_sub(1);
    let thumb_pos = (ratio * track_width as f32).round() as usize;
    let remaining = track_width.saturating_sub(thumb_pos);

    let active_line = "─".repeat(thumb_pos);
    let inactive_line = "─".repeat(remaining);

    let slider_line = Line::from(vec![
        Span::styled(active_line, Style::default().fg(config.color)),
        Span::styled("●", Style::default().fg(config.color)),
        Span::styled(inactive_line, Style::default().fg(t.slider_track)),
    ]);

    frame.render_widget(Paragraph::new(slider_line), slider_area);
}
