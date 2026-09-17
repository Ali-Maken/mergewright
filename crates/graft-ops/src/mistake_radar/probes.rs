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
use graft_git::history::read_commit;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};

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

/// Is `merge`'s recorded resolution what merging its parents gives?
///
/// One `git show --remerge-diff --name-only` spawn: git re-merges the two
/// parents in memory and diffs that against what was committed. An empty
/// diff is a clean merge; every line is a file the merge changed beyond
/// combining its sides.
///
/// Never a `Result`: "could not check" is one of the three answers the pane
/// shows, not a failure to report elsewhere. Anything that stops the check —
/// an unreadable commit, an octopus merge, a git too old to know
/// `--remerge-diff` — comes back as `NotCheckable` with the reason, because
/// absence of evidence is not a clean bill (PROMPT.md §1.4).
pub fn probe_evil_merge(repo: &RepoHandle, merge: Oid, stop: &dyn Stop) -> EvilVerdict {
    let commit = match read_commit(repo, merge) {
        Ok(commit) => commit,
        Err(error) => return EvilVerdict::NotCheckable(error.to_string()),
    };
    match commit.parents.len() {
        // A single-parent commit has nothing to re-merge.
        0 | 1 => return EvilVerdict::NotCheckable("not a merge".into()),
        2 => {}
        // `--remerge-diff` re-merges two sides; git itself refuses more.
        _ => return EvilVerdict::NotCheckable("octopus merge".into()),
    }
    let output = GitInvocation::new(repo)
        .arg(GitArg::Word("show".into()))
        .arg(GitArg::Flag("--remerge-diff"))
        .arg(GitArg::Flag("--format="))
        .arg(GitArg::Flag("--name-only"))
        .arg(GitArg::Oid(merge))
        .run(stop);
    match output {
        Ok(output) => {
            let files: Vec<Vec<u8>> = output
                .stdout
                .split(|byte| *byte == b'\n')
                .filter(|line| !line.is_empty())
                .map(<[u8]>::to_vec)
                .collect();
            if files.is_empty() {
                EvilVerdict::Clean
            } else {
                EvilVerdict::Evil { files }
            }
        }
        // Only reached when the probe is called outside a job: the job
        // runner drops a cancelled job's product before `update` sees it.
        Err(RunError::Stopped) => EvilVerdict::NotCheckable("cancelled".into()),
        Err(error) => EvilVerdict::NotCheckable(error.to_string()),
    }
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

    /// A fixture's checked-out tip (the merge, in every merge shape) and that
    /// tip's first parent (a plain commit).
    fn tip_and_parent(shape: graft_fixtures::Shape) -> (RepoHandle, Oid, Oid) {
        let repo = RepoHandle::open(&graft_fixtures::borrow(shape)).expect("the fixture opens");
        let graft_git::repo::HeadState::Branch(branch) =
            repo.head().expect("the fixture's HEAD reads")
        else {
            panic!("every merge fixture is checked out on a branch");
        };
        let peeled = repo
            .peeled_references(&())
            .expect("the fixture's refs read");
        let tip = peeled
            .iter()
            .find(|(name, _)| *name == branch)
            .map(|&(_, id)| id)
            .expect("the checked-out branch has a tip");
        let merge = read_commit(&repo, tip).expect("the tip reads");
        let parent = *merge.parents.first().expect("the tip has a parent");
        (repo, tip, parent)
    }

    #[test]
    fn the_evil_fixture_names_the_file_the_merge_invented() {
        let (repo, merge, parent) = tip_and_parent(graft_fixtures::Shape::EvilMerge);
        let EvilVerdict::Evil { files } = probe_evil_merge(&repo, merge, &()) else {
            panic!("the evil fixture's merge is evil");
        };
        assert_eq!(files, vec![b"evil.txt".to_vec()]);
        assert_eq!(
            probe_evil_merge(&repo, parent, &()),
            EvilVerdict::NotCheckable("not a merge".into())
        );
    }

    #[test]
    fn a_merge_that_only_combines_its_sides_is_clean() {
        let (repo, merge, _) = tip_and_parent(graft_fixtures::Shape::FoxtrotMerge);
        assert_eq!(probe_evil_merge(&repo, merge, &()), EvilVerdict::Clean);
    }

    #[test]
    fn an_octopus_merge_is_not_checkable() {
        let (repo, merge, _) = tip_and_parent(graft_fixtures::Shape::OctopusMerge);
        assert_eq!(
            probe_evil_merge(&repo, merge, &()),
            EvilVerdict::NotCheckable("octopus merge".into())
        );
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
