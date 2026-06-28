use crate::{AppMsg, state::ui::FocusMode};
use tui_input::InputRequest;

#[derive(Debug, Clone)]
pub enum UIMsg {
    EnterCommandMode,
    ExitCommandMode,
    CommandInput(InputRequest),
    SubmitCommand(String),
    Autocomplete,
    AutocompleteReverse,
    CommandHistoryPrev,
    CommandHistoryNext,
    OpenHelp,
    CloseHelp,
    ToggleLyrics,
    TogglePlaybackPanel,
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
    // Lyrics selector
    LyricsSelectorMoveUp,
    LyricsSelectorMoveDown,
    LyricsSelectorMoveLeft,
    LyricsSelectorMoveRight,
    LyricsSelectorSubmit,
    LyricsSelectorCancel,
    // DSP Studio
    ToggleDspStudio,
    DspSelectNext,
    DspSelectPrev,
    DspAdjustUp,
    DspAdjustDown,
    DspResetControl,
}

