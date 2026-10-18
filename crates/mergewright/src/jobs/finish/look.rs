//! The review's read-only look (ease-pass/12b): a solved file as it will
//! land, or what git combined on its own — each side's line counts since
//! the merge base and one sample marked by side.

use std::path::{Path, PathBuf};

use mergewright_core::oid::Oid;
use mergewright_git::diff::{
    DiffOptions, DiffSpec, FileDiff, FileEntry, LineKind, file_diff, line_origins, list_files,
};
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::Stop;

use crate::update::detail::MAX_LINES;
use crate::update::finish::{LookRead, Sample, SideCounts};

/// The result of a solved, still-unmerged file: the worktree text. Solved
/// files stay unmerged until `y` stages them (05b), so the index has no
/// answer yet; the file on disk is what `y` will stage.
///
/// # Errors
/// No worktree, or the file not reading, as one line.
pub fn look_result(repo: &RepoHandle, path: &Path) -> Result<String, String> {
    let root = repo
        .workdir()
        .ok_or_else(|| "this repository has no working tree".to_owned())?;
    let bytes = std::fs::read(root.join(path)).map_err(|error| error.to_string())?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Per combined path: lines added and deleted on each side since the merge
/// base (`git diff --numstat` merge-base→`HEAD` and merge-base→`MERGE_HEAD`),
/// and one sample — the first path changed on both sides, else the first
/// path — with its lines marked by side (`line_origins`).
///
/// # Errors
/// `HEAD`, `MERGE_HEAD` or the merge base not reading, or a diff failing.
pub fn combined_look(
    repo: &RepoHandle,
    paths: &[PathBuf],
    stop: &dyn Stop,
) -> Result<(Vec<SideCounts>, Option<Sample>), String> {
    let (base, head, theirs) = ends(repo)?;
    let options = DiffOptions::default();
    let yours_spec = DiffSpec::Range { base, head };
    let theirs_spec = DiffSpec::Range { base, head: theirs };
    let yours = list_files(repo, &yours_spec, &options, stop).map_err(|e| e.to_string())?;
    let theirs_list = list_files(repo, &theirs_spec, &options, stop).map_err(|e| e.to_string())?;
    let counts: Vec<SideCounts> = paths
        .iter()
        .map(|path| SideCounts {
            path: path.clone(),
            yours: lines_of(entry(&yours.files, path)),
            theirs: lines_of(entry(&theirs_list.files, path)),
        })
        .collect();
    let changed = |side: (u32, u32)| side != (0, 0);
    let pick = counts
        .iter()
        .find(|c| changed(c.yours) && changed(c.theirs))
        .or_else(|| counts.first());
    let sample = match pick {
        Some(chosen) => sample(
            repo,
            &chosen.path,
            [
                (&yours_spec, &yours.files),
                (&theirs_spec, &theirs_list.files),
            ],
            stop,
        )?,
        None => None,
    };
    Ok((counts, sample))
}

/// The merge base, `HEAD` and `MERGE_HEAD`.
fn ends(repo: &RepoHandle) -> Result<(Oid, Oid, Oid), String> {
    let head = repo
        .head_commit()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "HEAD names no commit".to_owned())?;
    let detail = mergewright_git::state::in_progress_detail(repo)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "no merge is in progress".to_owned())?;
    let theirs = detail
        .theirs
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "MERGE_HEAD names no commit".to_owned())?
        .id;
    let base = mergewright_git::history_reads::merge_base(repo, head, theirs)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "the two sides share no history".to_owned())?;
    Ok((base, head, theirs))
}

fn entry<'a>(files: &'a [FileEntry], path: &Path) -> Option<&'a FileEntry> {
    let want = path.as_os_str().as_encoded_bytes();
    files.iter().find(|entry| entry.path == want)
}

/// `(added, deleted)`; a binary file or no entry counts 0.
fn lines_of(entry: Option<&FileEntry>) -> (u32, u32) {
    entry.map_or((0, 0), |e| (e.added.unwrap_or(0), e.removed.unwrap_or(0)))
}

type Side<'a> = (&'a DiffSpec, &'a Vec<FileEntry>);

/// `path`'s sample: the result's first hunk, each line it adds over the
/// base marked by the side `line_origins` says it came from.
///
/// The result is the file on disk (git wrote the combined text there). With
/// theirs changed, the hunk is `HEAD`→disk — theirs' lines brought onto
/// yours, yours' lines in its context; with only yours changed, disk equals
/// `HEAD` and the hunk is base→`HEAD`. Every line of the hunk is offered to
/// `line_origins` as a result line, so yours' lines in its context are
/// matched too; a line neither side added stays plain.
fn sample(
    repo: &RepoHandle,
    path: &Path,
    [yours, theirs]: [Side<'_>; 2],
    stop: &dyn Stop,
) -> Result<Option<Sample>, String> {
    let options = DiffOptions::default();
    let read = |spec: &DiffSpec, files: &[FileEntry]| -> Result<Option<FileDiff>, String> {
        entry(files, path)
            .map(|e| file_diff(repo, spec, e, &options, MAX_LINES, stop).map_err(|x| x.to_string()))
            .transpose()
    };
    let yours_diff = read(yours.0, yours.1)?;
    let theirs_diff = read(theirs.0, theirs.1)?;
    let result = if theirs_diff.is_some() {
        let disk = list_files(repo, &DiffSpec::WorkingTreeVsHead, &options, stop)
            .map_err(|e| e.to_string())?;
        read(&DiffSpec::WorkingTreeVsHead, &disk.files)?
    } else {
        yours_diff.clone()
    };
    let Some(mut result) = result else {
        return Ok(None);
    };
    result.hunks.truncate(1);
    let Some(hunk) = result.hunks.first_mut() else {
        return Ok(None);
    };
    hunk.lines.retain(|line| line.kind != LineKind::Removed);
    for line in &mut hunk.lines {
        line.kind = LineKind::Added;
    }
    let empty = FileDiff {
        hunks: Vec::new(),
        ..result.clone()
    };
    let origins = line_origins(
        &result,
        yours_diff.as_ref().unwrap_or(&empty),
        theirs_diff.as_ref().unwrap_or(&empty),
    );
    let lines = result.hunks[0]
        .lines
        .iter()
        .filter_map(|line| {
            let n = line.new_no?;
            let origin = origins.iter().find(|o| o.line == n).map(|o| o.origin);
            Some((n, String::from_utf8_lossy(&line.text).into_owned(), origin))
        })
        .collect();
    Ok(Some(Sample {
        path: path.to_path_buf(),
        lines,
    }))
}

/// The look's read for one job: a clashed file's result or the combined
/// files'.
pub(in crate::jobs) fn read(
    repos: &crate::repos::RepoStore,
    ask: &crate::update::finish::LookAsk,
    stop: &dyn Stop,
) -> Result<LookRead, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    match ask {
        crate::update::finish::LookAsk::Clashed(path) => {
            look_result(&handle, path).map(LookRead::Clashed)
        }
        crate::update::finish::LookAsk::Combined(paths) => combined_look(&handle, paths, stop)
            .map(|(files, sample)| LookRead::Combined { files, sample }),
    }
}

#[cfg(test)]
mod tests;
