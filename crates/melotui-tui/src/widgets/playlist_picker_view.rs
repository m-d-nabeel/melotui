use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use crate::theme::Theme;
use melotui_core::{AppState, domain::PlaylistMeta};

fn theme() -> &'static Theme {
    static THEME: std::sync::OnceLock<Theme> = std::sync::OnceLock::new();
    THEME.get_or_init(Theme::default)
}

const MAX_VISIBLE: usize = 10;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let t = theme();
    let picker = state.playlist_picker();

    let query = picker.query.trim().to_lowercase();

    let all_playlists: Vec<&PlaylistMeta> = state
        .library
        .playlist_order
        .iter()
        .filter_map(|pid| state.library.playlists.get(pid))
        .collect();

    let mut filtered: Vec<(String, bool)> = all_playlists
        .iter()
        .filter(|p| p.name.to_lowercase().contains(&query))
        .map(|p| (p.name.clone(), false))
        .collect();

    let exact_match = filtered.iter().any(|(n, _)| n.to_lowercase() == query);
    if !query.is_empty() && !exact_match {
        filtered.push((format!("Create \"{}\"", picker.query), true));
    }

    let total = filtered.len();
    let scroll = picker.scroll_offset.min(total.saturating_sub(1));
    let visible_count = total.saturating_sub(scroll).min(MAX_VISIBLE);
    // +1 for the top border
    let list_height = if visible_count == 0 {
        0
    } else {
        visible_count as u16 + 1
    };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(list_height),
            Constraint::Length(1),
        ])
        .split(area);

    let list_area = layout[1];
    let input_area = layout[2];

    frame.render_widget(Clear, list_area);
    frame.render_widget(Clear, input_area);

    // input bar
    let input_content = Line::from(vec![
        Span::styled(
            "Playlist: ",
            Style::default()
                .fg(t.accent_info)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(&picker.query),
        Span::styled("█", Style::default().fg(t.fg_primary)),
    ]);

    let input_widget =
        Paragraph::new(input_content).style(Style::default().bg(t.bg_input));
    frame.render_widget(input_widget, input_area);

    // visible slice of the list
    let visible_items = &filtered[scroll..scroll + visible_count];

    let items: Vec<ListItem> = visible_items
        .iter()
        .enumerate()
        .map(|(vi, (name, is_create))| {
            let abs_idx = scroll + vi;
            let style = if abs_idx == picker.selection {
                Style::default().fg(Color::Black).bg(t.accent_info)
            } else {
                Style::default().fg(t.fg_primary).bg(t.bg_overlay)
            };

            let prefix = if *is_create { "+ " } else { "  " };
            ListItem::new(format!("{}{}", prefix, name)).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::TOP)
            .style(Style::default().bg(t.bg_overlay)),
    );

    frame.render_widget(list, list_area);
}
