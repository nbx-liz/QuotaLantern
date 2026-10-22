//! Claude Code status line adapter (ADR-0002).
//!
//! Claude Code passes one JSON document on stdin per status line update. It
//! contains identifying metadata (`session_id`, `cwd`, `transcript_path`, ...)
//! next to the values QuotaLantern needs. [`extract`] reads only the
//! allowlisted values into typed form; everything else is skipped while
//! parsing and never copied.
//!
//! The result is an intermediate [`Observation`], not a protocol `Event`: a
//! status line command is a new process per update and cannot own the
//! per-connection envelope (`run_id`, `source_id`, `seq`).

mod depth;
mod input;
mod render;
mod shape;

pub use render::render_status;
pub use shape::describe_shape;

use quotalantern_protocol::{Percent, PositiveSafeUint, UtcTimestamp};

/// Maximum size of one status line input, checked before parsing.
pub const MAX_INPUT_BYTES: usize = 1024 * 1024;
/// Maximum nesting depth of the status line input, checked before parsing.
pub const MAX_INPUT_DEPTH: usize = 16;

/// Allowlisted values from one status line input.
#[derive(Debug, Clone, PartialEq)]
pub struct Observation {
    /// `rate_limits.five_hour`.
    pub five_hour: Reading<QuotaWindow>,
    /// `rate_limits.seven_day`.
    pub seven_day: Reading<QuotaWindow>,
    /// `rate_limits.spend_limit` was present. It is never converted: it can
    /// exceed 100% and carries cost data.
    pub spend_limit_present: bool,
    /// `context_window`.
    pub context: Reading<ContextUsage>,
}

/// One value as received.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Reading<T> {
    /// Not in the input, or `null`: not acquired. Never shown as 0.
    Absent,
    /// Present and valid.
    Available(T),
    /// Present but outside the accepted range or form. Never clamped.
    Invalid,
}

/// One quota window as reported by Claude Code.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuotaWindow {
    /// Share of the window already used.
    pub used: Percent,
    /// When the window resets, if reported.
    pub resets_at: Option<UtcTimestamp>,
}

/// Context-window usage of the session that ran the status line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContextUsage {
    /// Share of the context window in use.
    pub used: Percent,
    /// Window size in tokens, if reported.
    pub capacity_tokens: Option<PositiveSafeUint>,
}

/// Why an input was rejected as a whole. Carries no input content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdapterError {
    /// The input exceeds [`MAX_INPUT_BYTES`].
    TooLarge,
    /// The input nests deeper than [`MAX_INPUT_DEPTH`].
    TooDeep,
    /// The input is not JSON, its top level is not an object, or it repeats
    /// an allowlisted key.
    Malformed,
}

impl AdapterError {
    /// Stable machine-readable code.
    pub const fn code(self) -> &'static str {
        match self {
            Self::TooLarge => "input_too_large",
            Self::TooDeep => "input_too_deep",
            Self::Malformed => "input_malformed",
        }
    }
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for AdapterError {}

/// Extracts the allowlisted values from one status line input.
pub fn extract(raw: &[u8]) -> Result<Observation, AdapterError> {
    if raw.len() > MAX_INPUT_BYTES {
        return Err(AdapterError::TooLarge);
    }
    if depth::exceeds(raw, MAX_INPUT_DEPTH) {
        return Err(AdapterError::TooDeep);
    }
    input::parse(raw)
        .map(input::StatusInput::into_observation)
        .ok_or(AdapterError::Malformed)
}
