use crate::command::Command;

pub fn suggest(input: &str, current_cycle: usize) -> (Option<String>, usize) {
    let entries = Command::help_entries();

    // Extract base command syntax without <args>
    let extract_bases = |syntax: &str| -> Vec<String> {
        let clean = syntax.replace(":", "");
        let before_args = clean.split('<').next().unwrap_or(&clean);

        // Handle "q | quit"
        before_args
            .split('|')
            .map(|s| s.trim().to_string())
            .collect()
    };

    if input.is_empty() {
        if entries.is_empty() {
            return (None, 0);
        }
        let idx = current_cycle % entries.len();
        let bases = extract_bases(entries[idx].0);
        return (Some(format!("{} ", bases.last().unwrap())), entries.len());
    }

    let mut matches = Vec::new();
    for (syntax, _) in entries {
        for base in extract_bases(syntax) {
            if base.starts_with(input) {
                matches.push(base);
            }
        }
    }

    if matches.is_empty() {
        return (None, 0);
    }

    matches.sort();
    matches.dedup();

    let idx = current_cycle % matches.len();
    (Some(format!("{} ", matches[idx])), matches.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggest_empty() {
        let s = suggest("", 0).0;
        assert!(s.is_some());
        assert!(s.unwrap().ends_with(' '));
    }

    #[test]
    fn test_suggest_prefix() {
        let s1 = suggest("d", 0).0.unwrap();
        assert_eq!(s1, "dl ");

        let s2 = suggest("pl", 0).0.unwrap();
        // Since there are multiple "pl" commands, it returns the 0th match sorted alphabetically
        // "pl add", "pl create", etc.
        assert!(s2.starts_with("pl "));
    }
}
