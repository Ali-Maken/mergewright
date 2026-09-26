//! What `B` blames, from what the diff cursor is on (design note pass 2b
//! B1's table), and where `^` puts the cursor in the parent — both pure.

use std::path::PathBuf;

use graft_core::oid::Oid;
use graft_git::diff::Hunk;
use graft_ui::detail::WorkingTreeSection;

use crate::update::detail::{DetailSpec, DiffCursor, Side};

/// Which version of which file `B` blames, and the line the cursor lands on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlameFrom {
    /// The file as of `commit`; `line` counts in that version.
    Commit {
        /// The version blamed.
        commit: Oid,
        /// The file's path in it.
        path: PathBuf,
        /// The line to land on.
        line: u32,
    },
    /// The file as of `commit`'s first parent: a removed line.
    Parent {
        /// The commit whose parent is blamed.
        commit: Oid,
        /// The file's path in the parent.
        path: PathBuf,
        /// The line to land on, counted in the parent.
        line: u32,
    },
    /// The file on disk.
    WorkingTree {
        /// The file's path.
        path: PathBuf,
        /// The line to land on; `None` for the first uncommitted line.
        line: Option<u32>,
    },
}

/// Why `B` opens nothing: said on the status line, no screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoBlame {
    /// Not in `HEAD` (untracked, or no `HEAD` yet).
    Untracked(PathBuf),
    /// No text to blame.
    Binary(PathBuf),
    /// The diff has not arrived, so there is no line yet.
    DiffNotLoaded,
}

/// B1's table: what the cursor is on → what to blame. `head` is `HEAD`'s
/// commit (`None` in a repository with no commit yet);
/// `staged_and_unstaged` is true when the changes pane's file has both.
///
/// # Errors
/// [`NoBlame`], in the table's order: untracked, binary, then no line.
pub fn blame_from(
    cursor: &DiffCursor,
    head: Option<Oid>,
    staged_and_unstaged: bool,
) -> Result<BlameFrom, NoBlame> {
    let path = cursor.path.clone();
    let old_path = cursor.old_path.clone().unwrap_or_else(|| path.clone());
    if cursor.section == Some(WorkingTreeSection::Untracked) {
        return Err(NoBlame::Untracked(path));
    }
    if cursor.binary {
        return Err(NoBlame::Binary(path));
    }
    let at = cursor.at.ok_or(NoBlame::DiffNotLoaded)?;
    let line = at.line;
    Ok(match (cursor.spec, at.side) {
        (DetailSpec::Commit(commit), Side::New) => BlameFrom::Commit { commit, path, line },
        (DetailSpec::Commit(commit), Side::Old) => BlameFrom::Parent {
            commit,
            path: old_path,
            line,
        },
        (DetailSpec::Compare { head, .. }, Side::New) => BlameFrom::Commit {
            commit: head,
            path,
            line,
        },
        (DetailSpec::Compare { base, .. }, Side::Old) => BlameFrom::Commit {
            commit: base,
            path: old_path,
            line,
        },
        (DetailSpec::WorkingTree, side) => {
            // The index numbers the unstaged diff's old side and the staged
            // diff's new side: neither the disk's lines nor HEAD's once the
            // file has edits in both, so neither blame can land on them.
            let index_numbered = matches!(
                (cursor.section, side),
                (Some(WorkingTreeSection::Unstaged), Side::Old)
                    | (Some(WorkingTreeSection::Staged), Side::New)
            );
            if staged_and_unstaged && index_numbered {
                return Ok(BlameFrom::WorkingTree { path, line: None });
            }
            match side {
                Side::New => BlameFrom::WorkingTree {
                    path,
                    line: Some(line),
                },
                Side::Old => {
                    let commit = head.ok_or_else(|| NoBlame::Untracked(path.clone()))?;
                    BlameFrom::Commit {
                        commit,
                        path: old_path,
                        line,
                    }
                }
            }
        }
    })
}

/// Where line `new_line` of a commit's version came from in its parent: the
/// first old-side line of the hunk that made it (a pure add: its old start,
/// at least 1). A line no hunk touched maps across by the hunks before it.
/// `hunks` is the commit's diff of the file, in file order.
#[must_use]
pub fn came_from(hunks: &[Hunk], new_line: u32) -> u32 {
    let mut shift: i64 = 0;
    for hunk in hunks {
        let end = hunk.new_start.saturating_add(hunk.new_lines);
        if new_line < hunk.new_start {
            break;
        }
        if new_line < end {
            return hunk.old_start.max(1);
        }
        shift += i64::from(hunk.old_lines) - i64::from(hunk.new_lines);
    }
    let old = (i64::from(new_line) + shift).max(1);
    u32::try_from(old).unwrap_or(u32::MAX)
}
