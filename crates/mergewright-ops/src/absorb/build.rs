//! Building: each hunk as a patch with context, and the trees and commit
//! objects the plan melds in. Objects only; no ref, index or file.

use std::path::Path;

use mergewright_core::oid::Oid;
use mergewright_git::diff::{
    DiffLine, FileDiff, Hunk, LineKind, PatchDirection, Selection, hunk_patch,
};
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{GitArg, GitInvocation, Stop};

use crate::facts::FactsError;
use crate::objects;
use crate::snapshot::TempIndex;

use super::read::FileAtHead;
use super::{Piece, StagedHunk, parse_oid, unparsable, with_index};

/// `HEAD`'s tree plus `pieces`, committed with no ref (parent `head`):
/// the commit a `fixup` row melds into its target — and, with every placed
/// piece, the index base the put-back measures the stays from
/// (`RewriteParams::index_base`). Built on a temporary index; the real one
/// is never written.
pub(crate) fn commit_pieces(
    repo: &RepoHandle,
    head: Oid,
    pieces: &[&Piece],
    subject: &str,
    stop: &dyn Stop,
) -> Result<Oid, FactsError> {
    let tree = tree_with(repo, head, pieces, stop)?;
    // `-F -` from stdin, not `objects::commit_tree`'s `-m`: the message
    // goes in verbatim (06d review).
    let commit = GitInvocation::new(repo)
        .args([
            GitArg::Flag("commit-tree"),
            GitArg::Oid(tree),
            GitArg::Flag("-p"),
            GitArg::Oid(head),
            GitArg::Flag("-F"),
            GitArg::Flag("-"),
        ])
        .stdin(format!("{subject}\n").into_bytes())
        .run(stop)
        .map_err(FactsError::of_history)?;
    parse_oid(String::from_utf8_lossy(&commit.stdout).trim())
}

/// `target` plus `pieces` is `parent`'s tree: melded in, they would leave
/// `target` changing nothing, which git's rebase stops on ("would make it
/// empty"), so the plan drops `target` instead (review m7-06 M3). The
/// pieces go on `target`'s own tree, as the commutation check put them.
pub(crate) fn empties(
    repo: &RepoHandle,
    target: Oid,
    parent: Oid,
    pieces: &[&Piece],
    stop: &dyn Stop,
) -> Result<bool, FactsError> {
    let melded = tree_with(repo, target, pieces, stop)?;
    Ok(melded == tree_of(repo, parent, stop)?)
}

/// `commit`'s tree id.
fn tree_of(repo: &RepoHandle, commit: Oid, stop: &dyn Stop) -> Result<Oid, FactsError> {
    objects::resolve(repo, &format!("{commit}^{{tree}}"), stop).map_err(FactsError::of_history)
}

/// `commit`'s tree plus `pieces`, written as a tree from a temporary index.
fn tree_with(
    repo: &RepoHandle,
    commit: Oid,
    pieces: &[&Piece],
    stop: &dyn Stop,
) -> Result<Oid, FactsError> {
    let index = TempIndex::fresh(repo.git_dir())?;
    with_index(repo, &index)
        .args([GitArg::Flag("read-tree"), GitArg::Oid(commit)])
        .run(stop)
        .map_err(FactsError::of_history)?;
    // Bottom of each file first: every patch is placed by `HEAD`'s line
    // numbers, which a change further down leaves as they were.
    let mut ordered = pieces.to_vec();
    ordered.sort_by(|a, b| {
        a.hunk
            .path
            .cmp(&b.hunk.path)
            .then(b.hunk.old.start.cmp(&a.hunk.old.start))
    });
    for piece in ordered {
        with_index(repo, &index)
            .args([
                GitArg::Flag("apply"),
                GitArg::Flag("--cached"),
                // What he staged goes in as staged: his `apply.whitespace=fix`
                // would strip trailing spaces, and the put-back would then
                // clash with his own lines (review m7-06 M5).
                GitArg::Flag("--whitespace=nowarn"),
                GitArg::Flag("-"),
            ])
            .stdin(piece.patch.clone())
            .run(stop)
            .map_err(FactsError::of_history)?;
    }
    let tree = with_index(repo, &index)
        .arg(GitArg::Flag("write-tree"))
        .run(stop)
        .map_err(FactsError::of_history)?;
    parse_oid(String::from_utf8_lossy(&tree.stdout).trim())
}

pub(super) fn staged_hunk(path: &Path, hunk: &Hunk) -> StagedHunk {
    StagedHunk {
        path: path.to_path_buf(),
        old: hunk.old_start..hunk.old_start + hunk.old_lines,
        new: hunk.new_start..hunk.new_start + hunk.new_lines,
    }
}

/// A hunk that is never applied (a new file's): no patch.
pub(super) fn bare(path: &Path, hunk: &Hunk) -> Piece {
    Piece {
        hunk: staged_hunk(path, hunk),
        patch: Vec::new(),
    }
}

/// A file with no hunks to name (binary, mode only, an empty new file).
pub(super) fn empty(path: &Path) -> Piece {
    Piece {
        hunk: StagedHunk {
            path: path.to_path_buf(),
            old: 0..0,
            new: 0..0,
        },
        patch: Vec::new(),
    }
}

/// `hunk` as a one-hunk patch with the line above and the line below from
/// `HEAD`'s file as context, where they exist.
pub(super) fn with_context(
    diff: &FileDiff,
    hunk: &Hunk,
    file: &FileAtHead,
) -> Result<Vec<u8>, FactsError> {
    let count = u32::try_from(file.lines.len()).unwrap_or(u32::MAX);
    // The last old line the hunk covers (an insertion: the line it follows).
    let (above, below) = if hunk.old_lines == 0 {
        (hunk.old_start, hunk.old_start + 1)
    } else {
        (
            hunk.old_start.saturating_sub(1),
            hunk.old_start + hunk.old_lines,
        )
    };
    let context = |line: u32| -> Option<DiffLine> {
        if !(1..=count).contains(&line) {
            return None;
        }
        let text = file.lines.get(usize::try_from(line - 1).ok()?)?.clone();
        Some(DiffLine {
            kind: LineKind::Context,
            text,
            old_no: Some(line),
            new_no: None,
            no_newline_at_eof: line == count && !file.ends_with_newline,
        })
    };
    let above = context(above);
    let below = context(below);
    let first = match (&above, hunk.old_lines) {
        (Some(line), _) => line.old_no.unwrap_or(hunk.old_start),
        (None, 0) => hunk.old_start + 1,
        (None, _) => hunk.old_start,
    };
    let mut lines = Vec::with_capacity(hunk.lines.len() + 2);
    lines.extend(above);
    lines.extend(hunk.lines.iter().cloned());
    lines.extend(below);
    let alone = FileDiff {
        entry: diff.entry.clone(),
        hunks: vec![Hunk {
            old_start: first,
            old_lines: 0,
            // Alone, the new side starts where the old one does.
            new_start: first,
            new_lines: 0,
            heading: Vec::new(),
            lines,
        }],
        binary: None,
        truncated: None,
        old_mode: diff.old_mode,
        new_mode: diff.new_mode,
    };
    hunk_patch(&alone, 0, &Selection::WholeHunk, PatchDirection::Forward)
        .map_err(|error| unparsable(format!("a staged hunk's patch: {error:?}")))
}
