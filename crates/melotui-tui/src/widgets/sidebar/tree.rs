use melotui_core::{
    AppState,
    library::projector::{SidebarRow, get_row_count, get_visible_rows},
    navigation::ids::{CategoryId, SidebarNodeId},
};
use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState,
    },
};
use std::collections::HashSet;
use unicode_width::UnicodeWidthStr;

use crate::widgets::sidebar::generate_marquee_delayed;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let nav = state.library_nav();
    let total_rows = get_row_count(&state.library, nav);
    let is_active = state.is_library_focused() && !state.is_mixer_focused();

    let t = crate::theme::get_theme(state);
    let border_style = if is_active {
        Style::default().fg(t.border_active)
    } else {
        Style::default().fg(t.border_inactive)
    };

    let title = if is_active {
        Line::from(vec![Span::styled(
            " Library ",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(t.border_active),
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

    let visible_height = inner_area.height as usize;
    if visible_height == 0 {
        return;
    }

    // Empty state: show placeholder when no rows
    if total_rows == 0 {
        let empty_msg = Paragraph::new(Line::from(vec![
            Span::styled("  ", Style::default().fg(t.fg_muted)),
            Span::styled(
                "No tracks found",
                Style::default()
                    .fg(t.fg_muted)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]))
        .alignment(Alignment::Center);

        let y_offset = inner_area.height / 2;
        let centered = Rect {
            x: inner_area.x,
            y: inner_area.y + y_offset,
            width: inner_area.width,
            height: 1,
        };
        frame.render_widget(empty_msg, centered);
        return;
    }

    let offset = nav.scroll_offset;
    let rows = get_visible_rows(&state.library, nav, offset, visible_height);

    let active_ancestors = get_playing_ancestors(state);

    let items: Vec<ListItem> = rows
        .iter()
        .map(|row| render_row(row, state, inner_area.width, is_active, &active_ancestors))
        .collect();

    let mut list_state = ListState::default();

    let selected_index = if let Some(sel_id) = state.get_library_selection() {
        rows.iter().position(|row| &row.id == sel_id)
    } else {
        None
    };
    list_state.select(selected_index);

    let list = List::new(items).highlight_style(Style::default().bg(t.bg_selected));
    frame.render_stateful_widget(list, inner_area, &mut list_state);

    // lean scrollbar
    let scrollbar = Scrollbar::default()
        .orientation(ScrollbarOrientation::VerticalRight)
        .symbols(symbols::scrollbar::VERTICAL)
        .begin_symbol(None)
        .end_symbol(None)
        .track_symbol(None)
        .thumb_style(Style::default().fg(t.fg_muted))
        .track_style(Style::default().fg(Color::Black));

    let mut scrollbar_state = ScrollbarState::new(total_rows).position(offset);

    frame.render_stateful_widget(
        scrollbar,
        area.inner(Margin {
            horizontal: 0,
            vertical: 1,
        }),
        &mut scrollbar_state,
    );
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

    let t = crate::theme::get_theme(state);

    // Semantic depth mapping for normal unselected rows
    let base_color = if matches!(row.id, SidebarNodeId::Track { .. }) {
        t.fg_tree_file
    } else {
        match row.level {
            0 => t.fg_tree_root,
            1 => t.fg_tree_folder,
            _ => t.fg_tree_subfolder,
        }
    };

    // color logic
    let row_color = if is_playing_track {
        t.accent_playing
    } else if is_ancestor {
        t.accent_ancestor
    } else if is_selected {
        t.fg_primary
    } else {
        base_color
    };

    // icon styles
    let icon_style = if is_playing_track {
        Style::default().fg(t.accent_playing)
    } else if panel_active {
        match row.id {
            SidebarNodeId::Folder(_) => Style::default().fg(t.icon_folder),
            SidebarNodeId::Playlist(_) => Style::default().fg(t.icon_playlist),
            SidebarNodeId::Category(_) => Style::default().fg(t.icon_category),
            SidebarNodeId::Artist(_)
            | SidebarNodeId::Album(_)
            | SidebarNodeId::Genre(_)
            | SidebarNodeId::Year(_) => Style::default().fg(t.icon_category),
            SidebarNodeId::Track { .. } => Style::default().fg(base_color),
            _ => Style::default().fg(t.fg_secondary),
        }
    } else {
        Style::default().fg(t.fg_muted)
    };

    // Get pre-padded icon string.
    // Nerd Font PUA glyphs need surrounding spaces for consistent terminal
    // rendering — without them the glyph compresses when adjacent to other
    // characters and expands on selection. The padding is intentional.
    let icon_str = get_icon(
        &row.id,
        row.is_expanded,
        row.is_expandable,
        is_playing_track,
    );

    // text styles
    let mut text_style = Style::default().fg(row_color);
    if is_playing_track || is_selected || row.level == 0 {
        text_style = text_style.add_modifier(Modifier::BOLD);
    }

    // Indent: 1 space per level for hierarchy depth
    let indent_level = row.level as usize;
    let indent_str = " ".repeat(indent_level);

    // Width: [indent] [icon_str] [label]
    // icon_str includes its own padding so no extra separator needed.
    // Nerd font icons are 2-cell wide but unicode_width reports 1,
    // so we hardcode the display width of each icon string.
    let icon_display_width = icon_display_cells(&icon_str);
    let meta_width = (indent_level + icon_display_width) as u16;
    let available_text_width = width.saturating_sub(meta_width);

    let label_text = if is_selected && panel_active {
        generate_marquee_delayed(&row.label, available_text_width as usize, state)
    } else if UnicodeWidthStr::width(row.label.as_str()) > available_text_width as usize {
        truncate_to_width(&row.label, available_text_width as usize)
    } else {
        row.label.clone()
    };

    // Build line
    let spans = vec![
        Span::raw(indent_str),
        Span::styled(icon_str, icon_style),
        Span::styled(label_text, text_style),
    ];

    ListItem::new(Line::from(spans))
}

/// Truncate a string to fit within `max_width` display columns, appending '…'.
fn truncate_to_width(s: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let mut width = 0;
    let mut result = String::new();
    for ch in s.chars() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + ch_width >= max_width {
            break;
        }
        width += ch_width;
        result.push(ch);
    }
    result.push('…');
    result
}

/// Returns a pre-padded icon string for the tree row.
///
/// Nerd Font glyphs live in Unicode's Private Use Area and render at
/// variable widths depending on adjacent characters. Surrounding each
/// glyph with spaces prevents compression/expansion artifacts.
///
/// Format for expandable nodes: "arrow icon " (e.g. "▾ 󰉋 ")
/// Format for leaf nodes:       "icon " (e.g. "♪ ")
fn get_icon(
    id: &SidebarNodeId,
    is_expanded: bool,
    is_expandable: bool,
    is_playing: bool,
) -> String {
    if is_playing {
        return "󰐊 ".to_string();
    }

    match id {
        SidebarNodeId::Root => "󰉋 ".to_string(),
        SidebarNodeId::Category(cat) => {
            let arrow = if is_expanded { "" } else { "" };
            let icon = match cat {
                CategoryId::Folders => "󰉋",
                CategoryId::Artists => "󰠃",
                CategoryId::Albums => "󰀥",
                CategoryId::Playlists => "󰲳",
                CategoryId::Genres => "󰊗",
                CategoryId::Years => "󰔠",
                CategoryId::Favorites => "󰋎",
                CategoryId::RecentlyPlayed => "󰥔",
                CategoryId::MostPlayed => "󰓎",
            };
            format!("{} {} ", arrow, icon)
        }
        SidebarNodeId::Folder(_) => {
            if is_expandable {
                let arrow = if is_expanded { "" } else { "" };
                format!("{} {} ", arrow, "󰉋")
            } else {
                "󰉋 ".to_string()
            }
        }
        SidebarNodeId::Playlist(_) => {
            if is_expandable {
                let arrow = if is_expanded { "" } else { "" };
                format!("{} {} ", arrow, "󰲳")
            } else {
                "󰲳 ".to_string()
            }
        }
        // Tracks are leaf nodes — no arrow, just icon
        SidebarNodeId::Track { .. } => "♪ ".to_string(),
        SidebarNodeId::Artist(_) => {
            if is_expandable {
                let arrow = if is_expanded { "" } else { "" };
                format!("{} {} ", arrow, "󰠃")
            } else {
                "󰠃 ".to_string()
            }
        }
        SidebarNodeId::Album(_) => {
            if is_expandable {
                let arrow = if is_expanded { "" } else { "" };
                format!("{} {} ", arrow, "󰀥")
            } else {
                "󰀥 ".to_string()
            }
        }
        SidebarNodeId::Genre(_) => {
            if is_expandable {
                let arrow = if is_expanded { "" } else { "" };
                format!("{} {} ", arrow, "󰊗")
            } else {
                "󰊗 ".to_string()
            }
        }
        SidebarNodeId::Year(_) => {
            if is_expandable {
                let arrow = if is_expanded { "" } else { "" };
                format!("{} {} ", arrow, "󰔠")
            } else {
                "󰔠 ".to_string()
            }
        }
    }
}

/// Estimate the display cell count of an icon string.
/// Nerd Font PUA glyphs are 2-cell wide in most terminal fonts,
/// but `unicode_width` reports them as 1. We account for this by
/// counting each non-ASCII, non-space character as 2 cells.
fn icon_display_cells(s: &str) -> usize {
    let mut width = 0;
    for ch in s.chars() {
        if ch == ' ' || ch.is_ascii() {
            width += 1;
        } else {
            // Nerd Font / PUA glyph — assume 2 terminal cells
            width += 2;
        }
    }
    width
}

fn get_playing_ancestors(state: &AppState) -> HashSet<SidebarNodeId> {
    let mut ancestors = HashSet::new();
    if let Some(playing_node) = state.get_playing_node() {
        let mut current = playing_node.clone();
        while let Some(parent) = current.parent(&state.library.roots) {
            ancestors.insert(parent.clone());
            current = parent;
        }
    }
    ancestors
}
