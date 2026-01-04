use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

#[derive(Clone, Copy)]
#[allow(unused)]
pub enum ProgressBarTheme {
    Capsule, // ██ on ░░░
    Retro,   // ████ on ▒▒▒
    Thin,    // ━━━━ on ────
}

pub struct ProgressBarConfig {
    pub theme: ProgressBarTheme,
    pub color: Color,
    pub width: usize,
    pub ratio: f64, // 0.0 to 1.0
}

pub fn render_progress_bar(config: ProgressBarConfig) -> Paragraph<'static> {
    let filled_count = (config.ratio * config.width as f64).round() as usize;
    let empty_count = config.width.saturating_sub(filled_count);

    let (filled_str, empty_str) = match config.theme {
        ProgressBarTheme::Capsule => {
            let body = if filled_count > 0 {
                if filled_count == 1 {
                    "●".to_string()
                } else {
                    let body_len = filled_count.saturating_sub(2);
                    format!("{}", "█".repeat(body_len))
                }
            } else {
                "".to_string()
            };
            (body, "▒".repeat(empty_count))
        }

        ProgressBarTheme::Retro => ("█".repeat(filled_count), "▒".repeat(empty_count)),

        ProgressBarTheme::Thin => ("━".repeat(filled_count), "─".repeat(empty_count)),
    };

    let line = Line::from(vec![
        Span::styled(filled_str, Style::default().fg(config.color)),
        Span::styled(empty_str, Style::default().fg(Color::DarkGray)),
    ]);

    Paragraph::new(line)
}
