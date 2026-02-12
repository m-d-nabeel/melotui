use crate::state::SpectrumStyle;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Quit,
    Rescan,
    Download { query: String },
    ChangeVisualizer { arg: SpectrumStyle },
    Unknown(String),
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::Quit => write!(f, "quit"),
            Command::Rescan => write!(f, "rescan"),
            Command::Download { query } => write!(f, "download {}", query),
            Command::ChangeVisualizer { arg } => write!(f, "visualizer {:?}", arg),
            Command::Unknown(s) => write!(f, "unknown: {}", s),
        }
    }
}

impl FromStr for Command {
    type Err = (); // We used Unknown variant for errors in the original code, so Err is not really used meaningfully here, or we could use String.
    // The original parse function returned Command, so FromStr should probably succeed and return Unknown on failure if we want to mimic it,
    // but FromStr usually returns Result.
    // However, the action handler expects a Command.
    // I'll make FromStr return Ok(Command) always, using Unknown for errors, to match previous behavior of `parse`.
    // Or better, return Result and let the caller handle it?
    // The previous `parse` returned `Command::Unknown`.
    // I will stick to that pattern for `FromStr` to be "infallible" in a sense that it returns a Command object.

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

            _ => Ok(Command::Unknown(cmd.to_string())),
        }
    }
}

impl Command {
    pub fn parse(input: &str) -> Self {
        Self::from_str(input).unwrap_or(Command::Unknown("parse error".into()))
    }
}
