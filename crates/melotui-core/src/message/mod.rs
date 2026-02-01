pub mod audio;
pub mod cover;
pub mod download;
pub mod library;
pub mod navigation;
pub mod playback;
pub mod ui;

use crate::message::{
    audio::AudioMsg, cover::CoverMsg, download::DownloadMsg, library::LibraryMsg,
    navigation::NavigationMsg, playback::PlaybackMsg, ui::UIMsg,
};

#[derive(Debug, Clone)]
pub enum AppMsg {
    // Lifecycle
    RequestQuit,
    Noop,

    // Domain messages
    Navigation(NavigationMsg),
    Playback(PlaybackMsg),
    Audio(AudioMsg),
    UI(UIMsg),
    Library(LibraryMsg),
    Download(DownloadMsg),
    Cover(CoverMsg),
}
