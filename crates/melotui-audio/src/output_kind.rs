//! What the music comes out of — speakers, headphones, a headset, a screen
//! — for the playback bar's icon.
//!
//! Played through the system's default output on Linux, the device opened is
//! the sound server's, named for the server, not for what it plays through;
//! PipeWire and PulseAudio know, and say through `pactl`: the default sink's
//! form factor, the port it plays through (`[Out] Headphones`), its icon.
//! Elsewhere, and for a device chosen by name, the device's own name tells:
//! `AirPods`, `Headphones (Realtek Audio)`, `MacBook Pro Speakers`.

use std::{
    process::Command,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

use melotui_core::state::audio::OutputKind;

/// How often to look again: headphones plugged in show within this.
const LOOK_EVERY: Duration = Duration::from_secs(3);

/// The output open now, for the watcher to look at.
#[derive(Debug, Default)]
pub struct Watched {
    /// The opened device's name.
    pub name: Option<String>,
    /// Whether it is the system's default output, and the sound server can
    /// be asked what that plays through.
    pub follows_default: bool,
}

/// What a device described by `text` is, from the words in it.
pub fn classify(text: &str) -> OutputKind {
    let text = text.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| text.contains(w));
    if has(&["headset", "hands-free", "handsfree"]) {
        OutputKind::Headset
    } else if has(&["headphone", "earphone", "earbud", "airpods", "buds"]) {
        OutputKind::Headphones
    } else if has(&[
        "hdmi",
        "displayport",
        "display port",
        "video-display",
        "television",
        "monitor",
    ]) {
        OutputKind::Display
    } else if has(&["speaker"]) {
        OutputKind::Speakers
    } else {
        OutputKind::Unknown
    }
}

/// What the sound server's default sink plays through, from `pactl`'s
/// description of it, the sink named `default` among `sinks`.
fn kind_of_sink(sinks: &serde_json::Value, default: &str) -> Option<OutputKind> {
    let sink = sinks
        .as_array()?
        .iter()
        .find(|s| s.get("name").and_then(|n| n.as_str()) == Some(default))?;
    let field = |key: &str| sink.get(key).and_then(|v| v.as_str()).unwrap_or("");
    let property = |key: &str| {
        sink.get("properties")
            .and_then(|p| p.get(key))
            .and_then(|v| v.as_str())
            .unwrap_or("")
    };
    // Most telling first: what the device says it is, the port it plays
    // through, its icon, its name.
    for text in [
        property("device.form_factor"),
        field("active_port"),
        property("device.icon_name"),
        field("description"),
    ] {
        let kind = classify(text);
        if kind != OutputKind::Unknown {
            return Some(kind);
        }
    }
    // A Bluetooth device that says nothing more is most likely worn.
    (property("device.bus") == "bluetooth").then_some(OutputKind::Headphones)
}

/// What the default output plays through, if the sound server says.
fn from_sound_server() -> Option<OutputKind> {
    let run = |args: &[&str]| -> Option<String> {
        let out = Command::new("pactl").args(args).output().ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
    };
    let default = run(&["get-default-sink"])?;
    let sinks: serde_json::Value =
        serde_json::from_str(&run(&["-f", "json", "list", "sinks"])?).ok()?;
    Some(kind_of_sink(&sinks, &default).unwrap_or(OutputKind::Unknown))
}

/// Watches what the output in `watched` plays through, telling each change.
pub fn watch(watched: Arc<Mutex<Watched>>) -> mpsc::Receiver<OutputKind> {
    let (tx, rx) = mpsc::channel();
    let _ = thread::Builder::new()
        .name("output-kind".into())
        .spawn(move || {
            let mut last = OutputKind::Unknown;
            // Without pactl, or a server to answer it, there is no asking
            // every few seconds.
            let mut can_ask = true;
            loop {
                let (name, follows_default) = match watched.lock() {
                    Ok(w) => (w.name.clone(), w.follows_default),
                    Err(_) => return,
                };
                let mut kind = None;
                if follows_default && can_ask {
                    kind = from_sound_server();
                    can_ask = kind.is_some();
                }
                let kind = kind
                    .filter(|k| *k != OutputKind::Unknown)
                    .unwrap_or_else(|| name.as_deref().map_or(OutputKind::Unknown, classify));
                if kind != last {
                    last = kind;
                    if tx.send(kind).is_err() {
                        return;
                    }
                }
                thread::sleep(LOOK_EVERY);
            }
        });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devices_are_told_apart_by_their_names() {
        assert_eq!(classify("MacBook Pro Speakers"), OutputKind::Speakers);
        assert_eq!(classify("AirPods Pro"), OutputKind::Headphones);
        assert_eq!(
            classify("Headphones (Realtek(R) Audio)"),
            OutputKind::Headphones
        );
        assert_eq!(classify("audio-headset"), OutputKind::Headset);
        assert_eq!(classify("[Out] HDMI3"), OutputKind::Display);
        assert_eq!(classify("PipeWire Sound Server"), OutputKind::Unknown);
    }

    #[test]
    fn the_sound_servers_default_sink_says_what_it_plays_through() {
        let sinks = serde_json::json!([
            {"name": "hdmi", "description": "HDMI / DisplayPort 1 Output", "active_port": "[Out] HDMI1",
             "properties": {"device.icon_name": "video-display"}},
            {"name": "card", "description": "On-Package Audio Speaker", "active_port": "[Out] Headphones",
             "properties": {"device.icon_name": "audio-speakers"}},
            {"name": "buds", "description": "Galaxy", "active_port": "",
             "properties": {"device.bus": "bluetooth"}},
            {"name": "set", "description": "WH-1000XM4", "active_port": "headset-output",
             "properties": {"device.form_factor": "headset"}},
        ]);
        assert_eq!(kind_of_sink(&sinks, "hdmi"), Some(OutputKind::Display));
        // The port it plays through beats the card's icon: headphones in
        // the jack of a card whose speakers are its usual output.
        assert_eq!(kind_of_sink(&sinks, "card"), Some(OutputKind::Headphones));
        assert_eq!(kind_of_sink(&sinks, "buds"), Some(OutputKind::Headphones));
        assert_eq!(kind_of_sink(&sinks, "set"), Some(OutputKind::Headset));
        assert_eq!(kind_of_sink(&sinks, "gone"), None);
    }
}
