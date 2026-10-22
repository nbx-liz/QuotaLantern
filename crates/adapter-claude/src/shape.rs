//! Shape probe for G0 audits: records which keys and JSON types a real
//! status line input contains, never any value.

use std::collections::BTreeSet;
use std::fmt;

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};

use crate::{AdapterError, MAX_INPUT_BYTES, MAX_INPUT_DEPTH, Observation, Reading, depth, extract};

/// Longest key recorded verbatim; longer keys are masked.
const MAX_KEY_LEN: usize = 64;
/// Recorded in place of a key that is not a plain lowercase identifier.
const MASKED_KEY: &str = "<dynamic>";

/// Key paths with JSON types, one per line, followed by the extraction
/// result per allowlisted value. Keys that are not plain lowercase
/// identifiers are masked as `<dynamic>`, because a key can itself carry
/// user data (a path, a project name). Array elements are merged under `[]`.
///
/// The input is walked by a visitor that keeps only types and plain keys,
/// so no value is copied, not even transiently into a JSON tree.
pub fn describe_shape(raw: &[u8]) -> Result<String, AdapterError> {
    if raw.len() > MAX_INPUT_BYTES {
        return Err(AdapterError::TooLarge);
    }
    if depth::exceeds(raw, MAX_INPUT_DEPTH) {
        return Err(AdapterError::TooDeep);
    }
    let mut lines = BTreeSet::new();
    let mut deserializer = serde_json::Deserializer::from_slice(raw);
    Node {
        path: "$".to_owned(),
        lines: &mut lines,
    }
    .deserialize(&mut deserializer)
    .and_then(|()| deserializer.end())
    .map_err(|_| AdapterError::Malformed)?;

    let mut out = String::new();
    for line in &lines {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(&extraction_summary(&extract(raw)));
    Ok(out)
}

/// One value at `path`; records its type and walks its children.
struct Node<'a> {
    path: String,
    lines: &'a mut BTreeSet<String>,
}

impl Node<'_> {
    fn record<E>(self, kind: &str) -> Result<(), E> {
        self.lines.insert(format!("{}\t{kind}", self.path));
        Ok(())
    }
}

impl<'de> DeserializeSeed<'de> for Node<'_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Node<'_> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }

    fn visit_unit<E: de::Error>(self) -> Result<(), E> {
        self.record("null")
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<(), E> {
        self.record("bool")
    }

    fn visit_i64<E: de::Error>(self, _: i64) -> Result<(), E> {
        self.record("number")
    }

    fn visit_u64<E: de::Error>(self, _: u64) -> Result<(), E> {
        self.record("number")
    }

    fn visit_f64<E: de::Error>(self, _: f64) -> Result<(), E> {
        self.record("number")
    }

    fn visit_str<E: de::Error>(self, _: &str) -> Result<(), E> {
        self.record("string")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        let child = format!("{}[]", self.path);
        let lines = self.lines;
        lines.insert(format!("{}\tarray", self.path));
        while seq
            .next_element_seed(Node {
                path: child.clone(),
                lines: &mut *lines,
            })?
            .is_some()
        {}
        Ok(())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let lines = self.lines;
        lines.insert(format!("{}\tobject", self.path));
        while let Some(MaskedKey(key)) = map.next_key()? {
            map.next_value_seed(Node {
                path: format!("{}.{key}", self.path),
                lines: &mut *lines,
            })?;
        }
        Ok(())
    }
}

/// An object key, kept only when it is a plain lowercase identifier.
struct MaskedKey(String);

impl<'de> Deserialize<'de> for MaskedKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(MaskedKeyVisitor)
    }
}

struct MaskedKeyVisitor;

impl Visitor<'_> for MaskedKeyVisitor {
    type Value = MaskedKey;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an object key")
    }

    fn visit_str<E: de::Error>(self, key: &str) -> Result<MaskedKey, E> {
        let plain = !key.is_empty()
            && key.len() <= MAX_KEY_LEN
            && key
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
        Ok(MaskedKey(if plain { key } else { MASKED_KEY }.to_owned()))
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
