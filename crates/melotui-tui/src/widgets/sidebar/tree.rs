use melotui_core::{
    AppState,
    library::projector::{SidebarRow, flatten_tree},
};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    // 1. GET PROJECTION (Pure Function)
    // We access the library_nav via the new accessor
    let rows = flatten_tree(&state.library, state.library_nav());

    // 2. CREATE LIST ITEMS
    let items: Vec<ListItem> = rows
        .iter()
        .map(|row| render_row(row, state, area.width))
        .collect();

    // 3. RESOLVE SELECTION INDEX
    // We use the accessor to find which ID is currently selected
    let selected_id = state.get_library_selection();

    let selected_index = selected_id
        .and_then(|id| rows.iter().position(|r| &r.id == id))
        .unwrap_or(0);

    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(selected_index));

    // 4. RENDER
    let block = Block::default()
        .borders(Borders::RIGHT)
        .style(Style::default().fg(Color::DarkGray));

    // Highlight style: Yellow + Bold
    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::Yellow),
    );

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_row(row: &SidebarRow, state: &AppState, width: u16) -> ListItem<'static> {
    // Check if this specific row is the selected one
    let is_selected = state
        .get_library_selection()
        .map(|id| id == &row.id)
        .unwrap_or(false);

    let style = if is_selected {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    // Indentation: 2 spaces per level
    let indent = "  ".repeat(row.level as usize);

    // Icon logic
    let icon = if row.is_expandable {
        if row.is_expanded { "▼ " } else { "▶ " }
    } else {
        "• " // Track leaf
    };

    // Marquee logic for the label
    let content = if is_selected {
        // Calculate available width for text: Total - (Indent + Icon + Borders)
        let available_width = width.saturating_sub(indent.len() as u16 + 2 + 1);
        generate_marquee(&row.label, available_width, state.time_elapsed())
    } else {
        row.label.clone()
    };

    let line = Line::from(vec![
        Span::raw(indent),
        Span::styled(
            icon,
            if is_selected {
                style
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ),
        Span::styled(content, style),
    ]);

    ListItem::new(line)
}

fn generate_marquee(text: &str, max_width: u16, time: f32) -> String {
    let width = max_width as usize;
    if text.len() <= width {
        return text.to_string();
    }

    let speed = 4.0; // chars per second
    let gap = 5;
    let full_len = text.len() + gap;

    let offset = (time * speed) as usize % full_len;
    let display_text = format!("{}{}{}", text, " ".repeat(gap), text);

    display_text.chars().skip(offset).take(width).collect()
}
