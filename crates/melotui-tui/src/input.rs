use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use tui_input::backend::crossterm::to_input_request;

use crate::keybinding::{KeyAction, KeyContext, KeyRegistry};
use melotui_core::{
    AppMsg, AppState,
    message::{
        download::DownloadMsg, library::LibraryMsg, navigation::NavigationMsg,
        playback::PlaybackMsg, ui::UIMsg,
    },
    navigation::ids::SidebarNodeId,
    state::UIMode,
};

pub struct InputHandler;

impl InputHandler {
    fn get_active_contexts(state: &AppState) -> Vec<KeyContext> {
        let mut contexts = Vec::with_capacity(3);

        // priority overlays
        if state.ui_show_help() {
            contexts.push(KeyContext::Help);
            return contexts;
        }

        if *state.ui_mode() == UIMode::Command {
            contexts.push(KeyContext::CommandMode);
            return contexts;
        }

        if *state.ui_mode() == UIMode::PlaylistPicker {
            contexts.push(KeyContext::PlaylistPicker);
            return contexts;
        }

        if *state.ui_mode() == UIMode::Confirm {
            contexts.push(KeyContext::Confirm);
            return contexts;
        }

        if state.is_showing_track_info() {
            contexts.push(KeyContext::TrackInfo);
            contexts.push(KeyContext::Global);
            return contexts;
        }

        if state.lyrics_search_state().is_some() {
            contexts.push(KeyContext::LyricsSelector);
            return contexts; // modal, blocks other inputs
        }

        // panel contexts
        if state.is_library_focused() {
            contexts.push(KeyContext::Library);
        } else if state.is_queue_focused() {
            contexts.push(KeyContext::Queue);
        }

        // global
        contexts.push(KeyContext::Global);
        contexts
    }

    pub fn handle(key: KeyEvent, state: &AppState, registry: &KeyRegistry) -> Option<AppMsg> {
        if key.kind == KeyEventKind::Release {
            return None;
        }

        let contexts = Self::get_active_contexts(state);

        let mut normalized_code = key.code;
        if let KeyCode::Char(c) = normalized_code
            && key
                .modifiers
                .contains(ratatui::crossterm::event::KeyModifiers::SHIFT)
            && c.is_ascii_uppercase()
        {
            normalized_code = KeyCode::Char(c.to_ascii_lowercase());
        }

        let mut action = None;
        'ctx_loop: for ctx in contexts {
            if let Some(bindings) = registry.bindings.get(&ctx) {
                for binding in bindings {
                    if binding.key == normalized_code && binding.modifiers == key.modifiers {
                        action = Some(binding.action);
                        break 'ctx_loop;
                    }
                }
            }
        }

        // command mode typing fallback
        let Some(act) = action else {
            if *state.ui_mode() == UIMode::Command {
                let evt = Event::Key(key);
                if let Some(req) = to_input_request(&evt) {
                    return Some(AppMsg::UI(UIMsg::CommandInput(req)));
                }
                return None;
            }
            if *state.ui_mode() == UIMode::PlaylistPicker
                && let KeyCode::Char(c) = key.code
            {
                return Some(AppMsg::UI(UIMsg::PlaylistPickerUpdateQuery(c)));
            }
            return None;
        };

        match act {
            // play selected or toggle expand
            KeyAction::Interact => {
                if state.is_library_focused() {
                    if state.get_selected_track_id_in_sidebar().is_some() {
                        Some(AppMsg::Playback(PlaybackMsg::RequestPlaySelected))
                    } else {
                        Some(AppMsg::Navigation(NavigationMsg::ToggleTreeExpand))
                    }
                } else if state.is_queue_focused()
                    && state.get_selected_track_id_in_queue().is_some()
                {
                    Some(AppMsg::Playback(PlaybackMsg::RequestPlaySelected))
                } else {
                    None
                }
            }

            // add to queue
            KeyAction::AddToQueue => {
                if state.is_library_focused()
                    && let Some(track_id) = state.get_selected_track_id_in_sidebar()
                {
                    return Some(AppMsg::Library(LibraryMsg::AddToQueue(track_id)));
                }
                None
            }

            // play next
            KeyAction::QueuePlayNext => {
                if state.is_library_focused()
                    && let Some(track_id) = state.get_selected_track_id_in_sidebar()
                {
                    return Some(AppMsg::Library(LibraryMsg::PlayNext(track_id)));
                }
                None
            }

            // toggle favorite
            KeyAction::ToggleFavorite => {
                if state.is_library_focused()
                    && let Some(track_id) = state.get_selected_track_id_in_sidebar()
                {
                    return Some(AppMsg::Library(LibraryMsg::RequestToggleFavorite(track_id)));
                }
                None
            }

            KeyAction::RemoveFromPlaylist => {
                if state.is_library_focused()
                    && let Some(SidebarNodeId::Track {
                        id: track_id,
                        parent,
                    }) = state.get_library_selection()
                    && let SidebarNodeId::Playlist(playlist_id) = **parent
                {
                    return Some(AppMsg::Library(LibraryMsg::RequestRemoveFromPlaylist {
                        playlist_id,
                        track_id: *track_id,
                    }));
                }
                None
            }

            // search cycling - only when search has results
            KeyAction::SearchNext => {
                if !state.search_matches().is_empty() {
                    return Some(AppMsg::Library(LibraryMsg::SearchNext));
                }
                None
            }
            KeyAction::SearchPrev => {
                if !state.search_matches().is_empty() {
                    return Some(AppMsg::Library(LibraryMsg::SearchPrev));
                }
                None
            }

            // Esc: cancel download first, then clear search
            KeyAction::CancelDownload => {
                if state.is_download_active() {
                    return Some(AppMsg::Download(DownloadMsg::RequestCancelDownload));
                }
                if !state.search_matches().is_empty() {
                    return Some(AppMsg::Library(LibraryMsg::ClearSearch));
                }
                None
            }

            // submit command
            KeyAction::SubmitCommand => Some(AppMsg::UI(UIMsg::SubmitCommand(
                state.ui_command_buffer().value().to_string(),
            ))),

            // direct mapping
            _ => act.to_msg(&state.config),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use melotui_core::{library::snapshot::Library, reducer::reduce};
    use ratatui::crossterm::event::KeyModifiers;

    fn test_state() -> AppState {
        let root = std::path::PathBuf::from("/tmp");
        let library = Library::new(vec![root], vec![], std::collections::HashMap::new());
        AppState::new(library, &melotui_core::config::AppConfig::default())
    }

    #[test]
    fn test_help_input_override() {
        let mut state = test_state();
        // Mutate state using reduce
        reduce(&mut state, AppMsg::UI(UIMsg::OpenHelp));

        // When help is shown, escape or 'q' closes it
        let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
        let registry = crate::keybinding::KeyRegistry::default();
        let msg = InputHandler::handle(esc_key, &state, &registry);

        assert!(matches!(msg, Some(AppMsg::UI(UIMsg::CloseHelp))));
    }

    #[test]
    fn test_command_mode_input() {
        let mut state = test_state();
        reduce(&mut state, AppMsg::UI(UIMsg::EnterCommandMode));

        let char_key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
        let registry = crate::keybinding::KeyRegistry::default();
        let msg = InputHandler::handle(char_key, &state, &registry);

        // Should dispatch as CommandInput
        assert!(matches!(msg, Some(AppMsg::UI(UIMsg::CommandInput(_)))));

        // Submit command on Enter
        let enter_key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        let registry = crate::keybinding::KeyRegistry::default();
        let msg = InputHandler::handle(enter_key, &state, &registry);

        assert!(matches!(msg, Some(AppMsg::UI(UIMsg::SubmitCommand(_)))));
    }

    #[test]
    fn test_playlist_picker_input() {
        use melotui_core::command::SearchField;
        use melotui_core::domain::{TrackMeta, ids::TrackId};

        let mut tracks = std::collections::HashMap::new();
        let track_id = TrackId(1);
        tracks.insert(
            track_id,
            TrackMeta {
                id: track_id,
                file_path: std::path::PathBuf::from("/tmp/test.mp3"),
                title: "Test Track".into(),
                artist: None,
                album: None,
                genre: None,
                year: None,
                is_favorite: false,
                track_number: None,
                duration_ms: 10000,
                bitrate: None,
                sample_rate: None,
                format: "mp3".into(),
                has_cover: false,
                file_size_bytes: 1024,
            },
        );

        let library = Library::new(vec![std::path::PathBuf::from("/tmp")], vec![], tracks);
        let mut state = AppState::new(library, &melotui_core::config::AppConfig::default());

        // Ensure a track is selected using Search
        reduce(
            &mut state,
            AppMsg::Library(LibraryMsg::Search {
                field: SearchField::Song,
                query: "Test".to_string(),
            }),
        );

        // Open the playlist picker
        reduce(&mut state, AppMsg::UI(UIMsg::EnterPlaylistPicker));

        // Typing updates query
        let char_key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::empty());
        let registry = crate::keybinding::KeyRegistry::default();
        let msg = InputHandler::handle(char_key, &state, &registry);

        assert!(matches!(
            msg,
            Some(AppMsg::UI(UIMsg::PlaylistPickerUpdateQuery('x')))
        ));

        // Enter submits picker
        let enter_key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        let registry = crate::keybinding::KeyRegistry::default();
        let msg = InputHandler::handle(enter_key, &state, &registry);

        assert!(matches!(msg, Some(AppMsg::UI(UIMsg::SubmitPlaylistPicker))));
    }

    #[test]
    fn test_global_playback_controls() {
        let state = test_state();

        let space_key = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::empty());
        let registry = crate::keybinding::KeyRegistry::default();
        let msg = InputHandler::handle(space_key, &state, &registry);

        assert!(matches!(
            msg,
            Some(AppMsg::Playback(PlaybackMsg::RequestTogglePlayback))
        ));
    }
}
