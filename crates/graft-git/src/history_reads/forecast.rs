//! Conflict forecasts from `git merge-tree`: one merge, or one replayed step.

#[allow(
    clippy::wildcard_imports,
    reason = "one module split in files: its parent is its prelude"
)]
use super::*;

/// The oldest git that can run `merge-tree --write-tree` at all.
const MERGE_TREE_MIN: GitVersion = GitVersion {
    major: 2,
    minor: 38,
    patch: 0,
};

/// The oldest git whose `merge-tree` understands `--merge-base`.
const MERGE_TREE_BASE_MIN: GitVersion = GitVersion {
    major: 2,
    minor: 40,
    patch: 0,
};

/// What a hypothetical merge of `theirs` into `ours` would do, per
/// `git merge-tree`.
///
/// **Per commit, not chained** (design note, Hard choice 5): forecasting a
/// whole rebase or cherry-pick means calling this once per replayed commit,
/// each time against its own predecessor — not by building the rebased
/// history through temporary commits, which needs object writes this reader
/// never does. A commit that would conflict against its immediate
/// predecessor but not against the final rebased state (or vice versa) is
/// not distinguished: the wording callers show is "may conflict", never
/// "will".
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Forecast {
    /// No conflicting paths.
    Clean,
    /// These paths would conflict.
    Conflicts {
        /// The conflicting paths, `merge-tree --name-only -z`'s raw bytes
        /// (never git's C-quoted form: see [`forecast_merge`]).
        paths: Vec<PathBuf>,
    },
    /// The forecast could not be made — an old git, or an answer this
    /// reader does not recognise. Never treated as an error: PROMPT.md
    /// §1.4's rule that absence of a check is reported as itself, not as a
    /// pass.
    NotCheckable(String),
}

/// Forecasts merging `theirs` into `ours`, optionally from a given merge
/// base (a rebase or cherry-pick's forecast: `--merge-base=<the replayed
/// commit's own parent>`).
///
/// `git merge-tree --write-tree --name-only --no-messages [--merge-base=<base>]
/// <ours> <theirs>`: exit 0 is a clean merge, exit 1 is a tree id line
/// followed by the conflicting paths, anything else is an error this
/// reader could not make sense of.
///
/// # Errors
/// [`RunError`] from the runner: git missing or stopped. **Never** for a
/// forecast that could not be made — that is [`Forecast::NotCheckable`].
pub fn forecast_merge(
    repo: &RepoHandle,
    git: &GitVersion,
    base: Option<Oid>,
    ours: Oid,
    theirs: Oid,
    stop: &dyn Stop,
) -> Result<Forecast, RunError> {
    if base.is_some() {
        if *git < MERGE_TREE_BASE_MIN {
            return Ok(Forecast::NotCheckable(
                "git 2.40+ needed for a rebase forecast".to_owned(),
            ));
        }
    } else if *git < MERGE_TREE_MIN {
        return Ok(Forecast::NotCheckable(
            "git 2.38+ needed to forecast a merge".to_owned(),
        ));
    }

    let mut args = vec![
        GitArg::Flag("merge-tree"),
        GitArg::Flag("--write-tree"),
        GitArg::Flag("--name-only"),
        GitArg::Flag("--no-messages"),
        // Without `-z`, `--name-only` prints conflicting paths through
        // git's C-quoting: any non-ASCII byte, `"`, `\` or control
        // character comes back as a quoted, octal-escaped literal instead
        // of the path's raw bytes (verified on git 2.54: a conflict in
        // `café.txt` prints `"caf\303\251.txt"`, quotes included). `-z`
        // NUL-terminates every line instead, tree id included, so the
        // same split handles both.
        GitArg::Flag("-z"),
    ];
    if let Some(base) = base {
        args.push(GitArg::Word(format!("--merge-base={base}")));
    }
    args.push(GitArg::Oid(ours));
    args.push(GitArg::Oid(theirs));

    let raw = GitInvocation::new(repo).args(args).run_raw(stop)?;
    let first_stderr_line = crate::commit_graph::first_line(&raw.output.stderr);
    match raw.code {
        Some(0) => Ok(Forecast::Clean),
        Some(1) => {
            let mut fields = raw.output.stdout.split(|&b| b == 0);
            match fields.next() {
                // A tree id in the first field: the rest, if any, are the
                // conflicting paths `--name-only -z` printed, raw bytes.
                Some(tree_id) if !tree_id.is_empty() => Ok(Forecast::Conflicts {
                    // The trailing NUL after the last path leaves one
                    // empty field at the end; drop it (and any other
                    // stray empties) rather than treat it as a path.
                    paths: fields
                        .filter(|field| !field.is_empty())
                        .map(bytes_to_pathbuf)
                        .collect(),
                }),
                // Exit 1 with nothing on stdout is `merge-tree` refusing
                // outright (a bad ref, say) rather than reporting a
                // conflict: the shape this reader knows how to trust is
                // gone, so this is not checkable, not an error.
                _ => Ok(Forecast::NotCheckable(first_stderr_line)),
            }
        }
        _ => Ok(Forecast::NotCheckable(first_stderr_line)),
    }
}

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
/// [`changed_paths`] from the merge base to `theirs`; then, per clashing
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
        return Ok(MergeCheck::NotCheckable(
            "git 2.38+ needed to forecast a merge".to_owned(),
        ));
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
    let first_stderr_line = crate::commit_graph::first_line(&raw.output.stderr);
    let mut fields = raw.output.stdout.split(|&b| b == 0);
    let tree = fields.next().filter(|field| !field.is_empty());
    // A clean exit has no records to read, so its tree is never needed.
    let (tree, clashing) = match (raw.code, tree) {
        (Some(0), _) => (None, Vec::new()),
        (Some(1), Some(tree)) => {
            let Ok(tree) = Oid::from_hex(String::from_utf8_lossy(tree).trim()) else {
                return Ok(MergeCheck::NotCheckable(format!(
                    "merge-tree printed {:?}, not a tree id",
                    String::from_utf8_lossy(tree)
                )));
            };
            let Some(paths) = conflicted_paths(fields) else {
                return Ok(MergeCheck::NotCheckable(
                    "merge-tree printed a conflicted-file record this reader does not know"
                        .to_owned(),
                ));
            };
            (Some(tree), paths)
        }
        // Exit 1 with nothing on stdout, or any other exit: as
        // `forecast_merge`, not checkable, not an error.
        _ => return Ok(MergeCheck::NotCheckable(first_stderr_line)),
    };

    let (changed, changed_capped) = changed_paths(repo, base, theirs, CHANGED_CAP)?;
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

/// One step of a chained replay: what `merge-tree` made of it.
///
/// [`Forecast`] without the tree is what put-back and stash need; a chained
/// rebase forecast (m7/05) needs the clean result's tree too, to make the
/// next step's `ours` from it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MergeStep {
    /// No conflicting paths; `tree` is the merged result.
    Clean {
        /// The tree `merge-tree --write-tree` wrote.
        tree: Oid,
    },
    /// These paths would conflict (raw bytes, as in [`Forecast::Conflicts`]).
    Conflicts {
        /// The conflicting paths.
        paths: Vec<PathBuf>,
    },
    /// Not checkable: an old git, or an answer this reader does not
    /// recognise. Never a clean bill.
    NotCheckable(String),
}

/// Replays `theirs` onto `ours` from `base` (the replayed commit's own
/// parent), the way one rebase step does, and returns the result tree.
///
/// `git merge-tree --write-tree --name-only --no-messages -z
/// --merge-base=<base> <ours> <theirs>` — the read [`forecast_merge`] runs,
/// with the tree id kept on a clean exit. `ours` may be a commit with no
/// ref (a `commit-tree` of the previous step's tree): git answers as for
/// the real commit (`forecast_merge_accepts_a_commit_tree_result_as_ours`).
/// git 2.40+ (`--merge-base`); older → [`MergeStep::NotCheckable`].
///
/// # Errors
/// [`RunError`] from the runner: git missing or stopped.
pub fn merge_tree_step(
    repo: &RepoHandle,
    git: &GitVersion,
    base: Oid,
    ours: Oid,
    theirs: Oid,
    stop: &dyn Stop,
) -> Result<MergeStep, RunError> {
    if *git < MERGE_TREE_BASE_MIN {
        return Ok(MergeStep::NotCheckable(
            "git 2.40+ needed for a rebase forecast".to_owned(),
        ));
    }
    let raw = GitInvocation::new(repo)
        .args([
            GitArg::Flag("merge-tree"),
            GitArg::Flag("--write-tree"),
            GitArg::Flag("--name-only"),
            GitArg::Flag("--no-messages"),
            GitArg::Flag("-z"),
            GitArg::Word(format!("--merge-base={base}")),
            GitArg::Oid(ours),
            GitArg::Oid(theirs),
        ])
        .run_raw(stop)?;
    let first_stderr_line = crate::commit_graph::first_line(&raw.output.stderr);
    let mut fields = raw.output.stdout.split(|&b| b == 0);
    let tree = fields.next().filter(|field| !field.is_empty());
    match (raw.code, tree) {
        (Some(0), Some(tree)) => match Oid::from_hex(String::from_utf8_lossy(tree).trim()) {
            Ok(tree) => Ok(MergeStep::Clean { tree }),
            Err(_) => Ok(MergeStep::NotCheckable(format!(
                "merge-tree printed {:?}, not a tree id",
                String::from_utf8_lossy(tree)
            ))),
        },
        (Some(1), Some(_)) => Ok(MergeStep::Conflicts {
            paths: fields
                .filter(|field| !field.is_empty())
                .map(bytes_to_pathbuf)
                .collect(),
        }),
        _ => Ok(MergeStep::NotCheckable(first_stderr_line)),
    }
}
