//! How many proptest cases this tier runs.
//!
//! PROMPT.md §7.1 sets two numbers: 64 cases in the fast tier, where a
//! property test is a smoke test that must not cost a minute, and 1000 in
//! the heavy tier, where it is actually trying to find something. The
//! difference is a feature flag, not a decision each test site makes for
//! itself, so that one command changes all of them.
//!
//! `PROPTEST_CASES` still wins. It is proptest's own environment variable
//! and the way a person narrows in on a failure ("run this one property ten
//! thousand times"); a hard-coded count here would silently ignore it.

/// The fast tier's count: a smoke test.
const FAST: u32 = 64;

/// The heavy tier's count.
const HEAVY: u32 = 1000;

/// How many cases to configure, or [`None`] to leave proptest's own
/// configuration — and therefore `PROPTEST_CASES` — alone.
///
/// ```text
/// fn config() -> ProptestConfig {
///     match graft_fixtures::proptest_cases() {
///         Some(cases) => ProptestConfig::with_cases(cases),
///         None => ProptestConfig::default(),
///     }
/// }
/// ```
pub fn proptest_cases() -> Option<u32> {
    if std::env::var_os("PROPTEST_CASES").is_some() {
        return None;
    }
    Some(if cfg!(feature = "heavy-tests") {
        HEAVY
    } else {
        FAST
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_count_matches_the_tier_this_was_compiled_for() {
        // The environment variable is process-wide, so this test reads
        // whichever state the run was started in rather than setting it:
        // `std::env::set_var` is unsound with other threads running, and
        // there are always other test threads.
        match std::env::var_os("PROPTEST_CASES") {
            Some(_) => assert_eq!(proptest_cases(), None),
            None if cfg!(feature = "heavy-tests") => assert_eq!(proptest_cases(), Some(HEAVY)),
            None => assert_eq!(proptest_cases(), Some(FAST)),
        }
    }
}
