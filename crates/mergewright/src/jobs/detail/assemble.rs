//! The pure commit-side assembly: git's file list and diff into the view
//! models the pane draws. No git runs here.

use std::collections::HashMap;

use mergewright_git::diff::{BinaryNote, FileDiff, FileList, FileStatus, LineKind, pairs};
use mergewright_git::status::UnmergedHow;
use mergewright_term::sanitize::{SanitizedText, sanitize_line, sanitize_path_bytes};
use mergewright_ui::detail::{
    DiffView, FileListView, FileRow, FileRowKind, FileStatusView, HunkView, LineKindView, LineView,
    SubmoduleView,
};
use mergewright_ui::diff_words::word_spans;
use mergewright_ui::syntax::{highlight, language_for_first_line, language_for_path};

use super::working_tree::UnmergedShapes;

/// Trailer keys that never make a body worth a `✉ message` row.
const TRAILERS: [&str; 3] = ["signed-off-by:", "co-authored-by:", "reviewed-by:"];

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
        // A commit's files are never unmerged, so there is no shape to name.
        &UnmergedShapes::new(),
        // A commit's gitlink rows keep their letter: the file list carries
        // no modes to tell one (13b NOTES).
        &HashMap::new(),
    );
    view
}

/// `indices` of `list`, in path order, pushed onto `view` as a directory
/// tree (directories first-seen, paths in byte order) starting at `base`
/// depth. Shared by the commit pane's flat list and the changes pane's
/// per-section ones, so a section's files nest exactly as a commit's do.
pub(super) fn push_tree(
    view: &mut FileListView,
    list: &FileList,
    indices: &[usize],
    base: u8,
    how: &UnmergedShapes,
    submodules: &HashMap<Vec<u8>, SubmoduleView>,
) {
    let mut order: Vec<usize> = indices.to_vec();
    order.sort_by(|&a, &b| list.files[a].path.cmp(&list.files[b].path));
    // A submodule is one thing, not a directory to open: its row names the
    // whole path at the section's own depth, after the section's files
    // (m7-power-tools/13).
    let (modules, order): (Vec<usize>, Vec<usize>) = order
        .into_iter()
        .partition(|&index| submodules.contains_key(&list.files[index].path));
    let push_modules = |view: &mut FileListView| {
        for index in &modules {
            let entry = &list.files[*index];
            if let Some(submodule) = submodules.get(&entry.path) {
                view.rows.push(FileRow {
                    depth: base,
                    name: sanitize_path_bytes(&entry.path),
                    kind: FileRowKind::File {
                        status: FileStatusView::Submodule(Box::new(submodule.clone())),
                        added: None,
                        removed: None,
                    },
                    entry: u32::try_from(*index).ok(),
                });
            }
        }
    };

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
                status: status_view(entry.status, how.get(&entry.path).copied()),
                added: entry.added,
                removed: entry.removed,
            },
            entry: u32::try_from(index).ok(),
        });
    }
    push_modules(view);
}

fn depth(levels: usize) -> u8 {
    u8::try_from(levels).unwrap_or(u8::MAX)
}

fn status_view(status: FileStatus, how: Option<UnmergedHow>) -> FileStatusView {
    match status {
        FileStatus::Added => FileStatusView::Added,
        FileStatus::Deleted => FileStatusView::Deleted,
        FileStatus::Modified => FileStatusView::Modified,
        FileStatus::Renamed { .. } => FileStatusView::Renamed,
        FileStatus::Copied { .. } => FileStatusView::Copied,
        FileStatus::TypeChanged => FileStatusView::TypeChanged,
        // `how` is git's porcelain `XY` pair in words ("both modified",
        // "deleted by them"). The diff does not carry the pair —
        // `mergewright_git::diff::FileStatus` maps every `U` to one variant — so
        // it comes from the working-tree status the same job reads, by
        // path. `None` where nothing read the status (a commit's diff has
        // no unmerged paths at all), and then all that is true is that it
        // is unmerged (m6-conflicts/04, closing KNOWN_GAPS row 472).
        FileStatus::Unmerged => FileStatusView::Unmerged {
            how: sanitize_line(how.map_or("unmerged", mergewright_git::status::UnmergedHow::words)),
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
        status: status_view(diff.entry.status, None),
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
fn assemble_hunk(hunk: &mergewright_git::diff::Hunk, word_tint: bool) -> HunkView {
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
mod tests;
