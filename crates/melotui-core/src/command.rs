//! The commands typed after `:`.
//!
//! [`COMMANDS`] describes every one of them: its names, what follows it and
//! what it does. Parsing looks names up in it, completion offers what it
//! lists, and the help screen is generated from it, so the three cannot drift
//! apart.

use crate::playback::state::RepeatMode;
use crate::state::{Effect, SpectrumStyle, ui::FocusMode};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchField {
    Artist,
    Song,
    Album,
    Genre,
    Year,
}

impl fmt::Display for SearchField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchField::Artist => write!(f, "artist"),
            SearchField::Song => write!(f, "song"),
            SearchField::Album => write!(f, "album"),
            SearchField::Genre => write!(f, "genre"),
            SearchField::Year => write!(f, "year"),
        }
    }
}

impl FromStr for SearchField {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "artist" | "a" => Ok(SearchField::Artist),
            "song" | "s" | "track" | "t" => Ok(SearchField::Song),
            "album" | "al" => Ok(SearchField::Album),
            "genre" | "g" => Ok(SearchField::Genre),
            "year" | "y" => Ok(SearchField::Year),
            _ => Err(()),
        }
    }
}

/// Where an argument's values come from, for completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Values {
    /// Free text or a number: nothing to offer.
    Free,
    /// One of these words.
    Words(&'static [&'static str]),
    /// A playlist's name.
    Playlist,
    /// A saved preset's name.
    Preset,
}

/// What follows a command's name.
#[derive(Debug)]
pub enum Then {
    /// Arguments, each completed from its values in turn.
    Args(&'static [Values]),
    /// One of these subcommands.
    Sub(&'static [Spec]),
}

/// A command or a subcommand.
#[derive(Debug)]
pub struct Spec {
    /// Its name, then any aliases.
    pub names: &'static [&'static str],
    /// Its arguments, as help shows them.
    pub usage: &'static str,
    pub help: &'static str,
    pub then: Then,
}

impl Spec {
    pub fn name(&self) -> &'static str {
        self.names[0]
    }

    /// Whether anything is typed after the name.
    pub fn takes_more(&self) -> bool {
        match self.then {
            Then::Args(args) => !args.is_empty(),
            Then::Sub(subs) => !subs.is_empty(),
        }
    }
}

const fn spec(
    names: &'static [&'static str],
    usage: &'static str,
    help: &'static str,
    then: Then,
) -> Spec {
    Spec {
        names,
        usage,
        help,
        then,
    }
}

const NOTHING: Then = Then::Args(&[]);
const TEXT: Then = Then::Args(&[Values::Free]);

const ON_OFF: &[&str] = &["on", "off"];
const REPEAT_MODES: &[&str] = &["off", "once", "track", "playlist"];
const FOCUS_MODES: &[&str] = &["full", "viz", "play", "vp", "lyrics"];
const VISUALIZERS: &[&str] = &["cava", "default", "bar", "particle", "radial", "nova"];
const PANELS: &[&str] = &["playback", "lyrics", "art", "info", "studio"];
const LYRICS_VIEWS: &[&str] = &["panel", "focus"];
const EQ_BANDS: &[&str] = &["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"];
const RESETTABLE: &[&str] = &[
    "bass",
    "treble",
    "reverb",
    "echo",
    "pan",
    "lowpass",
    "highpass",
    "autopan-rate",
    "autopan-depth",
    "exciter",
    "bitcrush",
    "width",
    "karaoke",
    "wobble",
    "pingpong",
    "phaser",
    "eq",
];

/// `:fx`'s subcommands: one per [`Effect`], with its range as [`Effect::range`]
/// gives it, then the EQ, resets and macros.
static EFFECTS: &[Spec] = &[
    spec(&["bass"], "<0.1-5>", "Bass (1 = flat)", TEXT),
    spec(&["treble"], "<0.1-5>", "Treble (1 = flat)", TEXT),
    spec(&["reverb"], "<0-1>", "Reverb", TEXT),
    spec(&["echo"], "<0-0.9>", "Echo", TEXT),
    spec(&["pan"], "<-1-1>", "Balance, left to right", TEXT),
    spec(
        &["lowpass"],
        "<200-20000>",
        "Low-pass cutoff in Hz (20000 = off)",
        TEXT,
    ),
    spec(
        &["highpass"],
        "<20-5000>",
        "High-pass cutoff in Hz (20 = off)",
        TEXT,
    ),
    spec(&["autopan-rate"], "<0-2>", "Auto-pan speed in Hz", TEXT),
    spec(&["autopan-depth"], "<0-1>", "Auto-pan depth", TEXT),
    spec(&["exciter"], "<0-5>", "Exciter", TEXT),
    spec(&["bitcrush"], "<0-1>", "Bitcrusher", TEXT),
    spec(&["width"], "<0-3>", "Stereo width (1 = normal)", TEXT),
    spec(&["karaoke"], "<0-1>", "Vocal removal", TEXT),
    spec(&["wobble"], "<0-1>", "Wobble", TEXT),
    spec(&["pingpong"], "<0-1>", "Ping-pong echo", TEXT),
    spec(&["phaser"], "<0-1>", "Phaser", TEXT),
    spec(
        &["eq"],
        "<1-10> <-12-12>",
        "EQ band gain in dB",
        Then::Args(&[Values::Words(EQ_BANDS), Values::Free]),
    ),
    spec(
        &["reset"],
        "[effect]",
        "Reset one effect, the EQ, or everything",
        Then::Args(&[Values::Words(RESETTABLE)]),
    ),
    spec(&["slowed"], "", "Slowed + reverb", NOTHING),
    spec(&["bassboost"], "", "Bass boosted", NOTHING),
    spec(&["spatial"], "", "Spatial", NOTHING),
];

/// Every command.
pub static COMMANDS: &[Spec] = &[
    spec(&["play"], "", "Resume, or play the selected track", NOTHING),
    spec(&["pause"], "", "Pause", NOTHING),
    spec(&["stop"], "", "Stop", NOTHING),
    spec(&["next"], "", "Next track", NOTHING),
    spec(&["prev", "previous"], "", "Previous track", NOTHING),
    spec(
        &["seek"],
        "<+s|-s|s|m:ss>",
        "Seek by or to a position",
        TEXT,
    ),
    spec(
        &["vol", "volume"],
        "<0-150|+n|-n>",
        "Volume in percent",
        TEXT,
    ),
    spec(&["speed"], "<0.5-2|+n|-n>", "Playback speed", TEXT),
    spec(
        &["shuffle"],
        "[on|off]",
        "Shuffle on or off, or toggle it",
        Then::Args(&[Values::Words(ON_OFF)]),
    ),
    spec(
        &["repeat"],
        "[off|once|track|playlist]",
        "Repeat mode, or the next one",
        Then::Args(&[Values::Words(REPEAT_MODES)]),
    ),
    spec(&["artist", "a"], "<query>", "Search artists", TEXT),
    spec(
        &["song", "s", "track", "t"],
        "<query>",
        "Search songs",
        TEXT,
    ),
    spec(&["album", "al"], "<query>", "Search albums", TEXT),
    spec(&["genre", "g"], "<query>", "Search genres", TEXT),
    spec(&["year", "y"], "<query>", "Search by year", TEXT),
    spec(
        &["noh", "nohlsearch", "clear"],
        "",
        "Clear the search",
        NOTHING,
    ),
    spec(&["sort"], "", "Next sort order", NOTHING),
    spec(&["rescan", "r", "scan"], "", "Rescan the library", NOTHING),
    spec(
        &["dl", "download"],
        "<query>",
        "Download from YouTube",
        TEXT,
    ),
    spec(
        &["pl", "playlist"],
        "",
        "Playlists",
        Then::Sub(&[
            spec(&["create", "new"], "<name>", "New playlist", TEXT),
            spec(
                &["del", "delete"],
                "<playlist>",
                "Delete a playlist",
                Then::Args(&[Values::Playlist]),
            ),
            spec(
                &["add"],
                "<playlist>",
                "Add the selected track to a playlist",
                Then::Args(&[Values::Playlist]),
            ),
            spec(
                &["rm", "remove"],
                "<playlist>",
                "Remove the selected track from a playlist",
                Then::Args(&[Values::Playlist]),
            ),
        ]),
    ),
    spec(
        &["queue"],
        "",
        "The queue",
        Then::Sub(&[spec(&["clear"], "", "Clear the queue", NOTHING)]),
    ),
    spec(
        &["lyrics"],
        "",
        "Lyrics",
        Then::Sub(&[
            spec(&["show"], "", "Show the lyrics panel", NOTHING),
            spec(&["hide"], "", "Hide the lyrics panel", NOTHING),
            spec(&["toggle"], "", "Show or hide the lyrics panel", NOTHING),
            spec(
                &["zoom"],
                "[panel|focus] <1-4>",
                "Lyrics size (panel or full view)",
                Then::Args(&[Values::Words(LYRICS_VIEWS), Values::Free]),
            ),
            spec(
                &["fetch"],
                "[artist - title]",
                "Fetch lyrics for the playing track",
                TEXT,
            ),
            spec(&["search"], "<query>", "Search for lyrics", TEXT),
        ]),
    ),
    spec(&["fx", "effect"], "", "Effects", Then::Sub(EFFECTS)),
    spec(
        &["preset", "dsp"],
        "",
        "Effect presets",
        Then::Sub(&[
            spec(&["list", "picker"], "", "Pick a preset", NOTHING),
            spec(&["save"], "<name>", "Save the current effects", TEXT),
            spec(
                &["load"],
                "<preset>",
                "Load a preset",
                Then::Args(&[Values::Preset]),
            ),
            spec(
                &["rm", "del", "delete"],
                "<preset>",
                "Delete a preset",
                Then::Args(&[Values::Preset]),
            ),
        ]),
    ),
    spec(&["studio"], "", "Show or hide the DSP studio", NOTHING),
    spec(
        &["focus", "zen"],
        "<mode>",
        "Layout: full, viz, play, vp or lyrics",
        Then::Args(&[Values::Words(FOCUS_MODES)]),
    ),
    spec(
        &["vis", "visualizer", "v"],
        "<style>",
        "Visualizer style",
        Then::Args(&[Values::Words(VISUALIZERS)]),
    ),
    spec(&["sidebar", "sb"], "<28-80>", "Sidebar width", TEXT),
    spec(
        &["toggle"],
        "<panel>",
        "Show or hide a panel",
        Then::Args(&[Values::Words(PANELS)]),
    ),
    spec(&["help"], "", "Keys and commands", NOTHING),
    spec(&["quit", "q", "exit"], "", "Quit", NOTHING),
];

/// The spec in `specs` called `name`, by its name or any alias.
pub fn find(specs: &'static [Spec], name: &str) -> Option<&'static Spec> {
    specs
        .iter()
        .find(|spec| spec.names.iter().any(|n| n.eq_ignore_ascii_case(name)))
}

/// A value given absolutely, or relative to the current one with `+` or `-`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Amount {
    To(f32),
    By(f32),
}

impl Amount {
    fn parse(text: &str) -> Option<Self> {
        let value: f32 = text.parse().ok().filter(|v: &f32| v.is_finite())?;
        Some(if text.starts_with(['+', '-']) {
            Amount::By(value)
        } else {
            Amount::To(value)
        })
    }

    /// The value this comes to from `current`.
    pub fn applied_to(self, current: f32) -> f32 {
        match self {
            Amount::To(value) => value,
            Amount::By(delta) => current + delta,
        }
    }

    /// This, given in percent, as a fraction.
    fn percent(self) -> Self {
        match self {
            Amount::To(value) => Amount::To(value / 100.0),
            Amount::By(delta) => Amount::By(delta / 100.0),
        }
    }
}

/// Which lyrics view a zoom is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LyricsView {
    /// The lyrics panel.
    Panel,
    /// The full lyrics view.
    Focus,
}

/// A panel `:toggle` shows or hides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Playback,
    Lyrics,
    Art,
    Info,
    Studio,
}

/// What `:fx reset` resets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reset {
    All,
    Eq,
    Effect(Effect),
}

/// One of the effect macros.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Macro {
    SlowedReverb,
    BassBoosted,
    Spatial,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Quit,
    Help,
    Rescan,
    Play,
    Pause,
    Stop,
    Next,
    Prev,
    SeekRelative(i64),
    SeekAbsolute(u64),
    /// Volume, as a fraction (1.0 = 100%).
    Volume(Amount),
    Speed(Amount),
    /// On, off, or toggled.
    Shuffle(Option<bool>),
    /// A mode, or the next one.
    Repeat(Option<RepeatMode>),
    Search {
        field: SearchField,
        query: String,
    },
    ClearSearch,
    Sort,
    Download {
        query: String,
    },
    CreatePlaylist {
        name: String,
    },
    DeletePlaylist {
        name: String,
    },
    AddToPlaylist {
        name: String,
    },
    RemoveFromPlaylist {
        name: String,
    },
    ClearQueue,
    /// Shown, hidden, or toggled.
    ShowLyrics(Option<bool>),
    /// A zoom for a view — the one on screen if none is named — or, with no
    /// zoom, a report of both.
    LyricsZoom {
        view: Option<LyricsView>,
        zoom: Option<f32>,
    },
    FetchLyrics {
        query: Option<String>,
    },
    SearchLyrics {
        query: String,
    },
    SetEffect {
        effect: Effect,
        amount: Amount,
    },
    /// An EQ band, counted from 0.
    SetEq {
        band: usize,
        amount: Amount,
    },
    ResetEffects(Reset),
    Macro(Macro),
    PresetSave {
        name: String,
    },
    PresetLoad {
        name: String,
    },
    PresetDelete {
        name: String,
    },
    PresetPicker,
    ToggleStudio,
    Focus {
        mode: FocusMode,
    },
    ChangeVisualizer {
        arg: SpectrumStyle,
    },
    Sidebar {
        width: u16,
    },
    TogglePanel(Panel),
    /// Not a command, or not a valid one; says why.
    Invalid(String),
}

pub fn shlex(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current_arg = String::new();
    let mut in_quotes = false;
    let mut quote_char = '\0';
    let mut escaped = false;

    for c in input.chars() {
        if escaped {
            current_arg.push(c);
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if in_quotes {
            if c == quote_char {
                in_quotes = false;
            } else {
                current_arg.push(c);
            }
        } else if c == '"' || c == '\'' {
            in_quotes = true;
            quote_char = c;
        } else if c.is_whitespace() {
            if !current_arg.is_empty() {
                args.push(current_arg.clone());
                current_arg.clear();
            }
        } else {
            current_arg.push(c);
        }
    }
    if !current_arg.is_empty() {
        args.push(current_arg);
    }
    args
}

fn invalid(message: impl Into<String>) -> Command {
    Command::Invalid(message.into())
}

/// `rest` as one piece of text, or the complaint that `what` needs it.
fn text(rest: &[String], what: &str) -> Result<String, Command> {
    let text = rest.join(" ");
    if text.is_empty() {
        Err(invalid(format!("{what} needs {}", needs(what))))
    } else {
        Ok(text)
    }
}

/// What a command's argument is, for complaints about it missing.
fn needs(what: &str) -> &'static str {
    let (command, sub) = what.split_once(' ').unwrap_or((what, ""));
    let spec = find(COMMANDS, command).and_then(|spec| match spec.then {
        Then::Sub(subs) => find(subs, sub),
        Then::Args(_) => Some(spec),
    });
    match spec.map(|spec| spec.usage) {
        Some("<name>") => "a name",
        Some("<playlist>") => "a playlist's name",
        Some("<preset>") => "a preset's name",
        _ => "a query",
    }
}

fn number<T: FromStr>(rest: &[String], what: &str, usage: &str) -> Result<T, Command> {
    rest.first()
        .and_then(|arg| arg.parse().ok())
        .ok_or_else(|| invalid(format!("{what} needs a number: {usage}")))
}

fn amount(rest: &[String], what: &str, usage: &str) -> Result<Amount, Command> {
    rest.first()
        .and_then(|arg| Amount::parse(arg))
        .ok_or_else(|| invalid(format!("{what} needs a number: {usage}")))
}

/// The subcommand of `spec` named first in `rest`, and what follows it.
fn sub<'a>(
    spec: &'static Spec,
    rest: &'a [String],
) -> Result<(&'static str, &'a [String]), Command> {
    let Then::Sub(subs) = spec.then else {
        unreachable!("{} has no subcommands", spec.name())
    };
    let choices = || {
        subs.iter()
            .map(|sub| sub.name())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let Some((name, rest)) = rest.split_first() else {
        return Err(invalid(format!(
            "{} needs one of: {}",
            spec.name(),
            choices()
        )));
    };
    match find(subs, name) {
        Some(sub) => Ok((sub.name(), rest)),
        None => Err(invalid(format!(
            "{} has no '{name}'; try one of: {}",
            spec.name(),
            choices()
        ))),
    }
}

/// `m:ss`, or a number of seconds.
fn seek(rest: &[String]) -> Result<Command, Command> {
    let usage = || invalid("seek needs +s, -s, s or m:ss (e.g. +10, -5, 90, 1:30)");
    let arg = rest.first().ok_or_else(usage)?;
    if arg.starts_with(['+', '-']) {
        return arg.parse().map(Command::SeekRelative).map_err(|_| usage());
    }
    let seconds = match arg.split_once(':') {
        Some((minutes, seconds)) => {
            let minutes: u64 = minutes.parse().map_err(|_| usage())?;
            let seconds: u64 = seconds.parse().map_err(|_| usage())?;
            if seconds >= 60 {
                return Err(usage());
            }
            minutes * 60 + seconds
        }
        None => arg.parse().map_err(|_| usage())?,
    };
    Ok(Command::SeekAbsolute(seconds))
}

fn on_off(rest: &[String], what: &str) -> Result<Option<bool>, Command> {
    match rest.first().map(|s| s.to_ascii_lowercase()).as_deref() {
        None | Some("toggle") => Ok(None),
        Some("on") => Ok(Some(true)),
        Some("off") => Ok(Some(false)),
        Some(other) => Err(invalid(format!("{what} takes on or off, not '{other}'"))),
    }
}

fn lyrics_zoom(rest: &[String]) -> Result<Command, Command> {
    let (view, rest) = match rest.first().map(|s| s.to_ascii_lowercase()).as_deref() {
        Some("panel") => (Some(LyricsView::Panel), &rest[1..]),
        Some("focus" | "full") => (Some(LyricsView::Focus), &rest[1..]),
        _ => (None, rest),
    };
    let zoom = match rest.first() {
        None => None,
        Some(arg) => Some(
            arg.trim_end_matches(['x', '×'])
                .parse::<f32>()
                .ok()
                .filter(|z| crate::config::LYRICS_ZOOM.contains(z))
                .ok_or_else(|| invalid("lyrics zoom takes a size from 1 to 4, e.g. 2 or 2.5"))?,
        ),
    };
    Ok(Command::LyricsZoom { view, zoom })
}

fn effect(name: &str, rest: &[String]) -> Result<Command, Command> {
    match name {
        "eq" => {
            let usage = "eq <band 1-10> <dB -12 to 12, or +n/-n>";
            let band: usize = number(rest, "fx eq", usage)?;
            if !(1..=10).contains(&band) {
                return Err(invalid(format!("fx eq: band {band} is not 1 to 10")));
            }
            let amount = amount(&rest[1..], "fx eq", usage)?;
            Ok(Command::SetEq {
                band: band - 1,
                amount,
            })
        }
        "reset" => match rest.first().map(|s| s.to_ascii_lowercase()).as_deref() {
            None | Some("all") => Ok(Command::ResetEffects(Reset::All)),
            Some("eq") => Ok(Command::ResetEffects(Reset::Eq)),
            Some(other) => Effect::from_name(other)
                .map(|e| Command::ResetEffects(Reset::Effect(e)))
                .ok_or_else(|| invalid(format!("fx reset: no effect called '{other}'"))),
        },
        "slowed" => Ok(Command::Macro(Macro::SlowedReverb)),
        "bassboost" => Ok(Command::Macro(Macro::BassBoosted)),
        "spatial" => Ok(Command::Macro(Macro::Spatial)),
        _ => {
            let effect = Effect::from_name(name).expect("every effect spec names an Effect");
            let usage = find(EFFECTS, name).map_or("", |spec| spec.usage);
            let amount = amount(rest, &format!("fx {name}"), usage)?;
            Ok(Command::SetEffect { effect, amount })
        }
    }
}

fn parse(input: &str) -> Result<Command, Command> {
    let tokens = shlex(input);
    let Some((name, rest)) = tokens.split_first() else {
        return Err(invalid("empty command"));
    };
    let spec = find(COMMANDS, name).ok_or_else(|| invalid(format!("no command '{name}'")))?;
    Ok(match spec.name() {
        "play" => Command::Play,
        "pause" => Command::Pause,
        "stop" => Command::Stop,
        "next" => Command::Next,
        "prev" => Command::Prev,
        "seek" => seek(rest)?,
        "vol" => Command::Volume(amount(rest, "vol", spec.usage)?.percent()),
        "speed" => Command::Speed(amount(rest, "speed", spec.usage)?),
        "shuffle" => Command::Shuffle(on_off(rest, "shuffle")?),
        "repeat" => Command::Repeat(
            match rest.first().map(|s| s.to_ascii_lowercase()).as_deref() {
                None => None,
                Some("off") => Some(RepeatMode::Off),
                Some("once") => Some(RepeatMode::PlayOnce),
                Some("track" | "one") => Some(RepeatMode::Track),
                Some("playlist" | "all") => Some(RepeatMode::Playlist),
                Some(other) => {
                    return Err(invalid(format!(
                        "repeat takes off, once, track or playlist, not '{other}'"
                    )));
                }
            },
        ),
        "artist" | "song" | "album" | "genre" | "year" => {
            let field = SearchField::from_str(spec.name()).expect("a search field");
            Command::Search {
                field,
                query: text(rest, spec.name())?,
            }
        }
        "noh" => Command::ClearSearch,
        "sort" => Command::Sort,
        "rescan" => Command::Rescan,
        "dl" => Command::Download {
            query: text(rest, "dl")?,
        },
        "pl" => {
            let (sub, rest) = sub(spec, rest)?;
            let name = text(rest, &format!("pl {sub}"))?;
            match sub {
                "create" => Command::CreatePlaylist { name },
                "del" => Command::DeletePlaylist { name },
                "add" => Command::AddToPlaylist { name },
                _ => Command::RemoveFromPlaylist { name },
            }
        }
        "queue" => {
            sub(spec, rest)?;
            Command::ClearQueue
        }
        "lyrics" => match sub(spec, rest)? {
            ("show", _) => Command::ShowLyrics(Some(true)),
            ("hide", _) => Command::ShowLyrics(Some(false)),
            ("toggle", _) => Command::ShowLyrics(None),
            ("zoom", rest) => lyrics_zoom(rest)?,
            ("fetch", rest) => Command::FetchLyrics {
                query: Some(rest.join(" ")).filter(|q| !q.is_empty()),
            },
            (_, rest) => Command::SearchLyrics {
                query: text(rest, "lyrics search")?,
            },
        },
        "fx" => {
            let (name, rest) = sub(spec, rest)?;
            effect(name, rest)?
        }
        "preset" => {
            if rest.is_empty() {
                return Ok(Command::PresetPicker);
            }
            let (sub, rest) = sub(spec, rest)?;
            match sub {
                "list" => Command::PresetPicker,
                "save" => Command::PresetSave {
                    name: text(rest, "preset save")?,
                },
                "load" => Command::PresetLoad {
                    name: text(rest, "preset load")?,
                },
                _ => Command::PresetDelete {
                    name: text(rest, "preset rm")?,
                },
            }
        }
        "studio" => Command::ToggleStudio,
        "focus" => {
            let arg = rest.first().map_or("", String::as_str);
            let mode = FocusMode::from_str(arg).map_err(|_| {
                invalid(format!(
                    "focus takes full, viz, play, vp or lyrics{}",
                    if arg.is_empty() {
                        String::new()
                    } else {
                        format!(", not '{arg}'")
                    }
                ))
            })?;
            Command::Focus { mode }
        }
        "vis" => {
            let arg = rest.join(" ");
            let style = SpectrumStyle::from_str(&arg)
                .map_err(|_| invalid("vis takes cava, default, bar, particle, radial or nova"))?;
            Command::ChangeVisualizer { arg: style }
        }
        "sidebar" => {
            let width: u16 = number(rest, "sidebar", spec.usage)?;
            if !(28..=80).contains(&width) {
                return Err(invalid(format!("sidebar width {width} is not 28 to 80")));
            }
            Command::Sidebar { width }
        }
        "toggle" => Command::TogglePanel(
            match rest.first().map(|s| s.to_ascii_lowercase()).as_deref() {
                Some("playback") => Panel::Playback,
                Some("lyrics") => Panel::Lyrics,
                Some("art") => Panel::Art,
                Some("info") => Panel::Info,
                Some("studio") => Panel::Studio,
                _ => {
                    return Err(invalid(
                        "toggle takes playback, lyrics, art, info or studio",
                    ));
                }
            },
        ),
        "help" => Command::Help,
        "quit" => Command::Quit,
        other => unreachable!("command '{other}' is in COMMANDS but not parsed"),
    })
}

impl FromStr for Command {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Ok(parse(input).unwrap_or_else(|invalid| invalid))
    }
}

impl Command {
    pub fn parse(input: &str) -> Self {
        parse(input).unwrap_or_else(|invalid| invalid)
    }

    /// One line per command for the help screen: how it is typed, and what it
    /// does.
    pub fn help_entries() -> Vec<(String, String)> {
        COMMANDS
            .iter()
            .map(|spec| match spec.then {
                Then::Sub(subs) => (
                    format!(":{} …", spec.name()),
                    format!(
                        "{}: {}",
                        spec.help,
                        subs.iter().map(|s| s.name()).collect::<Vec<_>>().join(", ")
                    ),
                ),
                Then::Args(_) => (
                    format!(":{} {}", spec.name(), spec.usage)
                        .trim_end()
                        .to_string(),
                    spec.help.to_string(),
                ),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn invalid(command: Command) -> String {
        match command {
            Command::Invalid(why) => why,
            other => panic!("parsed as {other:?}"),
        }
    }

    #[test]
    fn every_name_in_the_table_parses_as_its_command() {
        // A spec without a parser arm would panic in `parse`; any spec whose
        // name does not resolve comes out as "no command".
        for spec in COMMANDS {
            for name in spec.names {
                if let Command::Invalid(why) = Command::parse(name) {
                    assert!(!why.starts_with("no command"), ":{name}: {why}");
                }
            }
            if let Then::Sub(subs) = spec.then {
                for sub in subs {
                    for name in sub.names {
                        let input = format!("{} {name} 1", spec.name());
                        if let Command::Invalid(why) = Command::parse(&input) {
                            assert!(!why.contains("has no"), ":{input}: {why}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn every_effect_has_an_fx_subcommand_with_its_range() {
        for effect in Effect::ALL {
            let spec = find(EFFECTS, effect.name()).unwrap_or_else(|| panic!("{}", effect.name()));
            let (min, max) = effect.range();
            assert_eq!(spec.usage, format!("<{min}-{max}>"), "{}", effect.name());
            assert!(RESETTABLE.contains(&effect.name()));
        }
    }

    #[test]
    fn names_are_unique_at_every_level() {
        fn check(specs: &[Spec]) {
            let mut seen = std::collections::HashSet::new();
            for spec in specs {
                for name in spec.names {
                    assert!(seen.insert(*name), "'{name}' is taken twice");
                }
                if let Then::Sub(subs) = spec.then {
                    check(subs);
                }
            }
        }
        check(COMMANDS);
    }

    #[test]
    fn quit() {
        for name in ["q", "quit", "exit"] {
            assert_eq!(Command::parse(name), Command::Quit);
        }
    }

    #[test]
    fn search_fields() {
        assert_eq!(
            Command::parse("a Adele"),
            Command::Search {
                field: SearchField::Artist,
                query: "Adele".into()
            }
        );
        assert_eq!(
            Command::parse("song Hello there"),
            Command::Search {
                field: SearchField::Song,
                query: "Hello there".into()
            }
        );
        assert_eq!(invalid(Command::parse("a")), "artist needs a query");
    }

    #[test]
    fn playback() {
        assert_eq!(Command::parse("play"), Command::Play);
        assert_eq!(Command::parse("previous"), Command::Prev);
        assert_eq!(Command::parse("seek +10"), Command::SeekRelative(10));
        assert_eq!(Command::parse("seek -5"), Command::SeekRelative(-5));
        assert_eq!(Command::parse("seek 90"), Command::SeekAbsolute(90));
        assert_eq!(Command::parse("seek 1:30"), Command::SeekAbsolute(90));
        assert!(matches!(Command::parse("seek 1:75"), Command::Invalid(_)));
        assert_eq!(Command::parse("vol 80"), Command::Volume(Amount::To(0.8)));
        assert_eq!(
            Command::parse("volume +10"),
            Command::Volume(Amount::By(0.1))
        );
        assert_eq!(
            Command::parse("speed 1.25"),
            Command::Speed(Amount::To(1.25))
        );
        assert_eq!(
            Command::parse("speed -0.1"),
            Command::Speed(Amount::By(-0.1))
        );
        assert_eq!(Command::parse("shuffle"), Command::Shuffle(None));
        assert_eq!(Command::parse("shuffle off"), Command::Shuffle(Some(false)));
        assert_eq!(Command::parse("repeat"), Command::Repeat(None));
        assert_eq!(
            Command::parse("repeat track"),
            Command::Repeat(Some(RepeatMode::Track))
        );
        assert!(matches!(
            Command::parse("repeat sometimes"),
            Command::Invalid(_)
        ));
    }

    #[test]
    fn lyrics() {
        assert_eq!(
            Command::parse("lyrics show"),
            Command::ShowLyrics(Some(true))
        );
        assert_eq!(Command::parse("lyrics toggle"), Command::ShowLyrics(None));
        assert_eq!(
            Command::parse("lyrics zoom 2.5"),
            Command::LyricsZoom {
                view: None,
                zoom: Some(2.5)
            }
        );
        assert_eq!(
            Command::parse("lyrics zoom focus 3x"),
            Command::LyricsZoom {
                view: Some(LyricsView::Focus),
                zoom: Some(3.0)
            }
        );
        assert_eq!(
            Command::parse("lyrics zoom"),
            Command::LyricsZoom {
                view: None,
                zoom: None
            }
        );
        assert!(matches!(
            Command::parse("lyrics zoom 9"),
            Command::Invalid(_)
        ));
        assert_eq!(
            Command::parse("lyrics fetch"),
            Command::FetchLyrics { query: None }
        );
        assert_eq!(
            Command::parse("lyrics fetch Adele - Hello"),
            Command::FetchLyrics {
                query: Some("Adele - Hello".into())
            }
        );
        assert_eq!(
            invalid(Command::parse("lyrics")),
            "lyrics needs one of: show, hide, toggle, zoom, fetch, search"
        );
    }

    #[test]
    fn effects() {
        assert_eq!(
            Command::parse("fx bass 2"),
            Command::SetEffect {
                effect: Effect::Bass,
                amount: Amount::To(2.0)
            }
        );
        assert_eq!(
            Command::parse("fx pan -0.5"),
            Command::SetEffect {
                effect: Effect::Pan,
                amount: Amount::By(-0.5)
            }
        );
        assert_eq!(
            Command::parse("fx eq 3 -4"),
            Command::SetEq {
                band: 2,
                amount: Amount::By(-4.0)
            }
        );
        assert!(matches!(Command::parse("fx eq 11 2"), Command::Invalid(_)));
        assert_eq!(
            Command::parse("fx reset"),
            Command::ResetEffects(Reset::All)
        );
        assert_eq!(
            Command::parse("fx reset eq"),
            Command::ResetEffects(Reset::Eq)
        );
        assert_eq!(
            Command::parse("fx reset echo"),
            Command::ResetEffects(Reset::Effect(Effect::Echo))
        );
        assert_eq!(
            Command::parse("fx slowed"),
            Command::Macro(Macro::SlowedReverb)
        );
        assert_eq!(
            invalid(Command::parse("fx bass")),
            "fx bass needs a number: <0.1-5>"
        );
        assert!(invalid(Command::parse("fx loud 2")).starts_with("fx has no 'loud'"));
    }

    #[test]
    fn view() {
        assert_eq!(
            Command::parse("v radial"),
            Command::ChangeVisualizer {
                arg: SpectrumStyle::Radial
            }
        );
        assert!(matches!(Command::parse("v"), Command::Invalid(_)));
        assert_eq!(
            Command::parse("focus lyrics"),
            Command::Focus {
                mode: FocusMode::LyricsPlayback
            }
        );
        assert_eq!(Command::parse("sidebar 40"), Command::Sidebar { width: 40 });
        assert!(matches!(Command::parse("sidebar 10"), Command::Invalid(_)));
        assert_eq!(
            Command::parse("toggle art"),
            Command::TogglePanel(Panel::Art)
        );
        assert_eq!(Command::parse("studio"), Command::ToggleStudio);
        assert_eq!(Command::parse("help"), Command::Help);
    }

    #[test]
    fn playlists_and_presets() {
        assert_eq!(
            Command::parse("pl create My Playlist"),
            Command::CreatePlaylist {
                name: "My Playlist".into()
            }
        );
        assert_eq!(
            invalid(Command::parse("pl create")),
            "pl create needs a name"
        );
        assert_eq!(
            invalid(Command::parse("pl add")),
            "pl add needs a playlist's name"
        );
        assert_eq!(
            Command::parse("playlist remove Old"),
            Command::RemoveFromPlaylist { name: "Old".into() }
        );
        assert_eq!(Command::parse("preset"), Command::PresetPicker);
        assert_eq!(
            Command::parse("dsp load Night drive"),
            Command::PresetLoad {
                name: "Night drive".into()
            }
        );
        assert_eq!(Command::parse("queue clear"), Command::ClearQueue);
    }

    #[test]
    fn unknown() {
        assert_eq!(invalid(Command::parse("dance")), "no command 'dance'");
        assert_eq!(invalid(Command::parse("")), "empty command");
    }

    #[test]
    fn help_has_a_line_per_command() {
        let entries = Command::help_entries();
        assert_eq!(entries.len(), COMMANDS.len());
        assert!(entries.contains(&(
            ":seek <+s|-s|s|m:ss>".into(),
            "Seek by or to a position".into()
        )));
        assert!(
            entries
                .iter()
                .any(|(syntax, help)| syntax == ":lyrics …" && help.contains("zoom"))
        );
    }
}
