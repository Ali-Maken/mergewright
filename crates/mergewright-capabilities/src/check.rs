//! Checks a capability's citations against the repository on disk.
//!
//! This is the point of the whole database (see
//! `docs/capabilities/README.md`): a cited test that was renamed, a capture
//! that was never committed, or an ADR that doesn't exist must fail the
//! fast test tier, not wait for someone to notice by eye.

use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{Capability, DecisionPath};

/// One citation that didn't check out.
#[derive(Debug, Clone)]
pub struct Finding {
    /// The capability file the bad citation lives in, relative to the repo
    /// root.
    pub file: PathBuf,
    /// The line of the capability's heading — close enough to find the
    /// citation by eye, since citations aren't individually line-tracked.
    pub line: usize,
    /// The capability id, e.g. `M2-07`.
    pub capability_id: String,
    /// What kind of citation failed.
    pub kind: FindingKind,
    /// The exact text that was cited and could not be found.
    pub citation: String,
}

/// What sort of citation a [`Finding`] reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingKind {
    /// A `**Proven by:**` test name with no matching `fn` in `crates/`.
    DeadTest,
    /// A `**Looks like:**` capture path not on disk.
    MissingCapture,
    /// A `**Decided in:**` ADR or design note not on disk.
    MissingDecision,
    /// A `unverified_aspect` meta field with no corroborating phrase in the
    /// `**Status:**` prose — the field has drifted from the sentence it was
    /// added to name.
    UnverifiedAspectNotInProse,
}

impl fmt::Display for FindingKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::DeadTest => "dead test citation",
            Self::MissingCapture => "missing capture",
            Self::MissingDecision => "missing decision (ADR / design note)",
            Self::UnverifiedAspectNotInProse => {
                "unverified_aspect not corroborated by the Status prose"
            }
        })
    }
}

/// Phrases that corroborate a `unverified_aspect` meta field: the `Status`
/// prose must contain at least one, case-insensitively, or the field is
/// reporting a caveat the prose doesn't actually make (or no longer makes —
/// editing the prose to drop a caveat without also removing the meta field
/// is exactly the drift this check exists to catch). Only fires when
/// `unverified_aspect` is set, so a capability that merely *mentions* one of
/// these words in passing — `M0-03`'s "this is **broken**, not unverified"
/// — is never affected unless something also gives it the field.
const UNVERIFIED_ASPECT_TRIGGERS: &[&str] = &[
    "unverified",
    "unverifiable",
    "not confirmed",
    "not proven",
    "not verified",
    "not re-measured",
    "not re-verified",
    "left open",
    "stay open",
    "stays open",
    "left to the user",
    "not judged",
    "never been judged",
    "no test in this repo can prove",
    "genuinely unknowable",
    "still blank",
    "pending",
];

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: {} in {}: `{}` not found",
            self.file.display(),
            self.line,
            self.kind,
            self.capability_id,
            self.citation,
        )
    }
}

/// Checks every citation of every capability against `repo_root`. Returns
/// one [`Finding`] per bad citation — empty means everything held.
#[must_use]
pub fn check(repo_root: &Path, capabilities: &[Capability]) -> Vec<Finding> {
    let known_identifiers = collect_identifiers(&repo_root.join("crates"));
    let mut findings = Vec::new();

    for cap in capabilities {
        for test in &cap.tests {
            let Some(name) = test.rsplit("::").next() else {
                continue;
            };
            if !known_identifiers.contains(name) {
                findings.push(Finding {
                    file: cap.source_file.clone(),
                    line: cap.source_line,
                    capability_id: cap.id.clone(),
                    kind: FindingKind::DeadTest,
                    citation: test.clone(),
                });
            }
        }

        for capture in &cap.captures {
            if !repo_root.join(capture).is_file() {
                findings.push(Finding {
                    file: cap.source_file.clone(),
                    line: cap.source_line,
                    capability_id: cap.id.clone(),
                    kind: FindingKind::MissingCapture,
                    citation: capture.clone(),
                });
            }
        }

        for decision in &cap.decisions {
            let found = match &decision.resolved {
                DecisionPath::Exact(path) => repo_root.join(path).is_file(),
                DecisionPath::GlobPrefix { dir, prefix } => fs::read_dir(repo_root.join(dir))
                    .into_iter()
                    .flatten()
                    .filter_map(Result::ok)
                    .any(|entry| {
                        entry
                            .file_name()
                            .to_str()
                            .is_some_and(|name| name.starts_with(prefix.as_str()))
                    }),
            };
            if !found {
                findings.push(Finding {
                    file: cap.source_file.clone(),
                    line: cap.source_line,
                    capability_id: cap.id.clone(),
                    kind: FindingKind::MissingDecision,
                    citation: decision.cited.clone(),
                });
            }
        }

        if let Some(finding) = unverified_aspect_finding(cap) {
            findings.push(finding);
        }
    }

    findings
}

/// Checks a capability's `unverified_aspect` (if set) against its own
/// `**Status:**` prose, returning a [`Finding`] when the field has
/// drifted from the sentence it was added to name.
fn unverified_aspect_finding(cap: &Capability) -> Option<Finding> {
    let aspect = cap.unverified_aspect.as_ref()?;
    // Strip markdown emphasis markers first: authors write "explicitly
    // **not** verified", and a literal `**` sitting between "not" and
    // "verified" would otherwise hide a real corroborating phrase from a
    // plain substring search.
    let prose = cap.status_prose.replace('*', "").to_lowercase();
    let corroborated = UNVERIFIED_ASPECT_TRIGGERS
        .iter()
        .any(|trigger| prose.contains(trigger));
    if corroborated {
        return None;
    }
    Some(Finding {
        file: cap.source_file.clone(),
        line: cap.source_line,
        capability_id: cap.id.clone(),
        kind: FindingKind::UnverifiedAspectNotInProse,
        citation: aspect.clone(),
    })
}

/// Every identifier token (a maximal run of ASCII alphanumeric/`_`
/// characters) that appears anywhere in any `.rs` file under `dir`
/// (recursively, skipping `target`).
///
/// A citation's last path segment is checked against this whole-source
/// identifier set rather than only `fn` declarations, because
/// `**Proven by:**` cites at more than one granularity: a specific test
/// function (`opening_a_repository_reports_what_it_found`, unique enough
/// that a coincidental match elsewhere is practically impossible), but
/// also a test *module* (`mergewright::view::journal`) or a type/variant named in
/// the surrounding prose as evidence (`offer_state`, `Shape::ALL`). Both
/// are real, checkable claims — "this still exists in the source" — and a
/// renamed or deleted long, sentence-like test name is exactly the failure
/// this is built to catch (`docs/capabilities/README.md`).
fn collect_identifiers(dir: &Path) -> HashSet<String> {
    let mut names = HashSet::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        let Ok(entries) = fs::read_dir(&path) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let entry_path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                if entry.file_name() != "target" {
                    stack.push(entry_path);
                }
                continue;
            }
            if entry_path.extension().is_some_and(|ext| ext == "rs")
                && let Ok(content) = fs::read_to_string(&entry_path)
            {
                collect_identifiers_from(&content, &mut names);
            }
        }
    }
    names
}

/// Splits one file's source text into identifier tokens.
fn collect_identifiers_from(content: &str, names: &mut HashSet<String>) {
    for token in content.split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
        if !token.is_empty() {
            names.insert(token.to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Capability, Decision, Status};

    fn base_capability() -> Capability {
        Capability {
            id: "M9-01".to_owned(),
            title: "Sample".to_owned(),
            area: "sample".to_owned(),
            milestone: "m9-sample".to_owned(),
            status: Status::Works,
            unverified_aspect: None,
            status_prose: "works.".to_owned(),
            tests: Vec::new(),
            captures: Vec::new(),
            decided_in: String::new(),
            decisions: Vec::new(),
            source_file: PathBuf::from("docs/capabilities/sample.md"),
            source_line: 5,
        }
    }

    #[test]
    fn dead_test_citation_is_reported() {
        let dir = tempdir();
        fs::create_dir_all(dir.join("crates/mergewright/src")).unwrap();
        fs::write(
            dir.join("crates/mergewright/src/lib.rs"),
            "fn real_test_name() {}\n",
        )
        .unwrap();

        let mut cap = base_capability();
        cap.tests = vec![
            "mergewright::real_test_name".to_owned(),
            "mergewright::renamed_or_deleted".to_owned(),
        ];

        let findings = check(&dir, &[cap]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, FindingKind::DeadTest);
        assert_eq!(findings[0].citation, "mergewright::renamed_or_deleted");
        cleanup(dir);
    }

    #[test]
    fn missing_capture_is_reported() {
        let dir = tempdir();
        let mut cap = base_capability();
        cap.captures = vec!["docs/shots/does-not-exist.txt".to_owned()];
        let findings = check(&dir, &[cap]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, FindingKind::MissingCapture);
        cleanup(dir);
    }

    #[test]
    fn missing_adr_is_reported() {
        let dir = tempdir();
        fs::create_dir_all(dir.join("docs/adr")).unwrap();
        let mut cap = base_capability();
        cap.decisions = vec![Decision {
            cited: "ADR-0099".to_owned(),
            resolved: DecisionPath::GlobPrefix {
                dir: "docs/adr".to_owned(),
                prefix: "0099".to_owned(),
            },
        }];
        let findings = check(&dir, &[cap]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, FindingKind::MissingDecision);
        cleanup(dir);
    }

    #[test]
    fn present_adr_is_not_reported() {
        let dir = tempdir();
        fs::create_dir_all(dir.join("docs/adr")).unwrap();
        fs::write(dir.join("docs/adr/0099-thing.md"), "x").unwrap();
        let mut cap = base_capability();
        cap.decisions = vec![Decision {
            cited: "ADR-0099".to_owned(),
            resolved: DecisionPath::GlobPrefix {
                dir: "docs/adr".to_owned(),
                prefix: "0099".to_owned(),
            },
        }];
        let findings = check(&dir, &[cap]);
        assert!(findings.is_empty());
        cleanup(dir);
    }

    #[test]
    fn unverified_aspect_corroborated_by_prose_is_not_reported() {
        let dir = tempdir();
        let mut cap = base_capability();
        cap.status_prose = "works. Colour is unverified by eye.".to_owned();
        cap.unverified_aspect = Some("colour".to_owned());
        let findings = check(&dir, &[cap]);
        assert!(findings.is_empty());
        cleanup(dir);
    }

    #[test]
    fn unverified_aspect_with_no_trace_in_prose_is_reported() {
        let dir = tempdir();
        let mut cap = base_capability();
        cap.status_prose = "works. Everything about this was checked.".to_owned();
        cap.unverified_aspect = Some("colour".to_owned());
        let findings = check(&dir, &[cap]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, FindingKind::UnverifiedAspectNotInProse);
        cleanup(dir);
    }

    // A tiny throwaway-directory helper, since this crate has no
    // dev-dependency on `tempfile` (one dependency for four small tests
    // isn't worth it). The process id plus a per-process counter keeps
    // concurrent test threads from colliding on one directory.
    fn tempdir() -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "mergewright-capabilities-check-test-{}-{n}",
            std::process::id(),
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup(dir: PathBuf) {
        // Best-effort: a leftover temp dir from a killed test run is a
        // non-issue (next run gets a fresh counter value), and failing the
        // test over cleanup would hide the real assertion failure above it.
        drop(fs::remove_dir_all(dir));
    }
}
