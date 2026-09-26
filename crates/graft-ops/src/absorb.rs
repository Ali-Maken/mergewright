//! Absorb (M7 note "Pass 2a" R2; git-absorb, jj): each staged hunk goes to
//! the commit on his branch that last changed its lines, and a hunk that
//! fits none stays staged, with the reason in words.
//!
//! How a hunk is placed, against `base..HEAD`:
//! - its old lines are blamed at `HEAD`, restricted to the range
//!   (`git blame --porcelain -L … <base>..<HEAD>`); every one blamed to one
//!   commit C in the range → C;
//! - a pure insertion looks at the line above and the line below instead:
//!   both must be C's;
//! - lines blamed only to commits below the range → [`StaysWhy::BeforeYourBranch`];
//!   to two or more commits → [`StaysWhy::TwoCommitsChangedIt`]; a new file →
//!   [`StaysWhy::NewFile`]; anything else (a binary or mode-only change, an
//!   insertion at either end of a file) → [`StaysWhy::NoCommitChangedIt`];
//! - a hunk placed in C must also apply on C's own tree (the commutation
//!   check): `git apply --cached --check` on a temporary index. It fails →
//!   the hunk stays, [`StaysWhy::TwoCommitsChangedIt`] — a later commit
//!   changed the lines next to it.
//!
//! Each hunk is carried as a patch with **one line of `HEAD`'s content on
//! either side** as context, not as the bare `-U0` hunk. Why: git's replay
//! is a three-way merge, and a merge conflicts on a change next to another
//! one. The context line makes the commutation check fail exactly there (a
//! later commit changed the neighbouring line, so C's tree holds a different
//! one), where a bare `-U0` hunk would apply anywhere its own lines match —
//! and git places a context-free insertion by line number alone.
//!
//! Nothing here writes a ref, the index or a file. Placing hunks into
//! commits ([`commit_pieces`]) writes objects only, for `rewrite::plan`.

use std::collections::BTreeSet;
use std::ops::Range;
use std::path::PathBuf;

use graft_core::oid::Oid;
use graft_git::diff::{
    DiffLine, FileDiff, FileStatus, Hunk, LineKind, PatchDirection, Selection, hunk_patch,
    parse_unified,
};
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};
use graft_git::version::GitVersion;

use crate::facts::FactsError;
use crate::operation::Tip;
use crate::snapshot::{TempIndex, path_of};

/// One staged hunk: its file and its two sides, as git's `-U0` header says
/// them — `start..start + count`. An empty side sits at the line it follows
/// (`@@ -2,0 +3 @@` is `old: 2..2, new: 3..4`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagedHunk {
    /// The file, relative to the working tree.
    pub path: PathBuf,
    /// Its lines in `HEAD`'s file.
    pub old: Range<u32>,
    /// Its lines in the staged file.
    pub new: Range<u32>,
}

/// Why a staged hunk stays staged: the words the preview shows after
/// `stays staged ·`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaysWhy {
    /// No commit on his branch changed these lines (a binary or mode-only
    /// change; an insertion at the top or bottom of a file).
    NoCommitChangedIt,
    /// Two or more commits changed these lines, or a later commit changed
    /// the lines next to them.
    TwoCommitsChangedIt,
    /// A new file: no commit has it.
    NewFile,
    /// The lines are older than his branch.
    BeforeYourBranch,
}

/// Where each staged hunk goes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AbsorbMap {
    /// Each hunk that is placed, with the commit that gains it, in the
    /// staged diff's order.
    pub placed: Vec<(StagedHunk, Oid)>,
    /// Each hunk that stays staged, and why.
    pub stays: Vec<(StagedHunk, StaysWhy)>,
    /// The hunks that stay come back **unstaged**: the rewrite sets the
    /// edits aside and puts them back without `--index` (ADR-0045 §4), so a
    /// staged change to a file `HEAD` has loses its staging. A new file
    /// comes back staged. The preview says so.
    pub stays_unstaged: bool,
}

/// Where each staged hunk goes, in `base..HEAD` (`head` is `HEAD`'s branch
/// and tip). Reads only.
///
/// # Errors
/// [`FactsError`] when a read fails; [`FactsError::Stopped`] when `stop`
/// asks.
pub fn absorb_map(
    repo: &RepoHandle,
    _git: &GitVersion,
    head: &Tip,
    base: Oid,
    stop: &dyn Stop,
) -> Result<AbsorbMap, FactsError> {
    let mapped = mapped(repo, head.oid, base, stop)?;
    Ok(AbsorbMap {
        placed: mapped
            .placed
            .into_iter()
            .map(|(piece, target)| (piece.hunk, target))
            .collect(),
        stays: mapped
            .stays
            .into_iter()
            .map(|(piece, why)| (piece.hunk, why))
            .collect(),
        stays_unstaged: mapped.stays_unstaged,
    })
}

/// A staged hunk and the one-hunk patch that puts it in (with a line of
/// `HEAD`'s content either side, see the module doc).
pub(crate) struct Piece {
    pub(crate) hunk: StagedHunk,
    patch: Vec<u8>,
}

/// [`AbsorbMap`] with each hunk's patch kept, for `rewrite::plan`.
pub(crate) struct Mapped {
    pub(crate) placed: Vec<(Piece, Oid)>,
    pub(crate) stays: Vec<(Piece, StaysWhy)>,
    pub(crate) stays_unstaged: bool,
}

/// [`absorb_map`], with the patches.
pub(crate) fn mapped(
    repo: &RepoHandle,
    head: Oid,
    base: Oid,
    stop: &dyn Stop,
) -> Result<Mapped, FactsError> {
    if stop.should_stop() {
        return Err(FactsError::Stopped);
    }
    let range = commits_in(repo, base, head, stop)?;
    let mut placed = Vec::new();
    let mut stays = Vec::new();
    for diff in staged(repo, head, stop)? {
        let path = path_of(diff.entry.path.clone());
        if diff.entry.status == FileStatus::Added {
            for hunk in &diff.hunks {
                stays.push((bare(&path, hunk), StaysWhy::NewFile));
            }
            if diff.hunks.is_empty() {
                stays.push((empty(&path), StaysWhy::NewFile));
            }
            continue;
        }
        if diff.binary.is_some() || diff.hunks.is_empty() {
            stays.push((empty(&path), StaysWhy::NoCommitChangedIt));
            continue;
        }
        let file = FileAtHead::read(repo, head, &diff.entry.path, stop)?;
        let blamed = blame(repo, base, head, &diff, file.lines.len(), stop)?;
        for hunk in &diff.hunks {
            let piece = Piece {
                hunk: staged_hunk(&path, hunk),
                patch: with_context(&diff, hunk, &file)?,
            };
            match verdict(hunk, file.lines.len(), &blamed, &range)? {
                Ok(target) if commutes(repo, target, &piece.patch, stop)? => {
                    placed.push((piece, target));
                }
                Ok(_) => stays.push((piece, StaysWhy::TwoCommitsChangedIt)),
                Err(why) => stays.push((piece, why)),
            }
        }
    }
    let stays_unstaged = stays.iter().any(|(_, why)| *why != StaysWhy::NewFile);
    Ok(Mapped {
        placed,
        stays,
        stays_unstaged,
    })
}

/// `HEAD`'s tree plus `pieces`, committed with no ref (parent `head`):
/// the commit a `fixup` row melds into its target. Built on a temporary
/// index; the real one is never written.
pub(crate) fn commit_pieces(
    repo: &RepoHandle,
    head: Oid,
    pieces: &[&Piece],
    subject: &str,
    stop: &dyn Stop,
) -> Result<Oid, FactsError> {
    let index = TempIndex::fresh(repo.git_dir())?;
    with_index(repo, &index)
        .args([GitArg::Flag("read-tree"), GitArg::Oid(head)])
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
    let tree = parse_oid(String::from_utf8_lossy(&tree.stdout).trim())?;
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

// ── Reads ────────────────────────────────────────────────────────────────

/// The staged diff, `-U0`, one entry per file. Plumbing `diff-index`
/// rather than porcelain `git diff --cached`: the porcelain refreshes
/// `.git/index` (a read must not write it) and follows the user's
/// `diff.*` settings. `--no-renames`: a rename is its deletion and a new
/// file, so the new side is never placed into a commit that had the old.
fn staged(repo: &RepoHandle, head: Oid, stop: &dyn Stop) -> Result<Vec<FileDiff>, FactsError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("diff-index"),
            GitArg::Flag("--cached"),
            GitArg::Flag("-p"),
            GitArg::Count("-U", 0),
            GitArg::Flag("--no-color"),
            GitArg::Flag("--no-ext-diff"),
            GitArg::Flag("--no-renames"),
            GitArg::Oid(head),
        ])
        .run(stop)
        .map_err(FactsError::of_history)?;
    parse_unified(&output.stdout).map_err(|error| unparsable(format!("the staged diff: {error:?}")))
}

/// The commits in `base..head`.
fn commits_in(
    repo: &RepoHandle,
    base: Oid,
    head: Oid,
    stop: &dyn Stop,
) -> Result<BTreeSet<Oid>, FactsError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-list"),
            GitArg::Oid(head),
            GitArg::Word(format!("^{base}")),
        ])
        .run(stop)
        .map_err(FactsError::of_history)?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(parse_oid)
        .collect()
}

/// A file's lines at `HEAD`, for the context lines and for knowing where
/// it ends.
struct FileAtHead {
    lines: Vec<Vec<u8>>,
    /// The last line has its `\n`.
    ends_with_newline: bool,
}

impl FileAtHead {
    fn read(
        repo: &RepoHandle,
        head: Oid,
        path: &[u8],
        stop: &dyn Stop,
    ) -> Result<FileAtHead, FactsError> {
        let mut spec = format!("{head}:").into_bytes();
        spec.extend_from_slice(path);
        let output = GitInvocation::new(repo)
            .args([
                GitArg::Flag("cat-file"),
                GitArg::Flag("blob"),
                GitArg::Ref(spec),
            ])
            .run(stop)
            .map_err(FactsError::of_history)?;
        let bytes = output.stdout;
        let ends_with_newline = bytes.last().is_none_or(|last| *last == b'\n');
        let body = bytes.strip_suffix(b"\n").unwrap_or(&bytes);
        let lines = if bytes.is_empty() {
            Vec::new()
        } else {
            body.split(|byte| *byte == b'\n')
                .map(<[u8]>::to_vec)
                .collect()
        };
        Ok(FileAtHead {
            lines,
            ends_with_newline,
        })
    }
}

/// Which commit each line a hunk of `diff` asks about was last changed in,
/// by `HEAD`'s line number. One `blame` per file, every range at once.
fn blame(
    repo: &RepoHandle,
    base: Oid,
    head: Oid,
    diff: &FileDiff,
    line_count: usize,
    stop: &dyn Stop,
) -> Result<std::collections::BTreeMap<u32, Oid>, FactsError> {
    let count = u32::try_from(line_count).unwrap_or(u32::MAX);
    let wanted: BTreeSet<u32> = diff
        .hunks
        .iter()
        .flat_map(asks_about)
        .filter(|line| (1..=count).contains(line))
        .collect();
    if wanted.is_empty() {
        return Ok(std::collections::BTreeMap::new());
    }
    let mut args = vec![GitArg::Flag("blame"), GitArg::Flag("--porcelain")];
    for run in runs(&wanted) {
        args.push(GitArg::Flag("-L"));
        args.push(GitArg::Word(format!("{},{}", run.start, run.end - 1)));
    }
    args.push(GitArg::Word(format!("{base}..{head}")));
    args.push(GitArg::Path(diff.entry.path.clone()));
    let output = GitInvocation::new(repo)
        .args(args)
        .run(stop)
        .map_err(FactsError::of_history)?;
    parse_blame(&String::from_utf8_lossy(&output.stdout))
}

/// The lines of `HEAD`'s file a hunk is judged by: its old lines, or for a
/// pure insertion the line above and the line below.
fn asks_about(hunk: &Hunk) -> Vec<u32> {
    if hunk.old_lines == 0 {
        vec![hunk.old_start, hunk.old_start + 1]
    } else {
        (hunk.old_start..hunk.old_start + hunk.old_lines).collect()
    }
}

/// Contiguous runs of line numbers, as `-L` ranges want them.
fn runs(lines: &BTreeSet<u32>) -> Vec<Range<u32>> {
    let mut runs: Vec<Range<u32>> = Vec::new();
    for &line in lines {
        match runs.last_mut() {
            Some(run) if run.end == line => run.end = line + 1,
            _ => runs.push(line..line + 1),
        }
    }
    runs
}

/// `blame --porcelain`: each line's header is `<id> <orig> <final> [<n>]`;
/// the commit's details and the `\t`-led content follow and are skipped.
fn parse_blame(text: &str) -> Result<std::collections::BTreeMap<u32, Oid>, FactsError> {
    let mut blamed = std::collections::BTreeMap::new();
    for line in text.lines() {
        if line.starts_with('\t') {
            continue;
        }
        let mut words = line.split(' ');
        let (Some(id), Some(_orig), Some(last)) = (words.next(), words.next(), words.next()) else {
            continue;
        };
        let Ok(commit) = Oid::from_hex(id) else {
            // A detail line (`author …`, `filename …`, `boundary`).
            continue;
        };
        let last = last
            .parse::<u32>()
            .map_err(|_| unparsable(format!("blame line {line:?}")))?;
        blamed.insert(last, commit);
    }
    Ok(blamed)
}

// ── Judging one hunk ────────────────────────────────────────────────────

/// The commit a hunk goes to, or why it stays. The outer `Err` is a blame
/// that did not cover a line it was asked about.
fn verdict(
    hunk: &Hunk,
    line_count: usize,
    blamed: &std::collections::BTreeMap<u32, Oid>,
    range: &BTreeSet<Oid>,
) -> Result<Result<Oid, StaysWhy>, FactsError> {
    let count = u32::try_from(line_count).unwrap_or(u32::MAX);
    let lines = asks_about(hunk);
    if lines.iter().any(|line| !(1..=count).contains(line)) {
        // An insertion at the top or the bottom: one neighbour is missing.
        return Ok(Err(StaysWhy::NoCommitChangedIt));
    }
    let commits = lines
        .iter()
        .map(|line| {
            blamed
                .get(line)
                .copied()
                .ok_or_else(|| unparsable(format!("blame gave no commit for line {line}")))
        })
        .collect::<Result<BTreeSet<Oid>, FactsError>>()?;
    if commits.iter().all(|commit| !range.contains(commit)) {
        return Ok(Err(StaysWhy::BeforeYourBranch));
    }
    match commits.iter().next() {
        Some(only) if commits.len() == 1 => Ok(Ok(*only)),
        _ => Ok(Err(StaysWhy::TwoCommitsChangedIt)),
    }
}

/// `patch` applies on `target`'s own tree. A temporary index; the real one
/// is never read or written.
fn commutes(
    repo: &RepoHandle,
    target: Oid,
    patch: &[u8],
    stop: &dyn Stop,
) -> Result<bool, FactsError> {
    let index = TempIndex::fresh(repo.git_dir())?;
    with_index(repo, &index)
        .args([GitArg::Flag("read-tree"), GitArg::Oid(target)])
        .run(stop)
        .map_err(FactsError::of_history)?;
    let checked = with_index(repo, &index)
        .args([
            GitArg::Flag("apply"),
            GitArg::Flag("--cached"),
            GitArg::Flag("--check"),
            GitArg::Flag("-"),
        ])
        .stdin(patch.to_vec())
        .run(stop);
    match checked {
        Ok(_) => Ok(true),
        // git's "patch does not apply".
        Err(RunError::Exited { status: 1, .. }) => Ok(false),
        Err(other) => Err(FactsError::of_history(other)),
    }
}

// ── Building ─────────────────────────────────────────────────────────────

fn with_index(repo: &RepoHandle, index: &TempIndex) -> GitInvocation {
    GitInvocation::new(repo).env("GIT_INDEX_FILE", index.path())
}

fn staged_hunk(path: &std::path::Path, hunk: &Hunk) -> StagedHunk {
    StagedHunk {
        path: path.to_path_buf(),
        old: hunk.old_start..hunk.old_start + hunk.old_lines,
        new: hunk.new_start..hunk.new_start + hunk.new_lines,
    }
}

/// A hunk that is never applied (a new file's): no patch.
fn bare(path: &std::path::Path, hunk: &Hunk) -> Piece {
    Piece {
        hunk: staged_hunk(path, hunk),
        patch: Vec::new(),
    }
}

/// A file with no hunks to name (binary, mode only, an empty new file).
fn empty(path: &std::path::Path) -> Piece {
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
fn with_context(diff: &FileDiff, hunk: &Hunk, file: &FileAtHead) -> Result<Vec<u8>, FactsError> {
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

fn parse_oid(hex: &str) -> Result<Oid, FactsError> {
    Oid::from_hex(hex).map_err(|_| unparsable(format!("{hex:?} is not a commit id")))
}

fn unparsable(what: String) -> FactsError {
    FactsError::History(RunError::Unparsable(what))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_join_neighbouring_lines() {
        let lines: BTreeSet<u32> = [1, 2, 3, 5, 7, 8].into_iter().collect();
        assert_eq!(runs(&lines), vec![1..4, 5..6, 7..9]);
    }

    #[test]
    fn porcelain_blame_reads_each_final_line() {
        let a = "a".repeat(40);
        let b = "b".repeat(40);
        let text = format!(
            "{a} 1 3 2\nauthor x\nfilename f\n\tline three\n{a} 2 4\n\tline four\n\
             {b} 9 5 1\nauthor y\nboundary\nfilename f\n\tline five\n"
        );
        let blamed = parse_blame(&text).expect("it parses");
        let a = Oid::from_hex(&a).expect("an id");
        let b = Oid::from_hex(&b).expect("an id");
        assert_eq!(
            blamed.into_iter().collect::<Vec<_>>(),
            vec![(3, a), (4, a), (5, b)]
        );
    }
}
