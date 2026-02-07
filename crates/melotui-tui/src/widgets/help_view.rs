use crate::input::InputHandler;
use melotui_core::keybinding::{KeyBinding, KeyContext};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
};

// Catppuccin-inspired colors
const OVERLAY: Color = Color::Rgb(69, 71, 90);
const TEXT: Color = Color::Rgb(205, 214, 244);
const SUBTEXT: Color = Color::Rgb(166, 173, 200);
const ACCENT: Color = Color::Rgb(137, 180, 250); // Blue
const KEY_FG: Color = Color::Rgb(243, 139, 168); // Pink
const HEADER_FG: Color = Color::Rgb(166, 227, 161); // Green
const BORDER: Color = Color::Rgb(88, 91, 112);

pub fn draw(frame: &mut Frame, area: Rect, _state: &melotui_core::AppState) {
    // Use most of the screen for help
    let help_area = centered_rect(85, 90, area);

    // Clear background
    frame.render_widget(Clear, help_area);

    // Main block with custom border
    let block = Block::default()
        .title(
            Line::from(vec![
                Span::styled(
                    "  Keyboard Shortcuts ",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled("(press q or Esc to close)", Style::default().fg(SUBTEXT)),
            ])
            .alignment(Alignment::Center),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .padding(Padding::uniform(1));

    let inner = block.inner(help_area);
    frame.render_widget(block, help_area);

    // Split into sections
    let sections = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(inner);

    // Get bindings by context using references (no cloning)
    let registry = InputHandler::get_registry();

    render_section(
        frame,
        sections[0],
        " Global ",
        registry.bindings.get(&KeyContext::Global),
    );
    render_section(
        frame,
        sections[1],
        " Library ",
        registry.bindings.get(&KeyContext::Library),
    );
    render_section(
        frame,
        sections[2],
        " Queue ",
        registry.bindings.get(&KeyContext::Queue),
    );
}

fn render_section(frame: &mut Frame, area: Rect, title: &str, bindings: Option<&Vec<KeyBinding>>) {
    let block = Block::default()
        .title(
            Line::from(Span::styled(
                title,
                Style::default().fg(HEADER_FG).add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Center),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(OVERLAY))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Group bindings by description to combine multiple keys
    let mut grouped: std::collections::HashMap<&str, Vec<String>> =
        std::collections::HashMap::new();

    if let Some(bindings) = bindings {
        for binding in bindings {
            let key_str = format_key(binding);
            grouped.entry(binding.desc).or_default().push(key_str);
        }
    }

    // Build lines
    let mut lines: Vec<Line> = vec![];

    // Sort by description for consistent order
    let mut entries: Vec<_> = grouped.into_iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));

    for (desc, keys) in entries {
        // Join multiple keys with " or "
        let key_display = keys.join("  ");

        let line = Line::from(vec![
            Span::styled(format!("{:12}", key_display), Style::default().fg(KEY_FG)),
            Span::raw(" "),
            Span::styled(desc, Style::default().fg(TEXT)),
        ]);
        lines.push(line);
    }

    let paragraph = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: true });

    frame.render_widget(paragraph, inner);
}

fn format_key(binding: &melotui_core::keybinding::KeyBinding) -> String {
    use ratatui::crossterm::event::{KeyCode, KeyModifiers};

    let mut result = String::new();

    // Add modifier prefixes
    if binding.modifiers.contains(KeyModifiers::CONTROL) {
        result.push_str("Ctrl+");
    }
    if binding.modifiers.contains(KeyModifiers::ALT) {
        result.push_str("Alt+");
    }
    if binding.modifiers.contains(KeyModifiers::SHIFT) {
        // For characters, show uppercase; for special keys, show Shift+ prefix
        match binding.key {
            KeyCode::Char(c) if c.is_ascii_lowercase() => {
                // Will show uppercase below, no prefix needed
            }
            _ => result.push_str("Shift+"),
        }
    }

    // Add the key itself
    let key_str = match binding.key {
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(c) => format!("{}", c.to_uppercase()),
        KeyCode::Up => "↑".to_string(),
        KeyCode::Down => "↓".to_string(),
        KeyCode::Left => "←".to_string(),
        KeyCode::Right => "→".to_string(),
        KeyCode::Enter => "↵".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::BackTab => "Shift+Tab".to_string(),
        KeyCode::Backspace => "⌫".to_string(),
        KeyCode::Delete => "Del".to_string(),
        KeyCode::F(n) => format!("F{}", n),
        _ => format!("{:?}", binding.key),
    };

    result.push_str(&key_str);
    result
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
