use melotui_core::{
    AppMsg,
    domain::TrackId,
    message::{audio::AudioMsg, library::LibraryMsg, playback::PlaybackMsg},
};
use std::time::{Duration, Instant};

pub struct InputProcessor {
    last_add_track: Option<(TrackId, Instant)>,
}

impl InputProcessor {
    pub fn new() -> Self {
        Self {
            last_add_track: None,
        }
    }

    pub fn coalesce(&mut self, events: Vec<AppMsg>, debounce_ms: u64) -> Vec<AppMsg> {
        let mut out = Vec::with_capacity(events.len());

        for msg in events {
            match msg {
                // coalesce relative seeks
                AppMsg::Playback(PlaybackMsg::RequestSeekRelative(offset)) => {
                    let mut merged = false;
                    if let Some(last_msg) = out.last_mut() {
                        match last_msg {
                            AppMsg::Playback(PlaybackMsg::RequestSeekRelative(last_offset)) => {
                                *last_offset += offset;
                                merged = true;
                            }
                            AppMsg::Playback(PlaybackMsg::RequestSeekAbsolute(last_pos)) => {
                                // Absolute(A) + Relative(R) = Absolute(A + R)
                                // We must make sure it doesn't go below 0
                                let new_pos = (*last_pos as i64 + offset).max(0) as u64;
                                *last_pos = new_pos;
                                merged = true;
                            }
                            _ => {}
                        }
                    }
                    if !merged {
                        out.push(AppMsg::Playback(PlaybackMsg::RequestSeekRelative(offset)));
                    }
                }

                // coalesce absolute seeks
                AppMsg::Playback(PlaybackMsg::RequestSeekAbsolute(pos)) => {
                    let mut merged = false;
                    if let Some(last_msg) = out.last_mut() {
                        match last_msg {
                            AppMsg::Playback(PlaybackMsg::RequestSeekAbsolute(last_pos)) => {
                                // Absolute + Absolute = The latter overrides the former
                                *last_pos = pos;
                                merged = true;
                            }
                            AppMsg::Playback(PlaybackMsg::RequestSeekRelative(_)) => {
                                // Relative + Absolute = The absolute overwrites the relative. We can just replace the enum variant entirely.
                                *last_msg = AppMsg::Playback(PlaybackMsg::RequestSeekAbsolute(pos));
                                merged = true;
                            }
                            _ => {}
                        }
                    }
                    if !merged {
                        out.push(AppMsg::Playback(PlaybackMsg::RequestSeekAbsolute(pos)));
                    }
                }

                // coalesce volume
                AppMsg::Audio(AudioMsg::AdjustVolume(delta)) => {
                    let mut merged = false;
                    if let Some(AppMsg::Audio(AudioMsg::AdjustVolume(last_delta))) = out.last_mut()
                    {
                        *last_delta += delta;
                        merged = true;
                    }
                    if !merged {
                        out.push(AppMsg::Audio(AudioMsg::AdjustVolume(delta)));
                    }
                }

                // dedupe queue adds
                AppMsg::Library(LibraryMsg::AddToQueue(track_id)) => {
                    let now = Instant::now();
                    let should_add = match self.last_add_track {
                        Some((last_id, last_time)) if last_id == track_id => {
                            now.duration_since(last_time) > Duration::from_millis(debounce_ms)
                        }
                        Some(_) | None => true,
                    };

                    if should_add {
                        self.last_add_track = Some((track_id, now));
                        out.push(AppMsg::Library(LibraryMsg::AddToQueue(track_id)));
                    }
                }

                // throttling moved to core
                AppMsg::Playback(PlaybackMsg::RequestPlaySelected) => {
                    out.push(AppMsg::Playback(PlaybackMsg::RequestPlaySelected));
                }

                // pass through others
                _ => out.push(msg),
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coalesce_relative_seeks() {
        let mut processor = InputProcessor::new();
        let events = vec![
            AppMsg::Playback(PlaybackMsg::RequestSeekRelative(10)),
            AppMsg::Playback(PlaybackMsg::RequestSeekRelative(-5)),
            AppMsg::Playback(PlaybackMsg::RequestSeekRelative(15)),
        ];

        let out = processor.coalesce(events, 500);
        assert_eq!(out.len(), 1);
        match &out[0] {
            AppMsg::Playback(PlaybackMsg::RequestSeekRelative(offset)) => {
                assert_eq!(*offset, 20); // 10 - 5 + 15 = 20
            }
            _ => panic!("Expected RequestSeekRelative"),
        }
    }

    #[test]
    fn test_coalesce_absolute_overrides_relative() {
        let mut processor = InputProcessor::new();
        let events = vec![
            AppMsg::Playback(PlaybackMsg::RequestSeekRelative(10)),
            AppMsg::Playback(PlaybackMsg::RequestSeekAbsolute(60)),
        ];

        let out = processor.coalesce(events, 500);
        assert_eq!(out.len(), 1);
        match &out[0] {
            AppMsg::Playback(PlaybackMsg::RequestSeekAbsolute(pos)) => {
                assert_eq!(*pos, 60);
            }
            _ => panic!("Expected RequestSeekAbsolute"),
        }
    }

    #[test]
    fn test_coalesce_relative_modifies_absolute() {
        let mut processor = InputProcessor::new();
        let events = vec![
            AppMsg::Playback(PlaybackMsg::RequestSeekAbsolute(60)),
            AppMsg::Playback(PlaybackMsg::RequestSeekRelative(-15)),
            AppMsg::Playback(PlaybackMsg::RequestSeekRelative(5)),
        ];

        let out = processor.coalesce(events, 500);
        assert_eq!(out.len(), 1);
        match &out[0] {
            AppMsg::Playback(PlaybackMsg::RequestSeekAbsolute(pos)) => {
                assert_eq!(*pos, 50); // 60 - 15 + 5 = 50
            }
            _ => panic!("Expected RequestSeekAbsolute"),
        }
    }

    #[test]
    fn test_coalesce_relative_modifies_absolute_clamped() {
        let mut processor = InputProcessor::new();
        let events = vec![
            AppMsg::Playback(PlaybackMsg::RequestSeekAbsolute(10)),
            AppMsg::Playback(PlaybackMsg::RequestSeekRelative(-20)), // Should clamp to 0
        ];

        let out = processor.coalesce(events, 500);
        assert_eq!(out.len(), 1);
        match &out[0] {
            AppMsg::Playback(PlaybackMsg::RequestSeekAbsolute(pos)) => {
                assert_eq!(*pos, 0);
            }
            _ => panic!("Expected RequestSeekAbsolute"),
        }
    }
}
