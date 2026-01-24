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
use std::collections::HashSet;

use crate::widgets::sidebar::generate_marquee_delayed;

// Theme constants
const ACTIVE_BORDER: Color = Color::Cyan;
const INACTIVE_BORDER: Color = Color::Rgb(60, 60, 60);
const SELECTED_BG: Color = Color::Rgb(40, 44, 52);
const SELECTED_FG: Color = Color::White;

// Playing state colors
const PLAYING_TRACK_COLOR: Color = Color::Rgb(166, 227, 161); // Catppuccin Green (#a6e3a1)
const ANCESTOR_COLOR: Color = Color::Rgb(148, 226, 213); // Catppuccin Teal (#94e2d5)

// Icon colors
const FOLDER_ICON_COLOR: Color = Color::Rgb(215, 153, 33); // Gold
const PLAYLIST_ICON_COLOR: Color = Color::Rgb(177, 98, 134); // Purple
const CATEGORY_COLOR: Color = Color::Rgb(97, 175, 239); // Soft Blue

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let rows = flatten_tree(&state.library, state.library_nav());
    let is_active = state.is_library_focused();

    // Resolve selection
    let selected_id = state.get_library_selection();
    let selected_index = selected_id
        .and_then(|id| rows.iter().position(|r| &r.id == id))
        .unwrap_or(0);

    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(selected_index));

    // Calculate active context
    let active_ancestors = get_playing_ancestors(state);

    // Render rows
    let items: Vec<ListItem> = rows
        .iter()
        .map(|row| render_row(row, state, area.width, is_active, &active_ancestors))
        .collect();

    // Container
    let border_style = if is_active {
        Style::default().fg(ACTIVE_BORDER)
    } else {
        Style::default().fg(INACTIVE_BORDER)
    };

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

    // List
    let list = List::new(items).highlight_style(Style::default().bg(SELECTED_BG));

    frame.render_stateful_widget(list, inner_area, &mut list_state);
}

fn render_row(
    row: &SidebarRow,
    state: &AppState,
    width: u16,
    panel_active: bool,
    active_ancestors: &HashSet<SidebarNodeId>,
) -> ListItem<'static> {
    let is_selected = state
        .get_library_selection()
        .map(|id| id == &row.id)
        .unwrap_or(false);

    let is_playing_track = state
        .get_playing_node()
        .map(|active| active == &row.id)
        .unwrap_or(false);

    let is_ancestor = active_ancestors.contains(&row.id);

    // Color logic
    let row_color = if is_playing_track {
        PLAYING_TRACK_COLOR
    } else if is_ancestor {
        ANCESTOR_COLOR
    } else if is_selected {
        SELECTED_FG
    } else {
        Color::Gray
    };

    // Icon styles
    let icon_style = if is_playing_track {
        Style::default().fg(PLAYING_TRACK_COLOR)
    } else if panel_active {
        match row.id {
            SidebarNodeId::Folder(_) => Style::default().fg(FOLDER_ICON_COLOR),
            SidebarNodeId::Playlist(_) => Style::default().fg(PLAYLIST_ICON_COLOR),
            SidebarNodeId::Category(_) => Style::default().fg(CATEGORY_COLOR),
            _ => Style::default().fg(Color::Gray),
        }
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let icon = get_icon(&row.id, row.is_expanded, is_playing_track);

    // Text styles
    let mut text_style = Style::default().fg(row_color);
    if is_playing_track || is_selected {
        text_style = text_style.add_modifier(Modifier::BOLD);
    }

    // Content formatting
    let indent_level = row.level as usize;
    let indent_str = "  ".repeat(indent_level);

    let icon_width = 2;
    let meta_width = (indent_str.len() + icon_width + 3) as u16;
    let available_text_width = width.saturating_sub(meta_width);

    let label_text = if is_selected && panel_active {
        generate_marquee_delayed(&row.label, available_text_width as usize, state)
    } else if row.label.len() > available_text_width as usize {
        format!(
            "{}…",
            &row.label[0..available_text_width.saturating_sub(1) as usize]
        )
    } else {
        row.label.clone()
    };

    // Build line
    let mut spans = vec![
        Span::raw(indent_str),
        Span::styled(icon, icon_style),
        Span::raw(" "),
        Span::styled(label_text, text_style),
    ];
    if is_playing_track {
        spans.push(Span::styled(" 󰐊", Style::default().fg(PLAYING_TRACK_COLOR)));
    }

    ListItem::new(Line::from(spans))
}

fn get_icon(id: &SidebarNodeId, is_expanded: bool, is_playing: bool) -> String {
    if is_playing {
        return "  ".to_string();
    }

    let s = match id {
        SidebarNodeId::Root => " 󰉋 ",
        SidebarNodeId::Category(cat) => {
            if is_expanded {
                match cat {
                    CategoryId::Folders => " 󰉋 ",
                    CategoryId::Artists => " 󰠃 ",
                    CategoryId::Albums => " 󰀥 ",
                    CategoryId::Playlists => " 󰲳 ",
                }
            } else {
                match cat {
                    CategoryId::Folders => " 󰉋 ",
                    CategoryId::Artists => " 󰠃 ",
                    CategoryId::Albums => " 󰀥 ",
                    CategoryId::Playlists => " 󰲳 ",
                }
            }
        }
        SidebarNodeId::Folder(_) => {
            if is_expanded {
                "  "
            } else {
                "  "
            }
        }
        SidebarNodeId::Playlist(_) => {
            if is_expanded {
                "  "
            } else {
                "  "
            }
        }
        SidebarNodeId::Track { .. } => "  ",
        SidebarNodeId::Artist(_) => {
            if is_expanded {
                " 󰠃 "
            } else {
                " 󰠃 "
            }
        }
    };
    s.to_string()
}

fn get_playing_ancestors(state: &AppState) -> HashSet<SidebarNodeId> {
    let mut ancestors = HashSet::new();
    if let Some(playing_node) = state.get_playing_node() {
        let mut current = playing_node.clone();
        while let Some(parent) = current.parent(&state.library.root) {
            ancestors.insert(parent.clone());
            current = parent;
        }
    }
    ancestors
}
