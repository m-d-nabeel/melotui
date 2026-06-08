#[cfg(test)]
mod tests {
    use crate::{
        app_state::AppState,
        message::{AppMsg, ui::UIMsg},
        reducer::reduce,
    };

    fn test_state() -> AppState {
        use std::collections::HashMap;
        let root = std::path::PathBuf::from("/tmp");
        let library = crate::library::snapshot::LibrarySnapshot::new(root, vec![], HashMap::new());
        AppState::new(library)
    }

    #[test]
    fn test_reduce_noop() {
        let mut state = test_state();
        let effects = reduce(&mut state, AppMsg::Noop);
        assert!(effects.is_empty(), "Noop should produce no effects");
    }

    #[test]
    fn test_reduce_request_quit() {
        let mut state = test_state();
        let effects = reduce(&mut state, AppMsg::RequestQuit);
        
        assert!(effects.is_empty(), "RequestQuit should produce no side effects");
        assert_eq!(state.current_notification().unwrap().message, "Quit");
    }

    #[test]
    fn test_reduce_ui_msg() {
        let mut state = test_state();
        let effects = reduce(&mut state, AppMsg::UI(UIMsg::OpenHelp));
        
        assert!(state.ui_show_help(), "show_help should be true");
        assert!(effects.is_empty(), "UI toggle should produce no side effects");
    }
}
