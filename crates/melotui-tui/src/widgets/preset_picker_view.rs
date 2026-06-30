use super::modal_picker_view::{PickerConfig, draw_picker};
use melotui_core::AppState;
use ratatui::{Frame, layout::Rect};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let picker = state.preset_picker();
    draw_picker(
        frame,
        area,
        state,
        PickerConfig {
            prompt_label: "DSP Preset: ",
            query: &picker.query,
            all_items: picker.presets.clone(),
            create_label_format: "Save as \"{}\"",
            selection: picker.selection,
            scroll_offset: picker.scroll_offset,
            max_visible: 10,
        },
    );
}
