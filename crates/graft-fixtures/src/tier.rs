//! How many proptest cases this tier runs.
//!
//! PROMPT.md §7.1 sets two numbers: 64 cases in the fast tier, where a
//! property test is a smoke test that must not cost a minute, and 1000 in
//! the heavy tier, where it is actually trying to find something. The
//! difference is a feature flag, not a decision each test site makes for
//! itself, so that one command changes all of them.
//!
//! The count comes from this code and nothing else. `PROPTEST_CASES` used
//! to win, so that a person could narrow in on a failure; it also meant the
//! same `cargo test` ran a different test depending on whose shell it ran
//! in, and the heavy tier quietly shrank to 64 cases on a machine that had
//! the variable exported. Reproducibility is worth more than the shortcut
//! (ADR-0017). To hammer one property, edit its `ProptestConfig` for the
//! run.

/// The fast tier's count: a smoke test.
const FAST: u32 = 64;

/// The heavy tier's count.
const HEAVY: u32 = 1000;

/// How many cases to configure, for whichever tier this was compiled for:
/// 64 fast, 1000 with `--features heavy-tests`.
///
/// ```text
/// fn config() -> ProptestConfig {
///     ProptestConfig::with_cases(graft_fixtures::proptest_cases())
/// }
/// ```
pub fn proptest_cases() -> u32 {
    if cfg!(feature = "heavy-tests") {
        HEAVY
    } else {
        FAST
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_count_matches_the_tier_this_was_compiled_for() {
        let expected = if cfg!(feature = "heavy-tests") {
            HEAVY
        } else {
            FAST
        };
        assert_eq!(proptest_cases(), expected);
    }
}
