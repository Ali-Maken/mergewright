//! Pure formatting for the row columns that are not repository text: the
//! author's initials, the relative date, and the short SHA.
//!
//! Author *names* are repository text and cross the `SanitizedText` boundary
//! like the subject does (`app`, at the view-model boundary); initials are
//! computed from an already-sanitised name, so a caller sanitises the result
//! too rather than treating "derived from safe text" as safe by inspection
//! (ARCHITECTURE.md). The relative date and the SHA are Graft's own text —
//! a clock reading and hex digits — so neither one is repository-controlled
//! and neither needs to cross that boundary at all.

/// Two letters standing in for `name`, for the author chip.
///
/// The first letter of the first two whitespace-separated words
/// ("Rui M" → "RM"), or the first two letters of the only word when there is
/// just one ("Ada" → "AD"). An empty name has no initials.
#[must_use]
pub fn initials(name: &str) -> String {
    let mut words = name.split_whitespace();
    match (words.next(), words.next()) {
        (Some(first), Some(second)) => [first, second]
            .into_iter()
            .filter_map(|word| word.chars().next())
            .flat_map(char::to_uppercase)
            .collect(),
        (Some(only), None) => only.chars().take(2).flat_map(char::to_uppercase).collect(),
        (None, _) => String::new(),
    }
}

/// How long ago `then` was, relative to `now` — both Unix seconds — in the
/// short form a narrow column needs: `"2m"`, `"3h"`, `"1d"`.
///
/// Widens to `"{n}mo"` past 30 days and `"{n}y"` past 365; a commit from the
/// future (a clock skewed ahead, or a rebase authored ahead of "now") reads
/// as `"now"` rather than a negative count.
#[must_use]
pub fn relative_date(now: i64, then: i64) -> String {
    let elapsed = now.saturating_sub(then);
    if elapsed < 60 {
        "now".to_owned()
    } else if elapsed < 60 * 60 {
        format!("{}m", elapsed / 60)
    } else if elapsed < 24 * 60 * 60 {
        format!("{}h", elapsed / (60 * 60))
    } else if elapsed < 30 * 24 * 60 * 60 {
        format!("{}d", elapsed / (24 * 60 * 60))
    } else if elapsed < 365 * 24 * 60 * 60 {
        format!("{}mo", elapsed / (30 * 24 * 60 * 60))
    } else {
        format!("{}y", elapsed / (365 * 24 * 60 * 60))
    }
}

/// The first `len` hex characters of a full object id — `Display`'s output,
/// not a re-hash, so it always agrees with the id `app` looked up.
///
/// Hex is safe to draw as-is (`graft_core::oid::Oid`'s own doc comment): it
/// is drawn from a fixed digit alphabet, never from repository text.
#[must_use]
pub fn short_sha(full_hex: &str, len: usize) -> String {
    full_hex.chars().take(len).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_words_take_their_first_letters() {
        assert_eq!(initials("Rui M"), "RM");
        assert_eq!(initials("ada king"), "AK");
    }

    #[test]
    fn one_word_takes_its_first_two_letters() {
        assert_eq!(initials("Ada"), "AD");
        assert_eq!(initials("a"), "A");
    }

    #[test]
    fn an_empty_name_has_no_initials() {
        assert_eq!(initials(""), "");
        assert_eq!(initials("   "), "");
    }

    #[test]
    fn relative_date_picks_the_widest_unit_that_still_fits() {
        assert_eq!(relative_date(1000, 1000), "now");
        assert_eq!(relative_date(1000, 940), "1m");
        assert_eq!(relative_date(10_000, 0), "2h");
        assert_eq!(relative_date(100_000, 0), "1d");
        assert_eq!(relative_date(40 * 24 * 60 * 60, 0), "1mo");
        assert_eq!(relative_date(400 * 24 * 60 * 60, 0), "1y");
    }

    #[test]
    fn a_commit_from_the_future_reads_as_now() {
        assert_eq!(relative_date(0, 1000), "now");
    }

    #[test]
    fn short_sha_takes_the_leading_hex_digits() {
        assert_eq!(short_sha("a1b2c3d4e5f6", 7), "a1b2c3d");
    }
}
