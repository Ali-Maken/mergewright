//! The history place's jobs (m7-power-tools/21): the trace, and the whole
//! file at one commit.

use std::path::Path;
use std::sync::Arc;

use graft_core::oid::Oid;
use graft_git::diff::{DiffOptions, DiffSpec, FileEntry, FileStatus, file_diff};
use graft_git::repo::RepoHandle;
use graft_git::run::Stop;
use graft_git::trace::{
    LineRange, TraceCommit, blob_sizes, enclosing_function, file_history, line_history,
    map_to_head, whole_file,
};

use super::super::Cancel;
use super::assemble_diff;
use crate::repos::RepoStore;
use crate::update::WholeFileAnswer;
use crate::update::place::{FUNCTION_MAX, TRACE_CAP, TraceAnswer, TraceFailure, TraceRequest};

/// A trace job: for `L`, the function around the line (else the blame
/// block) and the commits that changed it; for `F`, the commits that
/// changed the file. One answer (ruling a).
pub(in crate::jobs) fn read_trace(
    repos: &RepoStore,
    request: &TraceRequest,
    stop: &Cancel,
) -> Result<Arc<TraceAnswer>, TraceFailure> {
    let repo = repos
        .shared()
        .map_err(|error| TraceFailure::Failed(error.to_string()))?;
    let handle = repo.handle();
    let mut rows: Vec<TraceCommit> = Vec::new();
    let mut sink = |batch: Vec<TraceCommit>| rows.extend(batch);
    let (range, end) = match request {
        TraceRequest::Lines {
            at,
            path,
            line,
            block,
            from_disk,
        } => {
            let (line, block) = if *from_disk {
                on_head(&handle, path, *line, block, stop)?
            } else {
                (*line, block.clone())
            };
            let range = match enclosing_function(&handle, *at, path, line, stop)
                .map_err(|error| failed(&error))?
            {
                Some(function) if span(&function) <= FUNCTION_MAX => function,
                // No function around it, or one too long to read as one
                // change (H1): the block blame showed.
                Some(_) | None => block,
            };
            let end = line_history(&handle, *at, path, &range, TRACE_CAP, &mut sink, stop)
                .map_err(|error| failed(&error))?;
            (Some(range), end)
        }
        TraceRequest::File { at, path } => {
            let end = file_history(&handle, *at, path, TRACE_CAP, &mut sink, stop)
                .map_err(|error| failed(&error))?;
            (None, end)
        }
    };
    Ok(Arc::new(TraceAnswer { range, rows, end }))
}

/// A trace error's words on one line: the list draws its failure on one row
/// (`couldn't trace: …`).
fn failed(error: &dyn std::fmt::Display) -> TraceFailure {
    let words = error.to_string();
    // `split` yields at least one piece, even of an empty string.
    let first = words.split('\n').next().unwrap_or(&words);
    TraceFailure::Failed(first.to_owned())
}

/// Lines in `range`, first to last.
fn span(range: &LineRange) -> u32 {
    let (start, end) = range.lines();
    end.saturating_sub(start) + 1
}

/// A working-tree blame's line and block, counted in `HEAD`. The line has
/// no `HEAD` counterpart → [`TraceFailure::Uncommitted`]. A block end that
/// does not map narrows the block to the line.
fn on_head(
    handle: &RepoHandle,
    path: &Path,
    line: u32,
    block: &LineRange,
    stop: &dyn Stop,
) -> Result<(u32, LineRange), TraceFailure> {
    let entry = FileEntry {
        path: path_bytes(path),
        old_path: None,
        status: FileStatus::Modified,
        added: None,
        removed: None,
    };
    let options = DiffOptions {
        context: 0,
        ignore_whitespace: false,
    };
    let diff = file_diff(
        handle,
        &DiffSpec::WorkingTreeVsHead,
        &entry,
        &options,
        usize::MAX,
        stop,
    )
    .map_err(|error| failed(&error))?;
    let mapped = map_to_head(&diff.hunks, line).ok_or(TraceFailure::Uncommitted(line))?;
    let (start, end) = block.lines();
    let block = match (
        map_to_head(&diff.hunks, start),
        map_to_head(&diff.hunks, end),
    ) {
        (Some(start), Some(end)) if start <= mapped && mapped <= end => {
            LineRange::Block { start, end }
        }
        _ => LineRange::Block {
            start: mapped,
            end: mapped,
        },
    };
    Ok((mapped, block))
}

/// The whole file at `commit`, its change against the first parent marked,
/// assembled as the detail pane assembles a diff. `ignore_whitespace`: git's
/// `-w`. A binary file also reads its sizes before and after (21c); a failed
/// size read is carried as its words, the diff standing.
pub(in crate::jobs) fn read_whole_file(
    repos: &RepoStore,
    commit: Oid,
    path: &Path,
    old_path: Option<&Path>,
    max_lines: usize,
    ignore_whitespace: bool,
    stop: &Cancel,
) -> Result<WholeFileAnswer, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let diff = whole_file(
        &handle,
        commit,
        path,
        old_path,
        max_lines,
        ignore_whitespace,
        stop,
    )
    .map_err(|error| error.to_string())?;
    let sizes = diff.binary.is_some().then(|| {
        blob_sizes(&handle, commit, path, old_path, stop).map_err(|error| error.to_string())
    });
    Ok(WholeFileAnswer {
        view: Arc::new(assemble_diff(&diff, true)),
        sizes,
    })
}

/// `path` as git spells it.
#[cfg(unix)]
fn path_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

/// `path` as git spells it.
#[cfg(not(unix))]
fn path_bytes(path: &Path) -> Vec<u8> {
    path.to_string_lossy().replace('\\', "/").into_bytes()
}
