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
    /// The merge's tree differs from a clean re-merge, but only in files
    /// both sides changed since the merge base — where git could have
    /// raised a conflict. That is what an ordinary hand-resolved conflict
    /// looks like, so it is not a finding.
    Resolved {
        /// The files that differ from the clean re-merge and that both
        /// sides changed, as paths in the repository.
        files: Vec<Vec<u8>>,
    },
    /// The merge changed files that only one side, or neither, had touched
    /// since the merge base: no conflict could have arisen there, so the
    /// edit came from the merge itself.
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
/// A non-empty remerge diff is **not** the same as an evil merge: it is
/// also what every hand-resolved conflict leaves behind, which is ordinary
/// work on any busy repository. So the differing files are split by a test
/// git semantics make exact in one direction — a conflict can only arise in
/// a file *both* sides changed since the merge base. Files only one side
/// (or neither) touched could not have conflicted, so a difference there
/// came from the merge itself: [`EvilVerdict::Evil`]. The rest is
/// [`EvilVerdict::Resolved`], which is never a finding.
///
/// What this cannot tell apart, and deliberately stays quiet about: a file
/// both sides changed. A real evil edit hidden inside a conflict
/// resolution, and a merge that re-resolved cleanly-mergeable both-side
/// edits, both land in `Resolved`. Warning on those would fire on ordinary
/// work, which is how a real warning gets ignored; `explain` says so in
/// the pane rather than the check guessing.
///
/// Splitting costs three more spawns (`merge-base`, then one `diff-tree`
/// per side) and only for a merge whose remerge diff is non-empty; a clean
/// merge still costs the one spawn.
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
                split_remerge_files(repo, [commit.parents[0], commit.parents[1]], files, stop)
            }
        }
        // Only reached when the probe is called outside a job: the job
        // runner drops a cancelled job's product before `update` sees it.
        Err(RunError::Stopped) => EvilVerdict::NotCheckable("cancelled".into()),
        Err(error) => EvilVerdict::NotCheckable(error.to_string()),
    }
}

/// Splits the files a remerge diff named into the ones a conflict explains
/// and the ones it cannot. See [`probe_evil_merge`] for why the merge base
/// is the dividing line.
///
/// Without a merge base there is nothing to measure "both sides changed it"
/// against, so the answer is `NotCheckable` with the reason, never a guess
/// either way: absence of evidence is not a clean bill, and it is not a
/// warning either (PROMPT.md §1.4).
///
/// **Renames cross the dividing line.** A remerge diff names the path the
/// merge produced; when one side renamed the file, that path is not the one
/// the other side changed, and a hand-resolved rename/modify conflict looked
/// evil. So each side is read with rename detection on, and a rename
/// `old -> new` carries the verdict across: if `old` was changed by both
/// sides, `new` is too. **Only a rename** — a copy is a new file, and letting
/// one carry the verdict would exempt every file copied from a contested one.
/// The cost is the blind spot widening the same way —
/// an evil edit to the post-rename path lands in `Resolved` — which is the
/// direction the probe already errs in for every file both sides touched.
fn split_remerge_files(
    repo: &RepoHandle,
    parents: [Oid; 2],
    files: Vec<Vec<u8>>,
    stop: &dyn Stop,
) -> EvilVerdict {
    let base = match merge_base(repo, parents, stop) {
        Ok(Some(base)) => base,
        Ok(None) => return EvilVerdict::NotCheckable("the parents share no history".into()),
        Err(reason) => return EvilVerdict::NotCheckable(reason),
    };
    let ours = match changed_paths(repo, base, parents[0], stop) {
        Ok(paths) => paths,
        Err(reason) => return EvilVerdict::NotCheckable(reason),
    };
    let theirs = match changed_paths(repo, base, parents[1], stop) {
        Ok(paths) => paths,
        Err(reason) => return EvilVerdict::NotCheckable(reason),
    };
    let mut both: HashSet<Vec<u8>> = ours
        .changed
        .intersection(&theirs.changed)
        .cloned()
        .collect();
    for (old, new) in ours.renames.iter().chain(theirs.renames.iter()) {
        if both.contains(old) {
            both.insert(new.clone());
        }
    }

    let (resolved, invented): (Vec<Vec<u8>>, Vec<Vec<u8>>) =
        files.into_iter().partition(|path| both.contains(path));
    if invented.is_empty() {
        EvilVerdict::Resolved { files: resolved }
    } else {
        EvilVerdict::Evil { files: invented }
    }
}

/// The best common ancestor of two parents, or `None` when they share no
/// history (`merge-base` exits 1 and prints nothing).
fn merge_base(
    repo: &RepoHandle,
    parents: [Oid; 2],
    stop: &dyn Stop,
) -> Result<Option<Oid>, String> {
    let output = GitInvocation::new(repo)
        .arg(GitArg::Word("merge-base".into()))
        .arg(GitArg::Oid(parents[0]))
        .arg(GitArg::Oid(parents[1]))
        .run(stop);
    let output = match output {
        Ok(output) => output,
        Err(RunError::Exited { status: 1, .. }) => return Ok(None),
        Err(RunError::Stopped) => return Err("cancelled".into()),
        Err(error) => return Err(error.to_string()),
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let Some(line) = text.lines().next() else {
        return Ok(None);
    };
    Oid::from_hex(line.trim())
        .map(Some)
        .map_err(|error| error.to_string())
}

/// What one side of a merge did to the tree since the merge base.
struct SideChanges {
    /// Every path the side touched, both ends of a **rename** included, so a
    /// path is present under whichever name the other side knows it by. A
    /// copy's source is not one of them: copying a file does not change it.
    changed: HashSet<Vec<u8>>,
    /// `(old, new)` for each **rename**, so [`split_remerge_files`] can carry
    /// "both sides changed this" from the old name to the new one. Copies are
    /// never listed here — see [`changed_paths`].
    renames: Vec<(Vec<u8>, Vec<u8>)>,
}

/// The paths that differ between two trees, with rename detection **on**:
/// `--name-status -z` prints `R090\0old\0new\0` for a rename, and both names
/// go into `changed`. Detection off (the earlier shape) named both ends too,
/// but lost which was which, and the pairing is what makes a rename conflict
/// explainable.
///
/// **Copies are not renames.** `-C` is not passed, but `diff.renames = copies`
/// in a repository's own config turns copy detection on regardless, so `C`
/// arrives here and is read as what it is: an *added* file whose content came
/// from somewhere. Only its new path counts as touched, and it never becomes a
/// rename pair. Treating a copy as a rename made every file copied from a file
/// both sides had changed immune to the probe — git/git's `c1ab07b8` copied a
/// contested `README.md` and its evil edit went unreported.
fn changed_paths(
    repo: &RepoHandle,
    from: Oid,
    to: Oid,
    stop: &dyn Stop,
) -> Result<SideChanges, String> {
    let output = GitInvocation::new(repo)
        .arg(GitArg::Word("diff-tree".into()))
        .arg(GitArg::Flag("-r"))
        .arg(GitArg::Flag("-M"))
        .arg(GitArg::Flag("--name-status"))
        .arg(GitArg::Flag("-z"))
        .arg(GitArg::Oid(from))
        .arg(GitArg::Oid(to))
        .run(stop)
        .map_err(|error| match error {
            RunError::Stopped => "cancelled".to_owned(),
            other => other.to_string(),
        })?;

    let tokens: Vec<&[u8]> = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|token| !token.is_empty())
        .collect();
    let mut changed = HashSet::new();
    let mut renames = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let status = tokens[i];
        i += 1;
        let two_ends = matches!(status.first(), Some(b'R' | b'C'));
        let wanted = if two_ends { 2 } else { 1 };
        if i + wanted > tokens.len() {
            return Err("diff-tree named a status with no path after it".to_owned());
        }
        match status.first() {
            // A rename is one file under a new name: the side touched both
            // ends, and the pair carries the verdict across in the caller.
            Some(b'R') => {
                let (old, new) = (tokens[i].to_vec(), tokens[i + 1].to_vec());
                changed.insert(old.clone());
                changed.insert(new.clone());
                renames.push((old, new));
            }
            // A copy is a *new* file. The side touched only the copy; the
            // source is named because that is where the content came from,
            // and it is in `changed` already if the side really changed it.
            // Never a rename pair: see `changed_paths`.
            Some(b'C') => {
                changed.insert(tokens[i + 1].to_vec());
            }
            _ => {
                changed.insert(tokens[i].to_vec());
            }
        }
        i += wanted;
    }
    Ok(SideChanges { changed, renames })
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

/// What the duplicate probe walks: every ref the user can merge (PROMPT.md
/// §1.4, "only across visible refs"). An allowlist, not `--all`: `--all`
/// also walks Graft's own `refs/graft/**` backups (every pre-rebase commit,
/// kept for `u`), `refs/stash` (whose base is the pre-rebase tip) and other
/// tools' bookkeeping such as `refs/prefetch/**` — so every rebased commit's
/// old self read as a copy. A copy reachable only from some other namespace
/// (e.g. `refs/pull/*`) is not flagged; it cannot be merged by accident.
///
/// Revisions, not flags: the walk ends with `--`, so a tracked file named
/// `HEAD` at the root cannot make the `HEAD` revision ambiguous (git exits
/// 128 otherwise). It therefore goes last on the command line.
pub(crate) fn duplicate_walk() -> [GitArg; 5] {
    [
        GitArg::Flag("--branches"),
        GitArg::Flag("--remotes"),
        GitArg::Flag("--tags"),
        GitArg::Ref(b"HEAD".to_vec()),
        GitArg::Separator,
    ]
}

/// Commits sharing a patch-id, among the newest `window` commits reachable
/// from a ref the user can merge ([`duplicate_walk`]).
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
        .arg(GitArg::Count("-n", window))
        .arg(GitArg::Flag("--format=%H"))
        .args(duplicate_walk())
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
        .arg(GitArg::Count("-n", window))
        .arg(GitArg::Flag("-p"))
        .args(duplicate_walk())
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
        // `Resolved` is a hand-resolved conflict: ordinary work, never a
        // finding.
        EvilVerdict::Clean | EvilVerdict::Resolved { .. } | EvilVerdict::NotCheckable(_) => None,
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

    /// A scratch repository with one new commit A on `main` (file `a.txt`).
    fn scratch_with_a() -> (graft_fixtures::Scratch, Oid) {
        let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
        let path = scratch.path().to_path_buf();
        let git = |args: &[&str]| {
            graft_fixtures::git::run(&path, args)
                .unwrap_or_else(|error| panic!("git {args:?} runs: {error}"))
        };
        git(&["config", "user.name", "Fixture"]);
        git(&["config", "user.email", "fixture@example.invalid"]);
        std::fs::write(path.join("a.txt"), "a\n").expect("A's file is written");
        git(&["add", "a.txt"]);
        git(&["commit", "-m", "A"]);
        let a = rev_parse(&scratch, "HEAD");
        (scratch, a)
    }

    /// The shape a rebase leaves: the current branch reset under A, a new
    /// commit B, and A's patch replayed on top. Returns A′.
    fn rebase_a_away(scratch: &graft_fixtures::Scratch, a: Oid) -> Oid {
        let path = scratch.path();
        let git = |args: &[&str]| {
            graft_fixtures::git::run(path, args)
                .unwrap_or_else(|error| panic!("git {args:?} runs: {error}"))
        };
        git(&["reset", "--hard", "HEAD~1"]);
        std::fs::write(path.join("b.txt"), "b\n").expect("B's file is written");
        git(&["add", "b.txt"]);
        git(&["commit", "-m", "B"]);
        git(&["cherry-pick", &a.to_string()]);
        rev_parse(scratch, "HEAD")
    }

    fn groups_holding(repo: &RepoHandle, id: Oid) -> Vec<Vec<Oid>> {
        probe_duplicates(repo, 500, &())
            .expect("the probe runs")
            .groups
            .into_iter()
            .filter(|group| group.contains(&id))
            .collect()
    }

    #[test]
    fn duplicates_skip_graft_backup_refs() {
        let (scratch, a) = scratch_with_a();
        move_ref(
            &scratch,
            "refs/graft/backup/main/01J0000000000000000000000",
            a,
        );
        let a2 = rebase_a_away(&scratch, a);
        let repo = RepoHandle::open(scratch.path()).expect("the scratch repository opens");
        assert_eq!(groups_holding(&repo, a2), Vec::<Vec<Oid>>::new());
    }

    #[test]
    fn duplicates_skip_the_stash() {
        let (scratch, a) = scratch_with_a();
        std::fs::write(scratch.path().join("a.txt"), "dirty\n").expect("a dirty edit");
        graft_fixtures::git::run(scratch.path(), &["stash"]).expect("the stash is made");
        let a2 = rebase_a_away(&scratch, a);
        let repo = RepoHandle::open(scratch.path()).expect("the scratch repository opens");
        assert_eq!(groups_holding(&repo, a2), Vec::<Vec<Oid>>::new());
    }

    #[test]
    fn duplicates_skip_other_bookkeeping_refs() {
        let (scratch, a) = scratch_with_a();
        move_ref(&scratch, "refs/prefetch/remotes/origin/main", a);
        let a2 = rebase_a_away(&scratch, a);
        let repo = RepoHandle::open(scratch.path()).expect("the scratch repository opens");
        assert_eq!(groups_holding(&repo, a2), Vec::<Vec<Oid>>::new());
    }

    /// The control: A kept on `feature`, its copy on `main` — both refs the
    /// user can merge, so the pair is still one finding.
    #[test]
    fn duplicates_across_two_branches_are_still_found() {
        let (scratch, a) = scratch_with_a();
        move_ref(&scratch, "refs/heads/feature", a);
        let a2 = rebase_a_away(&scratch, a);
        let repo = RepoHandle::open(scratch.path()).expect("the scratch repository opens");
        let mut groups = groups_holding(&repo, a2);
        // Both commits share a timestamp second, so `log`'s order between
        // them is not fixed; the pair is what matters.
        for group in &mut groups {
            group.sort();
        }
        let mut want = vec![a, a2];
        want.sort();
        assert_eq!(groups, vec![want]);
    }

    /// A tracked file named `HEAD` at the root makes a bare `HEAD` revision
    /// ambiguous (git exits 128) unless the revisions end with `--`.
    #[test]
    fn duplicates_are_found_beside_a_root_file_named_head() {
        let (scratch, a) = scratch_with_a();
        move_ref(&scratch, "refs/heads/feature", a);
        let a2 = rebase_a_away(&scratch, a);
        std::fs::write(scratch.path().join("HEAD"), "a file, not a ref\n")
            .expect("the HEAD file is written");
        graft_fixtures::git::run(scratch.path(), &["add", "HEAD"]).expect("HEAD is staged");
        graft_fixtures::git::run(scratch.path(), &["commit", "-m", "a file named HEAD"])
            .expect("HEAD is committed");
        let repo = RepoHandle::open(scratch.path()).expect("the scratch repository opens");
        assert_eq!(groups_holding(&repo, a2).len(), 1);
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

    /// The common case on any busy repository: a merge whose conflicts were
    /// resolved by hand. Its remerge diff is non-empty — which is what the
    /// probe used to call evil — but both sides changed the file, so a
    /// conflict explains the difference and nothing is flagged.
    #[test]
    fn a_hand_resolved_conflict_is_not_an_evil_merge() {
        let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
        let path = scratch.path().to_path_buf();
        let shared = path.join("shared.txt");
        let git = |args: &[&str]| {
            graft_fixtures::git::run(&path, args)
                .unwrap_or_else(|error| panic!("git {args:?} runs: {error}"))
        };
        git(&["config", "user.name", "Fixture"]);
        git(&["config", "user.email", "fixture@example.invalid"]);
        std::fs::write(&shared, "0\n").expect("the shared file is written");
        git(&["add", "shared.txt"]);
        git(&["commit", "-m", "Add the shared file"]);
        git(&["checkout", "-b", "side"]);
        std::fs::write(&shared, "theirs\n").expect("the side's edit is written");
        git(&["commit", "-am", "Their edit"]);
        git(&["checkout", "-"]);
        std::fs::write(&shared, "mine\n").expect("our edit is written");
        git(&["commit", "-am", "My edit"]);

        assert!(
            graft_fixtures::git::run(&path, &["merge", "side"]).is_err(),
            "the merge must conflict for this test to prove anything"
        );
        std::fs::write(&shared, "resolved by hand\n").expect("the resolution is written");
        git(&["add", "shared.txt"]);
        git(&["commit", "-m", "Merge branch 'side'"]);

        let head = String::from_utf8(git(&["rev-parse", "HEAD"])).expect("git prints hex");
        let merge = Oid::from_hex(head.trim()).expect("a full object id");
        let repo = RepoHandle::open(&path).expect("the scratch repository opens");

        assert_eq!(
            probe_evil_merge(&repo, merge, &()),
            EvilVerdict::Resolved {
                files: vec![b"shared.txt".to_vec()]
            }
        );
        assert_eq!(
            finding_from_evil(
                merge,
                &EvilVerdict::Resolved {
                    files: vec![b"shared.txt".to_vec()]
                },
                &Policy::default()
            ),
            None
        );
    }

    /// The same shape one step harder: one side renames the file *and*
    /// edits it, the other edits it where it was. git conflicts in the new
    /// name; the remerge diff names the new name; only the old name is in
    /// both sides' changed sets. Counting a rename's two ends as one file
    /// is what keeps this off the radar — before it, an ordinary merge
    /// after a file move said `EVIL MERGE`.
    #[test]
    fn a_hand_resolved_rename_conflict_is_not_an_evil_merge() {
        let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
        let path = scratch.path().to_path_buf();
        let git = |args: &[&str]| {
            graft_fixtures::git::run(&path, args)
                .unwrap_or_else(|error| panic!("git {args:?} runs: {error}"))
        };
        // Long enough that git's default 50% similarity finds the rename.
        let body = |first: &str| {
            let mut text = String::from(first);
            for line in 2..=40 {
                text.push_str(&format!("\nline {line}"));
            }
            text.push('\n');
            text
        };
        git(&["config", "user.name", "Fixture"]);
        git(&["config", "user.email", "fixture@example.invalid"]);
        std::fs::write(path.join("moved.txt"), body("line 1")).expect("the base file is written");
        git(&["add", "moved.txt"]);
        git(&["commit", "-m", "Add the file that moves"]);

        git(&["checkout", "-b", "side"]);
        git(&["mv", "moved.txt", "renamed.txt"]);
        std::fs::write(path.join("renamed.txt"), body("their first line"))
            .expect("the side's edit is written");
        git(&["add", "renamed.txt"]);
        git(&["commit", "-m", "Move it and edit line 1"]);
        git(&["checkout", "-"]);
        std::fs::write(path.join("moved.txt"), body("my first line")).expect("our edit is written");
        git(&["commit", "-am", "Edit line 1 where it was"]);

        assert!(
            graft_fixtures::git::run(&path, &["merge", "side"]).is_err(),
            "the merge must conflict for this test to prove anything"
        );
        std::fs::write(path.join("renamed.txt"), body("resolved by hand"))
            .expect("the resolution is written");
        git(&["add", "-A"]);
        git(&["commit", "-m", "Merge branch 'side'"]);

        let head = String::from_utf8(git(&["rev-parse", "HEAD"])).expect("git prints hex");
        let merge = Oid::from_hex(head.trim()).expect("a full object id");
        let repo = RepoHandle::open(&path).expect("the scratch repository opens");

        assert_eq!(
            probe_evil_merge(&repo, merge, &()),
            EvilVerdict::Resolved {
                files: vec![b"renamed.txt".to_vec()]
            },
            "the rename's two ends are one file, so the conflict explains it"
        );
    }

    /// A copy is not a rename. One side copies a file both sides are editing,
    /// then the merge quietly edits the copy — which only that one side ever
    /// touched, so it is exactly the evil edit the probe exists to catch.
    /// Carrying "both sides changed it" across a copy made every file copied
    /// from a contested file immune; this is the shape that caught it, taken
    /// from git/git's own `c1ab07b8` (`gitk-git/README.md`, a 100% copy of a
    /// `README.md` both sides had modified).
    #[test]
    fn an_evil_edit_to_a_copied_file_is_still_evil() {
        let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
        let path = scratch.path().to_path_buf();
        let git = |args: &[&str]| {
            graft_fixtures::git::run(&path, args)
                .unwrap_or_else(|error| panic!("git {args:?} runs: {error}"))
        };
        // Long enough that git's copy detection finds the copy.
        let body = |first: &str| {
            let mut text = String::from(first);
            for line in 2..=40 {
                text.push_str(&format!("\nline {line}"));
            }
            text.push('\n');
            text
        };
        git(&["config", "user.name", "Fixture"]);
        git(&["config", "user.email", "fixture@example.invalid"]);
        std::fs::write(path.join("readme.txt"), body("line 1")).expect("the base file is written");
        git(&["add", "readme.txt"]);
        git(&["commit", "-m", "Add the file that gets copied"]);

        git(&["checkout", "-b", "side"]);
        std::fs::create_dir(path.join("sub")).expect("the subdirectory is created");
        std::fs::write(path.join("sub/readme.txt"), body("line 1")).expect("the copy is written");
        std::fs::write(path.join("readme.txt"), body("their first line"))
            .expect("the side's edit is written");
        git(&["add", "-A"]);
        git(&["commit", "-m", "Copy it into sub/ and edit line 1"]);
        git(&["checkout", "-"]);
        std::fs::write(path.join("readme.txt"), body("my first line"))
            .expect("our edit is written");
        git(&["commit", "-am", "Edit line 1 where it was"]);

        assert!(
            graft_fixtures::git::run(&path, &["merge", "side"]).is_err(),
            "the merge must conflict for this test to prove anything"
        );
        std::fs::write(path.join("readme.txt"), body("resolved by hand"))
            .expect("the resolution is written");
        // The evil edit: a file only `side` ever touched, changed in the merge.
        std::fs::write(path.join("sub/readme.txt"), body("smuggled into the merge"))
            .expect("the evil edit is written");
        git(&["add", "-A"]);
        git(&["commit", "-m", "Merge branch 'side'"]);

        let head = String::from_utf8(git(&["rev-parse", "HEAD"])).expect("git prints hex");
        let merge = Oid::from_hex(head.trim()).expect("a full object id");
        let repo = RepoHandle::open(&path).expect("the scratch repository opens");

        assert_eq!(
            probe_evil_merge(&repo, merge, &()),
            EvilVerdict::Evil {
                files: vec![b"sub/readme.txt".to_vec()]
            },
            "only `side` touched the copy, so the merge's edit to it is unexplained"
        );
    }
}
