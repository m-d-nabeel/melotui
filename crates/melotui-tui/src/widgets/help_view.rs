use crate::keybinding::{KeyBinding, KeyContext};
use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
};

pub fn draw(
    frame: &mut Frame,
    area: Rect,
    state: &melotui_core::AppState,
    registry: &crate::keybinding::KeyRegistry,
) {
    // help screen area
    let help_area = centered_rect(90, 90, area);

    // clear bg
    frame.render_widget(Clear, help_area);

    let t = crate::theme::get_theme(state);

    // main block
    let block = Block::default()
        .title(
            Line::from(vec![
                Span::styled(
                    "  Keyboard Shortcuts ",
                    Style::default()
                        .fg(t.accent_primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "(press q or Esc to close)",
                    Style::default().fg(t.help_subtext),
                ),
            ])
            .alignment(Alignment::Center),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.help_border))
        .padding(Padding::uniform(1));

    let inner = block.inner(help_area);
    frame.render_widget(block, help_area);

    // Responsive column count based on width
    let col_count = if inner.width < 100 {
        2
    } else if inner.width < 160 {
        3
    } else {
        4
    };

    let constraints: Vec<Constraint> = (0..col_count).map(|_| Constraint::Fill(1)).collect();
    let sections = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(inner);

    // Render sections based on available columns
    let mut col = 0;
    if col < col_count {
        render_section(
            frame,
            sections[col],
            " Global ",
            registry.bindings.get(&KeyContext::Global),
            t,
        );
        col += 1;
    }
    if col < col_count {
        render_section(
            frame,
            sections[col],
            " Library ",
            registry.bindings.get(&KeyContext::Library),
            t,
        );
        col += 1;
    }
    if col < col_count {
        render_section(
            frame,
            sections[col],
            " Queue ",
            registry.bindings.get(&KeyContext::Queue),
            t,
        );
        col += 1;
    }
    if col < col_count {
        render_command_section(frame, sections[col], " Commands ", t);
    }
}

fn render_command_section(frame: &mut Frame, area: Rect, title: &str, t: &Theme) {
    let block = Block::default()
        .title(
            Line::from(Span::styled(
                title,
                Style::default()
                    .fg(t.help_header)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Center),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.help_overlay_bg))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    const COMMANDS: [(&str, &str); 14] = [
        (":q | :quit     ", "Quit App"),
        (":r | :rescan   ", "Rescan Lib"),
        (":dl <query>    ", "Download"),
        (":v <style>     ", "Visualizer"),
        (":noh           ", "Clear Search"),
        (":focus <mode>  ", "Focus Mode"),
        (":sidebar <n>   ", "Sidebar Width"),
        (":pl create <n> ", "New Playlist"),
        (":pl del <n>    ", "Del Playlist"),
        (":pl add <n>    ", "Add to Pl"),
        (":pl rm <n>     ", "Rem from Pl"),
        (":artist <q>    ", "Search Artist"),
        (":song <q>      ", "Search Song"),
        (":album <q>     ", "Search Album"),
    ];

    let mut lines = Vec::with_capacity(COMMANDS.len());
    for (syntax, desc) in COMMANDS {
        let line = Line::from(vec![
            Span::styled(syntax, Style::default().fg(t.help_key)),
            Span::raw(" "),
            Span::styled(desc, Style::default().fg(t.help_text)),
        ]);
        lines.push(line);
    }

    let paragraph = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, inner);
}

fn render_section(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    bindings: Option<&Vec<KeyBinding>>,
    t: &Theme,
) {
    let block = Block::default()
        .title(
            Line::from(Span::styled(
                title,
                Style::default()
                    .fg(t.help_header)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Center),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.help_overlay_bg))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // group bindings by description
    let mut grouped: std::collections::HashMap<&str, Vec<String>> =
        std::collections::HashMap::new();

    if let Some(bindings) = bindings {
        for binding in bindings {
            let key_str = format_key(binding);
            grouped.entry(binding.desc).or_default().push(key_str);
        }
    }

    // build lines
    let mut lines: Vec<Line> = vec![];

    // sort entries
    let mut entries: Vec<_> = grouped.into_iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));

    for (desc, keys) in entries {
        // join multiple keys
        let key_display = keys.join("  ");

        let line = Line::from(vec![
            Span::styled(
                format!("{:12}", key_display),
                Style::default().fg(t.help_key),
            ),
            Span::raw(" "),
            Span::styled(desc, Style::default().fg(t.help_text)),
        ]);
        lines.push(line);
    }

    let paragraph = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: true });

    frame.render_widget(paragraph, inner);
}

fn format_key(binding: &KeyBinding) -> String {
    use ratatui::crossterm::event::{KeyCode, KeyModifiers};

    let mut result = String::new();

    // add modifiers
    if binding.modifiers.contains(KeyModifiers::CONTROL) {
        result.push_str("Ctrl+");
    }
    if binding.modifiers.contains(KeyModifiers::ALT) {
        result.push_str("Alt+");
    }
    if binding.modifiers.contains(KeyModifiers::SHIFT) {
        // shift handling
        match binding.key {
            KeyCode::Char(c) if c.is_ascii_lowercase() => {
                // Will show uppercase below, no prefix needed
            }
            _ => result.push_str("Shift+"),
        }
    }

    // add the key
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
