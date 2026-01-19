use ratatui::crossterm::event::{KeyCode, KeyEvent};
use std::sync::OnceLock;

use melotui_core::{
    AppMsg, AppState,
    keybinding::{KeyAction, KeyBinding, KeyContext, KeyRegistry},
    message::{library::LibraryMsg, navigation::NavigationMsg, ui::UIMsg},
    state::UIMode,
};

static REGISTRY: OnceLock<KeyRegistry> = OnceLock::new();

pub struct InputHandler;

impl InputHandler {
    fn get_registry() -> &'static KeyRegistry {
        REGISTRY.get_or_init(KeyRegistry::default)
    }

    fn get_active_contexts(state: &AppState) -> Vec<KeyContext> {
        let mut contexts = Vec::with_capacity(3);

        // 1. Priority Overlays
        if state.ui_show_help() {
            contexts.push(KeyContext::Help);
            return contexts;
        }

        if *state.ui_mode() == UIMode::Command {
            contexts.push(KeyContext::CommandMode);
            return contexts;
        }

        // 2. Panel Contexts
        if state.is_library_focused() {
            contexts.push(KeyContext::Library);
        } else if state.is_queue_focused() {
            contexts.push(KeyContext::Queue);
        }

        // 3. Global
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

        // Fallback: Typing in command mode
        let Some(act) = action else {
            if *state.ui_mode() == UIMode::Command
                && let KeyCode::Char(c) = key.code
            {
                return Some(AppMsg::UI(UIMsg::UpdateCommand(c)));
            }
            return None;
        };

        match act {
            // H Key (Back / Collapse)
            KeyAction::CollapseView => {
                if state.is_library_focused() {
                    Some(AppMsg::Navigation(NavigationMsg::SwitchToPlaylists))
                } else if state.is_queue_focused() && state.is_queue_info_view() {
                    Some(AppMsg::Navigation(NavigationMsg::SwitchToTracks))
                } else {
                    None
                }
            }

            // L Key / Enter (Expand / Select)
            KeyAction::ExpandView => {
                if state.is_library_focused() {
                    Some(AppMsg::Navigation(NavigationMsg::SwitchToTracks))
                } else if state.is_queue_focused() && !state.is_queue_info_view() {
                    Some(AppMsg::Navigation(NavigationMsg::SwitchToTrackInfo))
                } else {
                    None
                }
            }

            // A Key (Add to Queue)
            KeyAction::AddToQueue => {
                if state.is_library_focused() {
                    if let Some(track_id) = state.get_selected_track_id_in_sidebar() {
                        return Some(AppMsg::Library(LibraryMsg::AddToQueue(track_id)));
                    }
                }
                None
            }

            // Shift+A or similar (Play Next)
            KeyAction::PlayNext => {
                if state.is_library_focused() {
                    if let Some(track_id) = state.get_selected_track_id_in_sidebar() {
                        return Some(AppMsg::Library(LibraryMsg::PlayNext(track_id)));
                    }
                }
                None
            }

            // Submit Command
            KeyAction::SubmitCommand => Some(AppMsg::UI(UIMsg::SubmitCommand(
                state.ui_command_buffer().clone(),
            ))),

            // Direct mapping for other keys (Play, Pause, Volume, etc.)
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

        add_context(KeyContext::Global);
        add_context(KeyContext::Library);
        add_context(KeyContext::Queue);

        all_docs
    }
}
