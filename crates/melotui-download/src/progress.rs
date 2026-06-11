pub fn parse_progress(line: &str) -> Option<u8> {
    let percent = line
        .split_whitespace()
        .find(|p| p.ends_with('%'))?
        .trim_end_matches('%')
        .parse::<f32>()
        .ok()?;

    Some(percent.round().clamp(0.0, 100.0) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_progress() {
        assert_eq!(parse_progress("[download]  45.3% of 12.5MiB"), Some(45));
        assert_eq!(parse_progress("[download] 100.0% of ~3.5MiB"), Some(100));
        assert_eq!(parse_progress("[download]   0.0% of 12.5MiB"), Some(0));
        assert_eq!(parse_progress("No percentage here"), None);
    }
}
