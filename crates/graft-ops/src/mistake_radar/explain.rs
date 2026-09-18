//! Plain-English accounts of each finding: what happened, why it matters,
//! what a fix would be (in words; the Fix… actions are M5). Present tense,
//! three short paragraphs at most, commits named through the caller's
//! `short` so the panel and the graph agree on how a SHA looks.

use graft_core::oid::Oid;

use super::{Evidence, Finding, Risk};

/// What the Explain panel shows for one finding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Explanation {
    /// Upper-case, e.g. `FOXTROT MERGE`.
    pub title: String,
    /// At most three, in reading order.
    pub paragraphs: Vec<String>,
    /// Labelled commits for the mini-graph, top to bottom.
    pub evidence: Vec<(Oid, String)>,
    /// The commits the graph keeps lit while the panel is open.
    pub highlight: Vec<Oid>,
}

/// A commit id renderer, as the graph shows them.
type Short<'a> = &'a dyn Fn(Oid) -> String;

/// Explain `f`. `short` renders a commit id the way the graph does.
pub fn explain(f: &Finding, short: Short) -> Explanation {
    match &f.evidence {
        Evidence::Foxtrot {
            merge,
            first_parent,
            real_tip,
            branch,
        } => foxtrot(*merge, *first_parent, *real_tip, branch, short),
        Evidence::Backward {
            merge,
            default_tip,
            into,
        } => backward(*merge, *default_tip, into.as_deref(), short),
        Evidence::AccidentalPull {
            merge,
            branch,
            remote,
        } => accidental_pull(*merge, branch, remote),
        Evidence::Risk(risk) => explain_risk(f.at, risk, short),
        Evidence::Evil { files } => evil(f.at, files, short),
        Evidence::Duplicate { copies } => duplicate(f.at, copies, short),
        Evidence::ForcePushed {
            remote_ref,
            previous,
            current,
        } => force_pushed(remote_ref, *previous, *current, short),
    }
}

fn foxtrot(
    merge: Oid,
    first_parent: Oid,
    real_tip: Oid,
    branch: &str,
    short: Short,
) -> Explanation {
    Explanation {
        title: "FOXTROT MERGE".into(),
        paragraphs: vec![
            format!(
                "This merge's first parent ({}) is not {branch}'s previous tip ({}). \
                 {branch}'s first-parent line was rewritten — usually by pulling on \
                 {branch} and pushing the result, or by merging {branch} into a branch \
                 and then fast-forwarding {branch} to it.",
                short(first_parent),
                short(real_tip)
            ),
            format!(
                "Anything that follows first parents — `git log --first-parent`, \
                 bisecting by merge, release notes — now walks through the other \
                 branch's commits instead of {branch}'s own history."
            ),
            format!(
                "A fix is to reset {branch} to {} (keeping a backup ref) and merge the \
                 branch again the normal way, or to revert this merge with `-m 1`.",
                short(real_tip)
            ),
        ],
        evidence: vec![
            (real_tip, format!("[{branch}] was here")),
            (merge, "this merge".into()),
            (first_parent, "first parent".into()),
        ],
        highlight: vec![merge, first_parent, real_tip],
    }
}

fn backward(merge: Oid, default_tip: Oid, into: Option<&str>, short: Short) -> Explanation {
    let target = into.map_or_else(|| "this branch".to_owned(), |b| format!("'{b}'"));
    Explanation {
        title: "BACKWARD MERGE".into(),
        paragraphs: vec![
            format!(
                "This merge brings the default branch ({}) into {target}: the default \
                 branch was merged into a feature branch, not the other way round.",
                short(default_tip)
            ),
            "The branch's history now carries a merge of the default branch, which \
             makes its eventual merge harder to read and to revert as one unit."
                .into(),
            "Some teams do this on purpose to keep a long-lived branch current; if so, \
             `[radar] backward = off` silences it. Otherwise rebasing the branch onto \
             the default branch gives the same result as a straight line."
                .into(),
        ],
        evidence: vec![
            (default_tip, "default branch".into()),
            (merge, "this merge".into()),
        ],
        highlight: vec![merge, default_tip],
    }
}

fn accidental_pull(merge: Oid, branch: &str, remote: &str) -> Explanation {
    Explanation {
        title: "ACCIDENTAL PULL MERGE".into(),
        paragraphs: vec![
            format!(
                "This merge joins '{branch}' with its own copy on {remote}: `git pull` \
                 found commits on both sides and merged them."
            ),
            "It carries no change of its own; it only makes the branch fork and rejoin \
             for no reason, and once pushed it rewrites the shared first-parent line."
                .into(),
            "Pulling with rebase (`git pull --rebase`, or `pull.rebase = true`) keeps \
             the branch straight. If this merge is not pushed yet, resetting to its \
             first parent and rebasing removes it."
                .into(),
        ],
        evidence: vec![(merge, "this merge".into())],
        highlight: vec![merge],
    }
}

fn evil(merge: Oid, files: &[Vec<u8>], short: Short) -> Explanation {
    let names: Vec<String> = files
        .iter()
        .take(5)
        .map(|f| String::from_utf8_lossy(f).into_owned())
        .collect();
    let more = files.len().saturating_sub(names.len());
    let listed = if more > 0 {
        format!("{} and {more} more", names.join(", "))
    } else {
        names.join(", ")
    };
    let plural = if files.len() == 1 { "" } else { "s" };
    Explanation {
        title: "EVIL MERGE".into(),
        paragraphs: vec![
            format!(
                "This merge changes {} file{plural} that only one side, or neither, had \
                 touched since the merge base: {listed}. No conflict could have arisen \
                 there, so the edit came from the merge itself.",
                files.len()
            ),
            "Those edits do not show in a diff against either parent, so a revert or a \
             rebase of the merge silently loses or duplicates them. Files both sides \
             changed are not listed: a hand-resolved conflict looks the same there, so \
             the check stays quiet rather than warn on ordinary work."
                .into(),
            format!(
                "`git show --remerge-diff {}` shows exactly what the merge added, the \
                 files both sides changed included. If the edits were meant, a separate \
                 commit after the merge keeps them visible.",
                short(merge)
            ),
        ],
        evidence: vec![(merge, "this merge".into())],
        highlight: vec![merge],
    }
}

fn duplicate(at: Oid, copies: &[Oid], short: Short) -> Explanation {
    let others: Vec<String> = copies.iter().map(|c| short(*c)).collect();
    let mut highlight = vec![at];
    highlight.extend(copies.iter().copied());
    let mut evidence = vec![(at, "this commit".into())];
    evidence.extend(copies.iter().map(|c| (*c, "same patch".to_owned())));
    Explanation {
        title: "DUPLICATE COMMIT".into(),
        paragraphs: vec![
            format!(
                "This commit's patch also appears as {}: the same change was \
                 cherry-picked or rebased onto more than one branch.",
                others.join(", ")
            ),
            "Merging those branches later applies it twice or conflicts, and history \
             shows one change as several commits."
                .into(),
            "Nothing to fix if the branches are meant to stay apart (a backport, say). \
             Otherwise drop the copy on the branch that will be merged."
                .into(),
        ],
        evidence,
        highlight,
    }
}

fn force_pushed(remote_ref: &str, previous: Oid, current: Oid, short: Short) -> Explanation {
    Explanation {
        title: "FORCE-PUSHED REMOTE".into(),
        paragraphs: vec![
            format!(
                "{remote_ref} moved from {} to {}, and the new tip does not descend from \
                 the old one: the remote branch was rewritten.",
                short(previous),
                short(current)
            ),
            format!(
                "Anything based on {} now sits on history the remote no longer has; a \
                 plain pull merges the two.",
                short(previous)
            ),
            format!(
                "Work on top of {} is best rebased onto {} (`git rebase --onto`). If the \
                 push was a mistake, the old tip is still {} until git prunes it.",
                short(previous),
                short(current),
                short(previous)
            ),
        ],
        evidence: vec![(previous, "was".into()), (current, "now".into())],
        highlight: vec![previous, current],
    }
}

fn explain_risk(at: Oid, risk: &Risk, short: Short) -> Explanation {
    let (title, paragraphs, label) = match risk {
        Risk::DetachedHead => (
            "DETACHED HEAD",
            vec![
                format!(
                    "HEAD points at {} directly, not at a branch. New commits made here \
                     belong to no branch.",
                    short(at)
                ),
                "Once something else is checked out, those commits are reachable only \
                 from the reflog, and git prunes them eventually."
                    .into(),
                "Creating a branch here (`git switch -c <name>`) keeps them.".into(),
            ],
            "HEAD".to_owned(),
        ),
        Risk::UnpushedOnDefault { branch, ahead } => (
            "UNPUSHED COMMITS",
            vec![
                format!(
                    "{branch} is {ahead} commit{} ahead of its upstream. Those commits \
                     exist only in this clone.",
                    if *ahead == 1 { "" } else { "s" }
                ),
                format!(
                    "Work on the default branch that is not pushed is one lost disk away \
                     from gone, and everyone else is building on an older {branch}."
                ),
                format!(
                    "Push {branch}, or move the commits to a feature branch if they were \
                     not meant for {branch}."
                ),
            ],
            format!("[{branch}]"),
        ),
        Risk::NoUpstream { branch } => (
            "NO UPSTREAM",
            vec![
                format!(
                    "'{branch}' has no upstream branch: git does not know where to push \
                     it or what to compare it with."
                ),
                "Ahead/behind counts and a plain `git push` need one.".into(),
                format!("`git push -u origin {branch}` sets it on the first push."),
            ],
            format!("[{branch}]"),
        ),
    };
    Explanation {
        title: title.into(),
        paragraphs,
        evidence: vec![(at, label)],
        highlight: vec![at],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mistake_radar::probes::{ForcePushVerdict, findings_from_force_push};
    use crate::mistake_radar::{Finding, Policy};
    use graft_core::mistake_radar::{MistakeRadarKind, Severity};

    fn oid(n: u8) -> Oid {
        Oid::from_bytes(&[n; 20]).expect("20 bytes is a sha1")
    }

    fn short(id: Oid) -> String {
        id.to_string()[..7].to_owned()
    }

    #[test]
    fn foxtrot_wording_and_mini_graph() {
        let (merge, first_parent, real_tip) = (oid(3), oid(1), oid(2));
        let f = Finding {
            kind: MistakeRadarKind::FoxtrotMerge,
            severity: Severity::Warning,
            at: merge,
            evidence: Evidence::Foxtrot {
                merge,
                first_parent,
                real_tip,
                branch: "main".into(),
            },
        };
        let e = explain(&f, &short);
        assert_eq!(e.title, "FOXTROT MERGE");
        assert!(e.paragraphs.len() <= 3);
        assert!(
            e.paragraphs[0].contains("first-parent"),
            "{}",
            e.paragraphs[0]
        );
        assert!(e.paragraphs[0].contains(&short(first_parent)));
        assert!(e.paragraphs[0].contains(&short(real_tip)));
        assert_eq!(e.highlight, vec![merge, first_parent, real_tip]);
        assert_eq!(
            e.evidence,
            vec![
                (real_tip, "[main] was here".to_owned()),
                (merge, "this merge".to_owned()),
                (first_parent, "first parent".to_owned()),
            ]
        );
    }

    #[test]
    fn every_kind_explains_in_at_most_three_paragraphs() {
        let findings = [
            Evidence::Backward {
                merge: oid(1),
                default_tip: oid(2),
                into: None,
            },
            Evidence::AccidentalPull {
                merge: oid(1),
                branch: "main".into(),
                remote: "origin".into(),
            },
            Evidence::Risk(Risk::DetachedHead),
            Evidence::Risk(Risk::UnpushedOnDefault {
                branch: "main".into(),
                ahead: 1,
            }),
            Evidence::Risk(Risk::NoUpstream {
                branch: "feature".into(),
            }),
            Evidence::Evil {
                files: vec![b"a.txt".to_vec(); 7],
            },
            Evidence::Duplicate {
                copies: vec![oid(2), oid(3)],
            },
            Evidence::ForcePushed {
                remote_ref: "origin/main".into(),
                previous: oid(2),
                current: oid(3),
            },
        ];
        for evidence in findings {
            let f = Finding {
                kind: MistakeRadarKind::RiskyState,
                severity: Severity::Info,
                at: oid(1),
                evidence,
            };
            let e = explain(&f, &short);
            assert!(!e.title.is_empty() && e.title == e.title.to_uppercase());
            assert!((1..=3).contains(&e.paragraphs.len()), "{}", e.title);
            assert!(
                !e.evidence.is_empty() && !e.highlight.is_empty(),
                "{}",
                e.title
            );
            assert!(e.paragraphs.iter().all(|p| !p.is_empty()));
        }
    }

    #[test]
    fn not_checkable_force_push_never_becomes_a_finding() {
        let verdict = ForcePushVerdict::NotCheckable("no reflog for origin/main".into());
        assert_eq!(findings_from_force_push(&verdict, &Policy::default()), None);
    }
}
