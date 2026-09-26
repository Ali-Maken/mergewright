//! One commit's change to a file, as the whole file: every line, that
//! commit's adds and removes in place.

use std::path::Path;

use graft_core::oid::Oid;

use crate::blame::path_bytes;
use crate::diff::{
    DiffError, DiffLine, FileDiff, FileEntry, FileStatus, Hunk, LineKind, Truncated, parse_unified,
};
use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation, Stop};

/// The runner's cap for one file's patch, as `diff::file_diff` uses.
const OUTPUT_CAP: usize = 64 * 1024 * 1024;

/// Context wide enough to hold any file in one hunk. Half of `i32::MAX`, so
/// git's own `2 × context` arithmetic cannot overflow.
const EVERY_LINE: u32 = 1_000_000_000;

/// `commit`'s change to `path` as one hunk from line 1 to the end: context
/// is every line. A merge is diffed against its first parent; a root commit
/// is all adds. `old_path` is the path before, when `commit` renamed it.
/// Over `max_lines` lines (`usize::MAX`: no cap), the hunk is cut there and
/// `truncated` says how much was kept. A commit that did not change the file
/// gives the file as all context.
///
/// This does not call `diff::file_diff`: its `DiffOptions::context` is a
/// `u8`, which cannot say "every line".
///
/// # Errors
/// [`DiffError`]: git failed, its patch did not parse, or it passed the
/// output cap.
pub fn whole_file(
    repo: &RepoHandle,
    commit: Oid,
    path: &Path,
    old_path: Option<&Path>,
    max_lines: usize,
    stop: &dyn Stop,
) -> Result<FileDiff, DiffError> {
    let parent = first_parent(repo, commit, stop)?;
    let mut invocation = GitInvocation::new(repo).args([
        GitArg::Flag("diff-tree"),
        GitArg::Flag("-p"),
        GitArg::Flag("-r"),
        GitArg::Flag("-M"),
        GitArg::Flag("--no-color"),
        GitArg::Flag("--no-ext-diff"),
        GitArg::Count("-U", EVERY_LINE),
    ]);
    invocation = match parent {
        Some(parent) => invocation.args([GitArg::Oid(parent), GitArg::Oid(commit)]),
        None => invocation.args([GitArg::Flag("--root"), GitArg::Oid(commit)]),
    };
    invocation = invocation
        .arg(GitArg::Separator)
        .arg(GitArg::Path(path_bytes(path)));
    if let Some(old) = old_path {
        invocation = invocation.arg(GitArg::Path(path_bytes(old)));
    }
    let output = invocation.output_cap(OUTPUT_CAP).run(stop)?;
    if output.truncated {
        return Err(DiffError::TooLarge { bytes: OUTPUT_CAP });
    }
    let mut diff = match parse_unified(&output.stdout)
        .map_err(DiffError::Parse)?
        .into_iter()
        .next()
    {
        Some(diff) => diff,
        None => unchanged(repo, commit, path, stop)?,
    };
    cut(&mut diff, max_lines);
    Ok(diff)
}

/// `commit`'s first parent; `None` for a root commit.
fn first_parent(repo: &RepoHandle, commit: Oid, stop: &dyn Stop) -> Result<Option<Oid>, DiffError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-list"),
            GitArg::Flag("--parents"),
            GitArg::Flag("-n1"),
            GitArg::Oid(commit),
        ])
        .run(stop)?;
    let text = String::from_utf8_lossy(&output.stdout);
    let Some(parent) = text.split_whitespace().nth(1) else {
        return Ok(None);
    };
    Oid::from_hex(parent).map(Some).map_err(|why| {
        DiffError::Run(crate::run::RunError::Unparsable(format!(
            "rev-list printed {parent:?} as a parent: {why}"
        )))
    })
}

/// The file at `commit`, as one all-context hunk: what a commit that did
/// not touch it looks like.
fn unchanged(
    repo: &RepoHandle,
    commit: Oid,
    path: &Path,
    stop: &dyn Stop,
) -> Result<FileDiff, DiffError> {
    let mut spec = format!("{commit}:").into_bytes();
    spec.extend(path_bytes(path));
    let content = GitInvocation::new(repo)
        .args([
            GitArg::Flag("cat-file"),
            GitArg::Flag("blob"),
            GitArg::Ref(spec),
        ])
        .output_cap(OUTPUT_CAP)
        .run(stop)?;
    if content.truncated {
        return Err(DiffError::TooLarge { bytes: OUTPUT_CAP });
    }
    let mut lines = Vec::new();
    let mut texts: Vec<&[u8]> = content.stdout.split(|&b| b == b'\n').collect();
    let no_final_newline = !texts.last().is_some_and(|last| last.is_empty());
    if !no_final_newline {
        texts.pop();
    }
    let count = texts.len();
    for (index, text) in texts.into_iter().enumerate() {
        let number = u32::try_from(index + 1).unwrap_or(u32::MAX);
        lines.push(DiffLine {
            kind: LineKind::Context,
            text: text.to_vec(),
            old_no: Some(number),
            new_no: Some(number),
            no_newline_at_eof: no_final_newline && index + 1 == count,
        });
    }
    let total = u32::try_from(count).unwrap_or(u32::MAX);
    let hunks = if lines.is_empty() {
        Vec::new()
    } else {
        vec![Hunk {
            old_start: 1,
            old_lines: total,
            new_start: 1,
            new_lines: total,
            heading: Vec::new(),
            lines,
        }]
    };
    Ok(FileDiff {
        entry: FileEntry {
            path: path_bytes(path),
            old_path: None,
            status: FileStatus::Modified,
            added: Some(0),
            removed: Some(0),
        },
        hunks,
        binary: None,
        truncated: None,
        old_mode: None,
        new_mode: None,
    })
}

/// Keeps the first `max_lines` lines of the one hunk, recounting its sides.
fn cut(diff: &mut FileDiff, max_lines: usize) {
    let total_lines: usize = diff.hunks.iter().map(|hunk| hunk.lines.len()).sum();
    if max_lines == usize::MAX || total_lines <= max_lines {
        return;
    }
    let mut left = max_lines;
    for hunk in &mut diff.hunks {
        hunk.lines.truncate(left);
        left -= hunk.lines.len();
        let side = |kind: LineKind| {
            u32::try_from(
                hunk.lines
                    .iter()
                    .filter(|line| line.kind == LineKind::Context || line.kind == kind)
                    .count(),
            )
            .unwrap_or(u32::MAX)
        };
        hunk.old_lines = side(LineKind::Removed);
        hunk.new_lines = side(LineKind::Added);
    }
    diff.hunks.retain(|hunk| !hunk.lines.is_empty());
    diff.truncated = Some(Truncated {
        shown_lines: max_lines,
        total_lines,
    });
}
