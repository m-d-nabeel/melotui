use crate::AppMsg;

#[derive(Debug, Clone)]
pub enum UIMsg {
    EnterCommandMode,
    ExitCommandMode,
    UpdateCommand(char),
    BackspaceCommand,
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
}
