/// Whether objects and arrays nest deeper than `max_depth`. Brackets inside
/// strings are ignored; anything else malformed is left to the JSON parser.
///
/// Unlike the internal event contract, provider input may contain arrays.
pub(crate) fn exceeds(bytes: &[u8], max_depth: usize) -> bool {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for &byte in bytes {
        if in_string {
            match (escaped, byte) {
                (true, _) => escaped = false,
                (false, b'\\') => escaped = true,
                (false, b'"') => in_string = false,
                _ => {}
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth += 1;
                if depth > max_depth {
                    return true;
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_objects_and_arrays() {
        let at_limit = format!("{}{}", "[{\"x\":".repeat(8), "0}]".repeat(8));
        assert!(!exceeds(at_limit.as_bytes(), 16));
        let over = format!("[{at_limit}]");
        assert!(exceeds(over.as_bytes(), 16));
    }

    #[test]
    fn ignores_brackets_inside_strings() {
        let raw = format!(r#"{{"x":"\"{}"}}"#, "[{".repeat(40));
        assert!(!exceeds(raw.as_bytes(), 16));
    }
}
