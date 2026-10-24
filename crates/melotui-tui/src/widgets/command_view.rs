//! The command bar: what is being typed, the completions of it in a strip
//! beneath, and what the highlighted one does along the bottom border — all
//! inside the bar, so nothing else on screen is covered.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use melotui_core::AppState;
use melotui_core::autocomplete::{self, Suggestion};

/// Rows the bar takes: its borders, the input, and the completions.
pub const HEIGHT: u16 = 4;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let t = crate::theme::get_theme(state);
    let (prefix, cycle) = state.ui_autocomplete();
    let value = state.ui_command_buffer().value();
    let typed = prefix.unwrap_or(value);
    let suggestions = autocomplete::complete(typed, &state.completion_names());
    // The completion Tab has chosen, while cycling through them.
    let chosen = (prefix.is_some() && cycle > 0).then(|| cycle - 1);

    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border_inactive))
        .style(Style::default().bg(t.bg_input));
    if let Some(about) = about(typed, &suggestions, chosen) {
        block = block.title_bottom(Line::from(vec![
            Span::raw(" "),
            Span::styled(about.0, Style::default().fg(t.accent_primary)),
            Span::styled(
                format!("  {} ", about.1),
                Style::default().fg(t.fg_secondary),
            ),
        ]));
    }
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let input = Rect { height: 1, ..inner };
    frame.render_widget(
        Paragraph::new(input_line(state, prefix.is_none(), &suggestions)),
        input,
    );

    if inner.height > 1 {
        let strip = Rect {
            y: inner.y + 1,
            height: 1,
            ..inner
        };
        frame.render_widget(
            Paragraph::new(strip_line(&suggestions, chosen, strip.width, state)),
            strip,
        );
    }
}

/// The line being typed, with the cursor, and — when not cycling through
/// completions — the rest of the first completion dimmed after it, as Tab
/// would fill it in.
fn input_line<'a>(state: &'a AppState, show_ghost: bool, suggestions: &[Suggestion]) -> Line<'a> {
    let t = crate::theme::get_theme(state);
    let buffer = state.ui_command_buffer();
    let value = buffer.value();
    let cursor = buffer.cursor();
    let cursor_style = Style::default().fg(t.bg_input).bg(t.fg_primary);

    let mut spans = vec![
        Span::styled(
            ":",
            Style::default()
                .fg(t.accent_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ];
    if cursor == value.chars().count() {
        spans.push(Span::raw(value));
        let ghost = if show_ghost && !value.is_empty() {
            suggestions
                .first()
                .and_then(|s| s.text.strip_prefix(value))
                .unwrap_or("")
        } else {
            ""
        };
        let mut ghost = ghost.chars();
        match ghost.next() {
            Some(first) => {
                spans.push(Span::styled(first.to_string(), cursor_style));
                spans.push(Span::styled(
                    ghost.collect::<String>(),
                    Style::default().fg(t.fg_muted),
                ));
            }
            None => spans.push(Span::styled("█", Style::default().fg(t.fg_primary))),
        }
    } else {
        let start = value
            .char_indices()
            .nth(cursor)
            .map_or(value.len(), |(i, _)| i);
        let len = value[start..].chars().next().map_or(0, char::len_utf8);
        spans.push(Span::raw(&value[..start]));
        spans.push(Span::styled(&value[start..start + len], cursor_style));
        spans.push(Span::raw(&value[start + len..]));
    }
    Line::from(spans)
}

/// The completions side by side, as many as fit in `width`, scrolled so the
/// chosen one shows, with arrows where more lie beyond either edge.
fn strip_line<'a>(
    suggestions: &'a [Suggestion],
    chosen: Option<usize>,
    width: u16,
    state: &AppState,
) -> Line<'a> {
    let t = crate::theme::get_theme(state);
    if suggestions.is_empty() {
        return Line::default();
    }
    const GAP: usize = 2;
    let room = usize::from(width).saturating_sub(4); // the arrows
    let fits = |start: usize| {
        let mut used = 0;
        suggestions[start..]
            .iter()
            .take_while(|s| {
                used += s.label.chars().count() + GAP;
                used <= room + GAP
            })
            .count()
            .max(1)
    };
    let mut start = 0;
    if let Some(chosen) = chosen {
        while chosen >= start + fits(start) {
            start += 1;
        }
    }
    let shown = fits(start);

    let arrow = Style::default().fg(t.fg_muted);
    let mut spans = vec![Span::styled(if start > 0 { "‹ " } else { "  " }, arrow)];
    for (i, s) in suggestions.iter().enumerate().skip(start).take(shown) {
        let style = if Some(i) == chosen {
            Style::default()
                .fg(t.bg_input)
                .bg(t.accent_primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.fg_secondary)
        };
        spans.push(Span::styled(s.label.as_str(), style));
        spans.push(Span::raw(" ".repeat(GAP)));
    }
    if start + shown < suggestions.len() {
        spans.pop();
        spans.push(Span::styled(" ›", arrow));
    }
    Line::from(spans)
}

/// What to say about the command along the bottom: the completion chosen
/// with Tab, or the command typed so far, or the only one it could become —
/// as how it is typed, and what it does.
fn about(
    typed: &str,
    suggestions: &[Suggestion],
    chosen: Option<usize>,
) -> Option<(String, String)> {
    let describe = |s: &Suggestion| {
        let words: Vec<&str> = s.text.split_whitespace().collect();
        let syntax = format!(":{} {}", words.join(" "), s.usage);
        (syntax.trim_end().to_string(), s.help.to_string())
    };
    if let Some(chosen) = chosen.and_then(|i| suggestions.get(i)) {
        return Some(describe(chosen));
    }
    if let Some((syntax, help)) = autocomplete::describe(typed) {
        return Some((syntax, help.to_string()));
    }
    match suggestions {
        [only] => Some(describe(only)),
        _ if typed.trim().is_empty() => Some((
            "Tab".to_string(),
            "completes · Enter runs · Esc closes · :help lists every command".to_string(),
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use melotui_core::library::snapshot::Library;
    use melotui_core::message::ui::UIMsg;
    use melotui_core::{AppMsg, reducer};
    use ratatui::{Terminal, backend::TestBackend};
    use std::collections::HashMap;
    use tui_input::InputRequest;

    fn typing(text: &str) -> AppState {
        let library = Library::new(vec![], vec![], HashMap::new());
        let mut state = AppState::new(library, &melotui_core::config::AppConfig::default());
        reducer::reduce(&mut state, AppMsg::UI(UIMsg::EnterCommandMode));
        for c in text.chars() {
            reducer::reduce(
                &mut state,
                AppMsg::UI(UIMsg::CommandInput(InputRequest::InsertChar(c))),
            );
        }
        state
    }

    /// The screen, 80 × 10, with the bar drawn at its bottom.
    fn screen(state: &AppState) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(80, 10)).unwrap();
        let frame = terminal
            .draw(|f| {
                let area = f.area();
                let bar = Rect {
                    y: area.bottom() - HEIGHT,
                    height: HEIGHT,
                    ..area
                };
                draw(f, bar, state);
            })
            .unwrap();
        let buffer = frame.buffer.clone();
        (0..10)
            .map(|y| (0..80).map(|x| buffer[(x, y)].symbol()).collect::<String>())
            .collect()
    }

    #[test]
    fn the_completions_sit_in_a_strip_under_the_input_and_cover_nothing_else() {
        let screen = screen(&typing("lyrics "));
        assert!(
            screen[..6].iter().all(|row| row.trim().is_empty()),
            "drawn above the bar"
        );
        let strip = &screen[8];
        for name in ["show", "hide", "toggle", "zoom", "fetch", "search"] {
            assert!(strip.contains(name), "{name} missing from {strip:?}");
        }
    }

    #[test]
    fn tab_highlights_a_completion_and_the_border_says_what_it_does() {
        let mut state = typing("lyrics ");
        reducer::reduce(&mut state, AppMsg::UI(UIMsg::Autocomplete));
        reducer::reduce(&mut state, AppMsg::UI(UIMsg::Autocomplete));
        reducer::reduce(&mut state, AppMsg::UI(UIMsg::Autocomplete));
        reducer::reduce(&mut state, AppMsg::UI(UIMsg::Autocomplete));
        let screen = screen(&state);
        assert!(screen[7].contains(": lyrics zoom"), "{:?}", screen[7]);
        assert!(
            screen[9].contains(":lyrics zoom [panel|focus] <1-4>")
                && screen[9].contains("Lyrics size"),
            "{:?}",
            screen[9]
        );
    }

    #[test]
    fn a_long_list_scrolls_to_keep_the_chosen_one_in_view() {
        let mut state = typing("");
        for _ in 0..25 {
            reducer::reduce(&mut state, AppMsg::UI(UIMsg::Autocomplete));
        }
        let screen = screen(&state);
        let strip = &screen[8];
        assert!(
            strip.starts_with("│‹"),
            "no arrow for the commands scrolled past: {strip:?}"
        );
        let chosen = autocomplete::complete("", &state.completion_names())[24]
            .label
            .clone();
        assert!(strip.contains(&chosen), "{chosen} not in {strip:?}");
    }

    #[test]
    fn the_rest_of_the_first_completion_shows_in_the_input() {
        let screen = screen(&typing("pau"));
        assert!(screen[7].contains(": pause"), "{:?}", screen[7]);
        assert!(screen[9].contains(":pause") && screen[9].contains("Pause"));
    }

    #[test]
    fn a_command_with_nothing_to_complete_says_what_it_takes() {
        let screen = screen(&typing("vol "));
        assert!(screen[9].contains(":vol <0-150|+n|-n>"), "{:?}", screen[9]);
        assert!(screen[9].contains("Volume in percent"));
        assert!(
            screen[8].trim_matches(['│', ' ']).is_empty(),
            "nothing to complete"
        );
    }

    #[test]
    fn with_nothing_typed_it_says_how_to_use_it() {
        let screen = screen(&typing(""));
        assert!(
            screen[9].contains("Tab") && screen[9].contains("Enter runs"),
            "{:?}",
            screen[9]
        );
        assert!(screen[8].contains("play") && screen[8].contains("pause"));
    }
}
