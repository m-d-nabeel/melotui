use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
};

use melotui_core::{AppState, NavMode};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let (title, items, selected, highlight_style) = match state.nav.mode {
        NavMode::Playlists => {
            let items: Vec<ListItem> = state
                .library
                .playlists
                .iter()
                .map(|p| ListItem::new(format!(" {}", p.name)))
                .collect();

            (
                "Playlists".to_string(),
                items,
                state.nav.selected_playlist,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
        }

        NavMode::Tracks => {
            let p = state.nav.selected_playlist;
            let playlist = &state.library.playlists[p];

            let items: Vec<ListItem> = playlist
                .tracks
                .iter()
                .map(|t| {
                    let name = t.file_name().unwrap().to_string_lossy().to_string();

                    let is_playing = state.playback.current.as_ref().is_some_and(|current| {
                        current.playlist == playlist.name && current.path == *t
                    });

                    if is_playing {
                        ListItem::new(format!("▶ {}", name))
                            .style(Style::default().fg(Color::LightGreen))
                    } else {
                        ListItem::new(format!("  {}", name))
                    }
                })
                .collect();

            (
                format!("Playlist: {}", playlist.name),
                items,
                state.nav.selected_tracks[p],
                Style::default().bg(Color::Yellow).fg(Color::Black),
            )
        }
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .highlight_style(highlight_style)
        .highlight_symbol("");

    frame.render_stateful_widget(
        list,
        area,
        &mut ListState::default().with_selected(Some(selected)),
    );
}
