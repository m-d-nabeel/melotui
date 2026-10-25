use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use melotui_core::AppState;

pub struct PickerConfig<'a> {
    pub prompt_label: &'a str,
    pub query: &'a str,
    pub all_items: Vec<String>,
    pub create_label_format: &'a str,
    pub selection: usize,
    pub scroll_offset: usize,
    pub max_visible: usize,
}

pub fn draw_picker(frame: &mut Frame, area: Rect, state: &AppState, config: PickerConfig) {
    let t = crate::theme::get_theme(state);
    let query_lower = config.query.trim().to_lowercase();

    let mut filtered: Vec<(String, bool)> = config
        .all_items
        .into_iter()
        .filter(|p| p.to_lowercase().contains(&query_lower))
        .map(|p| (p, false))
        .collect();

    let exact_match = filtered
        .iter()
        .any(|(n, _)| n.to_lowercase() == query_lower);
    if !config.query.trim().is_empty() && !exact_match {
        let create_str = config
            .create_label_format
            .replace("{}", config.query.trim());
        filtered.push((create_str, true));
    }

    let total = filtered.len();
    let scroll = config.scroll_offset.min(total.saturating_sub(1));
    let visible_count = total.saturating_sub(scroll).min(config.max_visible);
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

    crate::widgets::clear_for_popup(frame, list_area);
    crate::widgets::clear_for_popup(frame, input_area);

    let input_content = Line::from(vec![
        Span::styled(
            config.prompt_label,
            Style::default()
                .fg(t.accent_info)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(config.query),
        Span::styled("█", Style::default().fg(t.fg_primary)),
    ]);

    let input_widget = Paragraph::new(input_content).style(Style::default().bg(t.bg_input));
    frame.render_widget(input_widget, input_area);

    let visible_items = &filtered[scroll..scroll + visible_count];

    let items: Vec<ListItem> = visible_items
        .iter()
        .enumerate()
        .map(|(vi, (name, is_create))| {
            let abs_idx = scroll + vi;
            let style = if abs_idx == config.selection {
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
