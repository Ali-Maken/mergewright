//! The merge preview's read: what merging `theirs` into `ours` changes, and
//! per clashing file how many places clash.

use super::forecast::{MERGE_TREE_MIN, OLD_GIT_MERGE, tree_id};
#[allow(
    clippy::wildcard_imports,
    reason = "one module split in files: its parent is its prelude"
)]
use super::*;
use crate::run::RawOutput;

/// How many changed paths [`merge_forecast`] lists before it says "and
/// more" (`changed_capped`); the same 50 the operation previews show.
const CHANGED_CAP: usize = 50;

/// Why [`merge_forecast`] is not checkable for two tips with no common
/// ancestor: `git merge` refuses them by default too, so the preview never
/// promises a merge `y` cannot make.
pub const UNRELATED: &str = "the two branches share no history";

/// One file a merge would leave clashing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClashForecast {
    /// The path, raw bytes as in [`Forecast::Conflicts`].
    pub path: PathBuf,
    /// How many places in it clash: the conflict blocks git wrote. `None`
    /// when they could not be counted — no markers (a modify/delete or a
    /// rename clash), a binary file, or markers that did not parse.
    pub places: Option<usize>,
}

/// What merging `theirs` into `ours` would do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeForecast {
    /// The files the merge brings in: `theirs`' changes since the merge
    /// base, `diff-tree` order. Clashing files are listed here too.
    pub changed: Vec<PathBuf>,
    /// `changed` stopped at its cap; more files change.
    pub changed_capped: bool,
    /// The clashing files, in `merge-tree`'s order.
    pub clashes: Vec<ClashForecast>,
}

/// [`merge_forecast`]'s answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MergeCheck {
    /// The forecast was made.
    Checked(MergeForecast),
    /// It could not be — an old git, unrelated histories ([`UNRELATED`]),
    /// or an answer this reader does not recognise. Never a clean bill.
    NotCheckable(String),
}

/// Forecasts merging `theirs` into `ours`: the files it changes and, per
/// clashing file, how many places clash.
///
/// Reads, in order: the version gate ([`forecast_merge`]'s, same reason);
/// [`compare_span`] for the merge base (unrelated → [`UNRELATED`], no
/// merge-tree run); `git merge-tree --write-tree --no-messages -z <ours>
/// <theirs>` *without* `--name-only`, so exit 1 prints the conflicted-file
/// records (`<mode> <oid> <stage>\t<path>`) after the tree id; then
/// [`changed_paths_until`] from the merge base to `theirs`; then, per clashing
/// path, the blob `<tree>:<path>` from the written tree, parsed for its
/// conflict blocks. Like [`forecast_merge`], merge-tree writes the merged
/// tree's objects.
///
/// # Errors
/// [`RunError`] from the runner: git missing or stopped (a raised `stop`
/// spawns nothing). Never for a forecast that could not be made — that is
/// [`MergeCheck::NotCheckable`].
pub fn merge_forecast(
    repo: &RepoHandle,
    git: &GitVersion,
    ours: Oid,
    theirs: Oid,
    stop: &dyn Stop,
) -> Result<MergeCheck, RunError> {
    if *git < MERGE_TREE_MIN {
        return Ok(MergeCheck::NotCheckable(OLD_GIT_MERGE.to_owned()));
    }
    let base = match compare_span(repo, ours, theirs, stop)? {
        CompareSpan::Since { split, .. } => split,
        CompareSpan::NoSharedHistory => return Ok(MergeCheck::NotCheckable(UNRELATED.into())),
    };
    let raw = GitInvocation::new(repo)
        .args([
            GitArg::Flag("merge-tree"),
            GitArg::Flag("--write-tree"),
            GitArg::Flag("--no-messages"),
            // Raw path bytes, never C-quoted (see `forecast_merge`).
            GitArg::Flag("-z"),
            GitArg::Oid(ours),
            GitArg::Oid(theirs),
        ])
        .run_raw(stop)?;
    let (tree, clashing) = match read_records(&raw) {
        Ok(records) => records,
        Err(why) => return Ok(MergeCheck::NotCheckable(why)),
    };

    let (changed, changed_capped) = changed_paths_until(repo, base, theirs, CHANGED_CAP, stop)?;
    let mut clashes = Vec::with_capacity(clashing.len());
    for path in clashing {
        let places = match tree {
            Some(tree) => places_in(repo, tree, path, stop)?,
            None => None,
        };
        clashes.push(ClashForecast {
            path: bytes_to_pathbuf(path),
            places,
        });
    }
    Ok(MergeCheck::Checked(MergeForecast {
        changed,
        changed_capped,
        clashes,
    }))
}

/// merge-tree's answer, read: the written tree (on a clash only; a clean
/// exit has no records to read, so its tree is never needed) and the
/// clashing paths, or the not-checkable reason.
fn read_records(raw: &RawOutput) -> Result<(Option<Oid>, Vec<&[u8]>), String> {
    let mut fields = raw.output.stdout.split(|&b| b == 0);
    let tree = fields.next().filter(|field| !field.is_empty());
    match (raw.code, tree) {
        (Some(0), _) => Ok((None, Vec::new())),
        (Some(1), Some(tree)) => {
            let tree = tree_id(tree)?;
            let paths = conflicted_paths(fields).ok_or_else(|| {
                "merge-tree printed a conflicted-file record this reader does not know".to_owned()
            })?;
            Ok((Some(tree), paths))
        }
        // Exit 1 with nothing on stdout, or any other exit: as
        // `forecast_merge`, not checkable, not an error.
        _ => Err(crate::commit_graph::first_line(&raw.output.stderr)),
    }
}

/// The paths of merge-tree's conflicted-file records (`<mode> <oid>
/// <stage>\t<path>`, one per stage), each path once, first-seen order.
/// `None` when a record has no tab: a shape this reader cannot trust.
fn conflicted_paths<'a>(records: impl Iterator<Item = &'a [u8]>) -> Option<Vec<&'a [u8]>> {
    let mut paths: Vec<&[u8]> = Vec::new();
    for record in records.filter(|record| !record.is_empty()) {
        let tab = record.iter().position(|&b| b == b'\t')?;
        let path = &record[tab + 1..];
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    Some(paths)
}

/// How many conflict blocks the written tree's copy of `path` holds, or
/// `None` when that cannot be said (see [`ClashForecast::places`]).
fn places_in(
    repo: &RepoHandle,
    tree: Oid,
    path: &[u8],
    stop: &dyn Stop,
) -> Result<Option<usize>, RunError> {
    let mut spec = format!("{tree}:").into_bytes();
    spec.extend_from_slice(path);
    let raw = GitInvocation::new(repo)
        .args([
            GitArg::Flag("cat-file"),
            GitArg::Flag("blob"),
            GitArg::Ref(spec),
        ])
        .run_raw(stop)?;
    // A non-zero exit is the path missing from the written tree (git
    // leaves nothing there for some rename clashes): no blob, no count.
    // A truncated blob would be counted short, so it is not counted.
    if raw.code != Some(0) || raw.output.truncated {
        return Ok(None);
    }
    let blob = raw.output.stdout;
    // git's own binary test: a NUL in the content.
    if blob.contains(&0) {
        return Ok(None);
    }
    // A parse error is an answer here, not a failure: markers this parser
    // refuses (a wide `conflict-marker-size`, say) cannot be counted, and
    // the ticket's contract is `None` for that — the clash itself still
    // shows, by path.
    Ok(crate::conflict::parse(&blob)
        .ok()
        .map(|file| file.open())
        .filter(|&open| open > 0))
}

/// The paths `from` and `to` differ on, capped at `cap`.
///
/// `git diff-tree -r --name-only -z <from> <to>`. Never stopped: callers
/// that run it among other stoppable reads use [`changed_paths_until`].
///
/// # Errors
/// [`RunError`] from the runner: git missing, or either id naming no commit
/// in this repository.
pub fn changed_paths(
    repo: &RepoHandle,
    from: Oid,
    to: Oid,
    cap: usize,
) -> Result<(Vec<PathBuf>, bool), RunError> {
    changed_paths_until(repo, from, to, cap, &())
}

/// [`changed_paths`], abandoned once `stop` is raised.
///
/// # Errors
/// As [`changed_paths`], and [`RunError::Stopped`].
pub fn changed_paths_until(
    repo: &RepoHandle,
    from: Oid,
    to: Oid,
    cap: usize,
    stop: &dyn Stop,
) -> Result<(Vec<PathBuf>, bool), RunError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("diff-tree"),
            GitArg::Flag("-r"),
            GitArg::Flag("--name-only"),
            GitArg::Flag("-z"),
            GitArg::Oid(from),
            GitArg::Oid(to),
        ])
        .run(stop)?;

    let mut paths: Vec<&[u8]> = output
        .stdout
        .split(|&b| b == 0)
        .filter(|path| !path.is_empty())
        .collect();
    let capped = paths.len() > cap;
    paths.truncate(cap);
    Ok((paths.into_iter().map(bytes_to_pathbuf).collect(), capped))
}
