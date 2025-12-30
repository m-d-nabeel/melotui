#[derive(Debug)]
pub enum Command {
    Quit,
    Rescan,
    Download { query: String },
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

            _ => Command::Unknown(cmd.to_string()),
        };
        tracing::info!("Result CMD: {:?}", result_cmd);
        result_cmd
    }
}
