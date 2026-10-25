use crate::keybinding::{KeyBinding, KeyContext};
use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use unicode_width::UnicodeWidthStr;

/// One section of the help: a title, and what each key or command does.
struct Section {
    title: &'static str,
    rows: Vec<(String, String)>,
}

/// The help: each section in a box of its own, in one column or, on a wide
/// screen, two side by side; each description whole, wrapped under itself
/// if need be; no larger than it needs to be, and scrolled when it does not
/// all fit. How far it can scroll is left in `scroll_max` for the keys that
/// scroll it.
pub fn draw(
    frame: &mut Frame,
    area: Rect,
    state: &melotui_core::AppState,
    registry: &crate::keybinding::KeyRegistry,
    scroll_max: &std::cell::Cell<u16>,
) {
    let t = crate::theme::get_theme(state);
    let sections = [
        key_section("Global", registry.bindings.get(&KeyContext::Global)),
        key_section("Library", registry.bindings.get(&KeyContext::Library)),
        key_section("Queue", registry.bindings.get(&KeyContext::Queue)),
        key_section(
            "Mixer  ( Tab from the queue )",
            registry.bindings.get(&KeyContext::Mixer),
        ),
        Section {
            title: "Commands  ( : then Tab )",
            rows: melotui_core::Command::help_entries(),
        },
    ];

    // Two columns where they fit side by side at the width their sections
    // need, one otherwise; boxes no wider than what is in them.
    let gap = 2;
    // The frame's border and a space either side inside it.
    let chrome_x = 4;
    let room = area.width.saturating_sub(4);
    let single = natural_width(&sections)
        .min(74)
        .min(room.saturating_sub(chrome_x));
    let pair = {
        let heights: Vec<usize> = sections
            .iter()
            .map(|section| boxed(section, single, t).len())
            .collect();
        let split = balanced_split(&heights);
        let (left, right) = sections.split_at(split);
        (split, natural_width(left), natural_width(right).min(74))
    };
    // The second column may narrow, wrapping more, down to `NARROWEST`.
    const NARROWEST: u16 = 50;
    let right_room = room.saturating_sub(pair.1 + gap + chrome_x);
    let (columns, widths): (u16, Vec<u16>) = if right_room >= NARROWEST && sections.len() > 1 {
        (2, vec![pair.1, pair.2.min(right_room)])
    } else {
        (1, vec![single.max(20)])
    };
    let width = widths.iter().sum::<u16>() + gap * (columns - 1) + chrome_x;
    let column_lines: Vec<Vec<Line>> = if columns == 2 {
        let (left, right) = sections.split_at(pair.0);
        vec![stack(left, widths[0], t), stack(right, widths[1], t)]
    } else {
        vec![stack(&sections, widths[0], t)]
    };
    let total = column_lines.iter().map(Vec::len).max().unwrap_or(0) as u16;

    // As tall as the help is, up to four fifths of the screen.
    let height = (total + 2).min(area.height * 4 / 5).max(5).min(area.height);
    let help_area = Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    };
    let inner = Rect {
        x: help_area.x + 2,
        y: help_area.y + 1,
        width: help_area.width.saturating_sub(chrome_x),
        height: help_area.height.saturating_sub(2),
    };
    let max = total.saturating_sub(inner.height);
    scroll_max.set(max);
    let scroll = state.ui_help_scroll().min(max);

    crate::widgets::clear_for_popup(frame, help_area);
    let hint = Style::default().fg(t.help_subtext);
    let mut block = Block::default()
        .title(
            Line::from(Span::styled(
                " Keyboard Shortcuts ",
                Style::default()
                    .fg(t.accent_primary)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Center),
        )
        .title_bottom(
            Line::from(Span::styled(
                if max > 0 {
                    " ↑↓ j k scroll · g G ends · q close "
                } else {
                    " q close "
                },
                hint,
            ))
            .alignment(Alignment::Left),
        )
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(t.help_border))
        .style(Style::default().bg(t.help_overlay_bg));
    if max > 0 {
        block = block.title_bottom(
            Line::from(Span::styled(
                format!(
                    " {}–{} of {} ",
                    scroll + 1,
                    (scroll + inner.height).min(total),
                    total
                ),
                hint,
            ))
            .alignment(Alignment::Right),
        );
    }
    frame.render_widget(block, help_area);

    let areas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            widths
                .iter()
                .enumerate()
                .flat_map(|(c, &w)| {
                    let mut parts = vec![Constraint::Length(w)];
                    if c + 1 < widths.len() {
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
                .begin_symbol(None)
                .end_symbol(None)
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
/// same thing together, in the order they are defined: by kind, not by name.
fn key_section(title: &'static str, bindings: Option<&Vec<KeyBinding>>) -> Section {
    let mut rows: Vec<(&str, Vec<String>)> = Vec::new();
    for binding in bindings.into_iter().flatten() {
        let desc = binding.action.description();
        match rows.iter_mut().find(|(d, _)| *d == desc) {
            Some((_, keys)) => keys.push(format_key(binding)),
            None => rows.push((desc, vec![format_key(binding)])),
        }
    }
    Section {
        title,
        rows: rows
            .into_iter()
            .map(|(desc, keys)| (keys.join(" | "), desc.to_string()))
            .collect(),
    }
}

/// A section as a box `width` wide, its title in the top border: each key,
/// or command with what it takes dimmed, and what it does, the description
/// wrapped under itself where it runs long.
fn boxed(section: &Section, width: u16, t: &Theme) -> Vec<Line<'static>> {
    let width = width as usize;
    let border = Style::default().fg(t.help_border);
    let title_width = section.title.width();
    let mut lines = vec![Line::from(vec![
        Span::styled("╭─ ", border),
        Span::styled(
            section.title.to_string(),
            Style::default()
                .fg(t.help_header)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {}╮", "─".repeat(width.saturating_sub(title_width + 5))),
            border,
        ),
    ])];

    // Inside the box: a border and a space either side.
    let room = width.saturating_sub(4);
    let key_width = section
        .rows
        .iter()
        .map(|(key, _)| key.width())
        .max()
        .unwrap_or(0)
        .min(22)
        .min(room * 2 / 5)
        .max(4);
    let desc_width = room.saturating_sub(key_width + 2).max(8);
    let row = |spans: Vec<Span<'static>>, used: usize| {
        let mut all = vec![Span::styled("│ ", border)];
        all.extend(spans);
        all.push(Span::raw(" ".repeat(room.saturating_sub(used))));
        all.push(Span::styled(" │", border));
        Line::from(all)
    };
    for (key, desc) in &section.rows {
        // A command's name, then what it takes, dimmer.
        let (name, args) = match key.split_once(' ') {
            Some((name, args)) if key.starts_with(':') => (name.to_string(), format!(" {args}")),
            _ => (key.clone(), String::new()),
        };
        let key_spans = vec![
            Span::styled(name.clone(), Style::default().fg(t.help_key)),
            Span::styled(args.clone(), Style::default().fg(t.help_subtext)),
        ];
        let key_fits = key.width() <= key_width;
        if !key_fits {
            lines.push(row(key_spans.clone(), key.width().min(room)));
        }
        for (n, part) in wrap(desc, desc_width).into_iter().enumerate() {
            let part_width = part.width();
            let mut spans = if n == 0 && key_fits {
                let mut spans = key_spans.clone();
                spans.push(Span::raw(" ".repeat(key_width - key.width() + 2)));
                spans
            } else {
                vec![Span::raw(" ".repeat(key_width + 2))]
            };
            spans.push(Span::styled(part, Style::default().fg(t.help_text)));
            lines.push(row(spans, key_width + 2 + part_width));
        }
    }
    lines.push(Line::from(Span::styled(
        format!("╰{}╯", "─".repeat(width.saturating_sub(2))),
        border,
    )));
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

/// `sections` boxed `width` wide, one under the other, a line between.
fn stack(sections: &[Section], width: u16, t: &Theme) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for section in sections {
        if !lines.is_empty() {
            lines.push(Line::default());
        }
        lines.extend(boxed(section, width, t));
    }
    lines
}

/// Where to part sections of these heights into two columns, in order, for
/// the columns to come out most nearly the same height.
fn balanced_split(heights: &[usize]) -> usize {
    let total: usize = heights.iter().sum();
    (1..heights.len())
        .min_by_key(|&k| {
            let left: usize = heights[..k].iter().sum();
            left.max(total - left)
        })
        .unwrap_or(1)
}

/// How wide a box has to be for the widest of `sections` to fit unwrapped.
fn natural_width(sections: &[Section]) -> u16 {
    sections
        .iter()
        .map(|section| {
            let key = section
                .rows
                .iter()
                .map(|(k, _)| k.width())
                .max()
                .unwrap_or(0)
                .min(22);
            let desc = section
                .rows
                .iter()
                .map(|(_, d)| d.width())
                .max()
                .unwrap_or(0);
            (4 + key + 2 + desc).max(section.title.width() + 6) as u16
        })
        .max()
        .unwrap_or(20)
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
        assert!(
            text.contains(" 1–") && text.contains("j k scroll"),
            "{text}"
        );
        let (_, max) = drawn(300, 120);
        assert_eq!(max, 0, "everything shows at once");
    }

    #[test]
    fn two_columns_where_they_fit_and_each_section_boxed() {
        let (text, _) = drawn(115, 46);
        let first = text.lines().find(|l| l.contains("╭─ Global")).unwrap();
        assert!(first.contains("╭─ Queue"), "side by side: {first}");
        let (text, _) = drawn(70, 40);
        let first = text.lines().find(|l| l.contains("╭─ Global")).unwrap();
        assert!(
            !first.contains("╭─ Queue"),
            "one column when narrow: {first}"
        );
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
