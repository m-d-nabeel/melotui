//! MPRIS / System media controls integration via `souvlaki`.
//!
//! This module is conditionally compiled on Linux and macOS only.
//! It exposes the currently playing track to the OS media widget
//! and translates hardware media key presses into `AppMsg`s.

use melotui_core::{
    AppMsg, domain::TrackId, message::playback::PlaybackMsg, playback::state::PlaybackStatus,
};
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, PlatformConfig};
use std::path::PathBuf;
use std::sync::mpsc;

/// Messages sent from the MPRIS callback thread to the main loop.
pub enum MprisEvent {
    /// A hardware media key was pressed — translate to AppMsg.
    MediaKey(AppMsg),
}

/// Cached snapshot of what we last told the OS, to avoid redundant updates.
struct CachedState {
    track_id: Option<TrackId>,
    status: PlaybackStatus,
    cover_path: Option<PathBuf>,
    seek_epoch: u64,
    volume: f32,
}

pub struct MprisManager {
    controls: MediaControls,
    cached: CachedState,
}

impl MprisManager {
    /// Initialize the OS media controls.
    /// Returns `(MprisManager, Receiver)` — the receiver yields media key events.
    pub fn new() -> Option<(Self, mpsc::Receiver<MprisEvent>)> {
        let config = PlatformConfig {
            dbus_name: "melotui",
            display_name: "Melotui",
            hwnd: None,
        };

        let mut controls = match MediaControls::new(config) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to initialize MPRIS controls: {:?}", e);
                return None;
            }
        };

        let (tx, rx) = mpsc::channel();

        let attach_result = controls.attach(move |event: MediaControlEvent| {
            let msg = match event {
                MediaControlEvent::Play => {
                    Some(AppMsg::Playback(PlaybackMsg::RequestTogglePlayback))
                }
                MediaControlEvent::Pause => {
                    Some(AppMsg::Playback(PlaybackMsg::RequestTogglePlayback))
                }
                MediaControlEvent::Toggle => {
                    Some(AppMsg::Playback(PlaybackMsg::RequestTogglePlayback))
                }
                MediaControlEvent::Next => Some(AppMsg::Playback(PlaybackMsg::RequestPlayNext)),
                MediaControlEvent::Previous => Some(AppMsg::Playback(PlaybackMsg::RequestPlayPrev)),
                MediaControlEvent::Stop => Some(AppMsg::Playback(PlaybackMsg::RequestStop)),
                MediaControlEvent::Seek(direction) => {
                    let offset = match direction {
                        souvlaki::SeekDirection::Forward => 5,
                        souvlaki::SeekDirection::Backward => -5,
                    };
                    Some(AppMsg::Playback(PlaybackMsg::RequestSeekRelative(offset)))
                }
                MediaControlEvent::SeekBy(direction, duration) => {
                    let mut offset = duration.as_secs() as i64;
                    if let souvlaki::SeekDirection::Backward = direction {
                        offset = -offset;
                    }
                    Some(AppMsg::Playback(PlaybackMsg::RequestSeekRelative(offset)))
                }
                MediaControlEvent::SetPosition(position) => Some(AppMsg::Playback(
                    PlaybackMsg::RequestSeekAbsolute(position.0.as_secs()),
                )),
                MediaControlEvent::SetVolume(vol) => Some(AppMsg::Audio(
                    melotui_core::message::audio::AudioMsg::DispatchSetVolume(vol as f32),
                )),
                MediaControlEvent::Quit => Some(AppMsg::RequestQuit),
                _ => None,
            };

            if let Some(app_msg) = msg {
                let _ = tx.send(MprisEvent::MediaKey(app_msg));
            }
        });

        if let Err(e) = attach_result {
            tracing::warn!("Failed to attach MPRIS event handler: {:?}", e);
            return None;
        }

        tracing::info!("MPRIS media controls initialized");

        Some((
            Self {
                controls,
                cached: CachedState {
                    track_id: None,
                    status: PlaybackStatus::Stopped,
                    cover_path: None,
                    seek_epoch: 0,
                    volume: 1.0,
                },
            },
            rx,
        ))
    }

    /// Synchronize the OS media widget with the current app state.
    /// Only sends updates when something actually changed.
    pub fn update_state(&mut self, state: &melotui_core::AppState) {
        let current_id = state.get_playing_track_id();
        let current_status = state.playback_status();
        let current_cover = &state.current_cover_path;
        let current_epoch = state.seek_epoch();
        let current_volume = state.volume();

        let track_changed = current_id != self.cached.track_id;
        let status_changed = *current_status != self.cached.status;
        let cover_changed = *current_cover != self.cached.cover_path;
        let seek_changed = current_epoch != self.cached.seek_epoch;
        let volume_changed = (current_volume - self.cached.volume).abs() > 0.01;

        if volume_changed {
            if let Err(e) = self.controls.set_volume(current_volume as f64) {
                tracing::debug!("Failed to set MPRIS volume: {:?}", e);
            }
            self.cached.volume = current_volume;
        }

        // Update playback status and position
        if status_changed || track_changed || seek_changed {
            let playback = match current_status {
                PlaybackStatus::Playing => MediaPlayback::Playing {
                    progress: Some(souvlaki::MediaPosition(std::time::Duration::from_millis(
                        state.progress_ms(),
                    ))),
                },
                PlaybackStatus::Paused => MediaPlayback::Paused {
                    progress: Some(souvlaki::MediaPosition(std::time::Duration::from_millis(
                        state.progress_ms(),
                    ))),
                },
                PlaybackStatus::Stopped => MediaPlayback::Stopped,
            };

            if let Err(e) = self.controls.set_playback(playback) {
                tracing::debug!("Failed to set MPRIS playback status: {:?}", e);
            }

            self.cached.status = current_status.clone();
            self.cached.seek_epoch = current_epoch;
        }

        // Update metadata (track info + cover art)
        if track_changed || cover_changed {
            if let Some(track) = state.get_playing_track() {
                let cover_url = current_cover
                    .as_ref()
                    .map(|p| format!("file://{}", p.display()));

                let metadata = MediaMetadata {
                    title: Some(&track.title),
                    artist: track.artist.as_deref(),
                    album: track.album.as_deref(),
                    cover_url: cover_url.as_deref(),
                    duration: Some(std::time::Duration::from_millis(track.duration_ms)),
                };

                if let Err(e) = self.controls.set_metadata(metadata) {
                    tracing::debug!("Failed to set MPRIS metadata: {:?}", e);
                }
            } else {
                // No track playing — clear metadata
                let metadata = MediaMetadata {
                    title: None,
                    artist: None,
                    album: None,
                    cover_url: None,
                    duration: None,
                };
                let _ = self.controls.set_metadata(metadata);
            }

            self.cached.track_id = current_id;
            self.cached.cover_path = current_cover.clone();
        }
    }
}
