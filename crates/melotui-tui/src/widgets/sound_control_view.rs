use crate::theme::Theme;
use melotui_core::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
};

/// How to format the slider value display.
/// Using an enum instead of string-matching the label name.
#[derive(Clone, Copy)]
enum SliderFormat {
    Percent,    // "75%"
    Multiplier, // "x1.5"
}

struct SliderConfig<'a> {
    /// The control's number in the DSP studio, which the mixer's selection
    /// shares.
    control: usize,
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
    let t = crate::theme::get_theme(state);
    // Moved on to with Tab, it is marked as the tree and the queue are, and
    // the control its keys adjust picked out.
    let focused = state.is_mixer_focused();
    let selected = focused.then(|| state.dsp_selected_control());
    let (border, title) = if focused {
        (t.border_active, Style::default().fg(t.border_active))
    } else {
        (t.border_inactive, Style::default())
    };
    let block = Block::default()
        .title(" Mixer ")
        .title_style(title.add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border))
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
        t,
        selected,
        &SliderConfig {
            control: 10,
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
        t,
        selected,
        &SliderConfig {
            control: 11,
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
        t,
        selected,
        &SliderConfig {
            control: 12,
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
        t,
        selected,
        &SliderConfig {
            control: 13,
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
        t,
        selected,
        &SliderConfig {
            control: 17,
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
        t,
        selected,
        &SliderConfig {
            control: 18,
            label: "Echo",
            val: state.echo(),
            min: 0.0,
            max: 0.9,
            color: t.slider_echo,
            format: SliderFormat::Percent,
        },
    );
}

fn render_sleek_slider(
    frame: &mut Frame,
    area: Rect,
    t: &Theme,
    selected: Option<usize>,
    config: &SliderConfig,
) {
    let is_selected = selected == Some(config.control);
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

    let (label_style, value_style) = if is_selected {
        let row = Style::default().bg(t.dsp_selected_bg);
        (row.fg(config.color), row.fg(t.fg_primary))
    } else {
        (
            Style::default().fg(t.fg_muted),
            Style::default().fg(t.fg_muted),
        )
    };
    let label_span = Span::styled(config.label, label_style.add_modifier(Modifier::BOLD));
    frame.render_widget(Paragraph::new(label_span).style(label_style), header[0]);

    let val_str = config.format_value();
    frame.render_widget(
        Paragraph::new(val_str)
            .style(value_style)
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

#[cfg(test)]
mod tests {
    use super::*;
    use melotui_core::{
        AppMsg,
        library::snapshot::Library,
        message::{navigation::NavigationMsg, ui::UIMsg},
        reducer::reduce,
    };
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    fn drawn(state: &AppState) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(60, 8)).unwrap();
        terminal.draw(|f| draw(f, f.area(), state)).unwrap();
        terminal.backend().buffer().clone()
    }

    /// The cell where `word` starts.
    fn find(buf: &Buffer, word: &str) -> (u16, u16) {
        let area = buf.area;
        (0..area.height)
            .find_map(|y| {
                let row: String = (0..area.width).map(|x| buf[(x, y)].symbol()).collect();
                row.find(word).map(|x| (x as u16, y))
            })
            .unwrap()
    }

    #[test]
    fn moved_on_to_it_is_marked_and_its_selected_control_picked_out() {
        let mut state = AppState::new(
            Library::new(vec![], vec![], Default::default()),
            &melotui_core::config::AppConfig::default(),
        );
        let tab = || AppMsg::Navigation(NavigationMsg::ToggleLibraryQueue);
        // Bass: right of the volume.
        reduce(
            &mut state,
            AppMsg::UI(UIMsg::MixerSelect {
                columns: 1,
                rows: 0,
            }),
        );
        let t = crate::theme::get_theme(&state).clone();

        let buf = drawn(&state);
        assert_eq!(buf[(0, 0)].fg, t.border_inactive);
        assert_ne!(buf[find(&buf, "Bass")].bg, t.dsp_selected_bg, "not focused");

        reduce(&mut state, tab());
        reduce(&mut state, tab());
        let buf = drawn(&state);
        assert_eq!(buf[(0, 0)].fg, t.border_active);
        assert_eq!(buf[find(&buf, "Bass")].bg, t.dsp_selected_bg);
        assert_eq!(buf[find(&buf, "Bass")].fg, t.slider_bass);
        assert_ne!(buf[find(&buf, "Volume")].bg, t.dsp_selected_bg);
    }
}
