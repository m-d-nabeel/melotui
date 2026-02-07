#[derive(Debug, Clone)]
pub enum UIMsg {
    EnterCommandMode,
    ExitCommandMode,
    UpdateCommand(char),
    BackspaceCommand,
    SubmitCommand(String),
    OpenHelp,
    CloseHelp,
}
