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
    /// Scrolls the help by `by` lines, down for more, no further than `max`.
    ScrollHelp {
        by: i32,
        max: u16,
    },
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
    EnterPresetPicker,
    ExitPresetPicker,
    SubmitPresetPicker,
    PresetPickerMoveUp,
    PresetPickerMoveDown,
    PresetPickerUpdateQuery(char),
    PresetPickerBackspace,
    DeletePlaylistConfirm,
    ShowConfirm {
        message: String,
        on_confirm: Box<AppMsg>,
    },
    ConfirmYes,
    ConfirmNo,
    /// A notification from outside the reducer: a worker's error, say.
    Notify {
        kind: crate::state::NotificationKind,
        message: String,
    },
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
    DspSelectUp,
    DspSelectDown,
    DspSelectLeft,
    DspSelectRight,
    DspAdjustUp,
    DspAdjustDown,
    DspResetControl,
    DspMacroSlowedReverb,
    DspMacroBassBoosted,
    DspMacroSpatial,
    DspMacroResetAll,
}
