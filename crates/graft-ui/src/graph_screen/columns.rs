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

/// The commit's own recorded date, absolute, in its own offset:
/// `"2026-09-13 14:02 +0500"`. No date/time crate: this is the standard
/// days-since-epoch civil calendar conversion (Howard Hinnant's
/// `civil_from_days`), which needs nothing but integer arithmetic and never
/// disagrees with a library over leap years, so pulling one in for four
/// lines of formatting would only be another dependency to justify
/// (ARCHITECTURE.md's crate rules).
#[must_use]
pub fn absolute_date(seconds: i64, offset_seconds: i32) -> String {
    let local = seconds.saturating_add(i64::from(offset_seconds));
    let days = local.div_euclid(86_400);
    let seconds_of_day = local.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let sign = if offset_seconds < 0 { '-' } else { '+' };
    let offset_minutes = i64::from(offset_seconds.unsigned_abs()) / 60;
    let offset_hh = offset_minutes / 60;
    let offset_mm = offset_minutes % 60;
    format!(
        "{year:04}-{month:02}-{day:02} {hour:02}:{minute:02} {sign}{offset_hh:02}{offset_mm:02}"
    )
}

/// Civil (year, month, day) from a day count since the Unix epoch. Howard
/// Hinnant's public-domain algorithm ("chrono-Compatible Low-Level Date
/// Algorithms"), valid for every day a git commit's clock can plausibly
/// record.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097); // [0, 146096]
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365; // [0, 399]
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100); // [0, 365]
    let mp = (5 * day_of_year + 2) / 153; // [0, 11]
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "day_of_year and mp are non-negative and bounded to [1, 31] by construction"
    )]
    let day = (day_of_year - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "mp is in [0, 11] by construction, so this is in [1, 12]"
    )]
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    let year = if month <= 2 { year + 1 } else { year };
    (year, month, day)
}

/// The tooltip's date line: absolute (in the commit's own recorded offset)
/// and relative, together — `"2026-09-13 14:02 +0500 · 4h ago"`. `relative_date`'s
/// own `"now"` has no "ago" appended.
#[must_use]
pub fn tooltip_date(now: i64, seconds: i64, offset_seconds: i32) -> String {
    let relative = relative_date(now, seconds);
    let relative = if relative == "now" {
        relative
    } else {
        format!("{relative} ago")
    };
    format!("{} · {relative}", absolute_date(seconds, offset_seconds))
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

    #[test]
    fn absolute_date_applies_a_positive_offset() {
        // 2026-09-13 09:02 UTC + 5h = 14:02 local.
        let utc = 1_789_290_120;
        assert_eq!(absolute_date(utc, 5 * 60 * 60), "2026-09-13 14:02 +0500");
    }

    #[test]
    fn absolute_date_applies_a_negative_offset() {
        // 2026-09-13 09:02 UTC - 5h = 04:02 local.
        let utc = 1_789_290_120;
        assert_eq!(absolute_date(utc, -5 * 60 * 60), "2026-09-13 04:02 -0500");
    }

    #[test]
    fn absolute_date_handles_a_zero_offset() {
        assert_eq!(absolute_date(0, 0), "1970-01-01 00:00 +0000");
    }

    #[test]
    fn tooltip_date_appends_ago_except_for_now() {
        assert_eq!(
            tooltip_date(10_000, 0, 0),
            "1970-01-01 00:00 +0000 · 2h ago"
        );
        assert_eq!(tooltip_date(1000, 1000, 0), "1970-01-01 00:16 +0000 · now");
    }
}
