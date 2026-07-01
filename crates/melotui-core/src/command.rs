use crate::state::{SpectrumStyle, ui::FocusMode};
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

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Quit,
    Rescan,
    Download { query: String },
    FetchLyrics { query: Option<String> },
    SearchLyrics { query: String },
    Search { field: SearchField, query: String },
    ChangeVisualizer { arg: SpectrumStyle },
    ClearSearch,
    CreatePlaylist { name: String },
    DeletePlaylist { name: String },
    AddToPlaylist { name: String },
    RemoveFromPlaylist { name: String },
    Focus { mode: FocusMode },
    Sidebar { width: u16 },
    SeekRelative(i64),
    SeekAbsolute(u64),
    PresetSave { name: String },
    PresetLoad { name: String },
    PresetDelete { name: String },
    PresetPicker,
    Unknown(String),
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::Quit => write!(f, "quit"),
            Command::Rescan => write!(f, "rescan"),
            Command::Download { query } => write!(f, "download {}", query),
            Command::FetchLyrics { query } => {
                if let Some(q) = query {
                    write!(f, "lyrics fetch {}", q)
                } else {
                    write!(f, "lyrics fetch")
                }
            }
            Command::SearchLyrics { query } => write!(f, "lyrics search {}", query),
            Command::Search { field, query } => write!(f, "search {:?} {}", field, query),
            Command::ChangeVisualizer { arg } => write!(f, "visualizer {:?}", arg),
            Command::ClearSearch => write!(f, "noh"),
            Command::CreatePlaylist { name } => write!(f, "pl create {}", name),
            Command::DeletePlaylist { name } => write!(f, "pl del {}", name),
            Command::AddToPlaylist { name } => write!(f, "pl add {}", name),
            Command::RemoveFromPlaylist { name } => write!(f, "pl rm {}", name),
            Command::Focus { mode } => write!(f, "focus {:?}", mode),
            Command::Sidebar { width } => write!(f, "sidebar {}", width),
            Command::SeekRelative(offset) => write!(f, "seek {:+}", offset),
            Command::SeekAbsolute(pos) => write!(f, "seek {}", pos),
            Command::PresetSave { name } => write!(f, "preset save {}", name),
            Command::PresetLoad { name } => write!(f, "preset load {}", name),
            Command::PresetDelete { name } => write!(f, "preset rm {}", name),
            Command::PresetPicker => write!(f, "preset"),
            Command::Unknown(s) => write!(f, "unknown: {}", s),
        }
    }
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

impl FromStr for Command {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut tokens = shlex(input);
        if tokens.is_empty() {
            return Ok(Command::Unknown("empty command".into()));
        }

        let cmd = tokens.remove(0);

        match cmd.as_str() {
            "q" | "quit" | "exit" => Ok(Command::Quit),
            "r" | "rescan" | "scan" => Ok(Command::Rescan),
            "noh" | "nohlsearch" | "clear" => Ok(Command::ClearSearch),

            "download" | "dl" => {
                let query = tokens.join(" ");
                if query.is_empty() {
                    Ok(Command::Unknown("download requires a query".into()))
                } else {
                    Ok(Command::Download { query })
                }
            }

            "lyrics" => {
                let mut tokens_iter = tokens.iter();
                let sub = tokens_iter.next().map(|s| s.as_str()).unwrap_or("");
                match sub {
                    "fetch" => {
                        let query = tokens_iter.cloned().collect::<Vec<_>>().join(" ");
                        if query.is_empty() {
                            Ok(Command::FetchLyrics { query: None })
                        } else {
                            Ok(Command::FetchLyrics { query: Some(query) })
                        }
                    }
                    "search" => {
                        let query = tokens_iter.cloned().collect::<Vec<_>>().join(" ");
                        if query.is_empty() {
                            Ok(Command::Unknown("lyrics search requires a query".into()))
                        } else {
                            Ok(Command::SearchLyrics { query })
                        }
                    }
                    _ => Ok(Command::Unknown(format!(
                        "lyrics: unknown subcommand '{}'",
                        sub
                    ))),
                }
            }

            "v" | "vis" => {
                let arg = tokens.join(" ");
                if arg.is_empty() {
                    Ok(Command::Unknown("Visualizer requires an argument".into()))
                } else if let Ok(style) = SpectrumStyle::from_str(&arg) {
                    Ok(Command::ChangeVisualizer { arg: style })
                } else {
                    Ok(Command::Unknown(
                        "Allowed: Cava, Default, Radial, Nova".into(),
                    ))
                }
            }

            "pl" | "playlist" => {
                let sub = tokens.first().map(|s| s.as_str()).unwrap_or("");
                let name = if tokens.len() > 1 {
                    tokens[1..].join(" ")
                } else {
                    String::new()
                };
                match sub {
                    "create" | "new" => {
                        if name.is_empty() {
                            Ok(Command::Unknown("pl create requires a name".into()))
                        } else {
                            Ok(Command::CreatePlaylist { name })
                        }
                    }
                    "del" | "delete" => {
                        if name.is_empty() {
                            Ok(Command::Unknown("pl del requires a name".into()))
                        } else {
                            Ok(Command::DeletePlaylist { name })
                        }
                    }
                    "add" => {
                        if name.is_empty() {
                            Ok(Command::Unknown("pl add requires a playlist name".into()))
                        } else {
                            Ok(Command::AddToPlaylist { name })
                        }
                    }
                    "remove" | "rm" => {
                        if name.is_empty() {
                            Ok(Command::Unknown("pl rm requires a playlist name".into()))
                        } else {
                            Ok(Command::RemoveFromPlaylist { name })
                        }
                    }
                    _ => Ok(Command::Unknown(format!(
                        "pl: unknown subcommand '{}'",
                        sub
                    ))),
                }
            }

            "preset" | "dsp" => {
                let sub = tokens.first().map(|s| s.as_str()).unwrap_or("");
                let name = if tokens.len() > 1 {
                    tokens[1..].join(" ")
                } else {
                    String::new()
                };
                match sub {
                    "" | "list" | "picker" => Ok(Command::PresetPicker),
                    "save" => {
                        if name.is_empty() {
                            Ok(Command::Unknown("preset save requires a name".into()))
                        } else {
                            Ok(Command::PresetSave { name })
                        }
                    }
                    "load" => {
                        if name.is_empty() {
                            Ok(Command::Unknown("preset load requires a name".into()))
                        } else {
                            Ok(Command::PresetLoad { name })
                        }
                    }
                    "rm" | "del" | "delete" => {
                        if name.is_empty() {
                            Ok(Command::Unknown("preset rm requires a name".into()))
                        } else {
                            Ok(Command::PresetDelete { name })
                        }
                    }
                    _ => Ok(Command::Unknown(format!(
                        "preset: unknown subcommand '{}'",
                        sub
                    ))),
                }
            }

            // search by field: "artist adele", "song hello", etc.
            _ if SearchField::from_str(&cmd).is_ok() => {
                let field = SearchField::from_str(&cmd).unwrap();
                let query = tokens.join(" ");
                if query.is_empty() {
                    Ok(Command::Unknown(format!("{} requires a query", field)))
                } else {
                    Ok(Command::Search { field, query })
                }
            }

            // focus mode
            "focus" | "zen" => {
                let arg = tokens.first().map(|s| s.as_str()).unwrap_or("");
                if arg.is_empty() {
                    Ok(Command::Unknown(
                        "focus requires a mode: full, viz, play, vp".into(),
                    ))
                } else if let Ok(mode) = FocusMode::from_str(arg) {
                    Ok(Command::Focus { mode })
                } else {
                    Ok(Command::Unknown(format!(
                        "Unknown focus mode '{}'. Use: full, viz, play, vp",
                        arg
                    )))
                }
            }

            // sidebar width
            "sidebar" | "sb" => {
                let arg = tokens.first().map(|s| s.as_str()).unwrap_or("");
                match arg.parse::<u16>() {
                    Ok(w) if (28..=80).contains(&w) => Ok(Command::Sidebar { width: w }),
                    Ok(w) => Ok(Command::Unknown(format!(
                        "Sidebar width {} out of range (28-80)",
                        w
                    ))),
                    Err(_) => Ok(Command::Unknown("sidebar requires a number (28-80)".into())),
                }
            }

            // seeking
            "seek" => {
                let arg = tokens.first().map(|s| s.as_str()).unwrap_or("");
                if arg.is_empty() {
                    return Ok(Command::Unknown(
                        "seek requires an offset (e.g. +10, -5, or 60)".into(),
                    ));
                }

                if arg.starts_with('+') || arg.starts_with('-') {
                    match arg.parse::<i64>() {
                        Ok(offset) => Ok(Command::SeekRelative(offset)),
                        Err(_) => Ok(Command::Unknown(
                            "seek relative requires a valid number".into(),
                        )),
                    }
                } else {
                    match arg.parse::<u64>() {
                        Ok(pos) => Ok(Command::SeekAbsolute(pos)),
                        Err(_) => Ok(Command::Unknown(
                            "seek absolute requires a valid positive number".into(),
                        )),
                    }
                }
            }

            _ => Ok(Command::Unknown(cmd)),
        }
    }
}

impl Command {
    pub fn parse(input: &str) -> Self {
        Self::from_str(input).unwrap_or(Command::Unknown("parse error".into()))
    }

    pub fn help_entries() -> &'static [(&'static str, &'static str)] {
        &[
            (":q | :quit | :exit ", "Quit App"),
            (":r | :rescan       ", "Rescan Lib"),
            (":dl | :download <q>", "Download"),
            (":lyrics fetch      ", "Fetch Lyrics"),
            (":v | :vis <style>  ", "Visualizer"),
            (":noh               ", "Clear Search"),
            (":focus <mode>      ", "Focus Mode"),
            (":sidebar <n>       ", "Sidebar Width"),
            (":pl create | :playlist create <n>", "New Playlist"),
            (":pl del | :playlist del <n>   ", "Del Playlist"),
            (":pl add | :playlist add <n>   ", "Add to Pl"),
            (":pl rm | :playlist rm <n>    ", "Rem from Pl"),
            (":artist <q>        ", "Search Artist"),
            (":song <q>          ", "Search Song"),
            (":album <q>         ", "Search Album"),
            (":seek <+/->n       ", "Seek offset"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_quit() {
        assert_eq!(Command::parse("q"), Command::Quit);
        assert_eq!(Command::parse("quit"), Command::Quit);
        assert_eq!(Command::parse("exit"), Command::Quit);
    }

    #[test]
    fn test_parse_search_field() {
        assert_eq!(
            Command::parse("a Adele"),
            Command::Search {
                field: SearchField::Artist,
                query: "Adele".into()
            }
        );
        assert_eq!(
            Command::parse("artist Adele"),
            Command::Search {
                field: SearchField::Artist,
                query: "Adele".into()
            }
        );
        assert_eq!(
            Command::parse("song Hello"),
            Command::Search {
                field: SearchField::Song,
                query: "Hello".into()
            }
        );
    }

    #[test]
    fn test_parse_search_empty() {
        assert_eq!(
            Command::parse("a"),
            Command::Unknown("artist requires a query".into())
        );
    }

    #[test]
    fn test_parse_visualizer() {
        assert_eq!(
            Command::parse("v radial"),
            Command::ChangeVisualizer {
                arg: SpectrumStyle::Radial
            }
        );
        assert_eq!(
            Command::parse("v"),
            Command::Unknown("Visualizer requires an argument".into())
        );
        assert_eq!(
            Command::parse("v unknown"),
            Command::Unknown("Allowed: Cava, Default, Radial, Nova".into())
        );
    }

    #[test]
    fn test_parse_playlist() {
        assert_eq!(
            Command::parse("pl create My Playlist"),
            Command::CreatePlaylist {
                name: "My Playlist".into()
            }
        );
        assert_eq!(
            Command::parse("pl create"),
            Command::Unknown("pl create requires a name".into())
        );
        assert_eq!(
            Command::parse("pl rm Old"),
            Command::RemoveFromPlaylist { name: "Old".into() }
        );
    }

    #[test]
    fn test_parse_sidebar() {
        assert_eq!(Command::parse("sidebar 40"), Command::Sidebar { width: 40 });
        assert_eq!(
            Command::parse("sidebar 20"),
            Command::Unknown("Sidebar width 20 out of range (28-80)".into())
        );
        assert_eq!(
            Command::parse("sidebar abc"),
            Command::Unknown("sidebar requires a number (28-80)".into())
        );
    }

    #[test]
    fn test_parse_empty() {
        assert_eq!(Command::parse(""), Command::Unknown("empty command".into()));
        assert_eq!(
            Command::parse("   "),
            Command::Unknown("empty command".into())
        );
    }

    #[test]
    fn test_parse_seek() {
        assert_eq!(Command::parse("seek 10"), Command::SeekAbsolute(10));
        assert_eq!(Command::parse("seek 0"), Command::SeekAbsolute(0));
        assert_eq!(Command::parse("seek +15"), Command::SeekRelative(15));
        assert_eq!(Command::parse("seek -5"), Command::SeekRelative(-5));

        assert_eq!(
            Command::parse("seek"),
            Command::Unknown("seek requires an offset (e.g. +10, -5, or 60)".into())
        );
        assert_eq!(
            Command::parse("seek abc"),
            Command::Unknown("seek absolute requires a valid positive number".into())
        );
        assert_eq!(
            Command::parse("seek +abc"),
            Command::Unknown("seek relative requires a valid number".into())
        );
        assert_eq!(
            Command::parse("seek +"),
            Command::Unknown("seek relative requires a valid number".into())
        );
    }

    #[test]
    fn test_shlex() {
        assert_eq!(
            super::shlex("pl create My Playlist"),
            vec!["pl", "create", "My", "Playlist"]
        );
        assert_eq!(
            super::shlex("pl create \"My Playlist\""),
            vec!["pl", "create", "My Playlist"]
        );
        assert_eq!(
            super::shlex("search artist \"The Beatles\" song \"Let It Be\""),
            vec!["search", "artist", "The Beatles", "song", "Let It Be"]
        );
    }

    #[test]
    fn test_parse_quotes() {
        assert_eq!(
            Command::parse("pl create \"My Playlist\""),
            Command::CreatePlaylist {
                name: "My Playlist".into()
            }
        );
        assert_eq!(
            Command::parse("pl add \"My Playlist\""),
            Command::AddToPlaylist {
                name: "My Playlist".into()
            }
        );
        assert_eq!(
            Command::parse("artist \"The Beatles\""),
            Command::Search {
                field: SearchField::Artist,
                query: "The Beatles".into()
            }
        );
    }
}
