use crate::{AppMsg, AppState, UIMode, action_handlers::handler_process_command_input};

pub fn handle_toggle_help(state: &mut AppState) -> Vec<AppMsg> {
    state.ui.show_help = !state.ui.show_help;
    vec![]
}

pub fn handle_enter_command_mode(state: &mut AppState) -> Vec<AppMsg> {
    tracing::debug!("Enter Command Mode");
    state.ui.mode = UIMode::Command;
    state.ui.command_buffer.clear();
    vec![]
}

pub fn handle_exit_command_mode(state: &mut AppState) -> Vec<AppMsg> {
    tracing::debug!("Exit Command Mode");
    state.ui.mode = UIMode::Normal;
    state.ui.command_buffer.clear();
    vec![]
}

pub fn handle_update_command(state: &mut AppState, c: char) -> Vec<AppMsg> {
    state.ui.command_buffer.push(c);
    vec![]
}

pub fn handle_backspace_command(state: &mut AppState) -> Vec<AppMsg> {
    state.ui.command_buffer.pop();
    vec![]
}

pub fn handle_submit_command(state: &mut AppState, input: String) -> Vec<AppMsg> {
    tracing::info!("SubmitCommand called with :{:?}", input);
    handler_process_command_input(state, input)
}

pub fn handle_dispatch_set_visualizer(_state: &mut AppState, _active: bool) -> Vec<AppMsg> {
    vec![]
}
