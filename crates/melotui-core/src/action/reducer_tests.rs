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

        assert!(
            effects.is_empty(),
            "RequestQuit should produce no side effects"
        );
        assert_eq!(state.current_notification().unwrap().message, "Quit");
    }

    #[test]
    fn test_reduce_ui_msg() {
        let mut state = test_state();
        let effects = reduce(&mut state, AppMsg::UI(UIMsg::OpenHelp));

        assert!(state.ui_show_help(), "show_help should be true");
        assert!(
            effects.is_empty(),
            "UI toggle should produce no side effects"
        );
    }

    #[test]
    fn test_reduce_audio_msg() {
        let mut state = test_state();
        let initial_volume = state.volume();

        // Increase volume
        let effects = reduce(
            &mut state,
            AppMsg::Audio(crate::message::audio::AudioMsg::AdjustVolume(0.1)),
        );

        assert!(state.volume() > initial_volume, "Volume should increase");
        assert_eq!(effects.len(), 1, "Should dispatch volume change");
        assert!(matches!(
            effects[0],
            AppMsg::Audio(crate::message::audio::AudioMsg::DispatchSetVolume(_))
        ));
    }

    #[test]
    fn test_reduce_navigation_msg() {
        let mut state = test_state();

        assert!(!state.is_showing_track_info());
        let effects = reduce(
            &mut state,
            AppMsg::Navigation(crate::message::navigation::NavigationMsg::ToggleTrackInfo),
        );
        assert!(effects.is_empty());
        // It toggles only if a track is selected. By default no track is selected.
        assert!(
            !state.is_showing_track_info(),
            "Should not toggle track info if no track is selected"
        );
    }

    #[test]
    fn test_reduce_library_msg() {
        let mut state = test_state();
        let initial_shuffle = state.shuffle_active();

        let effects = reduce(
            &mut state,
            AppMsg::Library(crate::message::library::LibraryMsg::ToggleShuffle),
        );
        assert!(effects.is_empty());
        assert_eq!(
            state.shuffle_active(),
            !initial_shuffle,
            "Shuffle should toggle"
        );
    }

    #[test]
    fn test_reduce_playback_msg() {
        let mut state = test_state();
        state.session.playback.status = crate::playback::state::PlaybackStatus::Playing;

        let effects = reduce(
            &mut state,
            AppMsg::Playback(crate::message::playback::PlaybackMsg::RequestStop),
        );
        assert_eq!(effects.len(), 1);
        assert!(state.is_stopped(), "Playback status should be Stopped");
        assert!(matches!(
            effects[0],
            AppMsg::Playback(crate::message::playback::PlaybackMsg::DispatchStop)
        ));
    }
}
