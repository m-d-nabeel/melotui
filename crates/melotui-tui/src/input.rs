use ratatui::crossterm::event::{KeyCode, KeyEvent};
use std::sync::OnceLock;

use melotui_core::{
    AppMsg, AppState,
    keybinding::{KeyAction, KeyBinding, KeyContext, KeyRegistry},
    message::{library::LibraryMsg, navigation::NavigationMsg, ui::UIMsg},
    state::{LeftPanelMode, LibraryViewDepth, UIMode},
};

static REGISTRY: OnceLock<KeyRegistry> = OnceLock::new();

pub struct InputHandler;

impl InputHandler {
    fn get_registry() -> &'static KeyRegistry {
        REGISTRY.get_or_init(KeyRegistry::default)
    }

    fn get_active_contexts(state: &AppState) -> Vec<KeyContext> {
        let mut contexts = Vec::with_capacity(3);

        // Help overlay takes priority
        if state.session.ui.show_help {
            contexts.push(KeyContext::Help);
            return contexts;
        }

        // Command mode
        if state.session.ui.mode == UIMode::Command {
            contexts.push(KeyContext::CommandMode);
            return contexts;
        }

        // Active panel
        match state.session.nav.active_panel {
            LeftPanelMode::Library => contexts.push(KeyContext::Library),
            LeftPanelMode::Queue => contexts.push(KeyContext::Queue),
        }

        // Global
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

        // Fallback for typing in command mode
        let Some(act) = action else {
            if state.session.ui.mode == UIMode::Command
                && let KeyCode::Char(c) = key.code
            {
                return Some(AppMsg::UI(UIMsg::UpdateCommand(c)));
            }
            return None;
        };

        match act {
            // Collapse view
            KeyAction::CollapseView => {
                if state.session.nav.active_panel == LeftPanelMode::Library {
                    match state.session.nav.library.depth {
                        LibraryViewDepth::Tracks => {
                            Some(AppMsg::Navigation(NavigationMsg::SwitchToPlaylists))
                        }
                        LibraryViewDepth::TrackInfo => {
                            Some(AppMsg::Navigation(NavigationMsg::SwitchToTracks))
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }

            // Expand view
            KeyAction::ExpandView => {
                if state.session.nav.active_panel == LeftPanelMode::Library {
                    match state.session.nav.library.depth {
                        LibraryViewDepth::Playlists => {
                            Some(AppMsg::Navigation(NavigationMsg::SwitchToTracks))
                        }
                        LibraryViewDepth::Tracks => {
                            Some(AppMsg::Navigation(NavigationMsg::SwitchToTrackInfo))
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }

            // Add to queue
            KeyAction::AddToQueue => {
                if state.session.nav.active_panel == LeftPanelMode::Library
                    && state.session.nav.library.depth != LibraryViewDepth::Playlists
                {
                    let p_idx = state.session.nav.library.selected_playlist_idx;
                    if let Some(playlist) = state.library.playlists.get(p_idx) {
                        let t_idx = *state
                            .session
                            .nav
                            .library
                            .selected_tracks_curs
                            .get(p_idx)
                            .unwrap_or(&0);
                        if let Some(track) = playlist.tracks.get(t_idx) {
                            return Some(AppMsg::Library(LibraryMsg::AddToQueue(track.clone())));
                        }
                    }
                }
                None
            }

            // Play next
            KeyAction::PlayNext => {
                if state.session.nav.active_panel == LeftPanelMode::Library
                    && state.session.nav.library.depth != LibraryViewDepth::Playlists
                {
                    let p_idx = state.session.nav.library.selected_playlist_idx;
                    if let Some(playlist) = state.library.playlists.get(p_idx) {
                        let t_idx = *state
                            .session
                            .nav
                            .library
                            .selected_tracks_curs
                            .get(p_idx)
                            .unwrap_or(&0);
                        if let Some(track) = playlist.tracks.get(t_idx) {
                            return Some(AppMsg::Library(LibraryMsg::PlayNext(track.clone())));
                        }
                    }
                }
                None
            }

            // Submit command
            KeyAction::SubmitCommand => Some(AppMsg::UI(UIMsg::SubmitCommand(
                state.session.ui.command_buffer.clone(),
            ))),

            // Direct mapping
            _ => act.to_msg(),
        }
    }

    pub fn get_help_docs() -> Vec<KeyBinding> {
        let registry = Self::get_registry();
        let mut all_docs = Vec::new();

        let mut add_context = |ctx: KeyContext| {
            if let Some(bindings) = registry.bindings.get(&ctx) {
                all_docs.extend(bindings.clone());
            }
        };

        // Collect bindings in order
        add_context(KeyContext::Global);
        add_context(KeyContext::Library);
        add_context(KeyContext::Queue);

        all_docs
    }
}
