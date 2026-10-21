use std::fmt;

/// Why an event was rejected.
///
/// Carries no input content and no library message by design: neither may
/// leave the adapter boundary (`docs/security/guarantee-boundary.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseError {
    /// The serialized event exceeds [`crate::MAX_EVENT_BYTES`].
    TooLarge,
    /// The input is not valid UTF-8.
    InvalidUtf8,
    /// Objects or arrays nest deeper than [`crate::MAX_DEPTH`].
    TooDeep,
    /// `schema_version` is an integer literal other than
    /// [`crate::SCHEMA_VERSION`].
    UnsupportedVersion,
    /// Any other violation of the contract, including invalid JSON.
    Malformed,
}

impl ParseError {
    /// Stable machine-readable code.
    pub const fn code(self) -> &'static str {
        match self {
            Self::TooLarge => "event_too_large",
            Self::InvalidUtf8 => "invalid_utf8",
            Self::TooDeep => "nesting_too_deep",
            Self::UnsupportedVersion => "unsupported_schema_version",
            Self::Malformed => "malformed_event",
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for ParseError {}
