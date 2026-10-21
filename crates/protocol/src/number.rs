use serde::{Deserialize, Deserializer, de};

use crate::MAX_SAFE_INTEGER;

/// Unsigned integer constrained to `MIN..=MAX`.
///
/// Accepts only JSON integer literals: `1.0` and `1e3` are rejected even
/// though JSON Schema treats them as integers (ADR-0009, parser-only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bounded<const MIN: u64, const MAX: u64>(u64);

impl<const MIN: u64, const MAX: u64> Bounded<MIN, MAX> {
    /// Returns `None` outside `MIN..=MAX`.
    pub const fn new(value: u64) -> Option<Self> {
        if value >= MIN && value <= MAX {
            Some(Self(value))
        } else {
            None
        }
    }

    /// The contained value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl<'de, const MIN: u64, const MAX: u64> Deserialize<'de> for Bounded<MIN, MAX> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = u64::deserialize(deserializer)?;
        Self::new(value).ok_or_else(|| de::Error::custom("integer out of range"))
    }
}

/// `0..=2^53-1`.
pub type SafeUint = Bounded<0, MAX_SAFE_INTEGER>;
/// `1..=2^53-1`.
pub type PositiveSafeUint = Bounded<1, MAX_SAFE_INTEGER>;
/// Quota window length in seconds: one second up to ten years.
pub type WindowSeconds = Bounded<1, 315_576_000>;

/// Finite percentage in `0..=100`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Percent(f64);

impl Percent {
    /// Returns `None` for non-finite values or values outside `0..=100`.
    pub fn new(value: f64) -> Option<Self> {
        (value.is_finite() && (0.0..=100.0).contains(&value)).then_some(Self(value))
    }

    /// The contained value.
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for Percent {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = f64::deserialize(deserializer)?;
        Self::new(value).ok_or_else(|| de::Error::custom("percent out of range"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_limits() {
        assert!(SafeUint::new(MAX_SAFE_INTEGER).is_some());
        assert!(SafeUint::new(MAX_SAFE_INTEGER + 1).is_none());
        assert!(PositiveSafeUint::new(0).is_none());
        assert!(WindowSeconds::new(315_576_001).is_none());
    }

    #[test]
    fn percent_limits() {
        assert!(Percent::new(0.0).is_some());
        assert!(Percent::new(100.0).is_some());
        assert!(Percent::new(100.000_001).is_none());
        assert!(Percent::new(-0.1).is_none());
        assert!(Percent::new(f64::NAN).is_none());
        assert!(Percent::new(f64::INFINITY).is_none());
    }

    #[test]
    fn integers_reject_fraction_and_exponent_literals() {
        assert!(serde_json::from_str::<SafeUint>("1").is_ok());
        assert!(serde_json::from_str::<SafeUint>("1.0").is_err());
        assert!(serde_json::from_str::<SafeUint>("1e3").is_err());
        assert!(serde_json::from_str::<SafeUint>("-1").is_err());
    }
}
