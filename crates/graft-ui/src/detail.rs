//! The detail pane's view model: what a selected commit, a comparison or the
//! working tree shows beside the graph.
//!
//! Types only, plus the file list's cursor and folding. `ui` learns nothing
//! about git here: every string arrives as `SanitizedText`, every path already
//! sanitised, and the app assembles these on a job thread. The widgets that
//! draw them come later (`detail-pane-frame`, `diff-widget`).

use std::ops::Range;
use std::sync::Arc;

use graft_core::mistake_radar::Severity;
use graft_term::sanitize::SanitizedText;

use crate::syntax::SyntaxSpan;

/// What the pane is about.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Subject {
    /// Nothing selected yet.
    #[default]
    None,
    /// One commit, against its first parent.
    Commit {
        /// Short SHA.
        sha: SanitizedText,
    },
    /// Two commits compared (`x`).
    Compare {
        /// The older side.
        base: SanitizedText,
        /// The newer side.
        head: SanitizedText,
    },
    /// The working tree against `HEAD`.
    WorkingTree {
        /// Files staged.
        staged: u32,
        /// Files changed, not staged.
        unstaged: u32,
        /// Files git does not track.
        untracked: u32,
    },
}

/// The pane's header block (DESIGN.md §1.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailHeader {
    /// What the pane is about.
    pub subject: Subject,
    /// The subject line.
    pub summary: SanitizedText,
    /// Author name and email.
    pub author: SanitizedText,
    /// Author date, absolute.
    pub author_date: SanitizedText,
    /// Author date, relative ("3 days ago").
    pub relative: SanitizedText,
    /// Shown only when the committer differs from the author.
    pub committer: Option<SanitizedText>,
    /// `Co-authored-by` trailers.
    pub co_authors: Vec<SanitizedText>,
    /// Parent short SHAs.
    pub parents: Vec<SanitizedText>,
    /// What the diff is against, when it is not the obvious first parent.
    pub against: Option<SanitizedText>,
    /// Branches and tags holding the commit.
    pub contained: ContainedIn,
    /// Mistake Radar lines.
    pub radar: Vec<RadarLine>,
}

/// Which branches and tags hold the commit.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum ContainedIn {
    #[default]
    /// Not asked yet.
    NotAsked,
    /// The query is running.
    Checking,
    /// `more` counts names past the ones listed.
    Known {
        /// Local and remote branches.
        branches: Vec<SanitizedText>,
        /// Tags.
        tags: Vec<SanitizedText>,
        /// Names past the ones listed.
        more: usize,
    },
    /// Why the query failed.
    Failed(SanitizedText),
}

/// One Mistake Radar line in the header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RadarLine {
    /// A finding on this commit.
    Finding {
        /// How loudly it shows.
        severity: Severity,
        /// The finding's title.
        title: SanitizedText,
    },
    /// A merge not yet probed for evil changes.
    EvilUnchecked,
    /// The evil-merge probe is running.
    EvilChecking,
    /// The merge carries no changes of its own.
    EvilClean,
    /// The merge differs from a clean re-merge only in files both sides
    /// changed: a hand-resolved conflict, not a mistake. Shown so the pane
    /// does not read as "nothing happened here" when something did.
    EvilResolved {
        /// How many files the resolution touched.
        files: usize,
    },
    /// The evil-merge probe could not run, and why.
    EvilNotCheckable(SanitizedText),
    /// The force-push probe could not run, and why.
    ForcePushNotCheckable(SanitizedText),
}

/// A changed file's status, as the list shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileStatusView {
    /// Added in this change.
    Added,
    /// Deleted in this change.
    Deleted,
    /// Content changed.
    Modified,
    /// Moved, maybe edited.
    Renamed,
    /// Copied from another path.
    Copied,
    /// File type changed (file, symlink, submodule).
    TypeChanged,
    /// In conflict.
    Unmerged,
    /// Not tracked by git.
    Untracked,
}

/// What a row of the file list is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileRowKind {
    /// The `✉ message` entry: the commit body, `lines` long.
    Message {
        /// Body lines.
        lines: usize,
    },
    /// A directory; its children follow it at a greater depth.
    Dir {
        /// Children hidden.
        collapsed: bool,
    },
    /// A file; line counts are `None` for binary files.
    File {
        /// What happened to it.
        status: FileStatusView,
        /// Lines added.
        added: Option<u32>,
        /// Lines removed.
        removed: Option<u32>,
    },
}

/// One row of the file tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileRow {
    /// Nesting depth, 0 at the top.
    pub depth: u8,
    /// The last path component (directories end in `/`).
    pub name: SanitizedText,
    /// What the row is.
    pub kind: FileRowKind,
    /// Index into the git file list; `None` for the message and directories.
    pub entry: Option<u32>,
}

/// The changed files as a folding tree, with its cursor.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FileListView {
    /// Every row, folded or not, in tree order.
    pub rows: Vec<FileRow>,
    /// Index into `rows`; always a visible row.
    pub cursor: usize,
    /// First row on screen; the widget keeps the cursor in view.
    pub top: usize,
    /// Files changed.
    pub files: u32,
    /// Lines added across all files.
    pub added: u64,
    /// Lines removed across all files.
    pub removed: u64,
    /// The commit body the `✉ message` row opens; empty without that row.
    pub message: Vec<SanitizedText>,
}

impl FileListView {
    /// Rows not hidden inside a collapsed directory, with their index in `rows`.
    pub fn visible(&self) -> impl Iterator<Item = (usize, &FileRow)> {
        let mut hidden_below: Option<u8> = None;
        self.rows.iter().enumerate().filter(move |(_, row)| {
            if let Some(depth) = hidden_below {
                if row.depth > depth {
                    return false;
                }
                hidden_below = None;
            }
            if let FileRowKind::Dir { collapsed: true } = row.kind {
                hidden_below = Some(row.depth);
            }
            true
        })
    }

    /// Fold or unfold the directory at `row`; anything else is left alone.
    /// A cursor left inside the fold moves up to the directory.
    pub fn toggle_dir(&mut self, row: usize) {
        let Some(FileRow {
            kind: FileRowKind::Dir { collapsed },
            ..
        }) = self.rows.get_mut(row)
        else {
            return;
        };
        *collapsed = !*collapsed;
        if !self.visible().any(|(index, _)| index == self.cursor) {
            self.cursor = row;
        }
    }

    /// Move the cursor by `delta` visible rows, clamped to the ends.
    pub fn move_cursor(&mut self, delta: isize) {
        let visible: Vec<usize> = self.visible().map(|(index, _)| index).collect();
        let Some(last) = visible.len().checked_sub(1) else {
            return;
        };
        let here = visible
            .iter()
            .position(|&index| index == self.cursor)
            .unwrap_or(0);
        let target = here.saturating_add_signed(delta).min(last);
        self.cursor = visible[target];
    }

    /// The git file-list index under the cursor, if the cursor is on a file.
    pub fn selected_entry(&self) -> Option<u32> {
        self.rows.get(self.cursor).and_then(|row| row.entry)
    }
}

/// What kind of line a diff line is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKindView {
    /// Unchanged.
    Context,
    /// Only on the new side.
    Added,
    /// Only on the old side.
    Removed,
    /// The `@@` line opening a hunk.
    HunkHeading,
}

/// One drawable diff line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineView {
    /// What the line is.
    pub kind: LineKindView,
    /// Line number on the old side.
    pub old_no: Option<u32>,
    /// Line number on the new side.
    pub new_no: Option<u32>,
    /// Sanitised; a CRLF line ends in `␍`.
    pub text: SanitizedText,
    /// Char ranges that changed against the paired line (word tint).
    pub words: Vec<Range<usize>>,
    /// Syntax highlighting over `text`.
    pub syntax: Vec<SyntaxSpan>,
    /// Index of the paired line in the same hunk's `lines`.
    pub pair: Option<u32>,
}

/// One hunk; `lines[0]` is its `HunkHeading`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HunkView {
    /// The text after `@@ … @@`.
    pub heading: SanitizedText,
    /// The lines, heading first.
    pub lines: Vec<LineView>,
}

/// How the diff is laid out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DiffMode {
    /// One column, old above new.
    #[default]
    Unified,
    /// Old left, new right.
    SideBySide,
}

/// The diff options as the pane words them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiffOptionsView {
    /// Context lines around each change.
    pub context: u8,
    /// Whitespace-only changes hidden.
    pub ignore_whitespace: bool,
    /// Changed words tinted inside paired lines.
    pub word_tint: bool,
}

impl Default for DiffOptionsView {
    fn default() -> Self {
        Self {
            context: 3,
            ignore_whitespace: false,
            word_tint: true,
        }
    }
}

/// One file's diff, ready to draw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffView {
    /// The file's path.
    pub path: SanitizedText,
    /// Where it came from, for a rename or copy.
    pub old_path: Option<SanitizedText>,
    /// What happened to it.
    pub status: FileStatusView,
    /// The text diff.
    pub hunks: Vec<HunkView>,
    /// Why there is no text diff, for a binary file.
    pub binary: Option<SanitizedText>,
    /// `(shown, total)` lines when the diff was cut short.
    pub truncated: Option<(usize, usize)>,
    /// Not tracked by git: no diff to show.
    pub untracked: bool,
}

/// What the pane's body shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BodyView {
    /// Nothing to show, and why.
    Empty(SanitizedText),
    /// The commit body.
    Message {
        /// Body lines.
        lines: Vec<SanitizedText>,
    },
    /// A read is running, and what it is.
    Loading(SanitizedText),
    /// A read failed, and why.
    Failed(SanitizedText),
    /// One file's diff.
    Diff(Arc<DiffView>),
}

/// Which region has the keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DetailFocus {
    /// The graph.
    #[default]
    Graph,
    /// The file list.
    Files,
    /// The diff body.
    Diff,
}

/// Everything the detail pane draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailView {
    /// The header block.
    pub header: DetailHeader,
    /// The file list, once read.
    pub files: Option<Arc<FileListView>>,
    /// The body.
    pub body: BodyView,
    /// First body line on screen.
    pub body_top: usize,
    /// Which region has the keys.
    pub focus: DetailFocus,
    /// Unified or side by side.
    pub mode: DiffMode,
    /// The diff options in force.
    pub options: DiffOptionsView,
    /// The explain panel, when open.
    pub explain: Option<ExplainView>,
}

/// A Mistake Radar explanation, as the explain panel draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExplainView {
    /// The finding's title.
    pub title: SanitizedText,
    /// How loudly it shows.
    pub severity: Severity,
    /// What happened and what to do.
    pub paragraphs: Vec<SanitizedText>,
    /// A small drawing of the shape.
    pub sketch: Vec<SanitizedText>,
}

#[cfg(test)]
mod tests {
    use graft_term::sanitize::sanitize_line;

    use super::*;

    fn row(depth: u8, name: &str, kind: FileRowKind, entry: Option<u32>) -> FileRow {
        FileRow {
            depth,
            name: sanitize_line(name),
            kind,
            entry,
        }
    }

    fn file() -> FileRowKind {
        FileRowKind::File {
            status: FileStatusView::Modified,
            added: Some(1),
            removed: Some(1),
        }
    }

    fn tree() -> FileListView {
        FileListView {
            rows: vec![
                row(0, "a/", FileRowKind::Dir { collapsed: false }, None),
                row(1, "b/", FileRowKind::Dir { collapsed: false }, None),
                row(2, "c.rs", file(), Some(0)),
                row(1, "d.rs", file(), Some(1)),
                row(0, "e.rs", file(), Some(2)),
            ],
            ..FileListView::default()
        }
    }

    fn visible_names(list: &FileListView) -> Vec<&str> {
        list.visible().map(|(_, row)| row.name.as_str()).collect()
    }

    #[test]
    fn folding_a_directory_hides_only_its_children() {
        let mut list = tree();
        list.toggle_dir(1);
        assert_eq!(visible_names(&list), ["a/", "b/", "d.rs", "e.rs"]);
        list.toggle_dir(0);
        assert_eq!(visible_names(&list), ["a/", "e.rs"]);
    }

    #[test]
    fn the_cursor_skips_folded_rows_and_stops_at_the_ends() {
        let mut list = tree();
        list.toggle_dir(1);
        list.move_cursor(2);
        assert_eq!(list.cursor, 3);
        assert_eq!(list.selected_entry(), Some(1));
        list.move_cursor(10);
        assert_eq!(list.cursor, 4);
        list.move_cursor(-10);
        assert_eq!(list.cursor, 0);
        assert_eq!(list.selected_entry(), None);
    }

    #[test]
    fn folding_over_the_cursor_moves_it_to_the_directory() {
        let mut list = tree();
        list.cursor = 2;
        list.toggle_dir(1);
        assert_eq!(list.cursor, 1);
    }
}
