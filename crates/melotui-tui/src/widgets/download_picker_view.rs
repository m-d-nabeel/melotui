//! The list a download is picked from: what a search found, the songs and
//! then the rest of YouTube, filling in as the results come. It is drawn
//! where lyrics are, as the list a search for lyrics finds is.

use crate::lyrics::bidi;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use melotui_core::state::{DownloadCandidate, DownloadSearch, DownloadSource};

/// What the panel is called while the list is in it.
pub fn title(search: &DownloadSearch) -> String {
    let query = bidi::for_terminal(&search.query);
    if search.searching {
        format!(" Download: {query} · searching… ")
    } else {
        format!(" Download: {query} ")
    }
}

/// How a result says where it is from.
fn mark(source: DownloadSource) -> &'static str {
    match source {
        DownloadSource::Songs => "[M] ",
        DownloadSource::YouTube => "[Y] ",
    }
}

/// The results, one a row with its length at the right, under a line
/// saying what their marks mean; scrolled to keep the one selected in sight.
pub fn draw(frame: &mut Frame, area: Rect, search: &DownloadSearch, t: &crate::theme::Theme) {
    let muted = Style::default().fg(t.fg_muted);
    let [header, rows] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);

    let said = match (search.is_empty(), search.searching) {
        (true, true) => "  searching…",
        (true, false) => "  nothing found",
        (false, _) => "",
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "[M] YouTube Music · [Y] YouTube",
                Style::default()
                    .fg(t.accent_info)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(said, muted),
        ])),
        header,
    );

    let visible = usize::from(rows.height).max(1);
    let skip = (search.row + 1).saturating_sub(visible);
    let [names, lengths] =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(9)]).areas(rows);
    // Wide enough to carry the selected row's colour to the panel's edge.
    let filler = " ".repeat(usize::from(names.width));

    let (mut left, mut right) = (Vec::new(), Vec::new());
    let found = search.songs.iter().chain(&search.youtube);
    for (i, found) in found.enumerate().skip(skip).take(visible) {
        let (title, rest) = if i == search.row {
            let on = Style::default().fg(Color::Black).bg(t.accent_primary);
            (on.add_modifier(Modifier::BOLD), on)
        } else {
            (Style::default().fg(t.fg_primary), muted)
        };
        left.push(Line::from(vec![
            Span::styled(mark(found.source), rest),
            Span::styled(bidi::for_terminal(&found.title).into_owned(), title),
            Span::styled(credits(found), rest),
            Span::styled(filler.clone(), rest),
        ]));
        right.push(Line::styled(format!("{:>9}", length(found)), rest));
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
    use ratatui::{Terminal, backend::TestBackend};

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

    /// The list as it is drawn in a panel `height` rows high.
    fn screen(search: &DownloadSearch, height: u16) -> Vec<String> {
        let theme = crate::theme::Theme::default();
        let mut terminal = Terminal::new(TestBackend::new(80, height)).unwrap();
        terminal
            .draw(|f| draw(f, f.area(), search, &theme))
            .unwrap();
        let buf = terminal.backend().buffer();
        (0..height)
            .map(|y| (0..80).map(|x| buf[(x, y)].symbol()).collect())
            .collect()
    }

    #[test]
    fn a_search_says_it_is_searching_until_there_is_something_to_show() {
        let mut search = DownloadSearch {
            query: "one more time".into(),
            searching: true,
            ..Default::default()
        };
        assert_eq!(title(&search), " Download: one more time · searching… ");
        assert!(screen(&search, 5)[0].contains("searching…"));

        search.searching = false;
        assert_eq!(title(&search), " Download: one more time ");
        assert!(screen(&search, 5)[0].contains("nothing found"));
    }

    #[test]
    fn results_are_one_list_a_row_each_songs_first() {
        let search = DownloadSearch {
            songs: vec![found(DownloadSource::Songs, 1, "One More Time")],
            youtube: vec![found(
                DownloadSource::YouTube,
                2,
                "One More Time (Live 2007)",
            )],
            ..Default::default()
        };
        let said = screen(&search, 5);
        assert!(
            said[0].starts_with("[M] YouTube Music · [Y] YouTube"),
            "{said:?}"
        );
        assert!(
            said[1].starts_with("[M] One More Time  ·  Daft Punk  ·  Discovery"),
            "{said:?}"
        );
        assert!(said[1].trim_end().ends_with("1:01:01"), "{said:?}");
        assert!(
            said[2].starts_with("[Y] One More Time (Live 2007)  ·  Daft Punk"),
            "{said:?}"
        );
        assert!(said[2].trim_end().ends_with("1:02:02"), "{said:?}");
    }

    #[test]
    fn the_selected_result_stays_in_sight() {
        let search = DownloadSearch {
            songs: (0..8)
                .map(|n| found(DownloadSource::Songs, n, &format!("Song {n}")))
                .collect(),
            youtube: (0..8)
                .map(|n| found(DownloadSource::YouTube, n, &format!("Other {n}")))
                .collect(),
            row: 9,
            ..Default::default()
        };
        // A heading and four rows: the fourth is the selected one.
        let said = screen(&search, 5).join("\n");
        assert!(
            said.contains("Song 6") && said.contains("Other 1"),
            "{said}"
        );
        assert!(
            !said.contains("Song 5") && !said.contains("Other 2"),
            "{said}"
        );
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
