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
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

use melotui_core::state::audio::OutputKind;

/// How often to look again when nothing says that something changed:
/// headphones plugged in show within this.
const LOOK_EVERY: Duration = Duration::from_secs(3);
/// How often to look again all the same while the sound server is telling
/// of changes, in case one was missed.
const LOOK_EVERY_WHEN_TOLD: Duration = Duration::from_secs(60);
/// How long to let a burst of changes settle before looking: plugging
/// headphones in is a dozen events.
const SETTLE: Duration = Duration::from_millis(300);

/// Whether a line of `pactl subscribe` tells of something that can change
/// what the music comes out of: a sink, a card's port, the server's default,
/// or a stream moved — not the sources and clients that come and go all day.
fn is_output_event(line: &str) -> bool {
    ["on sink", "on card", "on server"]
        .iter()
        .any(|what| line.contains(what))
}

/// Has the sound server tell of changes, each sent on `changed`. `false` if
/// it cannot be asked to: no `pactl`, or no server.
///
/// Asking it every few seconds instead meant two processes started every
/// three seconds for as long as melotui ran, to learn that nothing had
/// changed.
fn subscribe(changed: mpsc::Sender<()>) -> bool {
    let child = Command::new("pactl")
        .arg("subscribe")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = child else {
        return false;
    };
    let Some(stdout) = child.stdout.take() else {
        return false;
    };
    thread::Builder::new()
        .name("output-events".into())
        .spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if is_output_event(&line) && changed.send(()).is_err() {
                    break;
                }
            }
            // The watcher has gone, or the server has: either way this is
            // over, and the watcher goes back to looking by itself.
            let _ = child.kill();
            let _ = child.wait();
        })
        .is_ok()
}

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
            let (changed_tx, changed) = mpsc::channel();
            let mut told = subscribe(changed_tx);
            // The output melotui itself has open, looked at every time: it
            // changes without the sound server having anything to say.
            let mut seen = (None::<String>, false);
            let mut look = true;
            let mut last_look = std::time::Instant::now();
            loop {
                let now = match watched.lock() {
                    Ok(w) => (w.name.clone(), w.follows_default),
                    Err(_) => return,
                };
                if now != seen {
                    seen = now;
                    look = true;
                }
                if !look {
                    match changed.recv_timeout(LOOK_EVERY) {
                        Ok(()) => {
                            thread::sleep(SETTLE);
                            while changed.try_recv().is_ok() {}
                            look = true;
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        // The server stopped telling.
                        Err(mpsc::RecvTimeoutError::Disconnected) => {
                            told = false;
                            thread::sleep(LOOK_EVERY);
                        }
                    }
                    // Told of changes, there is no asking until one comes,
                    // but for once in a long while.
                    let overdue = last_look.elapsed() >= LOOK_EVERY_WHEN_TOLD;
                    if !look && told && !overdue {
                        continue;
                    }
                }
                look = false;
                last_look = std::time::Instant::now();

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
            }
        });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_changes_to_where_the_sound_goes_are_looked_into() {
        assert!(is_output_event("Event 'change' on sink #58"));
        assert!(is_output_event("Event 'change' on card #47"));
        assert!(is_output_event("Event 'change' on server #0"));
        assert!(is_output_event("Event 'new' on sink-input #912"));
        assert!(!is_output_event("Event 'new' on client #3301"));
        assert!(!is_output_event("Event 'change' on source #59"));
        assert!(!is_output_event("Event 'remove' on source-output #77"));
    }

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
