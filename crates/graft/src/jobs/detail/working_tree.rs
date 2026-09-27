//! The changes pane's list: four sections over two diffs, and which git
//! diff each row reads.

use graft_git::diff::{DiffSpec, FileEntry, FileList, FileStatus};
use graft_git::status::UnmergedHow;
use graft_term::sanitize::sanitize_line;
use graft_ui::detail::{FileListView, FileRow, FileRowKind, WorkingTreeSection};

use super::assemble::push_tree;
use crate::update::detail::DetailSpec;

/// The git diff one changes-pane row shows: a STAGED row is the index
/// against `HEAD`, a CONFLICTS row is the worktree against `HEAD`, anything
/// else is the working tree against the index. Every other spec has one diff
/// and no sections.
#[must_use]
pub fn diff_spec_for(spec: DetailSpec, section: Option<WorkingTreeSection>) -> DiffSpec {
    match section {
        Some(WorkingTreeSection::Staged) => DiffSpec::Staged,
        // A conflicted path is read against `HEAD`, not against the index
        // (m5/21). "Worktree against the index" has no answer for it: the
        // index holds three stages of the path, not one, so `git diff-files`
        // replies with a **combined** diff (`diff --cc`, `@@@` headers) —
        // a different format, which the unified parser skips, leaving 0
        // hunks and a blank pane beside a file the changes pane has just
        // named under `CONFLICTS`.
        //
        // `git diff-index HEAD` on the same path is an ordinary two-way
        // diff of the file as it now stands — markers and all — against
        // what is committed. That is the question the user is asking of a
        // conflicted file, and it keeps answering it as they edit: the
        // markers disappear from the diff as they resolve them.
        Some(WorkingTreeSection::Unmerged) => DiffSpec::WorkingTreeVsHead,
        Some(WorkingTreeSection::Unstaged | WorkingTreeSection::Untracked) => DiffSpec::Unstaged,
        None => spec.diff_spec(),
    }
}

/// Each unmerged path's shape, by path, from one working-tree status.
pub type UnmergedShapes = std::collections::HashMap<Vec<u8>, UnmergedHow>;

/// [`UnmergedShapes`] from a status read: every entry git called unmerged
/// and told us the shape of, keyed by the same raw bytes the diff carries.
pub(super) fn shapes_of(status: &graft_git::status::WorkingTreeStatus) -> UnmergedShapes {
    use std::os::unix::ffi::OsStrExt;

    status
        .entries
        .iter()
        .filter_map(|entry| {
            Some((
                entry.path.as_os_str().as_bytes().to_vec(),
                entry.unmerged_how?,
            ))
        })
        .collect()
}

/// The changes pane's four sections, each with the files that belong in it.
///
/// A conflicted file belongs in exactly one: `CONFLICTS`. It is excluded
/// from the others **by path**, not by status — git reports an unmerged
/// path in the worktree diff as an ordinary modification too, so filtering
/// on `FileStatus::Unmerged` alone leaves it listed a second time under
/// `UNSTAGED` as though it were a change to stage.
fn sections_of(
    staged: &FileList,
    unstaged: &FileList,
) -> [(WorkingTreeSection, Vec<FileEntry>); 4] {
    let untracked = |entry: &&FileEntry| entry.status == FileStatus::Untracked;
    let unmerged = |entry: &&FileEntry| entry.status == FileStatus::Unmerged;
    let mut conflicted: std::collections::BTreeSet<Vec<u8>> = std::collections::BTreeSet::new();
    let conflicts: Vec<FileEntry> = staged
        .files
        .iter()
        .chain(unstaged.files.iter())
        .filter(unmerged)
        .filter(|entry| conflicted.insert(entry.path.clone()))
        .cloned()
        .collect();
    let settled = |entry: &&FileEntry| !conflicted.contains(&entry.path);
    [
        (WorkingTreeSection::Unmerged, conflicts),
        (
            WorkingTreeSection::Staged,
            staged.files.iter().filter(settled).cloned().collect(),
        ),
        (
            WorkingTreeSection::Unstaged,
            unstaged
                .files
                .iter()
                .filter(|entry| !untracked(entry) && settled(entry))
                .cloned()
                .collect(),
        ),
        (
            WorkingTreeSection::Untracked,
            unstaged.files.iter().filter(untracked).cloned().collect(),
        ),
    ]
}

/// The changes pane's file list: `CONFLICTS`, `STAGED`, `UNSTAGED` and
/// `UNTRACKED` sections, each over its own files as a directory tree, plus
/// the one merged [`FileList`] every row's `entry` indexes into
/// (m3-working-tree-undo-and-stash/04).
///
/// A file that is both staged and unstaged appears **twice** — once in each
/// section — because it is two different changes, and staging the second
/// does not undo the first. The merged list holds it twice too, so the two
/// rows read two different diffs.
///
/// A **conflicted** file is the exception, and comes first
/// (m5-history-and-ghost-preview/09). Git reports an unmerged path in both
/// diffs and once per stage, so the same file would otherwise appear up to
/// four times under headings that both read as ordinary changes. One row,
/// in `CONFLICTS`, is the truth: there is one thing to do to it.
///
/// An empty tree gets no section rows at all, just the `nothing to commit`
/// message, so the pane does not print three empty headings.
#[must_use]
pub fn assemble_working_tree(
    staged: &FileList,
    unstaged: &FileList,
    how: &UnmergedShapes,
) -> (FileListView, FileList) {
    let by_section = sections_of(staged, unstaged);

    let mut merged = FileList {
        files: Vec::new(),
        added: staged.added.saturating_add(unstaged.added),
        removed: staged.removed.saturating_add(unstaged.removed),
    };
    let mut sections: Vec<(WorkingTreeSection, Vec<usize>)> = Vec::new();
    for (section, files) in by_section {
        if files.is_empty() {
            continue;
        }
        let start = merged.files.len();
        merged.files.extend(files);
        sections.push((section, (start..merged.files.len()).collect()));
    }

    let mut view = FileListView {
        files: u32::try_from(merged.files.len()).unwrap_or(u32::MAX),
        added: merged.added,
        removed: merged.removed,
        ..FileListView::default()
    };
    for (section, indices) in &sections {
        view.rows.push(FileRow {
            depth: 0,
            name: sanitize_line(section.title()),
            kind: FileRowKind::Section {
                section: *section,
                count: indices.len(),
            },
            entry: None,
        });
        push_tree(&mut view, &merged, indices, 1, how);
    }
    // The cursor starts on the first file, not on a heading: opening the
    // pane should show a diff, and `Space` on a heading means "all of it".
    view.cursor = view
        .rows
        .iter()
        .position(|row| row.entry.is_some())
        .unwrap_or(0);
    (view, merged)
}

#[cfg(test)]
mod tests;
