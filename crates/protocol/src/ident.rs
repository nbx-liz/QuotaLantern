use serde::{Deserialize, Deserializer, de};
use uuid::Uuid;

/// Random per-run or per-connection identifier: a UUID v4 in canonical
/// lowercase hyphenated form. Never a persistent or real account identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EphemeralId(Uuid);

impl EphemeralId {
    /// Validates the canonical textual form.
    pub fn parse(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        if bytes.len() != 36 {
            return None;
        }
        let canonical = bytes.iter().enumerate().all(|(i, &b)| match i {
            8 | 13 | 18 | 23 => b == b'-',
            14 => b == b'4',
            19 => matches!(b, b'8' | b'9' | b'a' | b'b'),
            _ => matches!(b, b'0'..=b'9' | b'a'..=b'f'),
        });
        if !canonical {
            return None;
        }
        Uuid::try_parse(text).ok().map(Self)
    }

    /// The underlying UUID.
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl<'de> Deserialize<'de> for EphemeralId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).ok_or_else(|| de::Error::custom("invalid ephemeral id"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CANONICAL: &str = "abcdef01-0000-4000-a000-00000000000a";

    #[test]
    fn accepts_canonical_v4() {
        assert!(EphemeralId::parse(CANONICAL).is_some());
    }

    #[test]
    fn rejects_non_canonical_forms() {
        for text in [
            "ABCDEF01-0000-4000-A000-00000000000A",
            "{abcdef01-0000-4000-a000-00000000000a}",
            "abcdef0100004000a00000000000000a",
            "urn:uuid:abcdef01-0000-4000-a000-00000000000a",
            "abcdef01-0000-1000-a000-00000000000a",
            "abcdef01-0000-4000-c000-00000000000a",
            "account-real-user",
            "",
        ] {
            assert!(EphemeralId::parse(text).is_none(), "accepted {text:?}");
        }
    }
}
