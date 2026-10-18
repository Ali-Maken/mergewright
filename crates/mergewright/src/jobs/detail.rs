//! The detail pane's jobs: the changed-file list, one file's diff, and which
//! refs contain a commit.
//!
//! Every git read runs here, off the loop's thread, and so does the view-model
//! assembly: sanitising, pairing, word diffs and syntax highlighting. `update`
//! receives one finished `Arc` and never touches git bytes.

use std::sync::Arc;

use mergewright_core::oid::Oid;
use mergewright_git::contained::{Contained, contained_in};
use mergewright_git::diff::{
    DiffOptions, DiffSpec, FileDiff, FileEntry, FileList, FileStatus, file_diff, list_files,
};
use mergewright_git::history_reads::{CompareSpan, compare_span};
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{RunError, Stop};
use mergewright_git::status::WorkingTreeStatus;
use mergewright_term::sanitize::sanitize_line;
use mergewright_ui::detail::{
    CompareSummary, DiffView, FileListView, FileStatusView, SubmoduleCommitView, SubmoduleLogView,
    SubmoduleView,
};

use super::Cancel;
use crate::repos::RepoStore;
use crate::update::detail::DiffKey;

mod assemble;
mod blame;
mod history;
mod working_tree;

pub use assemble::{assemble_diff, assemble_files};
pub(super) use blame::read_blame;
pub(super) use history::{read_trace, read_whole_file};
#[cfg(test)]
pub use working_tree::assemble_working_tree;
use working_tree::{SubmoduleRows, assemble_changes, pointer_of, shapes_of, short_id};
pub use working_tree::{UnmergedShapes, diff_spec_for};

impl Stop for Cancel {
    fn should_stop(&self) -> bool {
        self.is_cancelled()
    }
}

/// List a spec's changed files, with the commit body for a single commit.
///
/// The changes pane is the one spec read twice: `Staged` (index against
/// `HEAD`) and `Unstaged` (working tree against the index, untracked files
/// included) are what the three sections *are*, and one
/// `WorkingTreeVsHead` read could tell neither apart nor list an untracked
/// file at all.
pub(super) fn read_files(
    repos: &RepoStore,
    spec: DiffSpec,
    options: DiffOptions,
    stop: &Cancel,
) -> Result<(Arc<FileListView>, Arc<FileList>), String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    if spec == DiffSpec::WorkingTreeVsHead {
        let staged =
            list_files(&handle, &DiffSpec::Staged, &options, stop).map_err(|e| e.to_string())?;
        let mut unstaged =
            list_files(&handle, &DiffSpec::Unstaged, &options, stop).map_err(|e| e.to_string())?;
        // The shape of each conflict ("both modified", "deleted by
        // them") and what git says of each submodule are the status' to
        // say, not the diff's, so the changes pane's own read fetches it
        // alongside the two diffs. A status that cannot be read costs the
        // CONFLICTS rows their wording and the submodule rows their facts
        // (they draw as the diff's plain letter) and nothing else, so it is
        // not this job's failure.
        let (how, submodules) = match mergewright_git::status::read(&handle, stop) {
            Ok(status) => (
                shapes_of(&status),
                submodule_rows(&handle, &status, &staged, &mut unstaged, options, stop)?,
            ),
            Err(_) => (UnmergedShapes::new(), SubmoduleRows::default()),
        };
        let (view, list) = assemble_changes(&staged, &unstaged, &how, &submodules);
        return Ok((Arc::new(view), Arc::new(list)));
    }
    // A compare (`x`, the only `Range`) lists from where the two split
    // (m7-14), and says how they relate for the header.
    let (spec, compare) = match spec {
        DiffSpec::Range { base, head } => {
            let (range, summary) = compare_range(&handle, base, head, stop)?;
            (range, Some(summary))
        }
        other => (other, None),
    };
    let list = list_files(&handle, &spec, &options, stop).map_err(|e| e.to_string())?;
    let message = match spec {
        DiffSpec::Commit { commit: id, .. } => Some(
            mergewright_git::history::read_commit(&handle, id)
                .map_err(|e| e.to_string())?
                .message,
        ),
        DiffSpec::Range { .. }
        | DiffSpec::WorkingTreeVsHead
        | DiffSpec::Staged
        | DiffSpec::Unstaged => None,
    };
    let mut view = assemble_files(&list, message.as_deref().map(body_of));
    view.compare = compare;
    Ok((Arc::new(view), Arc::new(list)))
}

/// Read and assemble one file's diff, handing back both the drawable view
/// model and the raw diff it was built from: `mergewright_git::diff::hunk_patch`
/// needs git's own bytes to build a stage or discard patch, and the view
/// model has been sanitised past the point of reversing.
pub(super) fn read_file_diff(
    repos: &RepoStore,
    key: &DiffKey,
    entry: &FileEntry,
    stop: &Cancel,
) -> Result<(Arc<DiffView>, Arc<FileDiff>), String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let spec = match diff_spec_for(key.spec, key.section) {
        // The same split the file list used: the oids are fixed, so
        // `merge-base` answers the same, at the cost of one more spawn per
        // file (m7-14; the span is not carried beside the list).
        DiffSpec::Range { base, head } => compare_range(&handle, base, head, stop)?.0,
        other => other,
    };
    let diff = file_diff(&handle, &spec, entry, &key.options, key.max_lines, stop)
        .map_err(|e| e.to_string())?;
    // Word spans are always worked out; whether they tint is the widget's call.
    let mut view = assemble_diff(&diff, true);
    // A submodule's diff pane lists commits, never the `Subproject commit`
    // patch (m7-power-tools/13). The raw diff stays as git gave it: a stage
    // of the path is still `git add` of the pointer.
    if let Some(submodule) = submodule_diff(&handle, &spec, &diff, stop)? {
        view.status = FileStatusView::Submodule(Box::new(submodule));
        view.hunks.clear();
    }
    Ok((Arc::new(view), Arc::new(diff)))
}

/// What the changes pane says of each submodule, and which have a row
/// (m7-power-tools/13). `unstaged` is edited to match git's status:
///
/// - a submodule git's status says nothing of in its worktree (no `C`, `M`
///   or `U`) loses its UNSTAGED row — "none true, no row";
/// - one with only untracked files inside gains one: git's worktree diff
///   is silent about it (measured, `diff-files` on `S..U`);
/// - one **not set up** gains one: git's status is silent about it too.
///
/// The ids come from each row's own `Subproject commit` patch — one small
/// diff per moved submodule, the same read its diff pane makes.
fn submodule_rows(
    handle: &RepoHandle,
    status: &WorkingTreeStatus,
    staged: &FileList,
    unstaged: &mut FileList,
    options: DiffOptions,
    stop: &Cancel,
) -> Result<SubmoduleRows, String> {
    use std::os::unix::ffi::OsStrExt;

    // The pointer's move, as `from`/`to` on an otherwise empty view.
    let moved = |spec: DiffSpec, entry: &FileEntry| -> Result<SubmoduleView, String> {
        let diff = file_diff(handle, &spec, entry, &options, POINTER_LINES, stop)
            .map_err(|e| e.to_string())?;
        // A gitlink row whose patch names no ids (none measured) draws its
        // other facts and no move; nothing is hidden, since `from`/`to` are
        // only the move's words.
        let (from, to) = pointer_of(&diff).unwrap_or((None, None));
        Ok(SubmoduleView {
            from: from.map(short_id),
            to: to.map(short_id),
            ..SubmoduleView::default()
        })
    };
    let mut rows = SubmoduleRows::default();
    for entry in &status.entries {
        let Some(facts) = entry.submodule else {
            continue;
        };
        if entry.unmerged {
            // A conflicted gitlink is CONFLICTS' row, worded by its shape.
            continue;
        }
        let path = entry.path.as_os_str().as_bytes().to_vec();
        if let Some(file) = staged.files.iter().find(|file| file.path == path) {
            rows.staged
                .insert(path.clone(), moved(DiffSpec::Staged, file)?);
        }
        let at = unstaged.files.iter().position(|file| file.path == path);
        if !(facts.pointer_moved || facts.changed_inside || facts.untracked_inside) {
            if let Some(at) = at {
                unstaged.files.remove(at);
            }
            continue;
        }
        let mut view = if let Some(at) = at {
            moved(DiffSpec::Unstaged, &unstaged.files[at])?
        } else {
            unstaged.files.push(gitlink_entry(path.clone()));
            SubmoduleView::default()
        };
        view.changed_inside = facts.changed_inside;
        view.untracked_inside = facts.untracked_inside;
        rows.unstaged.insert(path, view);
    }
    for path in mergewright_git::status::submodules_not_set_up(handle, stop).map_err(|e| e.to_string())? {
        let path = path.as_os_str().as_bytes().to_vec();
        if !unstaged.files.iter().any(|file| file.path == path) {
            unstaged.files.push(gitlink_entry(path.clone()));
        }
        rows.unstaged.insert(
            path,
            SubmoduleView {
                not_set_up: true,
                ..SubmoduleView::default()
            },
        );
    }
    Ok(rows)
}

/// Lines read of a gitlink's patch for its ids: it has two.
const POINTER_LINES: usize = 16;

/// An UNSTAGED entry for a submodule git's worktree diff did not list.
/// Staging it is `git add <path>`, the same as any other row.
fn gitlink_entry(path: Vec<u8>) -> FileEntry {
    FileEntry {
        path,
        old_path: None,
        status: FileStatus::Modified,
        added: None,
        removed: None,
    }
}

/// The diff pane's facts for a submodule path, or `None` for any other
/// path (m7-power-tools/13). A gitlink's patch gives the move; a row the
/// status added (untracked inside, not set up) has no patch, and the
/// worktree says which it is.
fn submodule_diff(
    handle: &RepoHandle,
    spec: &DiffSpec,
    diff: &FileDiff,
    stop: &Cancel,
) -> Result<Option<SubmoduleView>, String> {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let path = std::path::Path::new(OsStr::from_bytes(&diff.entry.path));
    let dir = handle.workdir().map(|top| top.join(path));
    let set_up = dir.as_ref().is_some_and(|dir| dir.join(".git").exists());
    let Some((from, to)) = pointer_of(diff) else {
        let is_gitlink_dir = *spec == DiffSpec::Unstaged
            && diff.hunks.is_empty()
            && dir.as_ref().is_some_and(|dir| dir.is_dir());
        return Ok(is_gitlink_dir.then(|| SubmoduleView {
            not_set_up: !set_up,
            log: Some(if set_up {
                SubmoduleLogView::Commits(Vec::new())
            } else {
                SubmoduleLogView::NotHere
            }),
            ..SubmoduleView::default()
        }));
    };
    let log = match (from, to) {
        (Some(from), Some(to)) if from != to && !set_up => SubmoduleLogView::NotHere,
        (Some(from), Some(to)) if from != to => {
            match mergewright_ops::submodule::submodule_log(handle, path, from, to, stop) {
                Ok(log) => SubmoduleLogView::Commits(
                    log.pulled_in
                        .iter()
                        .map(|commit| (true, commit))
                        .chain(log.rewound.iter().map(|commit| (false, commit)))
                        .map(|(pulled_in, (id, subject))| SubmoduleCommitView {
                            pulled_in,
                            id: short_id(*id),
                            subject: sanitize_line(subject),
                        })
                        .collect(),
                ),
                // git's refusal when a side's commit is not in the
                // submodule's repository (13a NOTES, measured: exit 128,
                // `Invalid symmetric difference expression`). Any other
                // failure is the job's.
                Err(mergewright_ops::facts::FactsError::History(RunError::Exited {
                    status: 128,
                    ..
                })) => SubmoduleLogView::NotHere,
                Err(error) => return Err(error.to_string()),
            }
        }
        _ => SubmoduleLogView::Commits(Vec::new()),
    };
    Ok(Some(SubmoduleView {
        from: from.map(short_id),
        to: to.map(short_id),
        not_set_up: !set_up,
        log: Some(log),
        ..SubmoduleView::default()
    }))
}

/// A compare's diff range (m7-14): `split..head` when the two share
/// history, else plain `base..head`, said as such in the summary.
fn compare_range(
    handle: &RepoHandle,
    base: Oid,
    head: Oid,
    stop: &Cancel,
) -> Result<(DiffSpec, CompareSummary), String> {
    match compare_span(handle, base, head, stop).map_err(|e| e.to_string())? {
        CompareSpan::Since { split, ahead } => Ok((
            DiffSpec::Range { base: split, head },
            CompareSummary::Since {
                split,
                commits: ahead,
            },
        )),
        CompareSpan::NoSharedHistory => Ok((
            DiffSpec::Range { base, head },
            CompareSummary::NoSharedHistory,
        )),
    }
}

#[cfg(test)]
mod blame_tests;
#[cfg(test)]
mod compare_tests;

/// Which refs contain a commit.
pub(super) fn read_contained(
    repos: &RepoStore,
    oid: Oid,
    stop: &Cancel,
) -> Result<Contained, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    contained_in(&repo.handle(), oid, stop).map_err(|e| e.to_string())
}

/// A commit message without its subject line and the blank lines after it.
fn body_of(message: &[u8]) -> &[u8] {
    let rest = match message.iter().position(|&b| b == b'\n') {
        Some(end) => &message[end + 1..],
        None => &[],
    };
    let start = rest
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(rest.len());
    &rest[start..]
}
