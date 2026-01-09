#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UIMode {
    #[default]
    Normal,
    Command,
}

#[derive(Debug, Clone, Default)]
pub struct UIState {
    pub mode: UIMode,
    pub command_buffer: String,
    pub show_help: bool,
}
