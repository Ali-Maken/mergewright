//! Reading git's transfer progress: the `Receiving objects:  45% (450/1000)`
//! lines a fetch, pull or push writes to stderr, one segment at a time.
//!
//! git redraws a progress line in place with `\r` and ends it with `\n`; the
//! runner's stderr sink hands over what lies between those, and this module
//! turns one such segment into a [`RemoteProgress`], or `None` when the
//! segment is anything else (a ref update, a `hint:`, an error). The runner
//! sets `LC_ALL=C`, so the phase names are git's English ones.

/// One progress segment, parsed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteProgress {
    /// What git is doing, lower-cased, without the colon: `receiving objects`.
    pub phase: String,
    /// How far through, when git gave a percentage.
    pub percent: Option<u8>,
    /// How many items are done: `450` of `(450/1000)`, or the lone count of
    /// `Enumerating objects: 12, done.` and `Total 12 (…)`.
    pub done: Option<u64>,
    /// How many items there are: `1000` of `(450/1000)`.
    pub total: Option<u64>,
    /// git's own throughput text, verbatim: `1.20 MiB | 2.30 MiB/s`.
    pub rate: Option<String>,
}

/// The most characters [`RemoteProgress::brief`] returns.
const BRIEF_MAX: usize = 32;

/// Words git starts a line with that are never progress, however the rest
/// reads.
const NOT_PHASES: [&str; 4] = ["hint", "error", "fatal", "warning"];

impl RemoteProgress {
    /// A short form for a footer: `receiving objects 45%`, else
    /// `receiving objects 450/1000`, else just the phase. At most 32
    /// characters; the counts are dropped when they would not fit.
    #[must_use]
    pub fn brief(&self) -> String {
        let detail = match (self.percent, self.done, self.total) {
            (Some(percent), _, _) => Some(format!("{percent}%")),
            (None, Some(done), Some(total)) => Some(format!("{done}/{total}")),
            _ => None,
        };
        let full = match detail {
            Some(detail) => format!("{} {detail}", self.phase),
            None => self.phase.clone(),
        };
        if full.chars().count() <= BRIEF_MAX {
            full
        } else {
            self.phase.chars().take(BRIEF_MAX).collect()
        }
    }
}

/// Parses one segment git wrote between `\r`/`\n` separators. A leading
/// `remote: ` (the server's own progress, relayed) is stripped, as is the
/// padding git puts after it.
#[must_use]
pub fn parse_progress(segment: &[u8]) -> Option<RemoteProgress> {
    let text = std::str::from_utf8(segment).ok()?.trim();
    let text = text.strip_prefix("remote:").map_or(text, str::trim_start);
    if let Some(rest) = text.strip_prefix("Total ") {
        return total_line(rest);
    }
    let (phase, rest) = text.split_once(':')?;
    let is_phase = !phase.is_empty()
        && phase.bytes().all(|b| b.is_ascii_alphabetic() || b == b' ')
        && !NOT_PHASES.contains(&phase.to_ascii_lowercase().as_str());
    if !is_phase {
        return None;
    }
    let mut progress = RemoteProgress {
        phase: phase.to_ascii_lowercase(),
        percent: None,
        done: None,
        total: None,
        rate: None,
    };
    let rest = rest.trim_start();
    let rest = rest.strip_suffix(", done.").unwrap_or(rest);
    if let Some((percent, after)) = rest.split_once('%') {
        progress.percent = Some(percent.trim().parse().ok()?);
        let after = after.trim_start();
        let counts = after.strip_prefix('(')?;
        let (counts, after) = counts.split_once(')')?;
        let (done, total) = counts.split_once('/')?;
        progress.done = Some(done.parse().ok()?);
        progress.total = Some(total.parse().ok()?);
        progress.rate = after
            .strip_prefix(',')
            .map(|rate| rate.trim().to_owned())
            .filter(|rate| !rate.is_empty());
    } else {
        // `Enumerating objects: 12` — a lone count, no percentage.
        progress.done = Some(rest.trim().parse().ok()?);
    }
    Some(progress)
}

/// `Total 12 (delta 3), reused 0 (delta 0)`, with `Total ` already gone.
fn total_line(rest: &str) -> Option<RemoteProgress> {
    let count = rest.split_whitespace().next()?;
    Some(RemoteProgress {
        phase: "total".to_owned(),
        percent: None,
        done: Some(count.parse().ok()?),
        total: None,
        rate: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn progress(
        phase: &str,
        percent: Option<u8>,
        done: Option<u64>,
        total: Option<u64>,
        rate: Option<&str>,
    ) -> RemoteProgress {
        RemoteProgress {
            phase: phase.to_owned(),
            percent,
            done,
            total,
            rate: rate.map(str::to_owned),
        }
    }

    /// The eight segment shapes git writes. The `remote: ` ones carry the
    /// trailing padding git 2.54.0 (Apple Git-157) was seen to add.
    #[test]
    fn accepts_the_documented_shapes() {
        let cases: [(&[u8], RemoteProgress); 8] = [
            (
                b"remote: Enumerating objects: 12, done.        ",
                progress("enumerating objects", None, Some(12), None, None),
            ),
            (
                b"remote: Counting objects:  50% (6/12)        ",
                progress("counting objects", Some(50), Some(6), Some(12), None),
            ),
            (
                b"remote: Compressing objects: 100% (3/3), done.        ",
                progress("compressing objects", Some(100), Some(3), Some(3), None),
            ),
            (
                b"Receiving objects:  45% (450/1000), 1.20 MiB | 2.30 MiB/s",
                progress(
                    "receiving objects",
                    Some(45),
                    Some(450),
                    Some(1000),
                    Some("1.20 MiB | 2.30 MiB/s"),
                ),
            ),
            (
                b"Resolving deltas:   6% (3/50)",
                progress("resolving deltas", Some(6), Some(3), Some(50), None),
            ),
            (
                b"Writing objects: 100% (3/3), 300 bytes | 300.00 KiB/s, done.",
                progress(
                    "writing objects",
                    Some(100),
                    Some(3),
                    Some(3),
                    Some("300 bytes | 300.00 KiB/s"),
                ),
            ),
            (
                b"Unpacking objects: 100% (3/3), 250 bytes | 250.00 KiB/s, done.",
                progress(
                    "unpacking objects",
                    Some(100),
                    Some(3),
                    Some(3),
                    Some("250 bytes | 250.00 KiB/s"),
                ),
            ),
            (
                b"Total 12 (delta 3), reused 0 (delta 0)",
                progress("total", None, Some(12), None, None),
            ),
        ];
        for (segment, expected) in cases {
            assert_eq!(
                parse_progress(segment),
                Some(expected),
                "{}",
                String::from_utf8_lossy(segment)
            );
        }
    }

    #[test]
    fn rejects_a_ref_update_line() {
        assert_eq!(
            parse_progress(b" * [new branch]      main       -> origin/main"),
            None
        );
        assert_eq!(parse_progress(b"   865fc1a..6402177  main -> main"), None);
        assert_eq!(parse_progress(b"From file:///tmp/bare"), None);
    }

    #[test]
    fn rejects_a_hint_and_an_error() {
        assert_eq!(parse_progress(b"hint: 3 things to know"), None);
        assert_eq!(
            parse_progress(
                b"hint: Updates were rejected because the tip of your current branch is behind"
            ),
            None
        );
        assert_eq!(
            parse_progress(b"error: failed to push some refs to 'x'"),
            None
        );
        assert_eq!(
            parse_progress(b"fatal: 'nope' does not appear to be a git repository"),
            None
        );
        assert_eq!(parse_progress(b"remote: no pushes today        "), None);
        assert_eq!(parse_progress(b""), None);
    }

    #[test]
    fn brief_prefers_percent_then_counts_then_phase() {
        let with_percent = progress("receiving objects", Some(45), Some(450), Some(1000), None);
        assert_eq!(with_percent.brief(), "receiving objects 45%");
        let counts = progress("counting objects", None, Some(6), Some(12), None);
        assert_eq!(counts.brief(), "counting objects 6/12");
        let bare = progress("enumerating objects", None, Some(12), None, None);
        assert_eq!(bare.brief(), "enumerating objects");
    }

    #[test]
    fn brief_never_passes_32_characters() {
        let long = progress(
            "counting objects",
            None,
            Some(123_456_789),
            Some(987_654_321),
            None,
        );
        assert!(
            long.brief().chars().count() <= BRIEF_MAX,
            "{}",
            long.brief()
        );
        assert_eq!(long.brief(), "counting objects");
    }
}
