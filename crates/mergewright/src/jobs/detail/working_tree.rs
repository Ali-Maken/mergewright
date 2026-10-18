//! The changes pane's list: four sections over two diffs, and which git
//! diff each row reads.

use std::collections::HashMap;

use mergewright_core::oid::Oid;
use mergewright_git::diff::{DiffSpec, FileDiff, FileEntry, FileList, FileStatus, LineKind};
use mergewright_git::status::UnmergedHow;
use mergewright_term::sanitize::{SanitizedText, sanitize_line};
use mergewright_ui::detail::{
    FileListView, FileRow, FileRowKind, SubmoduleView, WorkingTreeSection,
};

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

/// What each submodule row says, by path (the raw bytes the diff carries),
/// for the section it is drawn in (m7-power-tools/13). A path here draws
/// `▣` and its facts instead of a letter and counts.
#[derive(Clone, Debug, Default)]
pub struct SubmoduleRows {
    /// `STAGED`: a pointer move already in the index.
    pub staged: HashMap<Vec<u8>, SubmoduleView>,
    /// `UNSTAGED`: what git's status says of the submodule's worktree, or
    /// that it is not set up.
    pub unstaged: HashMap<Vec<u8>, SubmoduleView>,
}

impl SubmoduleRows {
    fn of(&self, section: WorkingTreeSection) -> Option<&HashMap<Vec<u8>, SubmoduleView>> {
        match section {
            WorkingTreeSection::Staged => Some(&self.staged),
            WorkingTreeSection::Unstaged => Some(&self.unstaged),
            WorkingTreeSection::Unmerged | WorkingTreeSection::Untracked => None,
        }
    }
}

/// The pointer a gitlink's patch moves, `(from, to)`: git prints a
/// gitlink's change as `-Subproject commit <old>` / `+Subproject commit
/// <new>` (with `-dirty` on a worktree that has changes inside). `None`
/// when `diff` is not a gitlink's — any other line in it, or no such line.
#[must_use]
pub fn pointer_of(diff: &FileDiff) -> Option<(Option<Oid>, Option<Oid>)> {
    let (mut from, mut to) = (None, None);
    for line in diff.hunks.iter().flat_map(|hunk| &hunk.lines) {
        let text = std::str::from_utf8(&line.text).ok()?;
        let id = text.strip_prefix("Subproject commit ")?;
        let id = Oid::from_hex(id.strip_suffix("-dirty").unwrap_or(id)).ok()?;
        match line.kind {
            LineKind::Removed => from = Some(id),
            LineKind::Added => to = Some(id),
            LineKind::Context => return None,
        }
    }
    (from.is_some() || to.is_some()).then_some((from, to))
}

/// An id as the rows show it: seven hex digits.
#[must_use]
pub fn short_id(id: Oid) -> SanitizedText {
    let hex = id.to_string();
    sanitize_line(hex.get(..7).unwrap_or(&hex))
}

/// [`UnmergedShapes`] from a status read: every entry git called unmerged
/// and told us the shape of, keyed by the same raw bytes the diff carries.
pub(super) fn shapes_of(status: &mergewright_git::status::WorkingTreeStatus) -> UnmergedShapes {
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
#[cfg(test)]
#[must_use]
pub fn assemble_working_tree(
    staged: &FileList,
    unstaged: &FileList,
    how: &UnmergedShapes,
) -> (FileListView, FileList) {
    assemble_changes(staged, unstaged, how, &SubmoduleRows::default())
}

/// [`assemble_working_tree`], with each path in `submodules` drawn as a
/// submodule row in its section (m7-power-tools/13). Which submodules have
/// a row at all is the caller's to decide — it is in the lists or not.
#[must_use]
pub fn assemble_changes(
    staged: &FileList,
    unstaged: &FileList,
    how: &UnmergedShapes,
    submodules: &SubmoduleRows,
) -> (FileListView, FileList) {
    let no_submodules = HashMap::new();
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
        let rows = submodules.of(*section).unwrap_or(&no_submodules);
        push_tree(&mut view, &merged, indices, 1, how, rows);
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
