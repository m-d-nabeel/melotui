use crate::state_registry::SpectrumStyle;

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

        // trimmed not empty will always return
        let mut parts = trimmed.split_whitespace();
        let cmd = parts.next().unwrap();

        let result_cmd = match cmd {
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
                } else if let Some(style) = SpectrumStyle::from_str(&arg) {
                    Command::ChangeVisualizer { arg: style }
                } else {
                    Command::Unknown("Only Cava, Default, and Bar are allowed.".into())
                }
            }

            _ => Command::Unknown(cmd.to_string()),
        };
        tracing::info!("Result CMD: {:?}", result_cmd);
        result_cmd
    }
}
