pub fn parse_progress(line: &str) -> Option<u8> {
    let percent = line
        .split_whitespace()
        .find(|p| p.ends_with('%'))?
        .trim_end_matches('%')
        .parse::<f32>()
        .ok()?;

    Some(percent.round().clamp(0.0, 100.0) as u8)
}
