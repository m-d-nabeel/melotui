#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UIMode {
    #[default]
    Normal,
    Command,
    Filter,
}

#[derive(Debug, Clone)]
pub struct UIState {
    pub mode: UIMode,
    pub command_buffer: String,
    pub show_help: bool,
    pub scrolloff: usize,
    pub terminal_size: (u16, u16),
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            mode: UIMode::default(),
            command_buffer: String::new(),
            show_help: false,
            scrolloff: 8,
            terminal_size: (0, 0),
        }
    }
}
