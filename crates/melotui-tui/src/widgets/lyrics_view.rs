use melotui_core::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let t = crate::theme::get_theme(state);

    // Minimalist Sound Control Panel Lyrics
    let block = Block::default()
        .title(" Lyrics ")
        .title_style(Style::default().add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border_inactive))
        .padding(Padding::horizontal(2));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let active_index = state.active_lyrics_line_index();

    let lines: Vec<Line> = match state.lyrics_data() {
        melotui_core::state::lyrics::LyricsData::Synced(lyrics) => {
            let mut l = Vec::new();

            let source_lines = lyrics.get_timed_lines();
            for (i, (_time, text)) in source_lines.iter().enumerate() {
                if Some(i) == active_index {
                    // Active line: Bold and bright accent color
                    l.push(Line::from(vec![Span::styled(
                        text.to_string(),
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(t.accent_primary),
                    )]));
                } else if Some(i) < active_index {
                    // Passed lines: Dimmed grey, but BOLD
                    l.push(Line::from(vec![Span::styled(
                        text.to_string(),
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(ratatui::style::Color::DarkGray),
                    )]));
                } else {
                    // Future lines: Slightly dimmed, but BOLD
                    l.push(Line::from(vec![Span::styled(
                        text.to_string(),
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(ratatui::style::Color::Gray),
                    )]));
                }
            }
            l
        }
        melotui_core::state::lyrics::LyricsData::Unsynced(text) => {
            let mut l = Vec::new();
            for line in text.lines() {
                l.push(Line::from(vec![Span::styled(
                    line,
                    Style::default().add_modifier(Modifier::BOLD).fg(t.fg_muted),
                )]));
            }
            l
        }
        melotui_core::state::lyrics::LyricsData::None => {
            vec![Line::from(vec![Span::styled(
                "No lyrics found.",
                Style::default().add_modifier(Modifier::BOLD).fg(t.fg_muted),
            )])]
        }
    };

    if let Some(search_state) = state.lyrics_search_state() {
        // Render search selector in two columns
        use ratatui::layout::{Constraint, Direction, Layout};

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(inner);

        // LRCLIB (Left column)
        let mut lrclib_lines = vec![Line::from(vec![Span::styled(
            "LRCLIB",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(t.accent_info),
        )])];
        for (i, res) in search_state.lrclib_results.iter().enumerate() {
            let is_selected = search_state.selected_column == 0 && search_state.selected_row == i;
            let style = if is_selected {
                Style::default()
                    .fg(ratatui::style::Color::Black)
                    .bg(t.accent_primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.fg_primary)
            };
            let prefix = if res.is_synced { "[S] " } else { "[U] " };
            lrclib_lines.push(Line::styled(
                format!("{}{}: {}", prefix, res.artist_name, res.track_name),
                style,
            ));
        }

        // OVH (Right column)
        let mut ovh_lines = vec![Line::from(vec![Span::styled(
            "OVH",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(t.accent_info),
        )])];
        for (i, res) in search_state.ovh_results.iter().enumerate() {
            let is_selected = search_state.selected_column == 1 && search_state.selected_row == i;
            let style = if is_selected {
                Style::default()
                    .fg(ratatui::style::Color::Black)
                    .bg(t.accent_primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.fg_primary)
            };
            ovh_lines.push(Line::styled(
                format!("{}: {}", res.artist_name, res.track_name),
                style,
            ));
        }

        let left_para = Paragraph::new(lrclib_lines).alignment(Alignment::Left);
        let right_para = Paragraph::new(ovh_lines).alignment(Alignment::Left);

        frame.render_widget(left_para, chunks[0]);
        frame.render_widget(right_para, chunks[1]);
        return;
    }

    let viewport_height = inner.height as usize;
    let mut scroll_offset = 0;

    if let Some(idx) = active_index {
        let half_viewport = viewport_height / 2;
        scroll_offset = idx.saturating_sub(half_viewport);
    }

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .scroll((scroll_offset as u16, 0));

    frame.render_widget(paragraph, inner);
}
