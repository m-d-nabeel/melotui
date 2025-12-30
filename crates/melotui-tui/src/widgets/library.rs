use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, List, ListItem},
};

use melotui_core::{AppState, NavMode};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let (title, items, selected) = match state.nav.mode {
        NavMode::Playlists => (
            "Playlists".to_string(),
            state
                .library
                .playlists
                .iter()
                .map(|p| ListItem::new(p.name.clone()))
                .collect::<Vec<_>>(),
            state.nav.selected_playlist,
        ),

        NavMode::Tracks => {
            let p = state.nav.selected_playlist;
            let playlist = &state.library.playlists[p];

            (
                format!("Playlist: {}", playlist.name),
                playlist
                    .tracks
                    .iter()
                    .map(|t| {
                        let name = t.file_name().unwrap().to_string_lossy().to_string();

                        let is_playing = state.playback.current.as_ref().is_some_and(|current| {
                            current.playlist == playlist.name && current.path == *t
                        });

                        if is_playing {
                            ListItem::new(format!("▶ {}", name))
                        } else {
                            ListItem::new(name)
                        }
                    })
                    .collect(),
                state.nav.selected_tracks[p],
            )
        }
    };
    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_symbol("> ");

    frame.render_stateful_widget(
        list,
        area,
        &mut ratatui::widgets::ListState::default().with_selected(Some(selected)),
    );
}
