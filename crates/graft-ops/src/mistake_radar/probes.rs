//! The verdicts the three lazy probes return, and how each becomes a
//! finding. The probes themselves (a `git` process each: `show
//! --remerge-diff`, the remote-tracking reflog, `patch-id` over a window)
//! are built by the `radar-probe-*` tickets; this module is the contract
//! they fill.
//!
//! "Not checkable" never becomes a finding: absence of evidence is not a
//! clean bill (PROMPT.md §1.4). The pane says "not checkable in this repo"
//! from the verdict itself.

use graft_core::mistake_radar::MistakeRadarKind;
use graft_core::oid::Oid;

use super::{Evidence, Finding, Policy};

/// What re-merging one merge's parents showed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvilVerdict {
    /// The merge's tree is what merging its parents gives.
    Clean,
    /// The merge changed files beyond combining its parents.
    Evil {
        /// The files whose content matches neither parent nor the clean
        /// merge, as paths in the repository.
        files: Vec<Vec<u8>>,
    },
    /// Why the check could not run (e.g. git too old for `--remerge-diff`).
    NotCheckable(String),
}

/// What the remote-tracking ref's reflog showed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ForcePushVerdict {
    /// No reflog to compare against (a fresh clone), or another stated reason.
    NotCheckable(String),
    /// Every recorded move was a fast-forward.
    Clean,
    /// The ref moved without descending from where it was.
    Rewritten {
        /// The ref, short (`origin/main`).
        remote_ref: String,
        /// Where it pointed before.
        previous: Oid,
        /// Where it points now.
        current: Oid,
    },
}

/// Commits sharing a patch-id, found over a bounded window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Duplicates {
    /// Each group holds every commit with one patch-id, at least two.
    pub groups: Vec<Vec<Oid>>,
    /// How many commits the probe looked at.
    pub window: u32,
    /// Whether the window covered every visible ref, or was cut short.
    pub complete: bool,
}

/// An evil-merge finding on `merge`, when the verdict is `Evil` and the
/// kind is on.
pub fn finding_from_evil(merge: Oid, v: &EvilVerdict, policy: &Policy) -> Option<Finding> {
    let severity = policy.severity(MistakeRadarKind::EvilMerge)?;
    match v {
        EvilVerdict::Evil { files } => Some(Finding {
            kind: MistakeRadarKind::EvilMerge,
            severity,
            at: merge,
            evidence: Evidence::Evil {
                files: files.clone(),
            },
        }),
        EvilVerdict::Clean | EvilVerdict::NotCheckable(_) => None,
    }
}

/// A force-push finding at the current tip, when the verdict is
/// `Rewritten` and the kind is on.
pub fn findings_from_force_push(v: &ForcePushVerdict, policy: &Policy) -> Option<Finding> {
    let severity = policy.severity(MistakeRadarKind::ForcePushedRemote)?;
    match v {
        ForcePushVerdict::Rewritten {
            remote_ref,
            previous,
            current,
        } => Some(Finding {
            kind: MistakeRadarKind::ForcePushedRemote,
            severity,
            at: *current,
            evidence: Evidence::ForcePushed {
                remote_ref: remote_ref.clone(),
                previous: *previous,
                current: *current,
            },
        }),
        ForcePushVerdict::Clean | ForcePushVerdict::NotCheckable(_) => None,
    }
}

/// One finding per copy, each listing the rest of its group. Groups of one
/// are not duplicates and yield nothing.
pub fn findings_from_duplicates(d: &Duplicates, policy: &Policy) -> Vec<Finding> {
    let Some(severity) = policy.severity(MistakeRadarKind::DuplicateCommit) else {
        return Vec::new();
    };
    d.groups
        .iter()
        .filter(|group| group.len() >= 2)
        .flat_map(|group| {
            group.iter().map(move |at| Finding {
                kind: MistakeRadarKind::DuplicateCommit,
                severity,
                at: *at,
                evidence: Evidence::Duplicate {
                    copies: group.iter().copied().filter(|c| c != at).collect(),
                },
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_core::mistake_radar::Severity;

    fn oid(n: u8) -> Oid {
        Oid::from_bytes(&[n; 20]).expect("20 bytes is a sha1")
    }

    #[test]
    fn duplicates_group_of_three_yields_three_findings_listing_the_other_two() {
        let d = Duplicates {
            groups: vec![vec![oid(1), oid(2), oid(3)], vec![oid(9)]],
            window: 500,
            complete: true,
        };
        let found = findings_from_duplicates(&d, &Policy::default());
        assert_eq!(found.len(), 3);
        for (f, expected_copies) in found.iter().zip([
            vec![oid(2), oid(3)],
            vec![oid(1), oid(3)],
            vec![oid(1), oid(2)],
        ]) {
            assert_eq!(f.kind, MistakeRadarKind::DuplicateCommit);
            assert_eq!(f.severity, Severity::Info);
            assert_eq!(
                f.evidence,
                Evidence::Duplicate {
                    copies: expected_copies
                }
            );
        }
        let mut off = Policy::default();
        off.set(MistakeRadarKind::DuplicateCommit, None);
        assert!(findings_from_duplicates(&d, &off).is_empty());
    }

    #[test]
    fn evil_only_when_evil() {
        let files = vec![b"evil.txt".to_vec()];
        let f = finding_from_evil(
            oid(1),
            &EvilVerdict::Evil {
                files: files.clone(),
            },
            &Policy::default(),
        )
        .expect("evil");
        assert_eq!(f.kind, MistakeRadarKind::EvilMerge);
        assert_eq!(f.severity, Severity::Warning);
        assert_eq!(f.at, oid(1));
        assert_eq!(f.evidence, Evidence::Evil { files });
        assert_eq!(
            finding_from_evil(oid(1), &EvilVerdict::Clean, &Policy::default()),
            None
        );
        assert_eq!(
            finding_from_evil(
                oid(1),
                &EvilVerdict::NotCheckable("git 2.35 lacks --remerge-diff".into()),
                &Policy::default()
            ),
            None
        );
    }

    #[test]
    fn force_push_sits_at_the_current_tip() {
        let v = ForcePushVerdict::Rewritten {
            remote_ref: "origin/main".into(),
            previous: oid(1),
            current: oid(2),
        };
        let f = findings_from_force_push(&v, &Policy::default()).expect("rewritten");
        assert_eq!(f.kind, MistakeRadarKind::ForcePushedRemote);
        assert_eq!(f.severity, Severity::Warning);
        assert_eq!(f.at, oid(2));
        assert_eq!(
            findings_from_force_push(&ForcePushVerdict::Clean, &Policy::default()),
            None
        );
    }
}
