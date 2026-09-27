//! Conflict forecasts from `git merge-tree`: one merge, or one replayed step.

#[allow(clippy::wildcard_imports)]
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
