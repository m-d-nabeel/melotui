//! Completion for the command bar, from the commands in [`COMMANDS`].

use crate::command::{COMMANDS, Spec, Then, Values, find};

/// Names completion offers that the command table cannot know.
#[derive(Debug, Clone, Default)]
pub struct Names {
    pub playlists: Vec<String>,
    pub presets: Vec<String>,
}

/// One way to complete what has been typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    /// The whole input with this completion in it.
    pub text: String,
    /// What is offered, as the list shows it.
    pub label: String,
    /// What follows it, as help shows it.
    pub usage: &'static str,
    pub help: &'static str,
}

fn starts_with(name: &str, prefix: &str) -> bool {
    name.to_lowercase().starts_with(&prefix.to_lowercase())
}

/// The input split into the words typed in full and the one being typed.
fn split(input: &str) -> (Vec<&str>, &str) {
    let mut words: Vec<&str> = input.split_whitespace().collect();
    let partial = if input.is_empty() || input.ends_with(char::is_whitespace) {
        ""
    } else {
        words.pop().unwrap_or("")
    };
    (words, partial)
}

/// What follows the first `count` words of `input`.
fn after_words(input: &str, count: usize) -> &str {
    let mut rest = input.trim_start();
    for _ in 0..count {
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        rest = rest[end..].trim_start();
    }
    rest
}

/// Where the words typed so far lead: the commands still to choose from, or
/// the command whose arguments are being typed and those typed already.
enum Place<'a> {
    Choosing(&'static [Spec]),
    Arguments(&'static Spec, &'static [Values], &'a [&'a str]),
}

/// Follows `done` through the command table; `None` if a word is not a
/// command there.
fn walk<'a>(done: &'a [&'a str]) -> Option<Place<'a>> {
    let mut level: &'static [Spec] = COMMANDS;
    for (i, word) in done.iter().enumerate() {
        let spec = find(level, word)?;
        match spec.then {
            Then::Sub(subs) => level = subs,
            Then::Args(args) => return Some(Place::Arguments(spec, args, &done[i + 1..])),
        }
    }
    Some(Place::Choosing(level))
}

/// Every completion of `input`, in the order the command table lists them.
pub fn complete(input: &str, names: &Names) -> Vec<Suggestion> {
    let (done, partial) = split(input);
    let prefix: String = done.iter().map(|word| format!("{word} ")).collect();
    match walk(&done) {
        None => Vec::new(),
        Some(Place::Choosing(specs)) => specs
            .iter()
            .filter_map(|spec| {
                // The command's own name if it fits, else the alias that does.
                let name = spec.names.iter().find(|name| starts_with(name, partial))?;
                let space = if spec.takes_more() { " " } else { "" };
                Some(Suggestion {
                    text: format!("{prefix}{name}{space}"),
                    label: name.to_string(),
                    usage: spec.usage,
                    help: spec.help,
                })
            })
            .collect(),
        Some(Place::Arguments(spec, args, typed)) => {
            // A name is the only argument and may run to several words.
            let index = match args.first() {
                Some(Values::Playlist | Values::Preset) => 0,
                _ => typed.len(),
            };
            let Some(values) = args.get(index) else {
                return Vec::new();
            };
            let more = index + 1 < args.len();
            match values {
                Values::Free => Vec::new(),
                Values::Words(words) => words
                    .iter()
                    .filter(|word| starts_with(word, partial))
                    .map(|word| Suggestion {
                        text: format!("{prefix}{word}{}", if more { " " } else { "" }),
                        label: word.to_string(),
                        usage: "",
                        help: spec.help,
                    })
                    .collect(),
                Values::Playlist | Values::Preset => {
                    // Names can have spaces in them, so everything after the
                    // command is the name being typed.
                    let words = done.len() - typed.len();
                    let command: String = done[..words].iter().map(|w| format!("{w} ")).collect();
                    let typed = after_words(input, words);
                    let (pool, help) = if *values == Values::Playlist {
                        (&names.playlists, "playlist")
                    } else {
                        (&names.presets, "preset")
                    };
                    pool.iter()
                        .filter(|name| starts_with(name, typed))
                        .map(|name| Suggestion {
                            text: format!("{command}{name}"),
                            label: name.clone(),
                            usage: "",
                            help,
                        })
                        .collect()
                }
            }
        }
    }
}

/// What the command being typed still takes, as help shows it —
/// `<0-150|+n|-n>` — and what it does, once its name is typed in full.
pub fn describe(input: &str) -> Option<(&'static str, &'static str)> {
    let (done, _) = split(input);
    match walk(&done)? {
        Place::Arguments(spec, _, _) => Some((spec.usage, spec.help)),
        Place::Choosing(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(input: &str) -> Vec<String> {
        complete(input, &names())
            .into_iter()
            .map(|s| s.label)
            .collect()
    }

    fn texts(input: &str) -> Vec<String> {
        complete(input, &names())
            .into_iter()
            .map(|s| s.text)
            .collect()
    }

    fn names() -> Names {
        Names {
            playlists: vec!["Road Trip".into(), "Rainy days".into(), "Gym".into()],
            presets: vec!["Night drive".into()],
        }
    }

    #[test]
    fn nothing_typed_offers_every_command() {
        assert_eq!(complete("", &names()).len(), COMMANDS.len());
    }

    #[test]
    fn command_names_by_prefix() {
        assert_eq!(labels("pa"), ["pause"]);
        assert_eq!(texts("pa"), ["pause"]);
        assert_eq!(
            labels("s"),
            [
                "stop", "seek", "speed", "shuffle", "song", "sort", "scan", "studio", "sidebar"
            ]
        );
        // An alias is offered when only it fits.
        assert_eq!(labels("volu"), ["volume"]);
        // Commands that take more get a space, ready for it.
        assert_eq!(texts("lyr"), ["lyrics "]);
    }

    #[test]
    fn subcommands() {
        assert_eq!(
            labels("lyrics "),
            ["show", "hide", "toggle", "zoom", "fetch", "search"]
        );
        assert_eq!(texts("lyrics z"), ["lyrics zoom "]);
        assert_eq!(labels("pl a"), ["add"]);
        assert_eq!(labels("fx b"), ["bass", "bitcrush", "bassboost"]);
        assert_eq!(texts("fx slo"), ["fx slowed"]);
    }

    #[test]
    fn arguments_from_fixed_words() {
        assert_eq!(labels("focus "), ["full", "viz", "play", "vp", "lyrics"]);
        assert_eq!(texts("focus l"), ["focus lyrics"]);
        assert_eq!(labels("vis ra"), ["radial"]);
        assert_eq!(labels("shuffle o"), ["on", "off"]);
        assert_eq!(labels("repeat t"), ["track"]);
        assert_eq!(texts("lyrics zoom p"), ["lyrics zoom panel "]);
        assert_eq!(texts("fx eq 1"), ["fx eq 1 ", "fx eq 10 "]);
        assert_eq!(labels("fx reset e"), ["echo", "exciter", "eq"]);
    }

    #[test]
    fn playlist_and_preset_names_even_with_spaces() {
        assert_eq!(labels("pl add "), ["Road Trip", "Rainy days", "Gym"]);
        assert_eq!(texts("pl add r"), ["pl add Road Trip", "pl add Rainy days"]);
        assert_eq!(texts("pl rm Road T"), ["pl rm Road Trip"]);
        assert_eq!(texts("preset load ni"), ["preset load Night drive"]);
        // Creating one has nothing to offer.
        assert!(labels("pl create ").is_empty());
    }

    #[test]
    fn free_text_and_numbers_have_nothing_to_offer_but_a_description() {
        assert!(labels("vol ").is_empty());
        assert_eq!(
            describe("vol 8"),
            Some(("<0-150|+n|-n>", "Volume in percent"))
        );
        assert_eq!(
            describe("fx eq 3 "),
            Some(("<1-10> <-12-12>", "EQ band gain in dB"))
        );
        assert_eq!(describe("fx"), None);
    }

    #[test]
    fn nothing_for_what_is_not_a_command() {
        assert!(labels("dance ").is_empty());
        assert!(labels("pl dance ").is_empty());
        assert!(labels("q extra").is_empty());
    }
}
