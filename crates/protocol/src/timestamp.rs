use serde::{Deserialize, Deserializer, de};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// RFC 3339 timestamp in UTC, written with a literal `Z`, no leap second and
/// at most nine fraction digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UtcTimestamp(OffsetDateTime);

impl UtcTimestamp {
    /// Validates the textual form, then the calendar date.
    pub fn parse(text: &str) -> Option<Self> {
        if !has_canonical_shape(text.as_bytes()) {
            return None;
        }
        OffsetDateTime::parse(text, &Rfc3339).ok().map(Self)
    }

    /// Converts Unix epoch seconds, as providers report reset times.
    /// Returns `None` outside years 0000-9999, which the textual form cannot express.
    pub fn from_unix_seconds(seconds: i64) -> Option<Self> {
        let instant = OffsetDateTime::from_unix_timestamp(seconds).ok()?;
        (0..=9999)
            .contains(&instant.year())
            .then_some(Self(instant))
    }

    /// The instant as a `time` value (UTC offset).
    pub const fn as_offset_date_time(&self) -> OffsetDateTime {
        self.0
    }
}

/// `YYYY-MM-DDTHH:MM:SS[.f{1,9}]Z` with hour 00-23, minute and second 00-59.
fn has_canonical_shape(bytes: &[u8]) -> bool {
    let digit = |i: usize| bytes.get(i).is_some_and(u8::is_ascii_digit);
    let at = |i: usize, expected: u8| bytes.get(i) == Some(&expected);
    let all_digits = |range: std::ops::Range<usize>| range.into_iter().all(digit);

    let fixed = all_digits(0..4)
        && at(4, b'-')
        && all_digits(5..7)
        && at(7, b'-')
        && all_digits(8..10)
        && at(10, b'T')
        && all_digits(11..13)
        && at(13, b':')
        && all_digits(14..16)
        && at(16, b':')
        && all_digits(17..19);
    if !fixed {
        return false;
    }
    let hour_ok = bytes[11] < b'2' || (bytes[11] == b'2' && bytes[12] <= b'3');
    if !hour_ok || bytes[14] > b'5' || bytes[17] > b'5' {
        return false;
    }
    match &bytes[19..] {
        [b'Z'] => true,
        [b'.', rest @ .., b'Z'] => {
            (1..=9).contains(&rest.len()) && rest.iter().all(u8::is_ascii_digit)
        }
        _ => false,
    }
}

impl<'de> Deserialize<'de> for UtcTimestamp {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).ok_or_else(|| de::Error::custom("invalid utc timestamp"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_canonical_forms() {
        for text in [
            "2026-10-21T09:00:00Z",
            "2026-10-21T23:59:59.1Z",
            "2026-10-21T00:00:00.123456789Z",
            "2028-02-29T12:00:00Z",
        ] {
            assert!(UtcTimestamp::parse(text).is_some(), "rejected {text:?}");
        }
    }

    #[test]
    fn converts_unix_seconds() {
        let parsed = UtcTimestamp::parse("2025-02-01T16:00:00Z").unwrap();
        assert_eq!(UtcTimestamp::from_unix_seconds(1_738_425_600), Some(parsed));
        assert!(UtcTimestamp::from_unix_seconds(0).is_some());
        // 10000-01-01T00:00:00Z and the instant just before year 0000.
        assert!(UtcTimestamp::from_unix_seconds(253_402_300_800).is_none());
        assert!(UtcTimestamp::from_unix_seconds(-62_167_219_201).is_none());
        assert!(UtcTimestamp::from_unix_seconds(i64::MAX).is_none());
    }

    #[test]
    fn rejects_non_canonical_forms() {
        for text in [
            "yesterday",
            "2026-10-21T18:00:00+09:00",
            "2026-10-21T09:00:00+00:00",
            "2026-10-21t09:00:00z",
            "2026-10-21 09:00:00Z",
            "2026-10-21T09:00Z",
            "2026-10-21T24:00:00Z",
            "2026-10-21T09:60:00Z",
            "2026-12-31T23:59:60Z",
            "2026-10-21T09:00:00.Z",
            "2026-10-21T09:00:00.1234567890Z",
            "2026-02-30T00:00:00Z",
            "2026-13-01T00:00:00Z",
            "",
        ] {
            assert!(UtcTimestamp::parse(text).is_none(), "accepted {text:?}");
        }
    }
}
