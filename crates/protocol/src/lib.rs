//! Internal event contract of QuotaLantern (ADR-0009).
//!
//! Adapters hand the core only events that pass [`parse_event`]. The
//! hand-written `schemas/event.schema.json` is the source of truth; this
//! crate must reach the same verdicts, plus the parser-only rejections that
//! JSON Schema cannot express (duplicate keys, size, depth, integers written
//! as fractions or exponents).

mod error;
mod event;
mod ident;
mod number;
mod scan;
mod timestamp;

use serde::Deserialize;

pub use error::ParseError;
pub use event::{
    Context, Environment, ErrorCode, Event, Metric, Provider, Quota, SourceState, State, Target,
    TokenCategory, TokenCounter,
};
pub use ident::EphemeralId;
pub use number::{Bounded, Percent, PositiveSafeUint, SafeUint, WindowSeconds};
pub use timestamp::UtcTimestamp;

/// The only schema version this crate accepts.
pub const SCHEMA_VERSION: u64 = 1;
/// Maximum size of one serialized event, checked before parsing.
pub const MAX_EVENT_BYTES: usize = 16 * 1024;
/// Maximum nesting depth of objects and arrays, checked before parsing.
pub const MAX_DEPTH: usize = 16;
/// Largest integer that survives a round trip through an IEEE 754 double.
pub const MAX_SAFE_INTEGER: u64 = (1 << 53) - 1;

/// Parses and validates one serialized event.
///
/// Failures are reported as fixed codes only; no part of the input is kept.
pub fn parse_event(raw: &[u8]) -> Result<Event, ParseError> {
    if raw.len() > MAX_EVENT_BYTES {
        return Err(ParseError::TooLarge);
    }
    let text = std::str::from_utf8(raw).map_err(|_| ParseError::InvalidUtf8)?;
    scan::check_structure(text.as_bytes(), MAX_DEPTH)?;
    check_version(text)?;
    serde_json::from_str(text).map_err(|_| ParseError::Malformed)
}

/// Distinguishes an unsupported `schema_version` from other failures, so that
/// the caller can stop the connection instead of dropping a single event.
fn check_version(text: &str) -> Result<(), ParseError> {
    #[derive(Deserialize)]
    struct VersionProbe {
        schema_version: Option<serde_json::Number>,
    }

    match serde_json::from_str::<VersionProbe>(text) {
        // Only an integer literal counts as a version; `1.0`, `1e0` and `-0`
        // are malformed like any other integer field.
        Ok(VersionProbe {
            schema_version: Some(version),
        }) if version.as_u64().is_some_and(|v| v != SCHEMA_VERSION) => {
            Err(ParseError::UnsupportedVersion)
        }
        // Anything else is judged by the full parse.
        _ => Ok(()),
    }
}
