//! Where a stopped rewrite is, for M6's banner: the step, the row git is on,
//! the rows still to come, and — with branches following — which branch of
//! the line the step belongs to.
//!
//! Read from git's own files (`rebase-merge/done`, `git-rebase-todo`,
//! `head-name`), never from git's `msgnum`/`end`: those count every todo
//! line, `update-ref` included, and the banner counts only his rows.

use std::io;
use std::path::Path;

use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{GitArg, GitInvocation, RunError, Stop};

use crate::facts::FactsError;

use super::todo::FORK_LABEL;
use super::{EditStop, Follower, ForkStop, RewriteProgress, TodoAction, TodoLine, saved};

/// One line of a todo file git is running.
enum Row {
    /// A row with a commit: pick, reword, edit, squash, fixup, drop.
    Commit(TodoLine),
    /// `update-ref <ref>`: a branch is done — one of the line's, or a fork's
    /// (ADR-0059), which is no line boundary.
    UpdateRef(String),
    /// `label <name>`: a fork's section begins when the name is Mergewright's.
    Label(String),
    /// `reset <name>`: a fork's section ends when the name is Mergewright's.
    Reset(String),
}

/// The label of a fork's section, `graft-fork-<n>`.
fn fork_label(name: &str) -> bool {
    name.starts_with(FORK_LABEL)
}

/// Where the rebase in progress is. `None` when no `rebase -i` is running
/// (an `apply`-style rebase has no `rebase-merge/`).
///
/// A rebase begun in the shell reads the same, with `begun_in_mergewright: false`
/// and no branch.
///
/// # Errors
/// [`FactsError::State`] when git's files cannot be read,
/// [`FactsError::SavedPlan`] when Mergewright's cannot, [`FactsError::History`]
/// when a short id does not resolve; [`FactsError::Stopped`].
pub fn progress(repo: &RepoHandle, stop: &dyn Stop) -> Result<Option<RewriteProgress>, FactsError> {
    if stop.should_stop() {
        return Err(FactsError::Stopped);
    }
    let merge = repo.git_dir().join("rebase-merge");
    if !merge.is_dir() {
        return Ok(None);
    }
    let done = rows(repo, &read_or_empty(&merge.join("done"))?, stop)?;
    let to_do = rows(repo, &read_or_empty(&merge.join("git-rebase-todo"))?, stop)?;
    let head_name = read_or_empty(&merge.join("head-name"))?;
    let head_name = head_name.trim();

    let done_rows: Vec<&TodoLine> = commits(&done).collect();
    let counted = (done_rows.len(), done_rows.len() + commits(&to_do).count());
    let current = done_rows.last().map(|line| (*line).clone());
    let section = open_section(&done);
    let fork = match section {
        Some(label) => fork_stop(repo, label, &done, &to_do, stop)?,
        None => None,
    };
    let pending = pending(&to_do, section.is_some());

    let plans = saved::all(repo).map_err(FactsError::SavedPlan)?;
    let plan = plans.iter().find(|plan| plan.top() == head_name);
    let branch = match plan {
        Some(plan) if plan.line.len() > 1 => {
            let finished = done
                .iter()
                .filter(|row| matches!(row, Row::UpdateRef(name) if in_line(plan, name)))
                .count();
            let index = (finished + 1).min(plan.line.len());
            let (name, _) = &plan.line[index - 1];
            let name = RefName::new(name.as_bytes()).map_err(|_| {
                FactsError::History(RunError::Unparsable(format!(
                    "the saved plan's branch {name:?}"
                )))
            })?;
            Some((name, index, plan.line.len()))
        }
        _ => None,
    };
    let followers = match (plan, &branch) {
        (Some(plan), Some((_, index, _))) => {
            followers(repo, plan, *index, &to_do, section.is_some(), stop)?
        }
        _ => Vec::new(),
    };
    Ok(Some(RewriteProgress {
        step: counted,
        current,
        pending,
        branch,
        begun_in_mergewright: plan.is_some(),
        followers,
        fork,
    }))
}

/// A branch of the saved plan's line, by full name.
fn in_line(plan: &saved::SavedPlan, name: &str) -> bool {
    plan.line.iter().any(|(branch, _)| branch == name)
}

/// The label of the fork section git is in: the last of Mergewright's `label`s
/// in `done` with no `reset` to it after.
fn open_section(done: &[Row]) -> Option<&str> {
    let mut open = None;
    for row in done {
        match row {
            Row::Label(name) if fork_label(name) => open = Some(name.as_str()),
            Row::Reset(name) if open == Some(name.as_str()) => open = None,
            _ => {}
        }
    }
    open
}

/// The fork a section moves: the last `update-ref` before the section's
/// `reset` — the todo puts the fork's own after the branches on it — and
/// where it leaves the line: the parent of the section's first commit row
/// (one `rev-parse`). Read from git's files alone, so a rebase of Mergewright's
/// todo begun in the shell names it too. `None` for a section with no
/// commit row or no `update-ref`.
fn fork_stop(
    repo: &RepoHandle,
    label: &str,
    done: &[Row],
    to_do: &[Row],
    stop: &dyn Stop,
) -> Result<Option<Box<ForkStop>>, FactsError> {
    let since_label = done
        .iter()
        .rposition(|row| matches!(row, Row::Label(name) if name == label))
        .map_or(done, |at| &done[at..]);
    let until_reset = to_do
        .iter()
        .position(|row| matches!(row, Row::Reset(name) if name == label))
        .map_or(to_do, |at| &to_do[..at]);
    let section = || since_label.iter().chain(until_reset);
    let name = section()
        .filter_map(|row| match row {
            Row::UpdateRef(name) => Some(name),
            _ => None,
        })
        .next_back()
        .map(|name| name.strip_prefix("refs/heads/").unwrap_or(name).to_owned());
    let first = section().find_map(|row| match row {
        Row::Commit(line) => Some(line.commit),
        _ => None,
    });
    let (Some(name), Some(first)) = (name, first) else {
        return Ok(None);
    };
    let at = resolve(repo, &format!("{first}^"), stop)?;
    Ok(Some(Box::new(ForkStop { name, at })))
}

/// The rows still to apply, oldest first: the line's, and — when git is in
/// a fork's section (`in_section`) — that section's rest, first. The rows
/// of a section not yet begun are not here (they are counted in `step`):
/// they belong to neither this branch nor a follower of the line.
fn pending(to_do: &[Row], in_section: bool) -> Vec<TodoLine> {
    let mut inside = in_section;
    let mut first = in_section;
    let mut out = Vec::new();
    for row in to_do {
        match row {
            Row::Label(name) if fork_label(name) => inside = true,
            Row::Reset(name) if fork_label(name) => {
                inside = false;
                first = false;
            }
            Row::Commit(line) if !inside || first => out.push(line.clone()),
            _ => {}
        }
    }
    out
}

/// The branches of the line above branch `index` (1-based), bottom up.
///
/// The todo still to run holds this branch's rows, its `update-ref`, then
/// each follower's rows ended by its own `update-ref` — except the top
/// branch (`head-name`), whose rows run to the end. So the k-th segment
/// after the first `update-ref` is follower k's. Authors come from the
/// tips the saved plan recorded before the rewrite: one `git log` spawn.
fn followers(
    repo: &RepoHandle,
    plan: &saved::SavedPlan,
    index: usize,
    to_do: &[Row],
    in_section: bool,
    stop: &dyn Stop,
) -> Result<Vec<Follower>, FactsError> {
    let above = &plan.line[index.min(plan.line.len())..];
    if above.is_empty() {
        return Ok(Vec::new());
    }
    // Commit rows per segment; segment 0 is this branch's. A fork's section
    // (ADR-0059) is no one's segment: its rows and its `update-ref`s are
    // skipped, the one git may be in already included.
    let mut segments = Vec::new();
    let mut current = 0usize;
    let mut in_section = in_section;
    for row in to_do {
        match row {
            Row::Label(name) if fork_label(name) => in_section = true,
            Row::Reset(name) if fork_label(name) => in_section = false,
            Row::Commit(_) if !in_section => current += 1,
            Row::UpdateRef(name) if in_line(plan, name) => {
                segments.push(current);
                current = 0;
            }
            _ => {}
        }
    }
    segments.push(current);
    let tips = above
        .iter()
        .map(|(name, tip)| {
            Oid::from_hex(tip).map_err(|_| {
                FactsError::History(RunError::Unparsable(format!(
                    "the saved plan's tip {tip:?} of {name:?}"
                )))
            })
        })
        .collect::<Result<Vec<Oid>, FactsError>>()?;
    let authors = tip_authors(repo, &tips, stop)?;
    above
        .iter()
        .zip(&tips)
        .enumerate()
        .map(|(k, ((name, _), tip))| {
            let tip_author = authors
                .iter()
                .find(|(id, _)| id == tip)
                .map(|(_, author)| author.clone())
                .ok_or_else(|| {
                    FactsError::History(RunError::Unparsable(format!(
                        "git log printed no author for {name}'s tip {tip}"
                    )))
                })?;
            Ok(Follower {
                branch: name.strip_prefix("refs/heads/").unwrap_or(name).to_owned(),
                tip_author,
                // A todo with fewer `update-ref` lines than the line has
                // branches (a hand `--edit-todo` removed one) leaves the
                // follower no segment: none of its rows are still to come.
                commits: segments.get(k + 1).copied().unwrap_or(0),
            })
        })
        .collect()
}

/// Each tip's author name (`%an`), in one `git log --no-walk` spawn.
fn tip_authors(
    repo: &RepoHandle,
    tips: &[Oid],
    stop: &dyn Stop,
) -> Result<Vec<(Oid, String)>, FactsError> {
    let mut args = vec![
        GitArg::Flag("log"),
        GitArg::Flag("--no-walk=unsorted"),
        GitArg::Flag("--format=%H%x00%an"),
    ];
    args.extend(tips.iter().map(|tip| GitArg::Word(tip.to_string())));
    let output = GitInvocation::new(repo)
        .args(args)
        .run(stop)
        .map_err(FactsError::of_history)?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .map(|line| {
            let unparsable = || {
                FactsError::History(RunError::Unparsable(format!(
                    "git log --format=%H%x00%an printed {line:?}"
                )))
            };
            let (id, author) = line.split_once('\0').ok_or_else(unparsable)?;
            let id = Oid::from_hex(id).map_err(|_| unparsable())?;
            Ok((id, author.to_owned()))
        })
        .collect()
}

/// Whether the rebase in progress stopped on an `edit` row, and on which
/// commit. `Some` iff git's `rebase-merge/amend` exists **and** the last
/// commit row of `rebase-merge/done` is `edit` — the second rules out any
/// other writer of the file. A conflict stop (even on an `edit` row, whose
/// pick conflicted) has no `amend` file and reads `None`; so does no rebase.
///
/// Costs two file reads outside an edit stop, one `rev-list` spawn at one.
///
/// # Errors
/// [`FactsError::State`] when git's files cannot be read,
/// [`FactsError::History`] when the commits' parents cannot;
/// [`FactsError::Stopped`].
pub fn edit_stop(repo: &RepoHandle, stop: &dyn Stop) -> Result<Option<EditStop>, FactsError> {
    if stop.should_stop() {
        return Err(FactsError::Stopped);
    }
    let merge = repo.git_dir().join("rebase-merge");
    let amend = read_or_empty(&merge.join("amend"))?;
    let amend = amend.trim();
    if amend.is_empty() {
        return Ok(None);
    }
    let done = rows(repo, &read_or_empty(&merge.join("done"))?, stop)?;
    let on_edit = commits(&done)
        .last()
        .is_some_and(|line| line.action == TodoAction::Edit);
    if !on_edit {
        return Ok(None);
    }
    let stopped = Oid::from_hex(amend).map_err(|_| {
        FactsError::History(RunError::Unparsable(format!(
            "rebase-merge/amend holds {amend:?}, not a commit id"
        )))
    })?;
    let parents = first_parents(repo, stopped, stop)?;
    let head_on_it = parents.head == parents.stopped;
    Ok(Some(EditStop {
        stopped,
        head_on_it,
    }))
}

/// `HEAD`'s first parent and the stopped commit's; `None` for a root.
struct FirstParents {
    head: Option<Oid>,
    stopped: Option<Oid>,
}

/// One `rev-list --no-walk=unsorted --parents HEAD <stopped>`: each line is
/// a commit and its parents. When `HEAD` *is* the stopped commit git prints
/// it once, so lines are matched by commit, not position.
fn first_parents(
    repo: &RepoHandle,
    stopped: Oid,
    stop: &dyn Stop,
) -> Result<FirstParents, FactsError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-list"),
            GitArg::Flag("--no-walk=unsorted"),
            GitArg::Flag("--parents"),
            GitArg::Flag("HEAD"),
            GitArg::Word(stopped.to_string()),
        ])
        .run(stop)
        .map_err(FactsError::of_history)?;
    let text = String::from_utf8_lossy(&output.stdout);
    let mut lines = Vec::new();
    for line in text.lines() {
        let mut ids = line.split(' ').map(|id| {
            Oid::from_hex(id).map_err(|_| {
                FactsError::History(RunError::Unparsable(format!(
                    "rev-list --parents printed {line:?}"
                )))
            })
        });
        let Some(commit) = ids.next() else { continue };
        lines.push((commit?, ids.next().transpose()?));
    }
    let missing = |what: &str| {
        FactsError::History(RunError::Unparsable(format!(
            "rev-list --parents printed no line for {what}"
        )))
    };
    // `HEAD`'s line is the first git prints: the order asked for.
    let head = lines.first().map(|(_, parent)| *parent);
    let stopped_parent = lines
        .iter()
        .find(|(id, _)| *id == stopped)
        .map(|(_, parent)| *parent);
    Ok(FirstParents {
        head: head.ok_or_else(|| missing("HEAD"))?,
        stopped: stopped_parent.ok_or_else(|| missing("the stopped commit"))?,
    })
}

/// The rows that carry a commit, in file order.
fn commits(rows: &[Row]) -> impl Iterator<Item = &TodoLine> {
    rows.iter().filter_map(|row| match row {
        Row::Commit(line) => Some(line),
        Row::UpdateRef(_) | Row::Label(_) | Row::Reset(_) => None,
    })
}

/// A file git may not have written yet reads as empty.
fn read_or_empty(path: &Path) -> Result<String, FactsError> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(FactsError::State(error)),
    }
}

/// A todo file's rows. Comments, blank lines and the lines Mergewright never
/// writes (`exec`, `break`, `noop`, `merge`) are not rows; `update-ref`,
/// `label` and `reset` are, for the boundaries they mark, never as steps.
fn rows(repo: &RepoHandle, text: &str, stop: &dyn Stop) -> Result<Vec<Row>, FactsError> {
    let mut rows = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
        match word {
            "update-ref" | "u" => {
                rows.push(Row::UpdateRef(rest.trim().to_owned()));
                continue;
            }
            "label" | "l" => {
                rows.push(Row::Label(rest.trim().to_owned()));
                continue;
            }
            "reset" | "t" => {
                rows.push(Row::Reset(rest.trim().to_owned()));
                continue;
            }
            _ => {}
        }
        let Some(action) = TodoAction::from_word(word) else {
            continue;
        };
        // `fixup -C <id>` / `fixup -c <id>`: the flag is not the id.
        let rest = match rest.split_once(' ') {
            Some((flag, after)) if flag.starts_with('-') => after,
            _ => rest,
        };
        let (id, subject) = rest.split_once(' ').unwrap_or((rest, ""));
        if id.is_empty() {
            return Err(FactsError::History(RunError::Unparsable(format!(
                "the todo line {line:?} names no commit"
            ))));
        }
        rows.push(Row::Commit(TodoLine {
            action,
            commit: resolve(repo, id, stop)?,
            subject: subject.trim_start_matches("# ").to_owned(),
        }));
    }
    Ok(rows)
}

/// A todo's commit id: full as git writes it after the editor, short when a
/// hand edit (`--edit-todo`) wrote it so.
fn resolve(repo: &RepoHandle, id: &str, stop: &dyn Stop) -> Result<Oid, FactsError> {
    if let Ok(oid) = Oid::from_hex(id) {
        return Ok(oid);
    }
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-parse"),
            GitArg::Flag("--verify"),
            GitArg::Word(format!("{id}^{{commit}}")),
        ])
        .run(stop)
        .map_err(FactsError::of_history)?;
    let text = String::from_utf8_lossy(&output.stdout);
    Oid::from_hex(text.trim()).map_err(|_| {
        FactsError::History(RunError::Unparsable(format!(
            "the todo's commit {id:?} did not resolve"
        )))
    })
}
