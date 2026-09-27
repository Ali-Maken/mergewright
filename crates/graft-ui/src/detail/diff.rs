//! The diff body's view types: lines, hunks, options, and the rows the pane draws.

#[allow(clippy::wildcard_imports)]
use super::*;

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

/// What a row of the drawn diff body is
/// (m3-working-tree-undo-and-stash/04). The body is a flat list of rows —
/// one heading per hunk, then that hunk's lines — and the diff cursor
/// indexes it. This is the same flattening `detail_pane::diff_body` draws,
/// so a cursor row and a drawn row are always the same row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyRow {
    /// A hunk's `@@` heading.
    Heading {
        /// Index into [`DiffView::hunks`].
        hunk: usize,
    },
    /// One content line of a hunk.
    Line {
        /// Index into [`DiffView::hunks`].
        hunk: usize,
        /// Index into that [`HunkView::lines`] — the heading `LineView` at
        /// `lines[0]` is never addressed, so this is always `>= 1`.
        line: usize,
    },
    /// The "… N more lines · L load all" footer.
    Truncation,
}

impl DiffView {
    /// Content lines of one hunk: its `lines` without the `HunkHeading` the
    /// assembler puts at `lines[0]` (which `heading` already stands for).
    fn content_lines(hunk: &HunkView) -> impl Iterator<Item = (usize, &LineView)> {
        hunk.lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.kind != LineKindView::HunkHeading)
    }

    /// How many rows the body draws: one heading plus its content lines per
    /// hunk, plus the truncation footer when there is one.
    #[must_use]
    pub fn body_len(&self) -> usize {
        let lines: usize = self
            .hunks
            .iter()
            .map(|hunk| 1 + Self::content_lines(hunk).count())
            .sum();
        lines + usize::from(self.truncated.is_some())
    }

    /// What body row `row` is, or `None` past the end.
    #[must_use]
    pub fn body_row(&self, row: usize) -> Option<BodyRow> {
        let mut left = row;
        for (index, hunk) in self.hunks.iter().enumerate() {
            if left == 0 {
                return Some(BodyRow::Heading { hunk: index });
            }
            left -= 1;
            let mut lines = Self::content_lines(hunk);
            let count = Self::content_lines(hunk).count();
            if left < count {
                let (line, _) = lines.nth(left)?;
                return Some(BodyRow::Line { hunk: index, line });
            }
            left -= count;
        }
        if left == 0 && self.truncated.is_some() {
            Some(BodyRow::Truncation)
        } else {
            None
        }
    }
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
