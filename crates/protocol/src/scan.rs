use crate::ParseError;

/// Structural pre-check that builds no value:
///
/// - any array is [`ParseError::Malformed`]: the contract has no array-typed
///   field, and serde's derived structs would otherwise accept a positional
///   array in place of an object;
/// - objects nested deeper than `max_depth` are [`ParseError::TooDeep`].
///
/// Brackets inside strings are ignored; anything else malformed is left to
/// the JSON parser.
pub(crate) fn check_structure(bytes: &[u8], max_depth: usize) -> Result<(), ParseError> {
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
            b'[' => return Err(ParseError::Malformed),
            b'{' => {
                depth += 1;
                if depth > max_depth {
                    return Err(ParseError::TooDeep);
                }
            }
            b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nested(levels: usize) -> String {
        format!("{}0{}", r#"{"x":"#.repeat(levels), "}".repeat(levels))
    }

    #[test]
    fn accepts_depth_at_limit() {
        assert_eq!(check_structure(nested(16).as_bytes(), 16), Ok(()));
    }

    #[test]
    fn rejects_depth_over_limit() {
        assert_eq!(
            check_structure(nested(17).as_bytes(), 16),
            Err(ParseError::TooDeep)
        );
    }

    #[test]
    fn rejects_any_array() {
        for raw in ["[]", r#"{"x":[1]}"#, r#"{"x":{"y":[]}}"#] {
            assert_eq!(
                check_structure(raw.as_bytes(), 16),
                Err(ParseError::Malformed),
                "{raw}"
            );
        }
    }

    #[test]
    fn ignores_square_brackets_inside_strings() {
        assert_eq!(check_structure(br#"{"x":"[]"}"#, 16), Ok(()));
    }

    #[test]
    fn ignores_brackets_inside_strings() {
        let raw = format!(r#"{{"x":"{}"}}"#, "{[".repeat(40));
        assert_eq!(check_structure(raw.as_bytes(), 16), Ok(()));
    }

    #[test]
    fn escaped_quote_does_not_end_string() {
        let raw = format!(r#"{{"x":"\"{}"}}"#, "{".repeat(40));
        assert_eq!(check_structure(raw.as_bytes(), 16), Ok(()));
    }
}
