//! The detail pane's jobs: the changed-file list, one file's diff, and which
//! refs contain a commit.
//!
//! Every git read runs here, off the loop's thread, and so does the view-model
//! assembly: sanitising, pairing, word diffs and syntax highlighting. `update`
//! receives one finished `Arc` and never touches git bytes.

use std::sync::Arc;

use graft_core::oid::Oid;
use graft_git::contained::{Contained, contained_in};
use graft_git::diff::{
    BinaryNote, DiffOptions, DiffSpec, FileDiff, FileEntry, FileList, FileStatus, LineKind,
    file_diff, list_files, pairs,
};
use graft_git::run::Stop;
use graft_term::sanitize::{SanitizedText, sanitize_line, sanitize_path_bytes};
use graft_ui::detail::{
    DiffView, FileListView, FileRow, FileRowKind, FileStatusView, HunkView, LineKindView, LineView,
    WorkingTreeSection,
};
use graft_ui::diff_words::word_spans;
use graft_ui::syntax::{highlight, language_for_first_line, language_for_path};

use super::Cancel;
use crate::repos::RepoStore;
use crate::update::detail::{DetailSpec, DiffKey};

/// Trailer keys that never make a body worth a `✉ message` row.
const TRAILERS: [&str; 3] = ["signed-off-by:", "co-authored-by:", "reviewed-by:"];

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
        let unstaged =
            list_files(&handle, &DiffSpec::Unstaged, &options, stop).map_err(|e| e.to_string())?;
        let (view, list) = assemble_working_tree(&staged, &unstaged);
        return Ok((Arc::new(view), Arc::new(list)));
    }
    let list = list_files(&handle, &spec, &options, stop).map_err(|e| e.to_string())?;
    let message = match spec {
        DiffSpec::Commit { commit: id, .. } => Some(
            graft_git::history::read_commit(&handle, id)
                .map_err(|e| e.to_string())?
                .message,
        ),
        DiffSpec::Range { .. }
        | DiffSpec::WorkingTreeVsHead
        | DiffSpec::Staged
        | DiffSpec::Unstaged => None,
    };
    let view = assemble_files(&list, message.as_deref().map(body_of));
    Ok((Arc::new(view), Arc::new(list)))
}

/// The git diff one changes-pane row shows: a STAGED row is the index
/// against `HEAD`, anything else is the working tree against the index.
/// Every other spec has one diff and no sections.
#[must_use]
pub fn diff_spec_for(spec: DetailSpec, section: Option<WorkingTreeSection>) -> DiffSpec {
    match section {
        Some(WorkingTreeSection::Staged) => DiffSpec::Staged,
        // A conflicted path's diff is the worktree against the index, the
        // same read as an unstaged change: that is the file with its
        // conflict markers in it, which is what the user needs to see.
        Some(
            WorkingTreeSection::Unstaged
            | WorkingTreeSection::Untracked
            | WorkingTreeSection::Unmerged,
        ) => DiffSpec::Unstaged,
        None => spec.diff_spec(),
    }
}

/// Read and assemble one file's diff, handing back both the drawable view
/// model and the raw diff it was built from: `graft_git::diff::hunk_patch`
/// needs git's own bytes to build a stage or discard patch, and the view
/// model has been sanitised past the point of reversing.
pub(super) fn read_file_diff(
    repos: &RepoStore,
    key: &DiffKey,
    entry: &FileEntry,
    stop: &Cancel,
) -> Result<(Arc<DiffView>, Arc<FileDiff>), String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let spec = diff_spec_for(key.spec, key.section);
    let diff = file_diff(
        &repo.handle(),
        &spec,
        entry,
        &key.options,
        key.max_lines,
        stop,
    )
    .map_err(|e| e.to_string())?;
    // Word spans are always worked out; whether they tint is the widget's call.
    let view = assemble_diff(&diff, true);
    Ok((Arc::new(view), Arc::new(diff)))
}

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

fn is_trailer(line: &str) -> bool {
    let lower = line.trim_start().to_ascii_lowercase();
    TRAILERS.iter().any(|key| lower.starts_with(key))
}

/// The changed files as a tree (directories first-seen, paths in byte order),
/// led by a `✉ message` row when `message` (the body, subject removed) has a
/// line that is not a trailer.
pub fn assemble_files(list: &FileList, message: Option<&[u8]>) -> FileListView {
    let mut view = FileListView {
        files: u32::try_from(list.files.len()).unwrap_or(u32::MAX),
        added: list.added,
        removed: list.removed,
        ..FileListView::default()
    };

    if let Some(body) = message {
        let text = String::from_utf8_lossy(body);
        let lines: Vec<&str> = text.trim_end().lines().collect();
        if lines
            .iter()
            .any(|line| !line.trim().is_empty() && !is_trailer(line))
        {
            view.message = lines.iter().map(|line| sanitize_line(line)).collect();
            view.rows.push(FileRow {
                depth: 0,
                name: sanitize_line("✉ message"),
                kind: FileRowKind::Message { lines: lines.len() },
                entry: None,
            });
        }
    }

    push_tree(
        &mut view,
        list,
        &(0..list.files.len()).collect::<Vec<_>>(),
        0,
    );
    view
}

/// `indices` of `list`, in path order, pushed onto `view` as a directory
/// tree (directories first-seen, paths in byte order) starting at `base`
/// depth. Shared by the commit pane's flat list and the changes pane's
/// per-section ones, so a section's files nest exactly as a commit's do.
fn push_tree(view: &mut FileListView, list: &FileList, indices: &[usize], base: u8) {
    let mut order: Vec<usize> = indices.to_vec();
    order.sort_by(|&a, &b| list.files[a].path.cmp(&list.files[b].path));

    let mut open_dirs: Vec<String> = Vec::new();
    for index in order {
        let entry = &list.files[index];
        let path = sanitize_path_bytes(&entry.path);
        let parts: Vec<&str> = path.as_str().split('/').collect();
        let Some((file, dirs)) = parts.split_last() else {
            continue;
        };
        let shared = open_dirs
            .iter()
            .zip(dirs.iter())
            .take_while(|(open, dir)| open == dir)
            .count();
        open_dirs.truncate(shared);
        for dir in &dirs[shared..] {
            view.rows.push(FileRow {
                depth: base.saturating_add(depth(open_dirs.len())),
                name: sanitize_line(&format!("{dir}/")),
                kind: FileRowKind::Dir { collapsed: false },
                entry: None,
            });
            open_dirs.push((*dir).to_owned());
        }
        view.rows.push(FileRow {
            depth: base.saturating_add(depth(dirs.len())),
            name: sanitize_line(file),
            kind: FileRowKind::File {
                status: status_view(entry.status),
                added: entry.added,
                removed: entry.removed,
            },
            entry: u32::try_from(index).ok(),
        });
    }
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
pub fn assemble_working_tree(staged: &FileList, unstaged: &FileList) -> (FileListView, FileList) {
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
        push_tree(&mut view, &merged, indices, 1);
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

fn depth(levels: usize) -> u8 {
    u8::try_from(levels).unwrap_or(u8::MAX)
}

fn status_view(status: FileStatus) -> FileStatusView {
    match status {
        FileStatus::Added => FileStatusView::Added,
        FileStatus::Deleted => FileStatusView::Deleted,
        FileStatus::Modified => FileStatusView::Modified,
        FileStatus::Renamed { .. } => FileStatusView::Renamed,
        FileStatus::Copied { .. } => FileStatusView::Copied,
        FileStatus::TypeChanged => FileStatusView::TypeChanged,
        // `how` wants git's porcelain `XY` pair in words ("both modified",
        // "deleted by them"). `graft_git::diff::FileStatus::Unmerged` does
        // not carry the pair — `file_list.rs` maps every `U` to one
        // variant — so the only true thing to say here is that it is
        // unmerged. Carrying `XY` through needs `graft-git`, which this
        // group does not hold; it is in the ticket's NOTES as a gap.
        FileStatus::Unmerged => FileStatusView::Unmerged {
            how: sanitize_line("unmerged"),
        },
        FileStatus::Untracked => FileStatusView::Untracked,
    }
}

/// One line's text, sanitised, with a CRLF ending kept visible as `␍`.
fn line_text(bytes: &[u8]) -> SanitizedText {
    let (body, crlf) = match bytes.strip_suffix(b"\r") {
        Some(body) => (body, true),
        None => (bytes, false),
    };
    let clean = sanitize_line(&String::from_utf8_lossy(body));
    if crlf {
        sanitize_line(&format!("{clean}\u{240D}"))
    } else {
        clean
    }
}

/// Build a file's drawable diff: sanitised lines, heading first in each hunk,
/// side-by-side pairs, word spans on paired lines (when `word_tint`), and
/// syntax spans over the whole file's lines in one highlighter pass.
pub fn assemble_diff(diff: &FileDiff, word_tint: bool) -> DiffView {
    let mut hunks: Vec<HunkView> = diff
        .hunks
        .iter()
        .map(|hunk| assemble_hunk(hunk, word_tint))
        .collect();

    highlight_hunks(&mut hunks, &sanitize_path_bytes(&diff.entry.path));

    DiffView {
        path: sanitize_path_bytes(&diff.entry.path),
        old_path: diff.entry.old_path.as_deref().map(sanitize_path_bytes),
        status: status_view(diff.entry.status),
        hunks,
        binary: diff.binary.map(|note| {
            sanitize_line(match note {
                BinaryNote::Differ => "binary file differs",
                BinaryNote::Patch => "binary patch not shown",
            })
        }),
        truncated: diff.truncated.map(|cut| (cut.shown_lines, cut.total_lines)),
        untracked: diff.entry.status == FileStatus::Untracked,
    }
}

/// One hunk: heading first, then its lines, paired and word-diffed.
fn assemble_hunk(hunk: &graft_git::diff::Hunk, word_tint: bool) -> HunkView {
    let heading = sanitize_line(&String::from_utf8_lossy(&hunk.heading));
    let mut lines = vec![LineView {
        kind: LineKindView::HunkHeading,
        old_no: None,
        new_no: None,
        text: heading.clone(),
        words: Vec::new(),
        syntax: Vec::new(),
        pair: None,
    }];
    lines.extend(hunk.lines.iter().map(|line| LineView {
        kind: match line.kind {
            LineKind::Context => LineKindView::Context,
            LineKind::Added => LineKindView::Added,
            LineKind::Removed => LineKindView::Removed,
        },
        old_no: line.old_no,
        new_no: line.new_no,
        text: line_text(&line.text),
        words: Vec::new(),
        syntax: Vec::new(),
        pair: None,
    }));

    for pair in pairs(hunk) {
        // `+ 1`: the heading sits at `lines[0]`.
        for (removed, added) in pair.removed.clone().zip(pair.added.clone()) {
            lines[removed + 1].pair = u32::try_from(added + 1).ok();
            lines[added + 1].pair = u32::try_from(removed + 1).ok();
        }
        if word_tint {
            let text = |i: usize| strip_cr(lines[i + 1].text.as_str()).to_owned();
            let old: Vec<String> = pair.removed.clone().map(text).collect();
            let new: Vec<String> = pair.added.clone().map(text).collect();
            let spans = word_spans(
                &old.iter().map(String::as_str).collect::<Vec<_>>(),
                &new.iter().map(String::as_str).collect::<Vec<_>>(),
            );
            for (i, words) in pair.removed.clone().zip(spans.removed) {
                lines[i + 1].words = words;
            }
            for (i, words) in pair.added.clone().zip(spans.added) {
                lines[i + 1].words = words;
            }
        }
    }
    HunkView { heading, lines }
}

fn strip_cr(text: &str) -> &str {
    text.strip_suffix('\u{240D}').unwrap_or(text)
}

/// Syntax spans for every content line, in one pass so parser state carries
/// from line to line (old and new sides interleaved, which is close enough).
fn highlight_hunks(hunks: &mut [HunkView], path: &SanitizedText) {
    let first_code = hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .find(|line| matches!(line.kind, LineKindView::Context | LineKindView::Added));
    let Some(language) = language_for_path(path.as_str())
        .or_else(|| first_code.and_then(|line| language_for_first_line(line.text.as_str())))
    else {
        return;
    };
    let texts: Vec<&str> = hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .filter(|line| line.kind != LineKindView::HunkHeading)
        .map(|line| strip_cr(line.text.as_str()))
        .collect();
    let mut spans = highlight(&language, &texts).into_iter();
    for line in hunks.iter_mut().flat_map(|hunk| &mut hunk.lines) {
        if line.kind != LineKindView::HunkHeading {
            line.syntax = spans.next().unwrap_or_default();
        }
    }
}

#[cfg(test)]
mod tests {
    use graft_git::diff::{DiffLine, Hunk};

    use super::*;

    fn entry(path: &str) -> FileEntry {
        FileEntry {
            path: path.into(),
            old_path: None,
            status: FileStatus::Modified,
            added: Some(1),
            removed: Some(1),
        }
    }

    fn names(view: &FileListView) -> Vec<(&str, u8)> {
        view.rows
            .iter()
            .map(|row| (row.name.as_str(), row.depth))
            .collect()
    }

    #[test]
    fn files_become_a_tree_with_directories_before_their_children() {
        let list = FileList {
            files: vec![entry("e.rs"), entry("a/b/c.rs"), entry("a/d.rs")],
            added: 3,
            removed: 3,
        };
        let view = assemble_files(&list, None);
        assert_eq!(
            names(&view),
            [("a/", 0), ("b/", 1), ("c.rs", 2), ("d.rs", 1), ("e.rs", 0)]
        );
        assert_eq!(view.rows[2].entry, Some(1));
        let mut folded = view.clone();
        folded.toggle_dir(1);
        assert_eq!(
            folded
                .visible()
                .map(|(_, r)| r.name.as_str())
                .collect::<Vec<_>>(),
            ["a/", "b/", "d.rs", "e.rs"]
        );
    }

    #[test]
    fn a_body_with_prose_leads_with_a_message_row_and_trailers_alone_do_not() {
        let list = FileList {
            files: vec![entry("e.rs")],
            added: 1,
            removed: 1,
        };
        let view = assemble_files(&list, Some(b"First line.\nSecond line.\n"));
        assert_eq!(view.rows[0].kind, FileRowKind::Message { lines: 2 });
        assert_eq!(view.message.len(), 2);
        let trailers = assemble_files(&list, Some(b"Signed-off-by: A <a@b>\n"));
        assert_eq!(names(&trailers), [("e.rs", 0)]);
        assert_eq!(body_of(b"Subject\n\nBody\n"), b"Body\n");
    }

    fn line(kind: LineKind, text: &[u8], old_no: Option<u32>, new_no: Option<u32>) -> DiffLine {
        DiffLine {
            kind,
            text: text.to_vec(),
            old_no,
            new_no,
            no_newline_at_eof: false,
        }
    }

    #[test]
    fn a_paired_hunk_gets_a_heading_pairs_words_syntax_and_safe_text() {
        let diff = FileDiff {
            entry: entry("src/lib.rs"),
            hunks: vec![Hunk {
                old_start: 1,
                old_lines: 2,
                new_start: 1,
                new_lines: 2,
                heading: b"fn main()".to_vec(),
                lines: vec![
                    line(LineKind::Removed, b"let x = 1;\x1b[31m", Some(1), None),
                    line(LineKind::Removed, b"let y = 2;\r", Some(2), None),
                    line(LineKind::Added, b"let x = 5;", None, Some(1)),
                    line(LineKind::Added, b"let y = 2; // same\r", None, Some(2)),
                ],
            }],
            binary: None,
            truncated: None,
            old_mode: None,
            new_mode: None,
        };
        let view = assemble_diff(&diff, true);
        let lines = &view.hunks[0].lines;
        assert_eq!(lines[0].kind, LineKindView::HunkHeading);
        assert_eq!(lines[1].pair, Some(3));
        assert_eq!(lines[3].pair, Some(1));
        assert!(!lines[3].words.is_empty(), "the changed number is tinted");
        assert!(
            lines[1].syntax.iter().any(|s| !s.range.is_empty()),
            "rust is highlighted"
        );
        assert!(
            !lines[1].text.as_str().contains('\x1b'),
            "ESC never reaches the view"
        );
        assert!(
            lines[2].text.as_str().ends_with('\u{240D}'),
            "CRLF shows as ␍"
        );
        assert!(!view.untracked);
    }
}

#[cfg(test)]
mod working_tree_tests {
    use graft_ui::detail::WorkingTreeSection::{Staged, Unstaged, Untracked};

    use super::*;

    fn file(path: &str, status: FileStatus) -> FileEntry {
        FileEntry {
            path: path.into(),
            old_path: None,
            status,
            added: Some(1),
            removed: Some(1),
        }
    }

    fn list(files: Vec<FileEntry>) -> FileList {
        let added = files.len() as u64;
        FileList {
            files,
            added,
            removed: added,
        }
    }

    /// Every row as `(name, depth, section)` — the shape the pane draws.
    fn rows(view: &FileListView) -> Vec<(&str, u8, Option<WorkingTreeSection>)> {
        view.rows
            .iter()
            .enumerate()
            .map(|(index, row)| (row.name.as_str(), row.depth, view.section_at(index)))
            .collect()
    }

    #[test]
    fn three_sections_count_their_own_files_and_a_shared_path_is_in_two() {
        // `both.rs` is staged *and* changed again since: two changes, two rows.
        let staged = list(vec![
            file("both.rs", FileStatus::Modified),
            file("s1.rs", FileStatus::Added),
        ]);
        let unstaged = list(vec![
            file("both.rs", FileStatus::Modified),
            file("u1.rs", FileStatus::Modified),
            file("new.txt", FileStatus::Untracked),
        ]);
        let (view, merged) = assemble_working_tree(&staged, &unstaged);
        assert_eq!(
            rows(&view),
            vec![
                ("STAGED", 0, Some(Staged)),
                ("both.rs", 1, Some(Staged)),
                ("s1.rs", 1, Some(Staged)),
                ("UNSTAGED", 0, Some(Unstaged)),
                ("both.rs", 1, Some(Unstaged)),
                ("u1.rs", 1, Some(Unstaged)),
                ("UNTRACKED", 0, Some(Untracked)),
                ("new.txt", 1, Some(Untracked)),
            ]
        );
        let counts: Vec<usize> = view
            .rows
            .iter()
            .filter_map(|row| match row.kind {
                FileRowKind::Section { count, .. } => Some(count),
                _ => None,
            })
            .collect();
        assert_eq!(counts, vec![2, 2, 1]);
        assert_eq!(merged.files.len(), 5, "every row indexes its own entry");
        // The cursor opens on a file, never on a heading.
        assert_eq!(view.cursor, 1);
        assert_eq!(view.selected_section(), Some(Staged));
    }

    #[test]
    fn an_untracked_file_is_a_row_of_its_own_section_not_an_unstaged_one() {
        let unstaged = list(vec![file("new.txt", FileStatus::Untracked)]);
        let (view, _) = assemble_working_tree(&list(Vec::new()), &unstaged);
        assert_eq!(
            rows(&view),
            vec![
                ("UNTRACKED", 0, Some(Untracked)),
                ("new.txt", 1, Some(Untracked))
            ]
        );
    }

    #[test]
    fn an_empty_section_gets_no_heading_and_a_clean_tree_gets_no_rows() {
        let (view, merged) = assemble_working_tree(&list(Vec::new()), &list(Vec::new()));
        assert!(view.rows.is_empty(), "{:?}", view.rows);
        assert!(merged.files.is_empty());
    }

    #[test]
    fn a_sections_entries_are_its_own_and_stop_at_the_next_heading() {
        let staged = list(vec![file("a.rs", FileStatus::Modified)]);
        let unstaged = list(vec![
            file("b.rs", FileStatus::Modified),
            file("c.rs", FileStatus::Modified),
        ]);
        let (view, _) = assemble_working_tree(&staged, &unstaged);
        assert_eq!(view.section_entries(0), vec![0]);
        assert_eq!(view.section_entries(2), vec![1, 2]);
        // A file row is not a section row: asking is safe and answers empty.
        assert!(view.section_entries(1).is_empty());
    }

    #[test]
    fn files_nest_under_their_section_one_level_deeper() {
        let unstaged = list(vec![file("src/a/b.rs", FileStatus::Modified)]);
        let (view, _) = assemble_working_tree(&list(Vec::new()), &unstaged);
        assert_eq!(
            rows(&view),
            vec![
                ("UNSTAGED", 0, Some(Unstaged)),
                ("src/", 1, Some(Unstaged)),
                ("a/", 2, Some(Unstaged)),
                ("b.rs", 3, Some(Unstaged)),
            ]
        );
    }

    #[test]
    fn a_staged_row_reads_the_index_and_every_other_row_the_working_tree() {
        assert_eq!(
            diff_spec_for(DetailSpec::WorkingTree, Some(Staged)),
            DiffSpec::Staged
        );
        assert_eq!(
            diff_spec_for(DetailSpec::WorkingTree, Some(Unstaged)),
            DiffSpec::Unstaged
        );
        assert_eq!(
            diff_spec_for(DetailSpec::WorkingTree, Some(Untracked)),
            DiffSpec::Unstaged
        );
        let commit = DetailSpec::Commit(
            graft_core::oid::Oid::from_bytes(&[7; 20]).expect("20 bytes is an oid"),
        );
        assert_eq!(diff_spec_for(commit, None), commit.diff_spec());
    }
}
