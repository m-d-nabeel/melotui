//! The command bar: what is being typed, the completions of it in a strip
//! beneath, and what the highlighted one does along the bottom border — all
//! inside the bar, so nothing else on screen is covered.
//!
//! Completions are drawn as virtual text, dim and grey as shell
//! autosuggestions and editors' inline hints are, so they read as offers
//! rather than as part of what was typed; only the one Tab has chosen takes
//! the accent colour.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use melotui_core::AppState;
use melotui_core::autocomplete::{self, Suggestion};

/// What the completions are for — what was typed before Tab started cycling,
/// or what is typed — and the completions: none until something is typed or
/// Tab is pressed, so an empty bar stays empty.
fn suggestions(state: &AppState) -> (String, Vec<Suggestion>) {
    let (prefix, _) = state.ui_autocomplete();
    let typed = prefix
        .unwrap_or(state.ui_command_buffer().value())
        .to_string();
    if typed.is_empty() && prefix.is_none() {
        return (typed, Vec::new());
    }
    let suggestions = autocomplete::complete(&typed, &state.completion_names());
    (typed, suggestions)
}

/// Rows the bar takes: its borders and the input, and a row for the
/// completions when there are any.
pub fn height(state: &AppState) -> u16 {
    if suggestions(state).1.is_empty() {
        3
    } else {
        4
    }
}

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let t = crate::theme::get_theme(state);
    let (prefix, cycle) = state.ui_autocomplete();
    let (typed, suggestions) = suggestions(state);
    let typed = typed.as_str();
    // The completion Tab has chosen, while cycling through them.
    let chosen = (prefix.is_some() && cycle > 0).then(|| cycle - 1);

    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border_inactive))
        .style(Style::default().bg(t.bg_input));
    // What the chosen or typed command still takes and what it does, as
    // virtual text like the completions: the command itself is in the input
    // already.
    if let Some((usage, help)) = about(typed, &suggestions, chosen) {
        let text = if usage.is_empty() {
            format!(" {help} ")
        } else {
            format!(" {usage} · {help} ")
        };
        block = block.title_bottom(Span::styled(text, Style::default().fg(t.fg_muted)));
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
                .fg(t.accent_primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.fg_muted)
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
/// as what it still takes, and what it does.
fn about(
    typed: &str,
    suggestions: &[Suggestion],
    chosen: Option<usize>,
) -> Option<(&'static str, &'static str)> {
    if let Some(chosen) = chosen.and_then(|i| suggestions.get(i)) {
        return Some((chosen.usage, chosen.help));
    }
    if let Some(described) = autocomplete::describe(typed) {
        return Some(described);
    }
    match suggestions {
        [only] => Some((only.usage, only.help)),
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

    fn screen_of(state: &AppState) -> Vec<String> {
        screen(state)
    }

    /// The screen, 80 × 10, with the bar drawn at its bottom.
    fn screen(state: &AppState) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(80, 10)).unwrap();
        let frame = terminal
            .draw(|f| {
                let area = f.area();
                let height = height(state);
                let bar = Rect {
                    y: area.bottom() - height,
                    height,
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
            screen[9].contains(" [panel|focus] <1-4> · Lyrics size") && !screen[9].contains(':'),
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
        assert!(screen[9].contains(" Pause "), "{:?}", screen[9]);
    }

    #[test]
    fn a_command_with_nothing_to_complete_says_what_it_takes() {
        let state = typing("vol ");
        assert_eq!(
            height(&state),
            3,
            "no row for completions there are none of"
        );
        let screen = screen(&state);
        assert!(screen[8].contains(": vol"), "{:?}", screen[8]);
        assert!(
            screen[9].contains(" <0-150|+n|-n> · Volume in percent "),
            "{:?}",
            screen[9]
        );
    }

    #[test]
    fn with_nothing_typed_the_bar_is_just_the_input() {
        let state = typing("");
        assert_eq!(height(&state), 3);
        let screen = screen(&state);
        assert!(screen[8].contains(": █"), "{:?}", screen[8]);
        assert!(
            screen[9].trim_matches(['╰', '─', '╯']).is_empty(),
            "{:?}",
            screen[9]
        );
        // Tab still offers every command.
        let mut tabbed = state;
        reducer::reduce(&mut tabbed, AppMsg::UI(UIMsg::Autocomplete));
        assert_eq!(height(&tabbed), 4);
        assert!(screen_of(&tabbed)[8].contains("pause"));
    }

    #[test]
    fn completions_are_dim_like_virtual_text_and_the_chosen_one_is_accented() {
        let mut state = typing("lyrics ");
        let t = crate::theme::get_theme(&state);
        let (muted, accent) = (t.fg_muted, t.accent_primary);
        let colour = |state: &AppState, word: &str| {
            let mut terminal = Terminal::new(TestBackend::new(80, 4)).unwrap();
            let frame = terminal.draw(|f| draw(f, f.area(), state)).unwrap();
            let buffer = frame.buffer.clone();
            let row: Vec<String> = (0..80)
                .map(|x| buffer[(x, 2)].symbol().to_string())
                .collect();
            let x = row.concat().find(word).unwrap();
            // Every cell in the strip is one character wide.
            let column = row.concat()[..x].chars().count() as u16;
            buffer[(column, 2)].fg
        };
        assert_eq!(colour(&state, "zoom"), muted);
        for _ in 0..4 {
            reducer::reduce(&mut state, AppMsg::UI(UIMsg::Autocomplete));
        }
        assert_eq!(colour(&state, "zoom"), accent);
        assert_eq!(colour(&state, "show"), muted);
    }

    #[test]
    fn the_description_is_dim_like_the_completions() {
        let state = typing("vol ");
        let t = crate::theme::get_theme(&state);
        let mut terminal = Terminal::new(TestBackend::new(80, 3)).unwrap();
        let frame = terminal.draw(|f| draw(f, f.area(), &state)).unwrap();
        let buffer = frame.buffer.clone();
        let row: String = (0..80).map(|x| buffer[(x, 2)].symbol()).collect();
        let x = row.find("Volume").unwrap();
        let column = row[..x].chars().count() as u16;
        assert_eq!(buffer[(column, 2)].fg, t.fg_muted);
        assert_eq!(buffer[(column - 4, 2)].fg, t.fg_muted, "the usage too");
    }
}
