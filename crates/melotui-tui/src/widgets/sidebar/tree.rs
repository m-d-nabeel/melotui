use melotui_core::{
    AppState,
    library::projector::{SidebarRow, flatten_tree},
    navigation::ids::{CategoryId, SidebarNodeId},
};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem},
};

use crate::widgets::sidebar::generate_marquee_delayed;

// Theme Constants
const ACTIVE_BORDER: Color = Color::Yellow;
const INACTIVE_BORDER: Color = Color::Rgb(60, 60, 60);
const SELECTED_BG: Color = Color::Rgb(50, 50, 50);
const PLAYING_FG: Color = Color::Rgb(142, 192, 124); // Gruvbox Green
const FOLDER_ICON_COLOR: Color = Color::Rgb(215, 153, 33);
const PLAYLIST_ICON_COLOR: Color = Color::Rgb(177, 98, 134);

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let rows = flatten_tree(&state.library, state.library_nav());
    let is_active = state.is_library_focused();

    // 1. Resolve Selection
    let selected_id = state.get_library_selection();
    let selected_index = selected_id
        .and_then(|id| rows.iter().position(|r| &r.id == id))
        .unwrap_or(0);

    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(selected_index));

    // 2. Render Rows
    let items: Vec<ListItem> = rows
        .iter()
        .map(|row| render_row(row, state, area.width, is_active))
        .collect();

    // 3. Container
    let border_style = if is_active {
        Style::default().fg(ACTIVE_BORDER)
    } else {
        Style::default().fg(INACTIVE_BORDER)
    };

    // removed icon from title as requested
    let title = if is_active {
        Line::from(vec![Span::styled(
            " Library ",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(ACTIVE_BORDER),
        )])
    } else {
        Line::from(" Library ")
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style);

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // 4. List Widget
    let list = List::new(items).highlight_style(Style::default().bg(SELECTED_BG));

    frame.render_stateful_widget(list, inner_area, &mut list_state);
}

fn render_row(
    row: &SidebarRow,
    state: &AppState,
    width: u16,
    panel_active: bool,
) -> ListItem<'static> {
    let is_selected = state
        .get_library_selection()
        .map(|id| id == &row.id)
        .unwrap_or(false);

    let playing_id = state.get_playing_track_id();
    let is_playing_track = match (&row.id, playing_id) {
        (SidebarNodeId::Track { id, .. }, Some(pid)) => *id == pid,
        _ => false,
    };
    // --- Icons & Styles ---
    let (icon, icon_style) = get_icon_and_style(row, is_playing_track, panel_active);

    // --- Indentation ---
    let indent_level = row.level as usize;
    let indent_str = "  ".repeat(indent_level);

    // --- Content / Marquee ---
    // Calculate available width: Total - (Indent + Icon + Padding + Borders)
    let icon_width = 2;
    let meta_width = (indent_str.len() + icon_width + 3) as u16;
    let available_text_width = width.saturating_sub(meta_width);

    let label_text = if is_selected && panel_active {
        generate_marquee_delayed(&row.label, available_text_width as usize, state)
    } else {
        // Truncate
        if row.label.len() > available_text_width as usize {
            format!(
                "{}…",
                &row.label[0..available_text_width.saturating_sub(1) as usize]
            )
        } else {
            row.label.clone()
        }
    };

    // --- Line Construction ---
    let mut spans = Vec::new();
    spans.push(Span::raw(indent_str));
    spans.push(Span::styled(icon, icon_style));
    spans.push(Span::raw(" "));

    let text_style = if is_playing_track {
        Style::default().fg(PLAYING_FG).add_modifier(Modifier::BOLD)
    } else if is_selected {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    spans.push(Span::styled(label_text, text_style));

    if is_playing_track {
        spans.push(Span::styled(" 󰐊", Style::default().fg(PLAYING_FG)));
    }

    ListItem::new(Line::from(spans))
}

fn get_icon_and_style(row: &SidebarRow, is_playing: bool, active: bool) -> (String, Style) {
    if is_playing {
        return ("".to_string(), Style::default().fg(PLAYING_FG));
    }

    let color = if active {
        match row.id {
            SidebarNodeId::Folder(_) => FOLDER_ICON_COLOR,
            SidebarNodeId::Playlist(_) => PLAYLIST_ICON_COLOR,
            SidebarNodeId::Category(_) => Color::Blue,
            _ => Color::Gray,
        }
    } else {
        Color::DarkGray
    };

    let icon = match &row.id {
        SidebarNodeId::Root => "󰉋",
        SidebarNodeId::Category(cat) => match cat {
            CategoryId::Folders => "󰉋",
            CategoryId::AllSongs => "󰎆",
            CategoryId::Artists => "󰠃",
            CategoryId::Albums => "󰀥",
            CategoryId::Playlists => "󰲳",
        },
        SidebarNodeId::Folder(_) => {
            if row.is_expanded {
                ""
            } else {
                ""
            }
        }
        SidebarNodeId::Playlist(_) => "",
        SidebarNodeId::Track { .. } => "", // Music note for tracks
        SidebarNodeId::Artist(_) => "󰠃",
    };

    (icon.to_string(), Style::default().fg(color))
}
