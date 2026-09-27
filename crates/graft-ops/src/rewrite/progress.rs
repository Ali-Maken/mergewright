//! Where a stopped rewrite is, for M6's banner: the step, the row git is on,
//! the rows still to come, and — with branches following — which branch of
//! the line the step belongs to.
//!
//! Read from git's own files (`rebase-merge/done`, `git-rebase-todo`,
//! `head-name`), never from git's `msgnum`/`end`: those count every todo
//! line, `update-ref` included, and the banner counts only his rows.

use std::io;
use std::path::Path;

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};

use crate::facts::FactsError;

use super::{EditStop, RewriteProgress, TodoAction, TodoLine, saved};

/// One line of a todo file git is running.
enum Row {
    /// A row with a commit: pick, reword, edit, squash, fixup, drop.
    Commit(TodoLine),
    /// `update-ref <ref>`: one branch of the line is done.
    UpdateRef,
}

/// Where the rebase in progress is. `None` when no `rebase -i` is running
/// (an `apply`-style rebase has no `rebase-merge/`).
///
/// A rebase begun in the shell reads the same, with `begun_in_graft: false`
/// and no branch.
///
/// # Errors
/// [`FactsError::State`] when git's files cannot be read,
/// [`FactsError::SavedPlan`] when Graft's cannot, [`FactsError::History`]
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
    let pending: Vec<TodoLine> = commits(&to_do).cloned().collect();
    let counted = (done_rows.len(), done_rows.len() + pending.len());
    let current = done_rows.last().map(|line| (*line).clone());

    let plans = saved::all(repo).map_err(FactsError::SavedPlan)?;
    let plan = plans.iter().find(|plan| plan.top() == head_name);
    let branch = match plan {
        Some(plan) if plan.line.len() > 1 => {
            let finished = done
                .iter()
                .filter(|row| matches!(row, Row::UpdateRef))
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
    Ok(Some(RewriteProgress {
        step: counted,
        current,
        pending,
        branch,
        begun_in_graft: plan.is_some(),
    }))
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
        Row::UpdateRef => None,
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

/// A todo file's rows. Comments, blank lines and the lines Graft never
/// writes (`exec`, `label`, `reset`, `break`, `noop`) are not rows the
/// banner counts.
fn rows(repo: &RepoHandle, text: &str, stop: &dyn Stop) -> Result<Vec<Row>, FactsError> {
    let mut rows = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
        if word == "update-ref" || word == "u" {
            rows.push(Row::UpdateRef);
            continue;
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
