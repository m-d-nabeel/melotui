pub mod command_view;
pub mod confirm_view;
pub mod dsp_studio_view;
pub mod help_view;
pub mod lyrics_view;
pub mod modal_picker_view;
pub mod notification_popup;
pub mod playback_view;
pub mod playlist_picker_view;
pub mod preset_picker_view;
pub mod sidebar;
pub mod sound_control_view;
pub mod visualizer_view;

use ratatui::{Frame, layout::Rect, widgets::Clear};
use unicode_width::UnicodeWidthStr;

/// Clears `area` for a popup, and with it a wide character, such as 紅,
/// just left of it whose right half lies under the popup: the terminal draws
/// such a character whole, and the cell under its right half is never
/// redrawn, so it showed through the popup's edge.
pub fn clear_for_popup(frame: &mut Frame, area: Rect) {
    frame.render_widget(Clear, area);
    let Some(left) = area.x.checked_sub(1) else {
        return;
    };
    let buf = frame.buffer_mut();
    for y in area.top()..area.bottom() {
        if let Some(cell) = buf.cell_mut((left, y))
            && cell.symbol().width() > 1
        {
            cell.set_symbol(" ");
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend, widgets::Paragraph};

    use super::*;

    #[test]
    fn a_wide_character_half_under_a_popup_goes() {
        let mut terminal = Terminal::new(TestBackend::new(10, 2)).unwrap();
        terminal
            .draw(|f| {
                // 紅蓮華 in columns 0 to 5, 蓮 in 2 and 3.
                f.render_widget(Paragraph::new("紅蓮華\n紅蓮華"), f.area());
                clear_for_popup(f, Rect::new(3, 0, 5, 1));
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        assert_eq!(buf[(0, 0)].symbol(), "紅");
        assert_eq!(buf[(2, 0)].symbol(), " ", "蓮 reached under the popup");
        assert_eq!(buf[(2, 1)].symbol(), "蓮", "the row below is not under it");
    }
}
