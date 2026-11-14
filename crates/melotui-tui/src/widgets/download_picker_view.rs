//! The list a download is picked from: what a search found, songs and
//! videos apart, filling in as the results come.

use crate::keybinding::{KeyAction, KeyContext, KeyRegistry};
use crate::lyrics::bidi;
use crate::widgets::help_view::key_for;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use melotui_core::{
    AppState,
    state::{DownloadCandidate, DownloadSearch, DownloadSource},
};

/// The most rows of results shown at once; a search finds no more a tab.
const ROWS: u16 = 8;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, registry: &KeyRegistry) {
    let Some(search) = state.download_search() else {
        return;
    };
    let t = crate::theme::get_theme(state);

    // Tabs, a gap, the results, a gap, the keys — inside a border.
    let width = area.width.saturating_sub(4).min(96);
    let height = (ROWS + 6).min(area.height);
    let [_, across, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(width),
        Constraint::Fill(1),
    ])
    .areas(area);
    let [_, popup, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height),
        Constraint::Fill(1),
    ])
    .areas(across);

    crate::widgets::clear_for_popup(frame, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.accent_info))
        .title(Line::from(Span::styled(
            format!(" Download: {} ", bidi::for_terminal(&search.query)),
            Style::default()
                .fg(t.accent_info)
                .add_modifier(Modifier::BOLD),
        )))
        .style(Style::default().bg(t.bg_overlay));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let [tabs, _, rows, _, keys] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    // Tabs, with how many each has found, and whether more are coming.
    let tab = |source: DownloadSource, name: &str| {
        let style = if search.tab == source {
            Style::default()
                .fg(Color::Black)
                .bg(t.accent_info)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.fg_secondary)
        };
        Span::styled(
            format!(" {name} ({}) ", search.results(source).len()),
            style,
        )
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            tab(DownloadSource::Songs, "Songs"),
            Span::raw(" "),
            tab(DownloadSource::Videos, "Videos"),
        ])),
        tabs,
    );
    if search.searching {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "searching… ",
                Style::default().fg(t.accent_warning),
            ))
            .alignment(Alignment::Right),
            tabs,
        );
    }

    draw_rows(frame, rows, search, t);

    let key = |action| {
        key_for(registry, KeyContext::DownloadPicker, action).unwrap_or_else(|| "—".to_string())
    };
    let hint = |action, does: &'static str| {
        [
            Span::styled(
                format!("[{}]", key(action)),
                Style::default().fg(t.accent_info),
            ),
            Span::styled(format!(" {does}   "), Style::default().fg(t.fg_muted)),
        ]
    };
    let hints: Vec<Span> = [
        hint(KeyAction::DownloadPickerSubmit, "download"),
        hint(KeyAction::DownloadPickerSwitchTab, "songs / videos"),
        hint(KeyAction::DownloadPickerCancel, "close"),
    ]
    .into_iter()
    .flatten()
    .collect();
    frame.render_widget(
        Paragraph::new(Line::from(hints)).alignment(Alignment::Center),
        keys,
    );
}

/// The results of the tab shown, each with its length at the right, scrolled
/// to keep the one selected in sight.
fn draw_rows(frame: &mut Frame, area: Rect, search: &DownloadSearch, t: &crate::theme::Theme) {
    let shown = search.shown();
    if shown.is_empty() {
        let said = match (search.searching, search.tab) {
            (true, _) => "Searching…",
            (false, DownloadSource::Songs) => "No songs found",
            (false, DownloadSource::Videos) => "No videos found",
        };
        frame.render_widget(
            Paragraph::new(Span::styled(said, Style::default().fg(t.fg_muted)))
                .alignment(Alignment::Center),
            area,
        );
        return;
    }

    let [names, lengths] =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(9)]).areas(area);
    let visible = usize::from(area.height).max(1);
    let skip = search.row.saturating_sub(visible - 1);

    let (mut left, mut right) = (Vec::new(), Vec::new());
    for (i, found) in shown.iter().enumerate().skip(skip).take(visible) {
        let selected = i == search.row;
        let (title, rest) = if selected {
            let on = Style::default().fg(Color::Black).bg(t.accent_primary);
            (on.add_modifier(Modifier::BOLD), on)
        } else {
            (
                Style::default().fg(t.fg_primary),
                Style::default().fg(t.fg_muted),
            )
        };
        left.push(
            Line::from(vec![
                Span::styled(format!(" {}", bidi::for_terminal(&found.title)), title),
                Span::styled(credits(found), rest),
            ])
            .style(rest),
        );
        right.push(Line::styled(format!("{} ", length(found)), rest));
    }
    frame.render_widget(Paragraph::new(left), names);
    frame.render_widget(Paragraph::new(right).alignment(Alignment::Right), lengths);
}

/// Who a result is by and the album it is on, as they follow its title.
fn credits(found: &DownloadCandidate) -> String {
    [&found.by, &found.album]
        .into_iter()
        .flatten()
        .map(|credit| format!("  ·  {}", bidi::for_terminal(credit)))
        .collect()
}

/// How long a result is, `4:08` or `1:00:01`; nothing if it does not say.
fn length(found: &DownloadCandidate) -> String {
    match found.duration_secs {
        None => String::new(),
        Some(secs) if secs >= 3600 => {
            format!("{}:{:02}:{:02}", secs / 3600, secs % 3600 / 60, secs % 60)
        }
        Some(secs) => format!("{}:{:02}", secs / 60, secs % 60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use melotui_core::library::snapshot::Library;
    use melotui_core::message::download::DownloadMsg;
    use melotui_core::{AppMsg, reducer};
    use ratatui::{Terminal, backend::TestBackend};
    use std::collections::HashMap;

    fn found(source: DownloadSource, n: u64, title: &str) -> DownloadCandidate {
        DownloadCandidate {
            source,
            url: format!("https://www.youtube.com/watch?v={source:?}{n}"),
            title: title.into(),
            by: Some("Daft Punk".into()),
            album: (source == DownloadSource::Songs).then(|| "Discovery".into()),
            duration_secs: Some(3600 + 61 * n),
        }
    }

    fn screen(state: &AppState) -> String {
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal
            .draw(|f| draw(f, f.area(), state, &KeyRegistry::default()))
            .unwrap();
        let buf = terminal.backend().buffer();
        (0..24)
            .map(|y| (0..80).map(|x| buf[(x, y)].symbol()).collect::<String>() + "\n")
            .collect()
    }

    fn empty() -> AppState {
        AppState::new(
            Library::new(vec![], vec![], HashMap::new()),
            &melotui_core::config::AppConfig::default(),
        )
    }

    fn searching(query: &str) -> AppState {
        let mut state = empty();
        reducer::reduce(
            &mut state,
            AppMsg::Download(DownloadMsg::RequestDownload {
                query: query.into(),
            }),
        );
        state
    }

    #[test]
    fn a_search_is_seen_at_once_and_its_results_as_they_come() {
        let mut state = searching("one more time");
        let said = screen(&state);
        assert!(said.contains("Download: one more time"), "{said}");
        assert!(said.contains("Searching…"), "{said}");
        assert!(said.contains("Songs (0)"), "{said}");

        let id = state.download_search().unwrap().id;
        for candidate in [
            found(DownloadSource::Songs, 1, "One More Time"),
            found(DownloadSource::Videos, 1, "One More Time (Official Video)"),
        ] {
            reducer::reduce(
                &mut state,
                AppMsg::Download(DownloadMsg::DownloadSearchResult { id, candidate }),
            );
        }
        let said = screen(&state);
        assert!(
            said.contains("One More Time  ·  Daft Punk  ·  Discovery"),
            "{said}"
        );
        assert!(said.contains("1:01:01"), "{said}");
        assert!(said.contains("Songs (1)") && said.contains("Videos (1)"));
        assert!(!said.contains("Official Video"), "the other tab's: {said}");
        assert!(said.contains("[↵] download"), "{said}");
    }

    #[test]
    fn nothing_is_drawn_with_no_search_open() {
        let said = screen(&empty());
        assert!(said.trim().is_empty(), "{said}");
    }

    #[test]
    fn a_length_is_said_as_a_clock_says_it() {
        let of = |secs| {
            length(&DownloadCandidate {
                duration_secs: secs,
                ..found(DownloadSource::Songs, 0, "")
            })
        };
        assert_eq!(of(Some(248)), "4:08");
        assert_eq!(of(Some(3601)), "1:00:01");
        assert_eq!(of(None), "");
    }
}
