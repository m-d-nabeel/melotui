use crate::keybinding::{KeyBinding, KeyContext};
use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};
use unicode_width::UnicodeWidthStr;

/// One section of the help: a title, and what each key or command does.
struct Section {
    title: &'static str,
    rows: Vec<(String, String)>,
}

/// The help: every key and command, in one or two columns as the width
/// allows, each description whole, wrapped if need be, and scrolled when it
/// does not all fit. How far it can scroll is left in `scroll_max` for the
/// keys that scroll it.
pub fn draw(
    frame: &mut Frame,
    area: Rect,
    state: &melotui_core::AppState,
    registry: &crate::keybinding::KeyRegistry,
    scroll_max: &std::cell::Cell<u16>,
) {
    let help_area = centered_rect(90, 90, area);
    frame.render_widget(Clear, help_area);
    let t = crate::theme::get_theme(state);

    let sections = [
        key_section(" Global ", registry.bindings.get(&KeyContext::Global)),
        key_section(" Library ", registry.bindings.get(&KeyContext::Library)),
        key_section(" Queue ", registry.bindings.get(&KeyContext::Queue)),
        Section {
            title: " Commands (: then Tab) ",
            rows: melotui_core::Command::help_entries(),
        },
    ];

    // Laid out before the frame's title can say where in it the view is.
    let inner_of = |area: Rect| {
        Block::default()
            .borders(Borders::ALL)
            .padding(Padding::new(2, 2, 1, 0))
            .inner(area)
    };
    let inner = inner_of(help_area);
    let columns: u16 = if inner.width >= 110 { 2 } else { 1 };
    let gap = 4;
    let column_width = (inner.width.saturating_sub(gap * (columns - 1)) / columns).max(10);
    let rendered: Vec<Vec<Line>> = sections
        .iter()
        .map(|section| section_lines(section, column_width, t))
        .collect();
    let column_lines = split_columns(rendered, columns as usize);
    let total = column_lines.iter().map(Vec::len).max().unwrap_or(0) as u16;
    let max = total.saturating_sub(inner.height);
    scroll_max.set(max);
    let scroll = state.ui_help_scroll().min(max);

    let position = if max == 0 {
        String::new()
    } else {
        format!(
            " · lines {}–{} of {}",
            scroll + 1,
            (scroll + inner.height).min(total),
            total
        )
    };
    let block = Block::default()
        .title(
            Line::from(vec![
                Span::styled(
                    " Keyboard Shortcuts ",
                    Style::default()
                        .fg(t.accent_primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("(j/k scroll · q or Esc to close{position}) "),
                    Style::default().fg(t.help_subtext),
                ),
            ])
            .alignment(Alignment::Center),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.help_border))
        .style(Style::default().bg(t.help_overlay_bg))
        .padding(Padding::new(2, 2, 1, 0));
    frame.render_widget(block, help_area);

    let areas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            (0..columns)
                .flat_map(|c| {
                    let mut parts = vec![Constraint::Length(column_width)];
                    if c + 1 < columns {
                        parts.push(Constraint::Length(gap));
                    }
                    parts
                })
                .collect::<Vec<_>>(),
        )
        .split(inner);
    for (lines, area) in column_lines.into_iter().zip(areas.iter().step_by(2)) {
        frame.render_widget(Paragraph::new(lines).scroll((scroll, 0)), *area);
    }

    if max > 0 {
        let mut bar = ScrollbarState::new(max as usize + 1).position(scroll as usize);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .style(Style::default().fg(t.help_border)),
            help_area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut bar,
        );
    }
}

/// The bindings of one context, one row per thing they do, keys that do the
/// same thing together.
fn key_section(title: &'static str, bindings: Option<&Vec<KeyBinding>>) -> Section {
    let mut grouped: std::collections::BTreeMap<&str, Vec<String>> = Default::default();
    for binding in bindings.into_iter().flatten() {
        grouped
            .entry(binding.action.description())
            .or_default()
            .push(format_key(binding));
    }
    Section {
        title,
        rows: grouped
            .into_iter()
            .map(|(desc, keys)| (keys.join(" | "), desc.to_string()))
            .collect(),
    }
}

/// A section as lines `width` wide: its title, then each key and what it
/// does, the description wrapped under itself where it runs long.
fn section_lines(section: &Section, width: u16, t: &Theme) -> Vec<Line<'static>> {
    let width = width as usize;
    let title_width = section.title.width();
    let mut lines = vec![Line::from(vec![
        Span::styled(
            section.title.trim().to_string(),
            Style::default()
                .fg(t.help_header)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {}", "─".repeat(width.saturating_sub(title_width))),
            Style::default().fg(t.help_border),
        ),
    ])];
    let key_width = section
        .rows
        .iter()
        .map(|(key, _)| key.width())
        .max()
        .unwrap_or(0)
        .min(width * 2 / 5)
        .max(4);
    let desc_width = width.saturating_sub(key_width + 2).max(8);
    for (key, desc) in &section.rows {
        let wrapped = wrap(desc, desc_width);
        let key_fits = key.width() <= key_width;
        if !key_fits {
            lines.push(Line::from(Span::styled(
                key.clone(),
                Style::default().fg(t.help_key),
            )));
        }
        for (n, part) in wrapped.into_iter().enumerate() {
            let lead = if n == 0 && key_fits {
                format!("{key}{}", " ".repeat(key_width - key.width() + 2))
            } else {
                " ".repeat(key_width + 2)
            };
            lines.push(Line::from(vec![
                Span::styled(lead, Style::default().fg(t.help_key)),
                Span::styled(part, Style::default().fg(t.help_text)),
            ]));
        }
    }
    lines.push(Line::default());
    lines
}

/// `text` in lines no wider than `width`, broken between words where it can.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let needed = if line.is_empty() {
            word.width()
        } else {
            line.width() + 1 + word.width()
        };
        if needed > width && !line.is_empty() {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
        // A word longer than the line is cut where it must be.
        while line.width() > width {
            let cut = line
                .char_indices()
                .scan(0, |w, (i, c)| {
                    *w += unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
                    Some((i, *w))
                })
                .find(|&(_, w)| w > width)
                .map_or(line.len(), |(i, _)| i);
            let rest = line.split_off(cut);
            lines.push(std::mem::replace(&mut line, rest));
        }
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

/// The sections' lines in `columns` columns, in order, split where the
/// columns come out most nearly the same height.
fn split_columns(sections: Vec<Vec<Line<'static>>>, columns: usize) -> Vec<Vec<Line<'static>>> {
    if columns < 2 || sections.len() < 2 {
        return vec![sections.into_iter().flatten().collect()];
    }
    let heights: Vec<usize> = sections.iter().map(Vec::len).collect();
    let total: usize = heights.iter().sum();
    let split = (1..sections.len())
        .min_by_key(|&k| {
            let left: usize = heights[..k].iter().sum();
            left.max(total - left)
        })
        .unwrap_or(1);
    let mut sections = sections.into_iter();
    let left = sections.by_ref().take(split).flatten().collect();
    let right = sections.flatten().collect();
    vec![left, right]
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

#[cfg(test)]
mod tests {
    use super::*;

    fn drawn(width: u16, height: u16) -> (String, u16) {
        let state = melotui_core::AppState::new(
            melotui_core::library::snapshot::Library::new(vec![], vec![], Default::default()),
            &melotui_core::config::AppConfig::default(),
        );
        let registry = crate::keybinding::KeyRegistry::default();
        let max = std::cell::Cell::new(0);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|f| draw(f, f.area(), &state, &registry, &max))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let text = (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        (text, max.get())
    }

    #[test]
    fn descriptions_are_drawn_whole() {
        let (text, _) = drawn(190, 50);
        assert!(text.contains("Resume, or play the selected track"));
        assert!(text.contains("Shuffle on or off, or toggle it"));
    }

    #[test]
    fn what_does_not_fit_is_scrolled_to_and_all_of_it_when_it_fits() {
        let (text, max) = drawn(100, 30);
        assert!(max > 0);
        assert!(text.contains(&format!("of {}", max + 24)), "{text}");
        let (_, max) = drawn(300, 120);
        assert_eq!(max, 0, "everything shows at once");
    }

    #[test]
    fn long_text_wraps_between_words() {
        let lines = wrap("Effects: bass, treble, reverb, echo, pan, lowpass", 20);
        assert!(lines.iter().all(|l| l.width() <= 20), "{lines:?}");
        assert_eq!(
            lines.join(" "),
            "Effects: bass, treble, reverb, echo, pan, lowpass"
        );
        assert_eq!(wrap("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
    }
}
