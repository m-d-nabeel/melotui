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

pub fn handle_lyrics_selector_move_up(state: &mut AppState) -> Vec<AppMsg> {
    if let Some(ref mut search_state) = state.session.lyrics.search_state {
        search_state.selected_row = search_state.selected_row.saturating_sub(1);
    }
    vec![]
}

pub fn handle_lyrics_selector_move_down(state: &mut AppState) -> Vec<AppMsg> {
    if let Some(ref mut search_state) = state.session.lyrics.search_state {
        let max_rows = if search_state.selected_column == 0 {
            search_state.lrclib_results.len()
        } else {
            search_state.ovh_results.len()
        };
        if max_rows > 0 && search_state.selected_row < max_rows - 1 {
            search_state.selected_row += 1;
        }
    }
    vec![]
}

pub fn handle_lyrics_selector_move_left(state: &mut AppState) -> Vec<AppMsg> {
    if let Some(ref mut search_state) = state.session.lyrics.search_state {
        search_state.selected_column = 0;
        let max_rows = search_state.lrclib_results.len();
        if max_rows > 0 && search_state.selected_row >= max_rows {
            search_state.selected_row = max_rows - 1;
        } else if max_rows == 0 {
            search_state.selected_row = 0;
        }
    }
    vec![]
}

pub fn handle_lyrics_selector_move_right(state: &mut AppState) -> Vec<AppMsg> {
    if let Some(ref mut search_state) = state.session.lyrics.search_state {
        search_state.selected_column = 1;
        let max_rows = search_state.ovh_results.len();
        if max_rows > 0 && search_state.selected_row >= max_rows {
            search_state.selected_row = max_rows - 1;
        } else if max_rows == 0 {
            search_state.selected_row = 0;
        }
    }
    vec![]
}

pub fn handle_lyrics_selector_submit(state: &mut AppState) -> Vec<AppMsg> {
    if let Some(search_state) = state.session.lyrics.search_state.take() {
        let candidate = if search_state.selected_column == 0 {
            search_state
                .lrclib_results
                .get(search_state.selected_row)
                .cloned()
        } else {
            search_state
                .ovh_results
                .get(search_state.selected_row)
                .cloned()
        };

        if let Some(c) = candidate {
            // We dispatch FetchLyrics using the candidate's exact artist/track,
            // relying on our existing FetchLyrics logic which queries LRCLIB and OVH in order.
            // Wait, we need to guarantee it fetches the *exact* one.
            // Since we use the exact `track_name` and `artist_name`, both APIs will find it properly.
            if let Some(track) = state.get_playing_track() {
                let mut save_path = track.file_path.clone();
                save_path.set_extension("lrc");

                let provider = match c.api {
                    crate::state::lyrics::LyricsApi::Lrclib => "lrclib".to_string(),
                    crate::state::lyrics::LyricsApi::Ovh => "ovh".to_string(),
                };

                return vec![AppMsg::Download(
                    crate::message::download::DownloadMsg::RequestFetchLyrics {
                        track_name: c.track_name,
                        artist_name: c.artist_name,
                        save_path,
                        provider: Some(provider),
                        provider_id: c.provider_id,
                    },
                )];
            }
        }
    }
    vec![]
}

pub fn handle_lyrics_selector_cancel(state: &mut AppState) -> Vec<AppMsg> {
    state.session.lyrics.search_state = None;
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

pub fn handle_toggle_lyrics(state: &mut AppState) -> Vec<AppMsg> {
    state.session.ui.show_lyrics = !state.session.ui.show_lyrics;
    vec![]
}

pub fn handle_toggle_playback_panel(state: &mut AppState) -> Vec<AppMsg> {
    state.session.ui.show_playback = !state.session.ui.show_playback;
    vec![]
}

pub fn handle_enter_command_mode(state: &mut AppState) -> Vec<AppMsg> {
    tracing::debug!("Enter Command Mode");
    state.session.ui.mode = UIMode::Command;
    state.session.ui.command_buffer.reset();
    vec![]
}

pub fn handle_exit_command_mode(state: &mut AppState) -> Vec<AppMsg> {
    state.session.ui.mode = UIMode::Normal;
    state.session.ui.command_buffer.reset();
    state.session.ui.history_index = None;
    vec![]
}

pub fn handle_command_input(state: &mut AppState, req: InputRequest) -> Vec<AppMsg> {
    state.session.ui.autocomplete_cycle = 0;
    state.session.ui.autocomplete_prefix = None;
    state.session.ui.history_index = None;
    state.session.ui.command_buffer.handle(req);
    vec![]
}

pub fn handle_submit_command(state: &mut AppState, input: String) -> Vec<AppMsg> {
    tracing::info!("SubmitCommand called with :{:?}", input);

    // Save to history if it's not empty
    if !input.trim().is_empty() {
        state.session.ui.command_history.push(input.clone());
    }
    state.session.ui.history_index = None;

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
        Command::FetchLyrics { query } => {
            if let Some(track) = state.get_playing_track() {
                let (track_name, artist_name) = if let Some(q) = query {
                    if let Some((a, t)) = q.split_once('-') {
                        (t.trim().to_string(), a.trim().to_string())
                    } else {
                        (q, String::new())
                    }
                } else {
                    let t = track.title.clone();
                    let a = match track.artist.clone() {
                        Some(ar) if !ar.is_empty() => ar,
                        _ => String::new(),
                    };
                    (t, a)
                };

                if track_name.is_empty() {
                    state.session.notification.current = Some(Notification::new(
                        NotificationKind::Warning,
                        "Cannot fetch lyrics: missing track name",
                    ));
                    return vec![];
                }

                let mut save_path = track.file_path.clone();
                save_path.set_extension("lrc");

                state.session.notification.current = Some(Notification::new(
                    NotificationKind::Info,
                    format!("Fetching lyrics for {}...", track_name),
                ));
                vec![AppMsg::Download(DownloadMsg::RequestFetchLyrics {
                    track_name,
                    artist_name,
                    save_path,
                    provider: None,
                    provider_id: None,
                })]
            } else {
                state.session.notification.current = Some(Notification::new(
                    NotificationKind::Warning,
                    "No track is currently playing",
                ));
                vec![]
            }
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
        Command::SearchLyrics { query } => {
            vec![AppMsg::Download(DownloadMsg::RequestSearchLyrics { query })]
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

pub fn handle_autocomplete(state: &mut AppState) -> Vec<AppMsg> {
    if state.session.ui.autocomplete_prefix.is_none() {
        state.session.ui.autocomplete_prefix =
            Some(state.session.ui.command_buffer.value().to_string());
    }

    let prefix = state.session.ui.autocomplete_prefix.as_ref().unwrap();

    let (suggestion, _) = crate::autocomplete::suggest(prefix, state.session.ui.autocomplete_cycle);
    if let Some(suggestion) = suggestion {
        state.session.ui.command_buffer =
            tui_input::Input::new(suggestion.clone()).with_cursor(suggestion.len());
        state.session.ui.autocomplete_cycle += 1;
    }
    vec![]
}

pub fn handle_autocomplete_reverse(state: &mut AppState) -> Vec<AppMsg> {
    if state.session.ui.autocomplete_prefix.is_none() {
        state.session.ui.autocomplete_prefix =
            Some(state.session.ui.command_buffer.value().to_string());
    }

    let prefix = state.session.ui.autocomplete_prefix.as_ref().unwrap();

    let (_, count) = crate::autocomplete::suggest(prefix, 0);
    if count > 0 {
        let current_cycle = state.session.ui.autocomplete_cycle;

        let prev_index = if count == 1 {
            0
        } else if (current_cycle % count) < 2 {
            (current_cycle % count) + count - 2
        } else {
            (current_cycle % count) - 2
        };

        let new_cycle = if count == 1 {
            0
        } else if (current_cycle % count) < 1 {
            (current_cycle % count) + count - 1
        } else {
            (current_cycle % count) - 1
        };

        state.session.ui.autocomplete_cycle = new_cycle;

        let (suggestion, _) = crate::autocomplete::suggest(prefix, prev_index);
        if let Some(suggestion) = suggestion {
            state.session.ui.command_buffer =
                tui_input::Input::new(suggestion.clone()).with_cursor(suggestion.len());
        }
    }
    vec![]
}

pub fn handle_command_history_prev(state: &mut AppState) -> Vec<AppMsg> {
    let entries = state.session.ui.command_history.entries();
    if entries.is_empty() {
        return vec![];
    }

    let mut idx = state.session.ui.history_index.unwrap_or(entries.len());
    if idx > 0 {
        idx -= 1;
        state.session.ui.history_index = Some(idx);
        let cmd = entries[idx].clone();
        state.session.ui.command_buffer = tui_input::Input::new(cmd.clone()).with_cursor(cmd.len());
    }
    vec![]
}

pub fn handle_command_history_next(state: &mut AppState) -> Vec<AppMsg> {
    let entries = state.session.ui.command_history.entries();
    if entries.is_empty() {
        return vec![];
    }

    if let Some(mut idx) = state.session.ui.history_index {
        if idx + 1 < entries.len() {
            idx += 1;
            state.session.ui.history_index = Some(idx);
            let cmd = entries[idx].clone();
            state.session.ui.command_buffer =
                tui_input::Input::new(cmd.clone()).with_cursor(cmd.len());
        } else {
            state.session.ui.history_index = None;
            state.session.ui.command_buffer = tui_input::Input::new("".to_string());
        }
    }
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

    #[test]
    fn test_handle_autocomplete_cycle() {
        let mut state = test_state();

        // Initial input "pl" has matches: "pl add", "pl create", "pl del", "pl rm", "playlist add", "playlist create", "playlist del", "playlist rm"
        state.session.ui.command_buffer = tui_input::Input::new("pl".to_string());

        // Forward 1: cycle 0 -> 1. Should show "pl add "
        handle_autocomplete(&mut state);
        assert_eq!(state.session.ui.autocomplete_cycle, 1);
        assert_eq!(state.session.ui.autocomplete_prefix.as_deref(), Some("pl"));
        let val1 = state.session.ui.command_buffer.value().to_string();
        assert_eq!(val1, "pl add ");

        // Forward 2: cycle 1 -> 2. Should show "pl create "
        handle_autocomplete(&mut state);
        assert_eq!(state.session.ui.autocomplete_cycle, 2);
        assert_eq!(state.session.ui.autocomplete_prefix.as_deref(), Some("pl"));
        let val2 = state.session.ui.command_buffer.value().to_string();
        assert_eq!(val2, "pl create ");

        // Reverse 1: cycle 2 -> 1. Should show "pl add " again
        handle_autocomplete_reverse(&mut state);
        assert_eq!(state.session.ui.autocomplete_cycle, 1);
        assert_eq!(state.session.ui.autocomplete_prefix.as_deref(), Some("pl"));
        let val3 = state.session.ui.command_buffer.value().to_string();
        assert_eq!(val3, "pl add ");

        // Reverse 2: cycle 1 -> 0. Should wrap around backwards to the LAST element, "playlist rm "
        handle_autocomplete_reverse(&mut state);
        assert_eq!(state.session.ui.autocomplete_cycle, 0); // 0 or whatever the last element's cycle is... actually wait! If length is 8, reverse from 1 goes to 0 (which shows the last element). Wait! If cycle is 0, the next is 0. We should test that it returns the last element.
        let val4 = state.session.ui.command_buffer.value().to_string();
        assert_eq!(val4, "playlist rm ");

        // Clear prefix explicitly
        use tui_input::InputRequest;
        crate::action::ui_action::handle_command_input(&mut state, InputRequest::InsertChar('x'));
        assert_eq!(state.session.ui.autocomplete_cycle, 0);
        assert_eq!(state.session.ui.autocomplete_prefix, None);
    }

    #[test]
    fn test_handle_command_history_integration() {
        let mut state = test_state();
        state.session.ui.command_history = crate::history::CommandHistory::default();

        handle_submit_command(&mut state, "pl create A".into());
        handle_submit_command(&mut state, "pl create B".into());

        assert_eq!(state.session.ui.history_index, None);

        handle_command_history_prev(&mut state);
        assert_eq!(state.session.ui.history_index, Some(1));
        assert_eq!(state.session.ui.command_buffer.value(), "pl create B");

        handle_command_history_prev(&mut state);
        assert_eq!(state.session.ui.history_index, Some(0));
        assert_eq!(state.session.ui.command_buffer.value(), "pl create A");

        handle_command_history_next(&mut state);
        assert_eq!(state.session.ui.history_index, Some(1));
        assert_eq!(state.session.ui.command_buffer.value(), "pl create B");

        handle_command_history_next(&mut state);
        assert_eq!(state.session.ui.history_index, None);
        assert_eq!(state.session.ui.command_buffer.value(), "");
    }
}
