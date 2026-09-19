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
};
use graft_ui::diff_words::word_spans;
use graft_ui::syntax::{highlight, language_for_first_line, language_for_path};

use super::Cancel;
use crate::repos::RepoStore;
use crate::update::detail::DiffKey;

/// Trailer keys that never make a body worth a `✉ message` row.
const TRAILERS: [&str; 3] = ["signed-off-by:", "co-authored-by:", "reviewed-by:"];

impl Stop for Cancel {
    fn should_stop(&self) -> bool {
        self.is_cancelled()
    }
}

/// List a spec's changed files, with the commit body for a single commit.
pub(super) fn read_files(
    repos: &RepoStore,
    spec: DiffSpec,
    options: DiffOptions,
    stop: &Cancel,
) -> Result<(Arc<FileListView>, Arc<FileList>), String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
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

/// Read and assemble one file's diff.
pub(super) fn read_file_diff(
    repos: &RepoStore,
    key: &DiffKey,
    entry: &FileEntry,
    stop: &Cancel,
) -> Result<Arc<DiffView>, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let spec = key.spec.diff_spec();
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
    Ok(Arc::new(assemble_diff(&diff, true)))
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

    let mut order: Vec<usize> = (0..list.files.len()).collect();
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
                depth: depth(open_dirs.len()),
                name: sanitize_line(&format!("{dir}/")),
                kind: FileRowKind::Dir { collapsed: false },
                entry: None,
            });
            open_dirs.push((*dir).to_owned());
        }
        view.rows.push(FileRow {
            depth: depth(dirs.len()),
            name: sanitize_line(file),
            kind: FileRowKind::File {
                status: status_view(entry.status),
                added: entry.added,
                removed: entry.removed,
            },
            entry: u32::try_from(index).ok(),
        });
    }
    view
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
        FileStatus::Unmerged => FileStatusView::Unmerged,
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
