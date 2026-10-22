//! Allowlisted subset of the status line input. Every other field is skipped
//! by serde without being copied. These types deliberately have no `Debug`.
//!
//! An allowlisted object or value of the wrong JSON type makes only its own
//! reading invalid; the input as a whole is rejected only when the top level
//! is not an object (ADR-0002).

use std::fmt;
use std::marker::PhantomData;

use quotalantern_protocol::{Percent, PositiveSafeUint, UtcTimestamp};
use serde::de::value::MapAccessDeserializer;
use serde::de::{self, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};

use crate::{ContextUsage, Observation, QuotaWindow, Reading};

#[derive(Deserialize)]
pub(crate) struct StatusInput {
    #[serde(default)]
    rate_limits: Section<RateLimits>,
    #[serde(default)]
    context_window: Section<ContextWindow>,
}

#[derive(Deserialize)]
struct RateLimits {
    #[serde(default)]
    five_hour: Section<Window>,
    #[serde(default)]
    seven_day: Section<Window>,
    #[serde(default)]
    spend_limit: Option<IgnoredAny>,
}

#[derive(Deserialize)]
struct Window {
    #[serde(default)]
    used_percentage: Scalar,
    #[serde(default)]
    resets_at: Scalar,
}

#[derive(Deserialize)]
struct ContextWindow {
    #[serde(default)]
    used_percentage: Scalar,
    #[serde(default)]
    context_window_size: Scalar,
}

/// A value present but unusable: wrong type or out of range.
struct Invalid;

/// `Ok(None)` when absent or `null`.
type Field<T> = Result<Option<T>, Invalid>;

/// An allowlisted object. Derived structs would also accept a JSON array and
/// fill fields by position, so only a JSON object reaches `T`.
#[derive(Default)]
enum Section<T> {
    #[default]
    Absent,
    Object(T),
    Other,
}

impl<T> Section<T> {
    fn object(self) -> Field<T> {
        match self {
            Self::Absent => Ok(None),
            Self::Object(value) => Ok(Some(value)),
            Self::Other => Err(Invalid),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Section<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(SectionVisitor(PhantomData))
    }
}

struct SectionVisitor<T>(PhantomData<T>);

impl<'de, T: Deserialize<'de>> Visitor<'de> for SectionVisitor<T> {
    type Value = Section<T>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        T::deserialize(MapAccessDeserializer::new(map)).map(Section::Object)
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(Section::Absent)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        drain_seq(seq).map(|()| Section::Other)
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self::Value, E> {
        Ok(Section::Other)
    }

    fn visit_i64<E: de::Error>(self, _: i64) -> Result<Self::Value, E> {
        Ok(Section::Other)
    }

    fn visit_u64<E: de::Error>(self, _: u64) -> Result<Self::Value, E> {
        Ok(Section::Other)
    }

    fn visit_f64<E: de::Error>(self, _: f64) -> Result<Self::Value, E> {
        Ok(Section::Other)
    }

    fn visit_str<E: de::Error>(self, _: &str) -> Result<Self::Value, E> {
        Ok(Section::Other)
    }
}

/// An allowlisted leaf value, kept as the JSON number form it arrived in.
#[derive(Default)]
enum Scalar {
    #[default]
    Absent,
    Unsigned(u64),
    Signed(i64),
    Float(f64),
    Other,
}

impl Scalar {
    fn number(self) -> Field<f64> {
        match self {
            Self::Absent => Ok(None),
            Self::Unsigned(value) => Ok(Some(value as f64)),
            Self::Signed(value) => Ok(Some(value as f64)),
            Self::Float(value) => Ok(Some(value)),
            Self::Other => Err(Invalid),
        }
    }

    fn integer(self) -> Field<i64> {
        match self {
            Self::Absent => Ok(None),
            Self::Unsigned(value) => i64::try_from(value).map(Some).map_err(|_| Invalid),
            Self::Signed(value) => Ok(Some(value)),
            Self::Float(_) | Self::Other => Err(Invalid),
        }
    }

    fn unsigned(self) -> Field<u64> {
        match self {
            Self::Absent => Ok(None),
            Self::Unsigned(value) => Ok(Some(value)),
            Self::Signed(_) | Self::Float(_) | Self::Other => Err(Invalid),
        }
    }
}

impl<'de> Deserialize<'de> for Scalar {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ScalarVisitor)
    }
}

struct ScalarVisitor;

impl<'de> Visitor<'de> for ScalarVisitor {
    type Value = Scalar;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(Scalar::Absent)
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Scalar::Unsigned(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Scalar::Signed(value))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        Ok(Scalar::Float(value))
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self::Value, E> {
        Ok(Scalar::Other)
    }

    fn visit_str<E: de::Error>(self, _: &str) -> Result<Self::Value, E> {
        Ok(Scalar::Other)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        drain_seq(seq).map(|()| Scalar::Other)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(Scalar::Other)
    }
}

/// A rejected container must still be consumed, or the parser fails on
/// its remaining elements.
fn drain_seq<'de, A: SeqAccess<'de>>(mut seq: A) -> Result<(), A::Error> {
    while seq.next_element::<IgnoredAny>()?.is_some() {}
    Ok(())
}

/// Top level of the input: anything but an object is rejected as a whole.
pub(crate) fn parse(raw: &[u8]) -> Option<StatusInput> {
    match serde_json::from_slice::<Section<StatusInput>>(raw) {
        Ok(Section::Object(input)) => Some(input),
        Ok(Section::Absent | Section::Other) | Err(_) => None,
    }
}

impl StatusInput {
    pub(crate) fn into_observation(self) -> Observation {
        let (five_hour, seven_day, spend_limit_present) = match self.rate_limits.object() {
            Ok(Some(limits)) => (
                reading(quota(limits.five_hour)),
                reading(quota(limits.seven_day)),
                limits.spend_limit.is_some(),
            ),
            Ok(None) => (Reading::Absent, Reading::Absent, false),
            Err(Invalid) => (Reading::Invalid, Reading::Invalid, false),
        };
        Observation {
            five_hour,
            seven_day,
            spend_limit_present,
            context: reading(context(self.context_window)),
        }
    }
}

fn reading<T>(field: Field<T>) -> Reading<T> {
    match field {
        Ok(Some(value)) => Reading::Available(value),
        Ok(None) => Reading::Absent,
        Err(Invalid) => Reading::Invalid,
    }
}

fn quota(section: Section<Window>) -> Field<QuotaWindow> {
    let Some(window) = section.object()? else {
        return Ok(None);
    };
    let Some(used) = window.used_percentage.number()? else {
        return Ok(None);
    };
    let used = Percent::new(used).ok_or(Invalid)?;
    let resets_at = window
        .resets_at
        .integer()?
        .map(|seconds| UtcTimestamp::from_unix_seconds(seconds).ok_or(Invalid))
        .transpose()?;
    Ok(Some(QuotaWindow { used, resets_at }))
}

fn context(section: Section<ContextWindow>) -> Field<ContextUsage> {
    let Some(window) = section.object()? else {
        return Ok(None);
    };
    let Some(used) = window.used_percentage.number()? else {
        return Ok(None);
    };
    let used = Percent::new(used).ok_or(Invalid)?;
    let capacity_tokens = window
        .context_window_size
        .unsigned()?
        .map(|size| PositiveSafeUint::new(size).ok_or(Invalid))
        .transpose()?;
    Ok(Some(ContextUsage {
        used,
        capacity_tokens,
    }))
}
