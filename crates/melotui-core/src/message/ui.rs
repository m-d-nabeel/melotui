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
    EnterFilterMode,
    ExitFilterMode,
    UpdateFilter(char),
    BackspaceFilter,
    SubmitFilter(String),
}
