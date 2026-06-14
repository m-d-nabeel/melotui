use std::fs::{self};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

const MAX_HISTORY: usize = 100;

#[derive(Debug, Clone, Default)]
pub struct CommandHistory {
    entries: Vec<String>,
}

impl CommandHistory {
    pub fn new() -> Self {
        let mut history = Self::default();
        history.load();
        history
    }

    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    pub fn push(&mut self, command: String) {
        if command.trim().is_empty() {
            return;
        }

        // Don't add if it's identical to the last command
        if self.entries.last() == Some(&command) {
            return;
        }

        self.entries.push(command.clone());
        if self.entries.len() > MAX_HISTORY {
            self.entries.remove(0);
        }

        self.save();
    }

    fn file_path() -> Option<PathBuf> {
        #[cfg(test)]
        {
            let mut path = std::env::temp_dir();
            path.push(format!("melotui_test_{}", std::process::id()));
            return Some(path.join("command_history"));
        }

        #[cfg(not(test))]
        {
            let mut path = dirs::data_dir()?;
            path.push("melotui");
            Some(path.join("command_history"))
        }
    }

    fn load(&mut self) {
        let path = match Self::file_path() {
            Some(p) => p,
            None => return,
        };

        if let Ok(file) = fs::File::open(&path) {
            let reader = BufReader::new(file);
            let mut lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

            if lines.len() > MAX_HISTORY {
                let skip = lines.len() - MAX_HISTORY;
                lines.drain(0..skip);
            }

            self.entries = lines;
        }
    }

    fn save(&self) {
        let path = match Self::file_path() {
            Some(p) => p,
            None => return,
        };

        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if let Ok(mut file) = fs::File::create(&path) {
            for entry in &self.entries {
                let _ = writeln!(file, "{}", entry);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_push_and_dedup() {
        let mut history = CommandHistory::default(); // default won't load from disk
        history.push("pl create A".to_string());
        history.push("pl create B".to_string());

        // Push duplicate consecutively
        history.push("pl create B".to_string());
        assert_eq!(history.entries().len(), 2);

        // Push empty string
        history.push("   ".to_string());
        assert_eq!(history.entries().len(), 2);
    }

    #[test]
    fn test_history_max_capacity() {
        let mut history = CommandHistory::default();
        for i in 0..105 {
            history.push(format!("cmd {}", i));
        }
        assert_eq!(history.entries().len(), MAX_HISTORY);
        assert_eq!(history.entries().first().unwrap(), "cmd 5"); // 0-4 removed
        assert_eq!(history.entries().last().unwrap(), "cmd 104");
    }
}
