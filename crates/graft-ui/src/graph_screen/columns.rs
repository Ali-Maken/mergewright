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
/// short form a narrow column needs: `"2m"`, `"3h"`, `"2d"`, `"2w"`, `"2mo"`,
/// `"1y"`.
///
/// Matches `git log --date=relative`'s own rounding and unit thresholds
/// (m1-the-graph/48; verified against the real `git` binary, table in that
/// ticket), just spelled compactly. Concretely, that means:
///
/// - each stage rounds to the *nearest* count, not down (`elapsed/unit`
///   floors; a commit 46h old floored to `1d`, but git — and now Graft —
///   call 46h `"2d"`, since it rounds to the nearest day);
/// - the rounding cascades stage to stage — minutes, then from minutes to
///   hours, then hours to days — exactly as git's does, because rounding
///   raw seconds straight to (say) days directly makes a different, wrong
///   call within a few thousand seconds of the week/month boundary;
/// - a unit never shows `1`: the smallest count entering minutes is `2`
///   (`89s` is still `"now"`; `90s` rounds straight to `"2m"`), and the same
///   holds for hours, days, weeks and months — this is git's own behaviour,
///   not a Graft quirk;
/// - weeks (`"2w"` .. `"10w"`) sit between days and months, a unit this
///   column didn't have before: git uses one there, so Graft now does too.
///
/// A commit from the future (a clock skewed ahead, or a rebase authored
/// ahead of "now") reads as `"now"` rather than a negative count.
#[must_use]
pub fn relative_date(now: i64, then: i64) -> String {
    let elapsed = now.saturating_sub(then);
    if elapsed < 90 {
        return "now".to_owned();
    }
    let minutes = (elapsed + 30) / 60;
    if minutes < 90 {
        return format!("{minutes}m");
    }
    let hours = (minutes + 30) / 60;
    if hours < 36 {
        return format!("{hours}h");
    }
    let days = (hours + 12) / 24;
    if days < 14 {
        return format!("{days}d");
    }
    if days < 70 {
        return format!("{}w", (days + 3) / 7);
    }
    if days < 365 {
        return format!("{}mo", (days + 15) / 30);
    }
    format!("{}y", (days + 182) / 365)
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
    fn a_commit_from_the_future_reads_as_now() {
        assert_eq!(relative_date(0, 1000), "now");
    }

    /// Every row is `relative_date(now, now - elapsed)`, `elapsed` a boundary
    /// from m1-the-graph/48's table, checked against real
    /// `git log --date=relative` output (git's compact word dropped, unit
    /// abbreviated: "2 days ago" -> "2d").
    #[test]
    fn matches_git_log_date_relative_at_every_boundary() {
        let cases: &[(i64, &str)] = &[
            // seconds: git counts seconds up to 89s; Graft has no seconds
            // unit, so both sides of that stage read "now".
            (1, "now"),
            (89, "now"),
            // 90s is git's first minute reading, and it is "2m" (never
            // "1m" — the minute stage's rounding never lands on 1).
            (90, "2m"),
            (91, "2m"),
            (5339, "89m"), // just under 89.5m, rounds down to 89m
            (5340, "89m"), // exactly 89.5m, rounds down (ties go down)
            (5341, "89m"),
            (5400, "2h"), // 90m rounds up out of the minute stage
            (7200, "2h"),
            (125_999, "35h"),
            (126_000, "35h"),
            (129_599, "2d"), // 36h rounds up out of the hour stage
            (129_600, "2d"),
            (1_123_199, "13d"),
            (1_123_200, "13d"),
            (1_209_599, "2w"), // 14d rounds up out of the day stage
            (1_209_600, "2w"),
            (5_443_200, "9w"),
            (6_002_969, "10w"),
            (6_002_970, "2mo"), // 70d rounds up out of the week stage
            (6_048_000, "2mo"),
            (30_412_800, "12mo"),
            (31_536_000, "1y"), // 365d rounds up out of the month stage
            (34_560_000, "1y"),
            (47_174_400, "1y"), // 1y6mo (git) rounds to the nearer year
            (157_680_000, "5y"),
            (172_800_000, "5y"),
            (180_000_000, "6y"),
            (315_360_000, "10y"),
            (630_720_000, "20y"),
        ];
        let now = 2_000_000_000;
        for &(elapsed, expected) in cases {
            assert_eq!(
                relative_date(now, now - elapsed),
                expected,
                "elapsed = {elapsed}s"
            );
        }
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
}
