use melotui_core::{
    AppMsg,
    domain::TrackId,
    message::{audio::AudioMsg, library::LibraryMsg, playback::PlaybackMsg},
};
use std::time::{Duration, Instant};

pub struct InputProcessor {
    last_add_track: Option<(TrackId, Instant)>,
    last_play_request: Option<Instant>,
}

impl InputProcessor {
    pub fn new() -> Self {
        Self {
            last_add_track: None,
            last_play_request: None,
        }
    }

    pub fn coalesce(&mut self, events: Vec<AppMsg>) -> Vec<AppMsg> {
        let mut out = Vec::with_capacity(events.len());

        for msg in events {
            match msg {
                // coalesce seeks
                AppMsg::Playback(PlaybackMsg::RequestSeek { offset_seconds }) => {
                    let mut merged = false;
                    if let Some(AppMsg::Playback(PlaybackMsg::RequestSeek {
                        offset_seconds: last_offset,
                    })) = out.last_mut()
                    {
                        *last_offset += offset_seconds;
                        merged = true;
                    }
                    if !merged {
                        out.push(AppMsg::Playback(PlaybackMsg::RequestSeek {
                            offset_seconds,
                        }));
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
                        Some((last_id, last_time)) => {
                            // same track: 500ms debounce
                            if last_id == track_id {
                                now.duration_since(last_time) > Duration::from_millis(500)
                            } else {
                                // diff track: allow
                                true
                            }
                        }
                        None => true,
                    };

                    if should_add {
                        self.last_add_track = Some((track_id, now));
                        out.push(AppMsg::Library(LibraryMsg::AddToQueue(track_id)));
                    }
                }

                // debounce play selection
                AppMsg::Playback(PlaybackMsg::RequestPlaySelected) => {
                    let now = Instant::now();
                    let should_play = match self.last_play_request {
                        Some(last_time) => {
                            now.duration_since(last_time) > Duration::from_millis(1000)
                        }
                        None => true,
                    };

                    if should_play {
                        self.last_play_request = Some(now);
                        out.push(AppMsg::Playback(PlaybackMsg::RequestPlaySelected));
                    }
                }

                // pass through others
                _ => out.push(msg),
            }
        }
        out
    }
}
