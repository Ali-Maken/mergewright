//! [`EntryId`]: a journal entry's ULID (m3/02).

use serde::{Deserialize, Serialize};

use crate::error::Error;

/// A journal entry's id: a ULID, sortable by creation time and safe inside a
/// ref name, which is where it also appears (`refs/graft/backup/<worktree>/
/// <id>`, ADR-0010).
///
/// Twenty-six Crockford base32 characters: 48 bits of Unix milliseconds, then
/// 80 random bits. Ordering is by string, which for ULIDs is by time.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct EntryId(String);

/// Crockford's base32 alphabet: digits and letters without `I`, `L`, `O`, `U`.
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// How many characters a ULID has.
const ULID_LEN: usize = 26;

impl EntryId {
    /// A new id for an entry created at `unix_ms` with 80 bits of `random`.
    ///
    /// The caller supplies both so this stays pure: the clock and the
    /// randomness are `ops`'s to obtain. Milliseconds outside the 48-bit
    /// range (negative, or past the year 10889) are clamped rather than
    /// wrapped, so an absurd clock still yields a sortable id.
    #[must_use]
    pub fn new(unix_ms: i64, random: [u8; 10]) -> Self {
        let millis = u128::from(u64::try_from(unix_ms).unwrap_or(0).min((1 << 48) - 1));
        let mut random_bits = 0u128;
        for byte in random {
            random_bits = (random_bits << 8) | u128::from(byte);
        }
        let value = (millis << 80) | random_bits;
        let text = (0..ULID_LEN)
            .map(|i| {
                let shift = 5 * (ULID_LEN - 1 - i);
                let index = ((value >> shift) & 0x1f) as usize;
                char::from(CROCKFORD[index])
            })
            .collect();
        Self(text)
    }

    /// Parses a ULID; lowercase letters are accepted and stored upper case.
    ///
    /// # Errors
    /// [`Error::EntryId`] when `s` is not 26 Crockford base32 characters
    /// with a first character of `0`–`7` (a 128-bit value has 130 bits of
    /// room in 26 characters; the top two must be zero).
    pub fn parse(s: &str) -> Result<Self, Error> {
        let invalid = || Error::EntryId {
            text: s.escape_debug().to_string(),
        };
        if s.len() != ULID_LEN {
            return Err(invalid());
        }
        let upper = s.to_ascii_uppercase();
        let well_formed = upper.bytes().all(|b| CROCKFORD.contains(&b));
        let first_in_range = upper.as_bytes()[0] <= b'7';
        if well_formed && first_in_range {
            Ok(Self(upper))
        } else {
            Err(invalid())
        }
    }

    /// The id's text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for EntryId {
    type Error = Error;

    fn try_from(text: String) -> Result<Self, Error> {
        Self::parse(&text)
    }
}

impl From<EntryId> for String {
    fn from(id: EntryId) -> Self {
        id.0
    }
}

impl std::fmt::Display for EntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ulid_encodes_time_then_random_in_26_crockford_characters() {
        let id = EntryId::new(1_700_000_000_000, [0xff; 10]);
        assert_eq!(id.as_str().len(), 26);
        // 1_700_000_000_000 ms is 0x18BCFE56800: the well-known ULID prefix.
        assert!(id.as_str().starts_with("01HF"), "{id}");
        assert!(id.as_str().ends_with("ZZZZZZZZZZZZZZZZ"), "{id}");
    }

    #[test]
    fn ulids_order_by_time_then_by_random() {
        let earlier = EntryId::new(1000, [0xff; 10]);
        let later = EntryId::new(1001, [0; 10]);
        assert!(earlier < later);
        let a = EntryId::new(1000, [0; 10]);
        let b = EntryId::new(1000, [1; 10]);
        assert!(a < b);
    }

    #[test]
    fn parse_round_trips_and_rejects_the_malformed() {
        let id = EntryId::new(42, [7; 10]);
        assert_eq!(EntryId::parse(id.as_str()).unwrap(), id);
        assert_eq!(
            EntryId::parse(&id.as_str().to_ascii_lowercase()).unwrap(),
            id
        );
        for bad in [
            "",
            "01HF",
            "8ZZZZZZZZZZZZZZZZZZZZZZZZZ",
            "01HFZZZZZZZZZZZZZZZZZZZZZU",
        ] {
            assert!(
                matches!(EntryId::parse(bad), Err(Error::EntryId { .. })),
                "{bad}"
            );
        }
    }

    #[test]
    fn absurd_clocks_clamp_instead_of_wrapping() {
        assert_eq!(EntryId::new(-5, [0; 10]), EntryId::new(0, [0; 10]));
        let max = EntryId::new(i64::MAX, [0; 10]);
        assert!(max.as_str().starts_with('7'), "{max}");
    }
}
