//! Types and pure parsers for git's diff output.
//!
//! `parse_unified` reads what `git diff-tree -p -M -C --no-color --no-ext-diff`
//! prints for one or many files; `parse_file_list` joins `--raw -z` with
//! `--numstat -z`; `pairs` turns a hunk's removed/added runs into side-by-side
//! pairs. Everything here is pure: no `git` spawn, no I/O. `diff-reads` (next
//! ticket) runs the commands and hands their bytes to these functions.

mod file_list;
mod pairs;
mod patch;
mod read;
mod unified;

pub use file_list::{parse_file_list, parse_worktree_file_list};
pub use pairs::{Pair, pairs};
pub use patch::{PatchDirection, PatchError, Selection, hunk_patch};
pub use read::{DiffError, DiffOptions, DiffSpec, file_diff, list_files};
pub use unified::parse_unified;

/// How a file changed between the two sides of a diff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileStatus {
    /// The file exists only on the new side.
    Added,
    /// The file exists only on the old side.
    Deleted,
    /// The file's content or mode changed, same path, no type change.
    Modified,
    /// The file moved, with the given content similarity (0-100).
    Renamed {
        /// Percent of content unchanged, as git's `-M` detector scored it.
        similarity: u8,
    },
    /// The file was copied from another path, with the given similarity.
    Copied {
        /// Percent of content unchanged, as git's `-C` detector scored it.
        similarity: u8,
    },
    /// The path's type changed (e.g. regular file to symlink), same path.
    TypeChanged,
    /// The path is unmerged (a conflict): no single "old"/"new" side.
    Unmerged,
    /// The path exists in the working tree but not in either side compared.
    Untracked,
}

/// One file's entry in a diff: its paths, how it changed, and its line
/// counts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEntry {
    /// The path on the new side (or the only side, if deleted).
    pub path: Vec<u8>,
    /// The path on the old side, only set for a rename or a copy.
    pub old_path: Option<Vec<u8>>,
    /// How the file changed.
    pub status: FileStatus,
    /// Lines added. `None` for a binary file.
    pub added: Option<u32>,
    /// Lines removed. `None` for a binary file.
    pub removed: Option<u32>,
}

/// Every file in a diff, with the total lines added and removed.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FileList {
    /// One entry per file, in the order git reported them.
    pub files: Vec<FileEntry>,
    /// Total lines added across every non-binary file.
    pub added: u64,
    /// Total lines removed across every non-binary file.
    pub removed: u64,
}

/// What a line in a hunk is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    /// Present on both sides, unchanged.
    Context,
    /// Present only on the new side.
    Added,
    /// Present only on the old side.
    Removed,
}

/// One line of a hunk's body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffLine {
    /// Context, added or removed.
    pub kind: LineKind,
    /// The line's text, without its trailing `\n`. A trailing `\r` (CRLF
    /// content) stays in this text; it is not stripped.
    pub text: Vec<u8>,
    /// This line's number on the old side, if it has one.
    pub old_no: Option<u32>,
    /// This line's number on the new side, if it has one.
    pub new_no: Option<u32>,
    /// Whether git printed `\ No newline at end of file` right after this
    /// line — meaning this line's side of the file has no trailing newline.
    pub no_newline_at_eof: bool,
}

/// One `@@ -old_start,old_lines +new_start,new_lines @@ heading` hunk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hunk {
    /// First line number on the old side.
    pub old_start: u32,
    /// How many old-side lines this hunk covers.
    pub old_lines: u32,
    /// First line number on the new side.
    pub new_start: u32,
    /// How many new-side lines this hunk covers.
    pub new_lines: u32,
    /// The text after the second `@@` on the header line (often a function
    /// signature); empty if git printed none.
    pub heading: Vec<u8>,
    /// The hunk's lines, in file order.
    pub lines: Vec<DiffLine>,
}

/// Which kind of note git printed instead of a binary file's content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryNote {
    /// `Binary files a/x and b/x differ` — no patch, git could not build one
    /// (or `--binary` was not asked for).
    Differ,
    /// A `GIT binary patch` block. Its body is skipped; nothing here can
    /// apply it.
    Patch,
}

/// How much of a file's diff was shown, filled in by `diff-reads` when a
/// large diff is capped. The parser itself never truncates, so this is
/// always `None` coming out of `parse_unified`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Truncated {
    /// Lines actually parsed and returned.
    pub shown_lines: usize,
    /// Lines the full diff has.
    pub total_lines: usize,
}

/// One file's full diff: its entry, its hunks (if any), and whether it is
/// binary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileDiff {
    /// The file's identity and change summary.
    pub entry: FileEntry,
    /// The file's hunks, empty for a binary file or a mode-only change.
    pub hunks: Vec<Hunk>,
    /// Set when git printed a binary note instead of hunks.
    pub binary: Option<BinaryNote>,
    /// Set by `diff-reads` when the diff was capped; always `None` here.
    pub truncated: Option<Truncated>,
    /// The old side's file mode (`old mode`/`deleted file mode`), if git
    /// printed one.
    pub old_mode: Option<u32>,
    /// The new side's file mode (`new mode`/`new file mode`), if git
    /// printed one.
    pub new_mode: Option<u32>,
}

/// Why a diff failed to parse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// The 1-indexed input line the problem was found on (`parse_unified`),
    /// or the record index (`parse_file_list`, which has no lines).
    pub line: usize,
    /// What was wrong, for a log or an error message.
    pub what: &'static str,
}
