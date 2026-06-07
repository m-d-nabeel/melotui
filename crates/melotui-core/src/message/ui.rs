use crate::{AppMsg, state::ui::FocusMode};

#[derive(Debug, Clone)]
pub enum UIMsg {
    EnterCommandMode,
    ExitCommandMode,
    CommandInput(ratatui::crossterm::event::KeyEvent),
    SubmitCommand(String),
    OpenHelp,
    CloseHelp,
    Resize(u16, u16),
    EnterPlaylistPicker,
    ExitPlaylistPicker,
    SubmitPlaylistPicker,
    PlaylistPickerMoveUp,
    PlaylistPickerMoveDown,
    PlaylistPickerUpdateQuery(char),
    PlaylistPickerBackspace,
    DeletePlaylistConfirm,
    ShowConfirm {
        message: String,
        on_confirm: Box<AppMsg>,
    },
    ConfirmYes,
    ConfirmNo,
    // Focus mode
    CycleFocusMode,
    SetFocusMode(FocusMode),
    // Sidebar width
    SetSidebarWidth(u16),
    AdjustSidebarWidth(i16),
}
