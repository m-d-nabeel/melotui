use super::modal_picker_view::{PickerConfig, draw_picker};
use melotui_core::AppState;
use ratatui::{Frame, layout::Rect};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let picker = state.playlist_picker();
    let all_items = state
        .library
        .playlist_order
        .iter()
        .filter_map(|pid| state.library.playlists.get(pid))
        .map(|p| p.name.clone())
        .collect();

    draw_picker(
        frame,
        area,
        state,
        PickerConfig {
            prompt_label: "Playlist: ",
            query: &picker.query,
            all_items,
            create_label_format: "Create \"{}\"",
            selection: picker.selection,
            scroll_offset: picker.scroll_offset,
            max_visible: state.config.ui.playlist_picker_max_visible,
        },
    );
}
