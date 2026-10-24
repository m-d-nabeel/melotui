use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Table},
};

use melotui_core::AppState;
use melotui_core::autocomplete::{self, Suggestion};

/// Completions shown at once; more scroll.
const MAX_SUGGESTIONS: usize = 10;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let t = crate::theme::get_theme(state);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border_inactive))
        .style(Style::default().bg(t.bg_input));

    let buffer = state.ui_command_buffer();
    let value = buffer.value();
    let cursor = buffer.cursor();

    let mut content_spans = vec![
        Span::styled(
            ":",
            Style::default()
                .fg(t.accent_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ];

    let cursor_style = Style::default().fg(t.bg_input).bg(t.fg_primary);
    if cursor == value.chars().count() {
        content_spans.push(Span::raw(value));
        // The rest of the first completion, dimmed, as Tab would fill it in.
        let ghost = ghost_text(state, value);
        let mut ghost = ghost.chars();
        match ghost.next() {
            Some(first) => {
                content_spans.push(Span::styled(first.to_string(), cursor_style));
                content_spans.push(Span::styled(
                    ghost.collect::<String>(),
                    Style::default().fg(t.fg_muted),
                ));
            }
            None => content_spans.push(Span::styled("█", Style::default().fg(t.fg_primary))),
        }
    } else {
        let start = value
            .char_indices()
            .nth(cursor)
            .map_or(value.len(), |(i, _)| i);
        let char_len = value[start..].chars().next().map_or(0, char::len_utf8);
        content_spans.push(Span::raw(&value[..start]));
        content_spans.push(Span::styled(&value[start..start + char_len], cursor_style));
        content_spans.push(Span::raw(&value[start + char_len..]));
    }

    let paragraph = Paragraph::new(Line::from(content_spans))
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

/// What the first completion of `value` adds to it, while not cycling
/// through completions.
fn ghost_text(state: &AppState, value: &str) -> String {
    if state.ui_autocomplete().0.is_some() || value.is_empty() {
        return String::new();
    }
    autocomplete::complete(value, &state.completion_names())
        .first()
        .and_then(|s| s.text.strip_prefix(value).map(str::to_string))
        .unwrap_or_default()
}

/// The completions of what is being typed, in a list just above the command
/// bar at the bottom of `area`, with the one Tab has chosen highlighted — or,
/// when there is nothing to complete, what the command being typed takes.
pub fn draw_suggestions(frame: &mut Frame, area: Rect, state: &AppState) {
    let t = crate::theme::get_theme(state);
    let (prefix, cycle) = state.ui_autocomplete();
    let typed = prefix
        .unwrap_or(state.ui_command_buffer().value())
        .to_string();
    let suggestions = autocomplete::complete(&typed, &state.completion_names());
    let chosen = (prefix.is_some() && cycle > 0).then(|| cycle - 1);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border_inactive))
        .style(Style::default().bg(t.bg_input));

    if suggestions.is_empty() {
        let Some((usage, help)) = autocomplete::describe(&typed) else {
            return;
        };
        let line = Line::from(vec![
            Span::styled(usage, Style::default().fg(t.accent_primary)),
            Span::styled(format!("  {help}"), Style::default().fg(t.fg_secondary)),
        ]);
        let width = (line.width() as u16 + 4).min(area.width);
        let popup = bottom_left(area, width, 3);
        frame.render_widget(Clear, popup);
        frame.render_widget(Paragraph::new(line).block(block), popup);
        return;
    }

    let visible = suggestions.len().min(MAX_SUGGESTIONS);
    let offset = chosen.map_or(0, |chosen| (chosen + 1).saturating_sub(visible));
    let label_width = column_width(&suggestions, |s| &s.label);
    let usage_width = column_width(&suggestions, |s| s.usage);
    let help_width = column_width(&suggestions, |s| s.help);
    let width = (label_width + usage_width + help_width + 2 * 2 + 4).min(area.width);
    let popup = bottom_left(area, width, visible as u16 + 2);

    let rows = suggestions
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible)
        .map(|(i, s)| {
            let row = Row::new(vec![
                Cell::from(Span::styled(
                    s.label.as_str(),
                    Style::default()
                        .fg(t.accent_primary)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(s.usage, Style::default().fg(t.fg_muted))),
                Cell::from(Span::styled(s.help, Style::default().fg(t.fg_secondary))),
            ]);
            if Some(i) == chosen {
                row.style(Style::default().bg(t.bg_selected))
            } else {
                row
            }
        });
    let title = if suggestions.len() > visible {
        format!(" {} of {} — Tab ", visible, suggestions.len())
    } else {
        " Tab ".to_string()
    };
    let table = Table::new(
        rows,
        [
            Constraint::Length(label_width),
            Constraint::Length(usage_width),
            Constraint::Min(0),
        ],
    )
    .column_spacing(2)
    .block(block.title(Span::styled(title, Style::default().fg(t.fg_muted))));
    frame.render_widget(Clear, popup);
    frame.render_widget(table, popup);
}

fn column_width<'a>(
    suggestions: &'a [Suggestion],
    text: impl Fn(&'a Suggestion) -> &'a str,
) -> u16 {
    suggestions
        .iter()
        .map(|s| text(s).chars().count() as u16)
        .max()
        .unwrap_or(0)
}

/// A `width` × `height` rectangle in the bottom left corner of `area`.
fn bottom_left(area: Rect, width: u16, height: u16) -> Rect {
    let height = height.min(area.height);
    Rect {
        x: area.x,
        y: area.bottom() - height,
        width: width.min(area.width),
        height,
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

    fn screen(state: &AppState) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(80, 16)).unwrap();
        let frame = terminal
            .draw(|f| {
                let area = f.area();
                let bar = Rect {
                    y: area.bottom() - 3,
                    height: 3,
                    ..area
                };
                let above = Rect {
                    height: area.height - 3,
                    ..area
                };
                draw(f, bar, state);
                draw_suggestions(f, above, state);
            })
            .unwrap();
        let buffer = frame.buffer.clone();
        (0..16)
            .map(|y| (0..80).map(|x| buffer[(x, y)].symbol()).collect::<String>())
            .collect()
    }

    #[test]
    fn typing_shows_the_completions_with_what_they_do() {
        let state = typing("lyrics ");
        let screen = screen(&state);
        let text = screen.join("\n");
        for line in [
            "zoom",
            "[panel|focus] <1-4>",
            "Lyrics size",
            "fetch",
            "search",
        ] {
            assert!(text.contains(line), "{line:?} missing from:\n{text}");
        }
    }

    #[test]
    fn the_rest_of_the_first_completion_shows_in_the_bar() {
        let state = typing("pau");
        let bar = &screen(&state)[14];
        assert!(bar.contains(": pause"), "{bar:?}");
    }

    #[test]
    fn with_nothing_to_complete_the_command_says_what_it_takes() {
        let state = typing("vol ");
        let text = screen(&state).join("\n");
        assert!(text.contains(":vol <0-150|+n|-n>"), "{text}");
        assert!(text.contains("Volume in percent"), "{text}");
    }
}
