use melotui_core::AppState;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let queue = state.queue();

    // 1. Build List Items
    let items: Vec<ListItem> = queue
        .iter()
        .map(|track_id| {
            // Access library snapshot directly via Arc
            let title = state
                .library
                .get_track(*track_id)
                .map(|t| t.title.as_str())
                .unwrap_or("Unknown Track");

            ListItem::new(Line::from(vec![Span::raw("  "), Span::raw(title)]))
        })
        .collect();

    // 2. Resolve Selection
    let selected_id = state.get_queue_selection();

    let selected_index = selected_id
        .and_then(|id| queue.iter().position(|&x| x == id))
        .unwrap_or(0);

    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(selected_index));

    // 3. Render
    let block = Block::default()
        .title(" Queue ")
        .borders(Borders::RIGHT)
        .style(Style::default().fg(Color::Cyan));

    let list = List::new(items)
        .block(block)
        // Cyan highlight for Queue mode
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan)
                .bg(Color::DarkGray),
        );

    frame.render_stateful_widget(list, area, &mut list_state);
}
