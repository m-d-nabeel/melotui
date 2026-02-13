use std::time::{Duration, Instant};

use crate::{
    AppMsg, AppState, Command, UIMode,
    message::{download::DownloadMsg, library::LibraryMsg, playback::PlaybackMsg},
    navigation::ids::SidebarNodeId,
    state::{
        Notification, NotificationKind, VisualizerMode,
        visualizer::{SpectrumStyle, is_cava_available},
    },
};

pub fn handle_open_help(state: &mut AppState) -> Vec<AppMsg> {
    state.session.ui.show_help = true;
    vec![]
}

pub fn handle_close_help(state: &mut AppState) -> Vec<AppMsg> {
    state.session.ui.show_help = false;
    vec![]
}

pub fn handle_resize(state: &mut AppState, w: u16, h: u16) -> Vec<AppMsg> {
    state.session.ui.terminal_size = (w, h);
    vec![]
}

pub fn handle_enter_command_mode(state: &mut AppState) -> Vec<AppMsg> {
    tracing::debug!("Enter Command Mode");
    state.session.ui.mode = UIMode::Command;
    state.session.ui.command_buffer.clear();
    vec![]
}

pub fn handle_exit_command_mode(state: &mut AppState) -> Vec<AppMsg> {
    tracing::debug!("Exit Command Mode");
    state.session.ui.mode = UIMode::Normal;
    state.session.ui.command_buffer.clear();
    vec![]
}

pub fn handle_update_command(state: &mut AppState, c: char) -> Vec<AppMsg> {
    state.session.ui.command_buffer.push(c);
    vec![]
}

pub fn handle_backspace_command(state: &mut AppState) -> Vec<AppMsg> {
    state.session.ui.command_buffer.pop();
    vec![]
}

pub fn handle_submit_command(state: &mut AppState, input: String) -> Vec<AppMsg> {
    tracing::info!("SubmitCommand called with :{:?}", input);
    process_command_input(state, input)
}

fn process_command_input(state: &mut AppState, input: String) -> Vec<AppMsg> {
    state.session.ui.mode = UIMode::Normal;
    state.session.ui.command_buffer.clear();

    let input = input.trim();
    if input.is_empty() {
        return vec![];
    }

    match Command::parse(input) {
        Command::Quit => {
            vec![AppMsg::RequestQuit]
        }
        Command::Rescan => {
            vec![AppMsg::Library(LibraryMsg::RequestScanLibrary)]
        }
        Command::Download { query } => {
            state.session.notification.current = Some(Notification {
                kind: NotificationKind::Info,
                message: format!("Downloading: {}", query),
                progress: None,
                created_at: Instant::now(),
                dismiss_after: Some(Duration::from_secs(2)),
            });
            vec![AppMsg::Download(DownloadMsg::RequestDownload { query })]
        }
        Command::ChangeVisualizer { arg } => {
            if arg == SpectrumStyle::Cava && !is_cava_available() {
                state.session.notification.current = Some(Notification {
                    kind: NotificationKind::Warning,
                    message: "CAVA is not available. Install CAVA for the CAVA visualizer style."
                        .to_string(),
                    progress: None,
                    created_at: Instant::now(),
                    dismiss_after: Some(Duration::from_secs(5)),
                });
                vec![]
            } else {
                state.session.spectrum_style = arg.clone();
                state.session.visualizer_mode = VisualizerMode::Spectrum;
                vec![AppMsg::Playback(PlaybackMsg::DispatchSetVisualizer {
                    spectrum_style: Some(arg),
                })]
            }
        }
        Command::Search { field, query } => {
            vec![AppMsg::Library(LibraryMsg::Search { field, query })]
        }
        Command::ClearSearch => {
            state.session.nav.library.search = Default::default();
            vec![]
        }
        Command::CreatePlaylist { name } => {
            vec![AppMsg::Library(LibraryMsg::RequestCreatePlaylist(name))]
        }
        Command::DeletePlaylist { name } => {
            vec![AppMsg::Library(LibraryMsg::RequestDeletePlaylist(name))]
        }
        Command::AddToPlaylist { name } => {
            // resolve selected track
            let track_id = match state.get_library_selection() {
                Some(SidebarNodeId::Track { id, .. }) => *id,
                _ => {
                    state.session.notification.current = Some(Notification {
                        kind: NotificationKind::Warning,
                        message: "Select a track first".to_string(),
                        progress: None,
                        created_at: Instant::now(),
                        dismiss_after: Some(Duration::from_secs(2)),
                    });
                    return vec![];
                }
            };

            // resolve playlist by name
            let playlist_id = match state.library.find_playlist_by_name(&name) {
                Some(id) => id,
                None => {
                    state.session.notification.current = Some(Notification {
                        kind: NotificationKind::Warning,
                        message: format!("Playlist '{}' not found", name),
                        progress: None,
                        created_at: Instant::now(),
                        dismiss_after: Some(Duration::from_secs(2)),
                    });
                    return vec![];
                }
            };

            vec![AppMsg::Library(LibraryMsg::RequestAddToPlaylist {
                playlist_id,
                track_id,
            })]
        }
        Command::Unknown(reason) => {
            state.session.notification.current = Some(Notification {
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

pub fn handle_dispatch_set_visualizer(
    _state: &mut AppState,
    _active: Option<SpectrumStyle>,
) -> Vec<AppMsg> {
    vec![]
}
