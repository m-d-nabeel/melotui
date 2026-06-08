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
    Search { field: SearchField, query: String },
    ChangeVisualizer { arg: SpectrumStyle },
    ClearSearch,
    CreatePlaylist { name: String },
    DeletePlaylist { name: String },
    AddToPlaylist { name: String },
    RemoveFromPlaylist { name: String },
    Focus { mode: FocusMode },
    Sidebar { width: u16 },
    Unknown(String),
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::Quit => write!(f, "quit"),
            Command::Rescan => write!(f, "rescan"),
            Command::Download { query } => write!(f, "download {}", query),
            Command::Search { field, query } => write!(f, "{} {}", field, query),
            Command::ChangeVisualizer { arg } => write!(f, "visualizer {:?}", arg),
            Command::ClearSearch => write!(f, "noh"),
            Command::CreatePlaylist { name } => write!(f, "pl create {}", name),
            Command::DeletePlaylist { name } => write!(f, "pl del {}", name),
            Command::AddToPlaylist { name } => write!(f, "pl add {}", name),
            Command::RemoveFromPlaylist { name } => write!(f, "pl rm {}", name),
            Command::Focus { mode } => write!(f, "focus {:?}", mode),
            Command::Sidebar { width } => write!(f, "sidebar {}", width),
            Command::Unknown(s) => write!(f, "unknown: {}", s),
        }
    }
}

impl FromStr for Command {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return Ok(Command::Unknown("empty command".into()));
        }

        let mut parts = trimmed.split_whitespace();
        let cmd = parts.next().unwrap_or("");

        match cmd {
            "q" | "quit" | "exit" => Ok(Command::Quit),
            "r" | "rescan" | "scan" => Ok(Command::Rescan),
            "noh" | "nohlsearch" | "clear" => Ok(Command::ClearSearch),

            "download" | "dl" => {
                let query = parts.collect::<Vec<_>>().join(" ");
                if query.is_empty() {
                    Ok(Command::Unknown("download requires a query".into()))
                } else {
                    Ok(Command::Download { query })
                }
            }

            "v" | "vis" => {
                let arg = parts.collect::<Vec<_>>().join(" ");
                if arg.is_empty() {
                    Ok(Command::Unknown("Visualizer requires an argument".into()))
                } else if let Ok(style) = SpectrumStyle::from_str(&arg) {
                    Ok(Command::ChangeVisualizer { arg: style })
                } else {
                    Ok(Command::Unknown(
                        "Allowed: Cava, Default, Bar, Particle".into(),
                    ))
                }
            }

            "pl" | "playlist" => {
                let sub = parts.next().unwrap_or("");
                let name = parts.collect::<Vec<_>>().join(" ");
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

            // search by field: "artist adele", "song hello", etc.
            _ if SearchField::from_str(cmd).is_ok() => {
                let field = SearchField::from_str(cmd).unwrap();
                let query = parts.collect::<Vec<_>>().join(" ");
                if query.is_empty() {
                    Ok(Command::Unknown(format!("{} requires a query", field)))
                } else {
                    Ok(Command::Search { field, query })
                }
            }

            // focus mode
            "focus" | "zen" => {
                let arg = parts.next().unwrap_or("");
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
                let arg = parts.next().unwrap_or("");
                match arg.parse::<u16>() {
                    Ok(w) if (28..=80).contains(&w) => Ok(Command::Sidebar { width: w }),
                    Ok(w) => Ok(Command::Unknown(format!(
                        "Sidebar width {} out of range (28-80)",
                        w
                    ))),
                    Err(_) => Ok(Command::Unknown("sidebar requires a number (28-80)".into())),
                }
            }

            _ => Ok(Command::Unknown(cmd.to_string())),
        }
    }
}

impl Command {
    pub fn parse(input: &str) -> Self {
        Self::from_str(input).unwrap_or(Command::Unknown("parse error".into()))
    }
}
