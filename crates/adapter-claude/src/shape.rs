//! Shape probe for G0 audits: records which keys and JSON types a real
//! status line input contains, never any value.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::{AdapterError, MAX_INPUT_BYTES, MAX_INPUT_DEPTH, Observation, Reading, depth, extract};

/// Longest key recorded verbatim; longer keys are masked.
const MAX_KEY_LEN: usize = 64;

/// Key paths with JSON types, one per line, followed by the extraction
/// result per allowlisted value. Keys that are not plain lowercase
/// identifiers are masked as `<dynamic>`, because a key can itself carry
/// user data (a path, a project name). Array elements are merged under `[]`.
pub fn describe_shape(raw: &[u8]) -> Result<String, AdapterError> {
    if raw.len() > MAX_INPUT_BYTES {
        return Err(AdapterError::TooLarge);
    }
    if depth::exceeds(raw, MAX_INPUT_DEPTH) {
        return Err(AdapterError::TooDeep);
    }
    let value: Value = serde_json::from_slice(raw).map_err(|_| AdapterError::Malformed)?;
    let mut lines = BTreeSet::new();
    collect(&value, "$", &mut lines);

    let mut out = String::new();
    for line in &lines {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(&extraction_summary(&extract(raw)));
    Ok(out)
}

fn collect(value: &Value, path: &str, lines: &mut BTreeSet<String>) {
    lines.insert(format!("{path}\t{}", type_name(value)));
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                collect(child, &format!("{path}.{}", mask(key)), lines);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect(child, &format!("{path}[]"), lines);
            }
        }
        _ => {}
    }
}

fn mask(key: &str) -> &str {
    let plain = !key.is_empty()
        && key.len() <= MAX_KEY_LEN
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
    if plain { key } else { "<dynamic>" }
}

const fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn extraction_summary(result: &Result<Observation, AdapterError>) -> String {
    match result {
        Err(err) => format!("# extract\terror:{}\n", err.code()),
        Ok(observation) => format!(
            "# five_hour\t{}\n# seven_day\t{}\n# spend_limit\t{}\n# context\t{}\n",
            state(&observation.five_hour),
            state(&observation.seven_day),
            if observation.spend_limit_present {
                "present_unsupported"
            } else {
                "absent"
            },
            state(&observation.context),
        ),
    }
}

const fn state<T>(reading: &Reading<T>) -> &'static str {
    match reading {
        Reading::Absent => "absent",
        Reading::Available(_) => "available",
        Reading::Invalid => "invalid",
    }
}
