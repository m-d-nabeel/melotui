use crate::{AppMsg, AppState};

pub fn handle_cover_message(
    state: &mut AppState,
    msg: crate::message::cover::CoverMsg,
) -> Vec<AppMsg> {
    use crate::message::cover::{CoverContext, CoverMsg};

    match msg {
        CoverMsg::RequestLoad { .. } => {
            vec![]
        }
        CoverMsg::Loaded { id, cover, context } => {
            match context {
                CoverContext::Playback => {
                    // TODO: Review - ensure cover matches playing track to avoid race conditions
                    if let Some(current_id) = state.get_playing_track_id() {
                        if current_id == id {
                            state.current_cover = cover;
                        }
                    }
                }
                CoverContext::Info => {
                    let selected_id = if state.is_library_focused() {
                        state.get_selected_track_id_in_sidebar()
                    } else {
                        state.get_selected_track_id_in_queue()
                    };

                    if let Some(sel_id) = selected_id {
                        if sel_id == id {
                            state.track_info_cover = cover;
                            state.track_info_cover_id = Some(id);
                        }
                    }
                }
            }
            vec![]
        }
    }
}
