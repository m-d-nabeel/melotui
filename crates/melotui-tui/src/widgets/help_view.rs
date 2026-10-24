use crate::keybinding::{KeyBinding, KeyContext};
use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding},
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
        .style(Style::default().bg(t.help_overlay_bg))
        .padding(Padding::uniform(1));

    let inner = block.inner(help_area);
    frame.render_widget(block, help_area);

    let (cols, rows) = if inner.width < 120 { (2, 2) } else { (4, 1) };

    let row_constraints: Vec<Constraint> = (0..rows).map(|_| Constraint::Fill(1)).collect();
    let row_layouts = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(inner);

    let mut sections = Vec::new();
    for row_area in row_layouts.iter() {
        let col_constraints: Vec<Constraint> = (0..cols).map(|_| Constraint::Fill(1)).collect();
        let col_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(col_constraints)
            .split(*row_area);
        sections.extend(col_layout.iter().copied());
    }

    // Render sections safely into the available slots
    if !sections.is_empty() {
        render_section(
            frame,
            sections[0],
            " Global ",
            registry.bindings.get(&KeyContext::Global),
            t,
        );
    }
    if sections.len() >= 2 {
        render_section(
            frame,
            sections[1],
            " Library ",
            registry.bindings.get(&KeyContext::Library),
            t,
        );
    }
    if sections.len() >= 3 {
        render_section(
            frame,
            sections[2],
            " Queue ",
            registry.bindings.get(&KeyContext::Queue),
            t,
        );
    }
    if sections.len() >= 4 {
        render_command_section(frame, sections[3], " Commands (: then Tab) ", t);
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
        .border_style(Style::default().fg(t.help_border))
        .style(Style::default().bg(t.help_overlay_bg))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let entries = melotui_core::Command::help_entries();
    let syntax_width = entries
        .iter()
        .map(|(syntax, _)| syntax.chars().count() as u16)
        .max()
        .unwrap_or(0)
        .min(inner.width / 2);
    let rows = entries.into_iter().map(|(syntax, desc)| {
        ratatui::widgets::Row::new(vec![
            ratatui::widgets::Cell::from(Span::styled(syntax, Style::default().fg(t.help_key))),
            ratatui::widgets::Cell::from(Span::styled(desc, Style::default().fg(t.help_text))),
        ])
    });

    let table = ratatui::widgets::Table::new(
        rows,
        [Constraint::Length(syntax_width), Constraint::Min(10)],
    );
    frame.render_widget(table, inner);
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
        .border_style(Style::default().fg(t.help_border))
        .style(Style::default().bg(t.help_overlay_bg))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // group bindings by description
    let mut grouped: std::collections::HashMap<&str, Vec<String>> =
        std::collections::HashMap::new();

    if let Some(bindings) = bindings {
        for binding in bindings {
            let key_str = format_key(binding);
            grouped
                .entry(binding.action.description())
                .or_default()
                .push(key_str);
        }
    }
    let mut entries: Vec<_> = grouped.into_iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));

    let rows: Vec<ratatui::widgets::Row> = entries
        .into_iter()
        .map(|(desc, keys)| {
            let key_display = keys.join(" | ");
            ratatui::widgets::Row::new(vec![
                ratatui::widgets::Cell::from(Span::styled(
                    key_display,
                    Style::default().fg(t.help_key),
                )),
                ratatui::widgets::Cell::from(Span::styled(desc, Style::default().fg(t.help_text))),
            ])
        })
        .collect();

    let table = ratatui::widgets::Table::new(rows, [Constraint::Length(15), Constraint::Min(10)]);
    frame.render_widget(table, inner);
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
            KeyCode::BackTab => {
                // BackTab inherently implies Shift in terminal, no prefix needed
            }
            _ => result.push_str("Shift+"),
        }
    }

    // add the key
    let key_str = match binding.key {
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(c) => {
            if binding.modifiers.contains(KeyModifiers::SHIFT) {
                format!("{}", c.to_uppercase())
            } else {
                format!("{}", c.to_lowercase())
            }
        }
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
