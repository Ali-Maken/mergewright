//! The classification rules: which facts make a commit a foxtrot, backward
//! or accidental-pull merge, and which ref facts are a risky state.
//!
//! All three merge kinds rest on one heuristic pair: "is this commit (and
//! each parent) on the default branch's first-parent chain", plus what
//! git's own merge subject says was merged and where. A merge on the default
//! chain whose subject says it was made on another branch (`Merge branch
//! 'main' into feature`) got there by fast-forward: foxtrot. The same subject
//! off the chain is a backward merge. A subject that says the branch was
//! merged with its own remote copy (`Merge branch 'main' of <url>`) is what
//! `git pull` writes: an accidental pull merge. That last case is *not* read
//! as a foxtrot even when it sits on the default chain, because it was made
//! on the default branch itself, whose local first-parent line is intact;
//! if it was then pushed, the remote's line is what got rewritten, and
//! `RepoFacts::previous_default_tips` (from the remote-tracking reflog) is
//! how that shows up. Pull-request subjects never count: GitHub puts the
//! base branch first by construction.
//!
//! One finding per commit, foxtrot before backward before accidental pull.
//! Evil merge, duplicates and force-push come only from the probes.

use graft_core::mistake_radar::MistakeRadarKind;
use graft_core::oid::Oid;

use super::{
    BranchFacts, CommitFacts, Evidence, Finding, HeadFacts, Policy, RepoFacts, Risk, UpstreamFacts,
};

/// What a git merge subject says was merged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergedName {
    /// The branch (or tag) merged in, without any remote prefix.
    pub branch: String,
    /// The remote it came from, when the subject names one: the URL or
    /// path in `Merge branch 'x' of <remote>`, the remote name in `Merge
    /// remote-tracking branch 'origin/x'`, the fork owner in a pull request.
    pub remote: Option<String>,
    /// The branch it was merged into, when the subject says (`into y`).
    pub into: Option<String>,
    /// `Merge pull request #N from owner/branch`.
    pub pull_request: bool,
}

/// Reads git's own merge subjects and nothing else. Recognised forms:
///
/// - `Merge branch 'x'`, `Merge branch 'x' into y`
/// - `Merge branch 'x' of <remote>`, with an optional ` into y`
/// - `Merge remote-tracking branch 'r/x'`, with an optional ` into y`
/// - `Merge tag 't'`, with an optional ` into y`
/// - `Merge pull request #N from owner/x`
///
/// Anything else — a reverted merge, a lower-case or hand-written subject,
/// an octopus `Merge branches …` — is `None`.
pub fn merged_branch_from_subject(subject: &str) -> Option<MergedName> {
    let rest = subject.strip_prefix("Merge ")?;
    if let Some(rest) = rest.strip_prefix("branch ") {
        let (branch, rest) = quoted(rest)?;
        let (remote, rest) = match rest.strip_prefix(" of ") {
            Some(rest) => {
                let (remote, tail) = rest.split_once(" into ").map_or((rest, ""), |(r, y)| {
                    // `split_once` ate the separator; put it back for `into`.
                    let tail_start = rest.len() - y.len() - " into ".len();
                    (r, &rest[tail_start..])
                });
                if remote.is_empty() || remote.contains(' ') {
                    return None;
                }
                (Some(remote.to_owned()), tail)
            }
            None => (None, rest),
        };
        return Some(MergedName {
            branch,
            remote,
            into: tail(rest)?.branch(),
            pull_request: false,
        });
    }
    if let Some(rest) = rest.strip_prefix("remote-tracking branch ") {
        let (name, rest) = quoted(rest)?;
        let (remote, branch) = name.split_once('/')?;
        return Some(MergedName {
            branch: branch.to_owned(),
            remote: Some(remote.to_owned()),
            into: tail(rest)?.branch(),
            pull_request: false,
        });
    }
    if let Some(rest) = rest.strip_prefix("tag ") {
        let (tag, rest) = quoted(rest)?;
        return Some(MergedName {
            branch: tag,
            remote: None,
            into: tail(rest)?.branch(),
            pull_request: false,
        });
    }
    if let Some(rest) = rest.strip_prefix("pull request #") {
        let (number, rest) = rest.split_once(' ')?;
        if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let (owner, branch) = rest.strip_prefix("from ")?.split_once('/')?;
        if owner.is_empty() || branch.is_empty() || branch.contains(' ') {
            return None;
        }
        return Some(MergedName {
            branch: branch.to_owned(),
            remote: Some(owner.to_owned()),
            into: None,
            pull_request: true,
        });
    }
    None
}

/// A `'name'` at the start of `rest`; the name and what follows the quote.
fn quoted(rest: &str) -> Option<(String, &str)> {
    let rest = rest.strip_prefix('\'')?;
    let (name, rest) = rest.split_once('\'')?;
    if name.is_empty() {
        return None;
    }
    Some((name.to_owned(), rest))
}

/// What follows the merged name in a subject.
enum Tail {
    /// Nothing: the merge was made on the current branch.
    Bare,
    /// ` into y`: the merge was made on `y`.
    Into(String),
}

impl Tail {
    fn branch(self) -> Option<String> {
        match self {
            Self::Bare => None,
            Self::Into(branch) => Some(branch),
        }
    }
}

/// The tail after the merged name; `None` for anything that is not one of
/// git's subjects.
fn tail(rest: &str) -> Option<Tail> {
    if rest.is_empty() {
        return Some(Tail::Bare);
    }
    let name = rest.strip_prefix(" into ")?;
    if name.is_empty() || name.contains(' ') {
        return None;
    }
    Some(Tail::Into(name.to_owned()))
}

/// The subject names the default branch and says the merge was made on
/// some other branch: `Merge branch 'main' into feature`, `Merge
/// remote-tracking branch 'origin/main' into feature`, or the local `Merge
/// branch 'main'` (a branch cannot be merged into itself, so this was made
/// elsewhere). Pull requests never count.
fn merges_default_from_elsewhere(m: &MergedName, default: &str) -> bool {
    if m.pull_request || m.branch != default {
        return false;
    }
    match m.into.as_deref() {
        Some(into) => into != default,
        None => m.remote.is_none(),
    }
}

/// The subject says the branch was merged with its own remote copy: the
/// `of <remote>` or `remote-tracking` forms, into the same branch (`into
/// x`), or with no `into` — which git omits only on its default branch, so
/// the branch must be the default for the copy to be its own.
fn merges_own_remote_copy(m: &MergedName, default: Option<&str>) -> bool {
    if m.pull_request || m.remote.is_none() {
        return false;
    }
    match m.into.as_deref() {
        Some(into) => into == m.branch,
        None => default.is_none_or(|d| m.branch == d),
    }
}

/// The first non-first parent that is on the default chain.
fn non_first_parent_on_chain(c: &CommitFacts) -> Option<Oid> {
    c.parents
        .iter()
        .zip(c.parents_on_default_chain)
        .skip(1)
        .find(|(_, on)| **on)
        .map(|(parent, _)| *parent)
}

/// Classify one commit. `None` for a non-merge, for a merge no rule
/// matches, and for a kind the policy has off. Pure and O(1).
pub fn classify_commit(c: &CommitFacts, repo: &RepoFacts, policy: &Policy) -> Option<Finding> {
    if c.parents.len() < 2 {
        return None;
    }
    let merged = merged_branch_from_subject(c.subject);
    foxtrot(c, repo, merged.as_ref(), policy)
        .or_else(|| backward(c, repo, merged.as_ref(), policy))
        .or_else(|| accidental_pull(c, repo, merged.as_ref(), policy))
}

/// A merge on the default chain whose non-first parent was the default
/// branch's real tip: the chain says so, the reflog says so, or the subject
/// says the merge was made on another branch and fast-forwarded here.
fn foxtrot(
    c: &CommitFacts,
    repo: &RepoFacts,
    merged: Option<&MergedName>,
    policy: &Policy,
) -> Option<Finding> {
    let severity = policy.severity(MistakeRadarKind::FoxtrotMerge)?;
    let default = repo.default_branch?;
    if !c.on_default_chain {
        return None;
    }
    let real_tip = non_first_parent_on_chain(c)
        .or_else(|| {
            c.parents[1..]
                .iter()
                .copied()
                .find(|p| repo.previous_default_tips.contains(p))
        })
        .or_else(|| {
            merged
                .filter(|m| merges_default_from_elsewhere(m, default))
                .map(|_| c.parents[1])
        })?;
    Some(Finding {
        kind: MistakeRadarKind::FoxtrotMerge,
        severity,
        at: c.id,
        evidence: Evidence::Foxtrot {
            merge: c.id,
            first_parent: c.parents[0],
            real_tip,
            branch: default.to_owned(),
        },
    })
}

/// A merge off the default chain that brought the default branch in.
fn backward(
    c: &CommitFacts,
    repo: &RepoFacts,
    merged: Option<&MergedName>,
    policy: &Policy,
) -> Option<Finding> {
    let severity = policy.severity(MistakeRadarKind::BackwardMerge)?;
    let default = repo.default_branch?;
    if c.on_default_chain {
        return None;
    }
    let by_subject = merged.filter(|m| merges_default_from_elsewhere(m, default));
    let default_tip = non_first_parent_on_chain(c).or_else(|| by_subject.map(|_| c.parents[1]))?;
    Some(Finding {
        kind: MistakeRadarKind::BackwardMerge,
        severity,
        at: c.id,
        evidence: Evidence::Backward {
            merge: c.id,
            default_tip,
            into: by_subject.and_then(|m| m.into.clone()),
        },
    })
}

/// A branch merged with its own remote copy, per the subject.
fn accidental_pull(
    c: &CommitFacts,
    repo: &RepoFacts,
    merged: Option<&MergedName>,
    policy: &Policy,
) -> Option<Finding> {
    let severity = policy.severity(MistakeRadarKind::AccidentalPullMerge)?;
    let m = merged.filter(|m| merges_own_remote_copy(m, repo.default_branch))?;
    Some(Finding {
        kind: MistakeRadarKind::AccidentalPullMerge,
        severity,
        at: c.id,
        evidence: Evidence::AccidentalPull {
            merge: c.id,
            branch: m.branch.clone(),
            remote: m.remote.clone()?,
        },
    })
}

/// The risky states: detached `HEAD`, unpushed commits on the default
/// branch. A branch with no upstream is not reported, default or not: a
/// fresh `git init` or a branch not pushed yet is not a mistake
/// (m7-power-tools/15 removed the non-default rule). An unborn
/// `HEAD` is nothing yet. All `RiskyState`, in the order given: `HEAD`
/// first, then the branches.
pub fn classify_refs(branches: &[BranchFacts], head: &HeadFacts, policy: &Policy) -> Vec<Finding> {
    let Some(severity) = policy.severity(MistakeRadarKind::RiskyState) else {
        return Vec::new();
    };
    let finding = |at: Oid, risk: Risk| Finding {
        kind: MistakeRadarKind::RiskyState,
        severity,
        at,
        evidence: Evidence::Risk(risk),
    };
    let mut findings = Vec::new();
    if let HeadFacts::Detached(at) = head {
        findings.push(finding(*at, Risk::DetachedHead));
    }
    for b in branches {
        match b.upstream {
            UpstreamFacts::Apart { ahead, .. } if b.is_default && ahead > 0 => {
                findings.push(finding(
                    b.tip,
                    Risk::UnpushedOnDefault {
                        branch: b.name.to_owned(),
                        ahead,
                    },
                ));
            }
            UpstreamFacts::None | UpstreamFacts::InSync | UpstreamFacts::Apart { .. } => {}
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_core::mistake_radar::Severity;

    fn oid(n: u8) -> Oid {
        Oid::from_bytes(&[n; 20]).expect("20 bytes is a sha1")
    }

    fn name(
        branch: &str,
        remote: Option<&str>,
        into: Option<&str>,
        pull_request: bool,
    ) -> MergedName {
        MergedName {
            branch: branch.into(),
            remote: remote.map(str::to_owned),
            into: into.map(str::to_owned),
            pull_request,
        }
    }

    #[test]
    fn subject_table() {
        let cases: &[(&str, Option<MergedName>)] = &[
            ("Merge branch 'x'", Some(name("x", None, None, false))),
            (
                "Merge branch 'x' into y",
                Some(name("x", None, Some("y"), false)),
            ),
            (
                "Merge branch 'x' of https://example.invalid/r.git",
                Some(name(
                    "x",
                    Some("https://example.invalid/r.git"),
                    None,
                    false,
                )),
            ),
            (
                "Merge remote-tracking branch 'origin/x'",
                Some(name("x", Some("origin"), None, false)),
            ),
            (
                "Merge pull request #12 from o/x",
                Some(name("x", Some("o"), None, true)),
            ),
            ("Merge tag 'v1.0'", Some(name("v1.0", None, None, false))),
            (
                "Merge branch 'a' of github.com:o/r into b",
                Some(name("a", Some("github.com:o/r"), Some("b"), false)),
            ),
            (
                "Merge remote-tracking branch 'origin/release/2.1' into feature",
                Some(name("release/2.1", Some("origin"), Some("feature"), false)),
            ),
            ("Revert \"Merge branch 'x'\"", None),
            ("merge branch 'x'", None),
            ("Merge branches 'a', 'b' and 'c'", None),
            ("Merge branch 'x' with love", None),
            ("Merge pull request #x from o/x", None),
            ("Merge branch ''", None),
        ];
        for (subject, expected) in cases {
            assert_eq!(
                &merged_branch_from_subject(subject),
                expected,
                "{subject:?}"
            );
        }
    }

    // A merge on the default chain: parents (feature tip, main's old tip).
    fn merge_on_main<'a>(subject: &'a str, parents: &'a [Oid], on: &'a [bool]) -> CommitFacts<'a> {
        CommitFacts {
            id: oid(9),
            parents,
            subject,
            on_default_chain: true,
            parents_on_default_chain: on,
        }
    }

    const REMOTES: &[&str] = &["origin"];

    fn repo(previous: &[Oid]) -> RepoFacts<'_> {
        RepoFacts {
            default_branch: Some("main"),
            remotes: REMOTES,
            previous_default_tips: previous,
        }
    }

    #[test]
    fn foxtrot_from_the_chain_without_a_subject_clue() {
        let parents = [oid(1), oid(2)];
        let c = merge_on_main("Sync up", &parents, &[true, true]);
        let f = classify_commit(&c, &repo(&[]), &Policy::default()).expect("a foxtrot");
        assert_eq!(f.kind, MistakeRadarKind::FoxtrotMerge);
        assert_eq!(f.severity, Severity::Warning);
        assert_eq!(f.at, oid(9));
        assert_eq!(
            f.evidence,
            Evidence::Foxtrot {
                merge: oid(9),
                first_parent: oid(1),
                real_tip: oid(2),
                branch: "main".into(),
            }
        );
    }

    #[test]
    fn foxtrot_from_previous_default_tips_only() {
        let parents = [oid(1), oid(2)];
        let c = merge_on_main("Sync up", &parents, &[true, false]);
        let f =
            classify_commit(&c, &repo(&[oid(7), oid(2)]), &Policy::default()).expect("a foxtrot");
        assert_eq!(f.kind, MistakeRadarKind::FoxtrotMerge);
        assert!(matches!(f.evidence, Evidence::Foxtrot { real_tip, .. } if real_tip == oid(2)));
    }

    #[test]
    fn foxtrot_from_the_subject_when_made_elsewhere() {
        let parents = [oid(1), oid(2)];
        for subject in [
            "Merge branch 'main' into feature",
            "Merge branch 'main'",
            "Merge remote-tracking branch 'origin/main' into feature",
            "Merge branch 'main' of https://example.invalid/r.git into feature",
        ] {
            let c = merge_on_main(subject, &parents, &[true, false]);
            let f = classify_commit(&c, &repo(&[]), &Policy::default());
            assert_eq!(
                f.map(|f| f.kind),
                Some(MistakeRadarKind::FoxtrotMerge),
                "{subject:?}"
            );
        }
    }

    #[test]
    fn made_on_main_is_a_pull_merge_not_a_foxtrot() {
        let parents = [oid(1), oid(2)];
        for subject in [
            "Merge branch 'main' of https://example.invalid/r.git",
            "Merge branch 'main' of https://example.invalid/r.git into main",
            "Merge remote-tracking branch 'origin/main'",
        ] {
            let c = merge_on_main(subject, &parents, &[true, false]);
            let f = classify_commit(&c, &repo(&[]), &Policy::default());
            assert_eq!(
                f.map(|f| f.kind),
                Some(MistakeRadarKind::AccidentalPullMerge),
                "{subject:?}"
            );
        }
    }

    #[test]
    fn ordinary_merges_on_main_are_silent() {
        let parents = [oid(1), oid(2)];
        for subject in [
            "Merge branch 'feature'",
            "Merge pull request #12 from fork/main",
            "Merge remote-tracking branch 'origin/feature'",
            "Merge branch 'feature' of https://example.invalid/r.git",
            "Merge tag 'v1.0'",
            "Merge branches 'a', 'b' and 'c'",
        ] {
            let c = merge_on_main(subject, &parents, &[true, false]);
            assert_eq!(
                classify_commit(&c, &repo(&[]), &Policy::default()),
                None,
                "{subject:?}"
            );
        }
    }

    #[test]
    fn policy_off_yields_nothing() {
        let parents = [oid(1), oid(2)];
        let c = merge_on_main("Merge branch 'main' into feature", &parents, &[true, true]);
        let mut policy = Policy::default();
        policy.set(MistakeRadarKind::FoxtrotMerge, None);
        assert_eq!(classify_commit(&c, &repo(&[]), &policy), None);
    }

    #[test]
    fn non_merge_is_never_classified() {
        let parents = [oid(1)];
        let c = CommitFacts {
            id: oid(9),
            parents: &parents,
            subject: "Merge branch 'main' into feature",
            on_default_chain: true,
            parents_on_default_chain: &[true],
        };
        assert_eq!(
            classify_commit(&c, &repo(&[oid(1)]), &Policy::default()),
            None
        );
        let root = CommitFacts {
            parents: &[],
            parents_on_default_chain: &[],
            ..c
        };
        assert_eq!(classify_commit(&root, &repo(&[]), &Policy::default()), None);
    }

    #[test]
    fn backward_off_the_chain_by_parent_or_subject() {
        let parents = [oid(1), oid(2)];
        let off_chain = |subject, on| CommitFacts {
            id: oid(9),
            parents: &parents,
            subject,
            on_default_chain: false,
            parents_on_default_chain: on,
        };
        let c = off_chain("Merge branch 'main' into feature", &[false, false]);
        let f = classify_commit(&c, &repo(&[]), &Policy::default()).expect("backward");
        assert_eq!(f.kind, MistakeRadarKind::BackwardMerge);
        assert_eq!(f.severity, Severity::Info);
        assert_eq!(
            f.evidence,
            Evidence::Backward {
                merge: oid(9),
                default_tip: oid(2),
                into: Some("feature".into()),
            }
        );

        let c = off_chain("Sync with main", &[false, true]);
        let f = classify_commit(&c, &repo(&[]), &Policy::default()).expect("backward");
        assert_eq!(
            f.evidence,
            Evidence::Backward {
                merge: oid(9),
                default_tip: oid(2),
                into: None,
            }
        );

        let c = off_chain("Merge branch 'other' into feature", &[false, false]);
        assert_eq!(classify_commit(&c, &repo(&[]), &Policy::default()), None);
    }

    #[test]
    fn accidental_pull_needs_the_branch_to_be_its_own() {
        let parents = [oid(1), oid(2)];
        let off_chain = |subject| CommitFacts {
            id: oid(9),
            parents: &parents,
            subject,
            on_default_chain: false,
            parents_on_default_chain: &[false, false],
        };
        let c = off_chain("Merge branch 'feature' of https://example.invalid/r.git into feature");
        let f = classify_commit(&c, &repo(&[]), &Policy::default()).expect("accidental");
        assert_eq!(
            f.evidence,
            Evidence::AccidentalPull {
                merge: oid(9),
                branch: "feature".into(),
                remote: "https://example.invalid/r.git".into(),
            }
        );
        let c = off_chain("Merge remote-tracking branch 'origin/feature' into feature");
        assert_eq!(
            classify_commit(&c, &repo(&[]), &Policy::default()).map(|f| f.kind),
            Some(MistakeRadarKind::AccidentalPullMerge)
        );
        let c = off_chain("Merge branch 'other' of https://example.invalid/r.git into feature");
        assert_eq!(classify_commit(&c, &repo(&[]), &Policy::default()), None);
    }

    #[test]
    fn no_default_branch_leaves_only_the_pull_rule() {
        let parents = [oid(1), oid(2)];
        let bare = RepoFacts {
            default_branch: None,
            remotes: &[],
            previous_default_tips: &[],
        };
        let c = merge_on_main("Merge branch 'main' into feature", &parents, &[true, true]);
        assert_eq!(classify_commit(&c, &bare, &Policy::default()), None);
        let c = merge_on_main(
            "Merge branch 'main' of https://example.invalid/r.git",
            &parents,
            &[true, false],
        );
        assert_eq!(
            classify_commit(&c, &bare, &Policy::default()).map(|f| f.kind),
            Some(MistakeRadarKind::AccidentalPullMerge)
        );
    }

    /// `main` two ahead, `feature` without upstream, `behind` behind,
    /// `level` in sync.
    fn sample_branches() -> [BranchFacts<'static>; 4] {
        let apart = |ahead, behind| UpstreamFacts::Apart { ahead, behind };
        let branch = |name, n, is_default, upstream| BranchFacts {
            name,
            tip: oid(n),
            is_default,
            upstream,
        };
        [
            branch("main", 1, true, apart(2, 0)),
            branch("feature", 2, false, UpstreamFacts::None),
            branch("behind", 3, false, apart(0, 4)),
            branch("level", 4, false, UpstreamFacts::InSync),
        ]
    }

    #[test]
    fn risky_states() {
        let branches = sample_branches();
        let found = classify_refs(&branches, &HeadFacts::Detached(oid(5)), &Policy::default());
        let risks: Vec<(Oid, Risk)> = found
            .into_iter()
            .map(|f| {
                assert_eq!(f.kind, MistakeRadarKind::RiskyState);
                assert_eq!(f.severity, Severity::Info);
                match f.evidence {
                    Evidence::Risk(risk) => (f.at, risk),
                    other => panic!("not a risk: {other:?}"),
                }
            })
            .collect();
        assert_eq!(
            risks,
            vec![
                (oid(5), Risk::DetachedHead),
                (
                    oid(1),
                    Risk::UnpushedOnDefault {
                        branch: "main".into(),
                        ahead: 2
                    }
                ),
            ]
        );
    }

    /// m7-power-tools/15 (:777): a branch nobody has pushed yet is how every
    /// branch starts, not a mistake — `P` sets its upstream when it is time.
    #[test]
    fn a_branch_without_upstream_has_no_finding() {
        let branches = [BranchFacts {
            name: "feature",
            tip: oid(2),
            is_default: false,
            upstream: UpstreamFacts::None,
        }];
        for head in [HeadFacts::Branch("feature"), HeadFacts::Unborn] {
            assert!(classify_refs(&branches, &head, &Policy::default()).is_empty());
        }
    }

    #[test]
    fn default_without_upstream_and_unborn_head_are_not_risks() {
        let branches = [BranchFacts {
            name: "main",
            tip: oid(1),
            is_default: true,
            upstream: UpstreamFacts::None,
        }];
        assert!(classify_refs(&branches, &HeadFacts::Unborn, &Policy::default()).is_empty());
        assert!(
            classify_refs(&branches, &HeadFacts::Branch("main"), &Policy::default()).is_empty()
        );
        let mut off = Policy::default();
        off.set(MistakeRadarKind::RiskyState, None);
        assert!(classify_refs(&branches, &HeadFacts::Detached(oid(2)), &off).is_empty());
    }
}
