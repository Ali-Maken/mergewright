//! The verdicts the three lazy probes return, and how each becomes a
//! finding. The probes themselves (a `git` process each: `show
//! --remerge-diff`, the remote-tracking reflog, `patch-id` over a window)
//! are built by the `radar-probe-*` tickets; this module is the contract
//! they fill.
//!
//! "Not checkable" never becomes a finding: absence of evidence is not a
//! clean bill (PROMPT.md §1.4). The pane says "not checkable in this repo"
//! from the verdict itself.

use std::collections::{HashMap, HashSet};

use graft_core::mistake_radar::MistakeRadarKind;
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::history::{previous_tips, read_commit};
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

/// Did `remote_ref` move to `current` without descending from where the
/// reflog last saw it?
///
/// Two reads: the remote-tracking ref's reflog for where it pointed before,
/// then one `git merge-base --is-ancestor <previous> <current>` spawn whose
/// exit code is the whole answer — 0 means the new tip descends from the old
/// one (an ordinary fetch), 1 means it does not (the remote was rewritten).
/// Any other exit, or no reflog at all, is `NotCheckable`: a fresh clone has
/// nothing to compare against, and absence of evidence is not a clean bill
/// (PROMPT.md §1.4).
///
/// `current` is passed in rather than read here so the verdict is about the
/// same tip the caller classified; re-reading the ref could answer about a
/// tip that landed after.
pub fn probe_force_push(
    repo: &RepoHandle,
    _branch: &str,
    remote_ref: &RefName,
    current: Oid,
    stop: &dyn Stop,
) -> ForcePushVerdict {
    let tips = match previous_tips(repo, remote_ref, 1) {
        Ok(tips) => tips,
        Err(error) => return ForcePushVerdict::NotCheckable(error.to_string()),
    };
    let Some(previous) = tips.first().copied() else {
        return ForcePushVerdict::NotCheckable("no remote-tracking reflog".into());
    };
    let output = GitInvocation::new(repo)
        .arg(GitArg::Word("merge-base".into()))
        .arg(GitArg::Flag("--is-ancestor"))
        .arg(GitArg::Oid(previous))
        .arg(GitArg::Oid(current))
        .run(stop);
    match output {
        // Exit 0: the old tip is an ancestor of the new one, so the remote
        // only moved forward.
        Ok(_) => ForcePushVerdict::Clean,
        // Exit 1 is `merge-base --is-ancestor`'s "no", not a failure.
        Err(RunError::Exited { status: 1, .. }) => ForcePushVerdict::Rewritten {
            remote_ref: short_remote_ref(remote_ref),
            previous,
            current,
        },
        // Only reached when the probe is called outside a job: the job
        // runner drops a cancelled job's product before `update` sees it.
        Err(RunError::Stopped) => ForcePushVerdict::NotCheckable("cancelled".into()),
        Err(error) => ForcePushVerdict::NotCheckable(error.to_string()),
    }
}

/// `refs/remotes/origin/main` as `origin/main`, which is what the pane and
/// the explanation show. Anything else is left as it is.
fn short_remote_ref(remote_ref: &RefName) -> String {
    let text = String::from_utf8_lossy(remote_ref.as_bytes());
    match text.strip_prefix("refs/remotes/") {
        Some(short) => short.to_owned(),
        None => text.into_owned(),
    }
}

/// A hex line's `Oid`, tolerant of a trailing newline; anything else is
/// dropped by the caller rather than failing the whole probe.
fn parse_oid(bytes: &[u8]) -> Option<Oid> {
    Oid::from_hex(std::str::from_utf8(bytes).ok()?.trim()).ok()
}

/// Commits sharing a patch-id, among the newest `window` commits reachable
/// from any ref.
///
/// Three `git` spawns: `log --format=%H` names the window itself (and
/// whether it covers the whole history, from whether fewer than `window`
/// commits came back); `log -p` gives their patches; `patch-id --stable`
/// reads those patches from `stdin` and folds each to a stable id, printing
/// `"<patch-id> <sha>"` per line. Grouping those lines by patch-id and
/// dropping groups of one is what makes a duplicate: two commits with no
/// ancestry between them that made the same change. A merge's patch is
/// empty and never appears in `patch-id`'s output — expected, not an error,
/// since an empty patch cannot repeat with anything.
///
/// Groups, and the commits within them, are ordered by `log`'s own order
/// (newest first) for determinism.
///
/// # Errors
/// Any of the three spawns failing.
pub fn probe_duplicates(
    repo: &RepoHandle,
    window: u32,
    stop: &dyn Stop,
) -> Result<Duplicates, RunError> {
    let log_output = GitInvocation::new(repo)
        .arg(GitArg::Word("log".into()))
        .arg(GitArg::Flag("--all"))
        .arg(GitArg::Count("-n", window))
        .arg(GitArg::Flag("--format=%H"))
        .run(stop)?;
    let oids: Vec<Oid> = log_output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .filter_map(parse_oid)
        .collect();
    let complete = oids.len() < window as usize;

    let patches = GitInvocation::new(repo)
        .arg(GitArg::Word("log".into()))
        .arg(GitArg::Flag("--all"))
        .arg(GitArg::Count("-n", window))
        .arg(GitArg::Flag("-p"))
        .run(stop)?
        .stdout;

    let ids_output = GitInvocation::new(repo)
        .arg(GitArg::Word("patch-id".into()))
        .arg(GitArg::Flag("--stable"))
        .stdin(patches)
        .run(stop)?;

    // Only oids from the window count: `patch-id` can print entries for
    // objects outside it in principle, and this keeps the probe answering
    // strictly about the window it says it looked at.
    let known: HashSet<Oid> = oids.iter().copied().collect();
    let mut by_patch_id: HashMap<&str, Vec<Oid>> = HashMap::new();
    let ids_text = String::from_utf8_lossy(&ids_output.stdout);
    for line in ids_text.lines() {
        let mut parts = line.split_whitespace();
        let (Some(patch_id), Some(sha)) = (parts.next(), parts.next()) else {
            continue;
        };
        let Some(oid) = Oid::from_hex(sha).ok().filter(|oid| known.contains(oid)) else {
            continue;
        };
        by_patch_id.entry(patch_id).or_default().push(oid);
    }

    let position: HashMap<Oid, usize> = oids.iter().enumerate().map(|(i, &oid)| (oid, i)).collect();
    let mut groups: Vec<Vec<Oid>> = by_patch_id
        .into_values()
        .filter(|group| group.len() >= 2)
        .map(|mut group| {
            group.sort_by_key(|oid| position[oid]);
            group
        })
        .collect();
    groups.sort_by_key(|group| group.iter().map(|oid| position[oid]).min());

    Ok(Duplicates {
        groups,
        window,
        complete,
    })
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

    /// A writable copy of the trivial fixture with its remote-tracking
    /// reflog turned on, plus two commits: an older one and the tip.
    fn tracked() -> (graft_fixtures::Scratch, RepoHandle, Oid, Oid) {
        let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
        graft_fixtures::git::run(scratch.path(), &["config", "core.logAllRefUpdates", "true"])
            .expect("the copy takes config");
        let older = rev_parse(&scratch, "HEAD~2");
        let tip = rev_parse(&scratch, "HEAD");
        let repo = RepoHandle::open(scratch.path()).expect("the copy opens");
        (scratch, repo, older, tip)
    }

    fn rev_parse(scratch: &graft_fixtures::Scratch, rev: &str) -> Oid {
        let out = graft_fixtures::git::run(scratch.path(), &["rev-parse", rev])
            .expect("the fixture resolves the revision");
        Oid::from_hex(String::from_utf8_lossy(&out).trim()).expect("git printed hex")
    }

    fn move_ref(scratch: &graft_fixtures::Scratch, refname: &str, to: Oid) {
        graft_fixtures::git::run(scratch.path(), &["update-ref", refname, &to.to_string()])
            .expect("the ref moves");
    }

    fn origin_main() -> RefName {
        RefName::new("refs/remotes/origin/main").expect("a valid ref name")
    }

    #[test]
    fn a_remote_ref_that_moved_off_its_old_tip_was_rewritten() {
        let (scratch, repo, older, tip) = tracked();
        // The remote was at the tip, then came back holding an older commit:
        // the new tip does not descend from the old one.
        move_ref(&scratch, "refs/remotes/origin/main", tip);
        move_ref(&scratch, "refs/remotes/origin/main", older);

        assert_eq!(
            probe_force_push(&repo, "main", &origin_main(), older, &()),
            ForcePushVerdict::Rewritten {
                remote_ref: "origin/main".into(),
                previous: tip,
                current: older,
            }
        );
    }

    #[test]
    fn a_remote_ref_that_only_moved_forward_is_clean() {
        let (scratch, repo, older, tip) = tracked();
        move_ref(&scratch, "refs/remotes/origin/main", older);
        move_ref(&scratch, "refs/remotes/origin/main", tip);

        assert_eq!(
            probe_force_push(&repo, "main", &origin_main(), tip, &()),
            ForcePushVerdict::Clean
        );
    }

    #[test]
    fn the_cherry_picked_duplicate_is_found_in_its_own_group() {
        let repo = RepoHandle::open(&graft_fixtures::borrow(
            graft_fixtures::Shape::CherryPickDuplicates,
        ))
        .expect("the fixture opens");
        let d = probe_duplicates(&repo, 500, &()).expect("the probe runs");
        assert_eq!(d.window, 500);
        assert!(d.complete, "4 commits is well under the window");
        assert_eq!(
            d.groups.len(),
            1,
            "one pair shares a patch-id: {:?}",
            d.groups
        );
        assert_eq!(d.groups[0].len(), 2);
    }

    #[test]
    fn a_fixture_with_no_repeated_content_has_no_groups() {
        let repo = RepoHandle::open(&graft_fixtures::borrow(graft_fixtures::Shape::Trivial))
            .expect("the fixture opens");
        let d = probe_duplicates(&repo, 500, &()).expect("the probe runs");
        assert_eq!(d.groups, Vec::<Vec<Oid>>::new());
        assert!(d.complete);
    }

    #[test]
    fn a_remote_ref_with_only_its_creation_entry_is_not_checkable() {
        let (scratch, repo, _older, tip) = tracked();
        move_ref(&scratch, "refs/remotes/origin/fresh", tip);
        let fresh = RefName::new("refs/remotes/origin/fresh").expect("a valid ref name");

        assert_eq!(
            probe_force_push(&repo, "fresh", &fresh, tip, &()),
            ForcePushVerdict::NotCheckable("no remote-tracking reflog".into())
        );
    }
}
