use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::theme::Theme;

#[derive(Clone, Copy)]
#[allow(unused)]
pub enum ProgressBarTheme {
    Capsule, // rounded capsule style
    Retro,   // blocky retro style
    Thin,    // thin line style
}

pub struct ProgressBarConfig {
    pub theme: ProgressBarTheme,
    pub color: Color,
    pub width: usize,
    pub ratio: f64, // 0.0 to 1.0
}

pub fn render_progress_bar(config: ProgressBarConfig, t: &Theme) -> Paragraph<'static> {
    let width = config.width;
    if width == 0 {
        return Paragraph::new(Line::default());
    }

    let filled_count = (config.ratio * width as f64).round() as usize;
    let empty_count = width.saturating_sub(filled_count);

    let (filled_str, empty_str) = match config.theme {
        ProgressBarTheme::Capsule => {
            // Simple fill — no caps that eat 2 chars (the old dead-zone bug)
            ("━".repeat(filled_count), "─".repeat(empty_count))
        }

        ProgressBarTheme::Retro => ("█".repeat(filled_count), "▒".repeat(empty_count)),

        ProgressBarTheme::Thin => ("━".repeat(filled_count), "─".repeat(empty_count)),
    };

    let line = Line::from(vec![
        Span::styled(filled_str, Style::default().fg(config.color)),
        Span::styled(empty_str, Style::default().fg(t.slider_track)),
    ]);

    Paragraph::new(line)
}
