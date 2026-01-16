use melotui_core::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
};

struct SliderConfig<'a> {
    label: &'a str,
    val: f32,
    min: f32,
    max: f32,
    color: Color,
}

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    // 1. Container Block with Margins
    // We use Padding to ensure content doesn't touch the borders
    let block = Block::default()
        .title(" Mixer ")
        .title_style(Style::default().add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::LightBlue))
        .padding(Padding::horizontal(2)); // Horizontal margin inside the box

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // 2. Grid Layout: 3 Columns
    // Using Constraint::Fill(1) ensures they share remaining space EQUALLY.
    // We use Length(2) for gaps between columns.
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

    // Skip the spacers (indices 1 and 3)
    let col1 = layout[0];
    let col2 = layout[2];
    let col3 = layout[4];

    // --- Helper to split column into 2 rows with a Vertical Gap ---
    let split_rows = |rect| {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),   // Top Control
                Constraint::Length(1), // Vertical Gap
                Constraint::Fill(1),   // Bottom Control
            ])
            .split(rect)
    };

    let c1 = split_rows(col1);
    let c2 = split_rows(col2);
    let c3 = split_rows(col3);

    // --- RENDER WIDGETS ---
    // Note: cX[0] is top, cX[2] is bottom (skipping the gap at [1])

    // Column 1: Master
    render_sleek_slider(
        frame,
        c1[0],
        &SliderConfig {
            label: "Volume",
            val: state.volume(),
            min: 0.0,
            max: 1.5,
            color: Color::Green,
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
            color: Color::Blue,
        },
    );

    // Column 2: EQ
    render_sleek_slider(
        frame,
        c2[0],
        &SliderConfig {
            label: "Bass",
            val: state.bass(),
            min: 0.0,
            max: 5.0,
            color: Color::Yellow,
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
            color: Color::Cyan,
        },
    );

    // Column 3: FX
    render_sleek_slider(
        frame,
        c3[0],
        &SliderConfig {
            label: "Reverb",
            val: state.reverb(),
            min: 0.0,
            max: 1.0,
            color: Color::Magenta,
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
            color: Color::Red,
        },
    );
}

fn render_sleek_slider(frame: &mut Frame, area: Rect, config: &SliderConfig) {
    let ratio = ((config.val - config.min) / (config.max - config.min)).clamp(0.0, 1.0);

    // 1. Center Content Vertically in the cell
    // We reserve exactly 2 lines (Label + Slider)
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Top Spacer
            Constraint::Length(1), // Row 1: Label & Value
            Constraint::Length(1), // Row 2: Slider Line
            Constraint::Min(0),    // Bottom Spacer
        ])
        .split(area);

    let label_area = layout[1];
    let slider_area = layout[2];

    // 2. Row 1: Label (Left) --- Value (Right)
    let header = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Fill(1), Constraint::Fill(1)])
        .split(label_area);

    // Left: Label "Volume"
    let label_span = Span::styled(
        config.label,
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::DarkGray),
    );
    frame.render_widget(Paragraph::new(label_span), header[0]);

    // Right: Value "80%"
    let val_str = if config.label == "Speed" || config.label == "Bass" || config.label == "Treble" {
        format!("x{:.1}", config.val)
    } else {
        format!("{:.0}%", config.val * 100.0)
    };
    frame.render_widget(
        Paragraph::new(val_str)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Right),
        header[1],
    );

    // 3. Row 2: Sleek Slider "─────●──────"
    let width = slider_area.width as usize;
    if width < 1 {
        return;
    }

    // Logic:
    // 1. Calculate thumb position (0 to width-1)
    // 2. Active part (left of thumb) is colored
    // 3. Inactive part (right of thumb) is dark gray
    // 4. Thumb is colored circle

    let track_width = width.saturating_sub(1); // reserve 1 char for thumb
    let thumb_pos = (ratio * track_width as f32).round() as usize;
    let remaining = track_width.saturating_sub(thumb_pos);

    // Use thin line "─" for specific sleek look, or "━" for bolder look
    // Your reference image uses a thin line.
    let active_line = "─".repeat(thumb_pos);
    let inactive_line = "─".repeat(remaining);

    let track_color = Color::Rgb(60, 60, 60); // Deep gray for inactive part

    let slider_line = Line::from(vec![
        // Active Track (Colored)
        Span::styled(active_line, Style::default().fg(config.color)),
        // Thumb (Colored Circle)
        Span::styled("●", Style::default().fg(config.color)),
        // Inactive Track (Dark Gray)
        Span::styled(inactive_line, Style::default().fg(track_color)),
    ]);

    frame.render_widget(Paragraph::new(slider_line), slider_area);
}
