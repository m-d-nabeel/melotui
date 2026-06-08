use ratatui::crossterm::event::{KeyCode, KeyEvent};
use std::sync::OnceLock;

use melotui_core::{
    AppMsg, AppState,
    keybinding::{KeyAction, KeyContext, KeyRegistry},
    message::{
        download::DownloadMsg, library::LibraryMsg, navigation::NavigationMsg,
        playback::PlaybackMsg, ui::UIMsg,
    },
    navigation::ids::SidebarNodeId,
    state::UIMode,
};

static REGISTRY: OnceLock<KeyRegistry> = OnceLock::new();

pub struct InputHandler;

impl InputHandler {
    pub fn get_registry() -> &'static KeyRegistry {
        REGISTRY.get_or_init(KeyRegistry::default)
    }

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

    pub fn handle(key: KeyEvent, state: &AppState) -> Option<AppMsg> {
        let registry = Self::get_registry();
        let contexts = Self::get_active_contexts(state);

        let mut action = None;
        'ctx_loop: for ctx in contexts {
            if let Some(bindings) = registry.bindings.get(&ctx) {
                for binding in bindings {
                    if binding.key == key.code && binding.modifiers == key.modifiers {
                        action = Some(binding.action);
                        break 'ctx_loop;
                    }
                }
            }
        }

        // command mode typing fallback
        let Some(act) = action else {
            if *state.ui_mode() == UIMode::Command {
                return Some(AppMsg::UI(UIMsg::CommandInput(key)));
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
            KeyAction::ToggleTreeExpand => {
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
            _ => act.to_msg(),
        }
    }
}
