use crate::state::SpectrumStyle;
use std::str::FromStr;

#[derive(Debug)]
pub enum Command {
    Quit,
    Rescan,
    Download { query: String },
    ChangeVisualizer { arg: SpectrumStyle },
    Unknown(String),
}

impl Command {
    pub fn parse(input: &str) -> Self {
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return Command::Unknown("empty command".into());
        }

        let mut parts = trimmed.split_whitespace();
        let cmd = parts.next().unwrap();

        match cmd {
            "q" | "quit" | "exit" => Command::Quit,
            "r" | "rescan" | "scan" => Command::Rescan,

            "download" | "dl" => {
                let query = parts.collect::<Vec<_>>().join(" ");
                if query.is_empty() {
                    Command::Unknown("download requires a query".into())
                } else {
                    Command::Download { query }
                }
            }

            "v" | "vis" => {
                let arg = parts.collect::<Vec<_>>().join(" ");
                if arg.is_empty() {
                    Command::Unknown("Visualizer requires an argument".into())
                } else if let Ok(style) = SpectrumStyle::from_str(&arg) {
                    Command::ChangeVisualizer { arg: style }
                } else {
                    Command::Unknown("Allowed: Cava, Default, Bar, Particle".into())
                }
            }

            _ => Command::Unknown(cmd.to_string()),
        }
    }
}
