use crate::{
    AppMsg, AppState, Command, UIMode,
    domain::PlaylistMeta,
    message::{download::DownloadMsg, library::LibraryMsg, playback::PlaybackMsg, ui::UIMsg},
    navigation::ids::SidebarNodeId,
    state::{
        Notification, NotificationKind, VisualizerMode,
        ui::{ConfirmDialog, FocusMode, PlaylistPickerState},
        visualizer::{SpectrumStyle, is_cava_available},
    },
};
use tui_input::InputRequest;

pub fn handle_open_help(state: &mut AppState) -> Vec<AppMsg> {
    state.session.ui.show_help = true;
    vec![]
}

pub fn handle_show_confirm(
    state: &mut AppState,
    message: String,
    on_confirm: Box<AppMsg>,
) -> Vec<AppMsg> {
    state.session.ui.mode = UIMode::Confirm;
    state.session.ui.confirm_dialog = Some(ConfirmDialog {
        message,
        on_confirm,
    });
    vec![]
}

pub fn handle_confirm_yes(state: &mut AppState) -> Vec<AppMsg> {
    let action = state
        .session
        .ui
        .confirm_dialog
        .take()
        .map(|d| *d.on_confirm);
    state.session.ui.mode = UIMode::Normal;
    match action {
        Some(msg) => vec![msg],
        None => vec![],
    }
}

pub fn handle_confirm_no(state: &mut AppState) -> Vec<AppMsg> {
    state.session.ui.confirm_dialog = None;
    state.session.ui.mode = UIMode::Normal;
    vec![]
}

pub fn handle_delete_playlist_confirm(state: &mut AppState) -> Vec<AppMsg> {
    let pid = match state.get_library_selection() {
        Some(SidebarNodeId::Playlist(id)) => *id,
        _ => return vec![],
    };
    let name = match state.library.get_playlist(pid) {
        Some(p) => p.name.clone(),
        None => return vec![],
    };
    handle_show_confirm(
        state,
        format!("Delete playlist \"{}\"?", name),
        Box::new(AppMsg::Library(LibraryMsg::RequestDeletePlaylist(name))),
    )
}

pub fn handle_enter_playlist_picker(state: &mut AppState) -> Vec<AppMsg> {
    let track_id = match state.get_library_selection() {
        Some(SidebarNodeId::Track { id, .. }) => *id,
        _ => {
            state.session.notification.current = Some(Notification::new(
                NotificationKind::Warning,
                "Select a track first",
            ));
            return vec![];
        }
    };

    state.session.ui.mode = UIMode::PlaylistPicker;
    state.session.ui.playlist_picker = PlaylistPickerState {
        query: String::new(),
        selection: 0,
        scroll_offset: 0,
        track_id: Some(track_id),
    };
    vec![]
}

pub fn handle_exit_playlist_picker(state: &mut AppState) -> Vec<AppMsg> {
    state.session.ui.mode = UIMode::Normal;
    state.session.ui.playlist_picker = Default::default();
    vec![]
}

pub fn handle_playlist_picker_move_up(state: &mut AppState) -> Vec<AppMsg> {
    let picker = &mut state.session.ui.playlist_picker;
    picker.selection = picker.selection.saturating_sub(1);
    if picker.selection < picker.scroll_offset {
        picker.scroll_offset = picker.selection;
    }
    vec![]
}

pub fn handle_playlist_picker_move_down(state: &mut AppState) -> Vec<AppMsg> {
    let max_idx = picker_max_index(state);
    let picker = &mut state.session.ui.playlist_picker;
    if picker.selection < max_idx {
        picker.selection += 1;
    }
    let viewport = state.config.ui.playlist_picker_max_visible;
    if picker.selection >= picker.scroll_offset + viewport {
        picker.scroll_offset = picker.selection + 1 - viewport;
    }
    vec![]
}

pub fn handle_playlist_picker_update_query(state: &mut AppState, c: char) -> Vec<AppMsg> {
    state.session.ui.playlist_picker.query.push(c);
    state.session.ui.playlist_picker.selection = 0;
    state.session.ui.playlist_picker.scroll_offset = 0;
    vec![]
}

pub fn handle_playlist_picker_backspace(state: &mut AppState) -> Vec<AppMsg> {
    state.session.ui.playlist_picker.query.pop();
    state.session.ui.playlist_picker.selection = 0;
    state.session.ui.playlist_picker.scroll_offset = 0;
    vec![]
}

/// total item count for the filtered picker list (including the "Create" option)
fn picker_total_items(state: &AppState) -> usize {
    let query = state.session.ui.playlist_picker.query.trim().to_lowercase();
    let filtered = state
        .library
        .playlist_order
        .iter()
        .filter_map(|pid| state.library.playlists.get(pid))
        .filter(|p| p.name.to_lowercase().contains(&query))
        .count();

    let exact = state
        .library
        .playlist_order
        .iter()
        .filter_map(|pid| state.library.playlists.get(pid))
        .any(|p| p.name.to_lowercase() == query);

    if !query.is_empty() && !exact {
        filtered + 1
    } else {
        filtered
    }
}

fn picker_max_index(state: &AppState) -> usize {
    picker_total_items(state).saturating_sub(1)
}

pub fn handle_submit_playlist_picker(state: &mut AppState) -> Vec<AppMsg> {
    let picker = &state.session.ui.playlist_picker;
    let track_id = match picker.track_id {
        Some(id) => id,
        None => return vec![], // should not happen
    };

    let query = picker.query.trim();

    let lower_query = query.to_lowercase();
    let playlists: Vec<&PlaylistMeta> = state
        .library
        .playlist_order
        .iter()
        .filter_map(|pid| state.library.playlists.get(pid))
        .filter(|p| p.name.to_lowercase().contains(&lower_query))
        .collect();

    // selection logic:
    // in bounds: picked existing
    // out of bounds/empty: create new

    let mut msgs = vec![];

    if picker.selection < playlists.len() {
        // picked existing
        let playlist = playlists[picker.selection];
        msgs.push(AppMsg::Library(LibraryMsg::RequestAddToPlaylist {
            playlist_id: playlist.id,
            track_id,
        }));
    } else if !query.is_empty() {
        // create playlist then add track once ID arrives
        msgs.push(AppMsg::Library(LibraryMsg::RequestCreatePlaylist(
            query.to_string(),
        )));
        state.session.ui.pending_add_track = Some(track_id);
    }

    // close picker
    msgs.push(AppMsg::UI(UIMsg::ExitPlaylistPicker));

    state.session.ui.mode = UIMode::Normal;
    state.session.ui.playlist_picker = Default::default();

    msgs
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
    state.session.ui.command_buffer.reset();
    vec![]
}

pub fn handle_exit_command_mode(state: &mut AppState) -> Vec<AppMsg> {
    tracing::debug!("Exit Command Mode");
    state.session.ui.mode = UIMode::Normal;
    state.session.ui.command_buffer.reset();
    vec![]
}

pub fn handle_command_input(state: &mut AppState, req: InputRequest) -> Vec<AppMsg> {
    state.session.ui.command_buffer.handle(req);
    vec![]
}

pub fn handle_submit_command(state: &mut AppState, input: String) -> Vec<AppMsg> {
    tracing::info!("SubmitCommand called with :{:?}", input);
    process_command_input(state, input)
}

fn process_command_input(state: &mut AppState, input: String) -> Vec<AppMsg> {
    state.session.ui.mode = UIMode::Normal;
    state.session.ui.command_buffer.reset();

    let input = input.trim();
    if input.is_empty() {
        return vec![];
    }

    match Command::parse(input) {
        Command::SeekRelative(offset) => {
            vec![AppMsg::Playback(PlaybackMsg::RequestSeekRelative(offset))]
        }
        Command::SeekAbsolute(pos) => {
            vec![AppMsg::Playback(PlaybackMsg::RequestSeekAbsolute(pos))]
        }
        Command::Quit => {
            vec![AppMsg::RequestQuit]
        }
        Command::Rescan => {
            vec![AppMsg::Library(LibraryMsg::RequestScanLibrary)]
        }
        Command::Download { query } => {
            state.session.notification.current = Some(Notification::new(
                NotificationKind::Info,
                format!("Downloading: {}", query),
            ));
            vec![AppMsg::Download(DownloadMsg::RequestDownload { query })]
        }
        Command::ChangeVisualizer { arg } => {
            if arg == SpectrumStyle::Cava && !is_cava_available() {
                state.session.notification.current = Some(
                    Notification::new(
                        NotificationKind::Warning,
                        "CAVA is not available. Install CAVA for the CAVA visualizer style.",
                    )
                    .with_duration(5),
                );
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
                    state.session.notification.current = Some(Notification::new(
                        NotificationKind::Warning,
                        "Select a track first",
                    ));
                    return vec![];
                }
            };

            let playlist_id = match state.library.find_playlist_by_name(&name) {
                Some(id) => id,
                None => {
                    state.session.notification.current = Some(Notification::new(
                        NotificationKind::Warning,
                        format!("Playlist '{}' not found", name),
                    ));
                    return vec![];
                }
            };

            vec![AppMsg::Library(LibraryMsg::RequestAddToPlaylist {
                playlist_id,
                track_id,
            })]
        }
        Command::RemoveFromPlaylist { name } => {
            let track_id = match state.get_library_selection() {
                Some(SidebarNodeId::Track { id, .. }) => *id,
                _ => {
                    state.session.notification.current = Some(Notification::new(
                        NotificationKind::Warning,
                        "Select a track first",
                    ));
                    return vec![];
                }
            };

            let playlist_id = match state.library.find_playlist_by_name(&name) {
                Some(id) => id,
                None => {
                    state.session.notification.current = Some(Notification::new(
                        NotificationKind::Warning,
                        format!("Playlist '{}' not found", name),
                    ));
                    return vec![];
                }
            };

            vec![AppMsg::Library(LibraryMsg::RequestRemoveFromPlaylist {
                playlist_id,
                track_id,
            })]
        }
        Command::Unknown(reason) => {
            state.session.notification.current = Some(Notification::new(
                NotificationKind::Error,
                format!("Unknown command: {}", reason),
            ));
            vec![]
        }
        Command::Focus { mode } => handle_set_focus_mode(state, mode),
        Command::Sidebar { width } => handle_set_sidebar_width(state, width),
    }
}

pub fn handle_dispatch_set_visualizer(
    _state: &mut AppState,
    _active: Option<SpectrumStyle>,
) -> Vec<AppMsg> {
    vec![]
}

pub fn handle_cycle_focus_mode(state: &mut AppState) -> Vec<AppMsg> {
    let next = state.session.ui.focus_mode.cycle();
    state.session.ui.focus_mode = next;
    state.session.notification.current = Some(
        Notification::new(NotificationKind::Info, format!("Focus: {}", next.label()))
            .with_duration(2),
    );
    vec![]
}

pub fn handle_set_focus_mode(state: &mut AppState, mode: FocusMode) -> Vec<AppMsg> {
    state.session.ui.focus_mode = mode;
    state.session.notification.current = Some(
        Notification::new(NotificationKind::Info, format!("Focus: {}", mode.label()))
            .with_duration(2),
    );
    vec![]
}

pub fn handle_set_sidebar_width(state: &mut AppState, width: u16) -> Vec<AppMsg> {
    state.session.ui.sidebar_width = width;
    state.session.notification.current = Some(
        Notification::new(NotificationKind::Info, format!("Sidebar width: {}", width))
            .with_duration(2),
    );
    vec![]
}

pub fn handle_adjust_sidebar_width(state: &mut AppState, delta: i16) -> Vec<AppMsg> {
    let current = state.session.ui.sidebar_width as i16;
    let term_w = state.terminal_size().0 as i16;

    let mut new_width = current + delta;

    if delta < 0 {
        if current > 80 {
            new_width = 80;
        } else if new_width < 28 {
            new_width = 0;
        }
    } else if delta > 0 {
        if current == 0 {
            new_width = 28;
        } else if new_width > 80 {
            new_width = term_w;
        }
    }

    new_width = new_width.clamp(0, term_w.max(80));
    state.session.ui.sidebar_width = new_width as u16;
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::snapshot::Library;
    use std::collections::HashMap;

    fn test_state() -> AppState {
        let root = std::path::PathBuf::from("/tmp");
        let library = Library::new(vec![root], vec![], HashMap::new());
        AppState::new(library, &crate::config::AppConfig::default())
    }

    #[test]
    fn test_handle_open_help() {
        let mut state = test_state();
        assert!(!state.session.ui.show_help);

        let effects = handle_open_help(&mut state);
        assert!(state.session.ui.show_help);
        assert!(effects.is_empty());
    }

    #[test]
    fn test_handle_command_mode() {
        let mut state = test_state();
        assert_eq!(state.session.ui.mode, UIMode::Normal);

        let effects = handle_enter_command_mode(&mut state);
        assert_eq!(state.session.ui.mode, UIMode::Command);
        assert!(effects.is_empty());

        let effects = handle_exit_command_mode(&mut state);
        assert_eq!(state.session.ui.mode, UIMode::Normal);
        assert!(effects.is_empty());
    }

    #[test]
    fn test_handle_submit_command_quit() {
        let mut state = test_state();
        state.session.ui.mode = UIMode::Command;

        let effects = handle_submit_command(&mut state, "quit".to_string());

        assert_eq!(
            state.session.ui.mode,
            UIMode::Normal,
            "Should return to normal mode"
        );
        assert_eq!(effects.len(), 1);
        assert!(matches!(effects[0], AppMsg::RequestQuit));
    }

    #[test]
    fn test_handle_submit_command_unknown() {
        let mut state = test_state();
        state.session.ui.mode = UIMode::Command;

        let effects = handle_submit_command(&mut state, "invalidcmd".to_string());

        assert_eq!(state.session.ui.mode, UIMode::Normal);
        assert!(effects.is_empty());
        assert!(matches!(
            state.current_notification().unwrap().kind,
            NotificationKind::Error
        ));
    }
}
