use std::time::{Duration, Instant};

use crate::{
    AppMsg, AppState, Command, UIMode,
    state::{Notification, NotificationKind},
};

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
    process_command_input(state, input)
}

fn process_command_input(state: &mut AppState, input: String) -> Vec<AppMsg> {
    state.ui.mode = UIMode::Normal;
    state.ui.command_buffer.clear();

    let input = input.trim();
    if input.is_empty() {
        return vec![];
    }

    match Command::parse(input) {
        Command::Quit => {
            vec![AppMsg::RequestQuit]
        }
        Command::Rescan => {
            vec![AppMsg::RequestRescanLibrary]
        }
        Command::Download { query } => {
            state.notification.current = Some(Notification {
                kind: NotificationKind::Info,
                message: format!("Downloading: {}", query),
                progress: None,
                created_at: Instant::now(),
                dismiss_after: Some(Duration::from_secs(2)),
            });
            vec![AppMsg::RequestDownload { query }]
        }
        Command::ChangeVisualizer { arg } => {
            state.spectrum_style = arg;
            vec![]
        }
        Command::Unknown(reason) => {
            state.notification.current = Some(Notification {
                kind: NotificationKind::Error,
                message: format!("Unknown command: {}", reason),
                progress: None,
                created_at: Instant::now(),
                dismiss_after: Some(Duration::from_secs(2)),
            });
            vec![]
        }
    }
}

pub fn handle_dispatch_set_visualizer(_state: &mut AppState, _active: bool) -> Vec<AppMsg> {
    vec![]
}
