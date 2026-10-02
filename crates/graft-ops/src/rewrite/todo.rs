//! The pure half of a rewrite: which rows meld into which, the message a
//! squash group gets unless he writes one, what cannot run, the autosquash
//! marks, and the todo text git is handed.

use std::collections::BTreeMap;

use graft_core::oid::Oid;

use super::{ForkSection, SquashGroup, TodoAction, TodoLine, TodoProblem};

/// Every squash group in the todo: a kept row (pick, reword or edit) and the
/// squash and fixup rows melded into it, in todo order of the kept rows.
///
/// A melding row melds into the nearest kept row below it — older, earlier
/// in the todo — passing over dropped rows, as git does: a dropped row
/// writes no commit to meld into. A melding row with no kept row below it is
/// in no group; [`validate`] refuses it. A kept row nothing melds into is
/// not a group.
#[must_use]
pub fn squash_groups(todo: &[TodoLine]) -> Vec<SquashGroup> {
    let mut groups: Vec<SquashGroup> = Vec::new();
    let mut kept: Option<usize> = None;
    for (index, line) in todo.iter().enumerate() {
        if line.action.keeps() {
            kept = Some(index);
        } else if line.action.melds()
            && let Some(into) = kept
        {
            match groups.last_mut() {
                Some(group) if group.kept == into => group.melded.push(index),
                _ => groups.push(SquashGroup {
                    kept: into,
                    melded: vec![index],
                }),
            }
        }
    }
    for group in &mut groups {
        group.melded.reverse();
    }
    groups
}

/// The message a squash group gets unless he writes his own: the kept row's
/// whole message, a blank line, then each melded **squash** row's message,
/// newest first. A fixup row adds nothing (git's rule). Pass 1b found the
/// opposite order — git's own, oldest first — surprised him after the fact.
///
/// `message_of` is a todo index → that commit's whole message.
#[must_use]
pub fn default_message(
    group: &SquashGroup,
    message_of: &dyn Fn(usize) -> String,
    todo: &[TodoLine],
) -> String {
    let squashed = group
        .melded
        .iter()
        .filter(|&&row| todo.get(row).map(|line| line.action) == Some(TodoAction::Squash));
    let parts: Vec<String> = std::iter::once(&group.kept)
        .chain(squashed)
        .map(|&row| message_of(row).trim_end().to_owned())
        .collect();
    let mut message = parts.join("\n\n");
    message.push('\n');
    message
}

/// Whether git can run this todo: every melding row has a kept row below
/// it, and at least one row keeps a commit.
///
/// # Errors
/// [`TodoProblem::MeldsIntoNothing`] for the first melding row with nothing
/// below it; [`TodoProblem::EverythingDropped`] when no row keeps a commit.
pub fn validate(todo: &[TodoLine]) -> Result<(), TodoProblem> {
    let mut kept = false;
    for (row, line) in todo.iter().enumerate() {
        if line.action.keeps() {
            kept = true;
        } else if line.action.melds() && !kept {
            return Err(TodoProblem::MeldsIntoNothing { row });
        }
    }
    if kept {
        Ok(())
    } else {
        Err(TodoProblem::EverythingDropped)
    }
}

/// `fixup! X` and `squash! X` rows pre-marked `Fixup`/`Squash` and moved
/// directly above X — right after it, oldest first — after any earlier ones
/// moved there, which is where `git rebase --autosquash` puts them.
/// Nothing is applied unseen: the rows come back marked, for the editor to
/// show (lazygit #4664), and `p` un-marks them.
///
/// X is the oldest earlier row whose subject is the rest of the line (any
/// further `fixup! `/`squash! ` prefixes stripped, as git does), or whose id
/// starts with it. A marker naming nothing earlier stays a pick.
pub(super) fn autosquash(todo: Vec<TodoLine>) -> Vec<TodoLine> {
    let mut out: Vec<TodoLine> = Vec::with_capacity(todo.len());
    // For each row of `out`: the row it was moved above, if it was.
    let mut moved_to: Vec<Option<Oid>> = Vec::with_capacity(todo.len());
    for mut line in todo {
        let Some((action, target)) = marker(&line.subject) else {
            out.push(line);
            moved_to.push(None);
            continue;
        };
        let Some(at) = out.iter().position(|row| names(row, target)) else {
            out.push(line);
            moved_to.push(None);
            continue;
        };
        let target_oid = out[at].commit;
        let mut insert = at + 1;
        while moved_to.get(insert).copied().flatten() == Some(target_oid) {
            insert += 1;
        }
        line.action = action;
        out.insert(insert, line);
        moved_to.insert(insert, Some(target_oid));
    }
    out
}

/// `Some((Fixup|Squash, rest))` for a `fixup! `/`squash! ` subject, `rest`
/// with any further markers stripped.
fn marker(subject: &str) -> Option<(TodoAction, &str)> {
    let (action, mut rest) = match subject.strip_prefix("fixup! ") {
        Some(rest) => (TodoAction::Fixup, rest),
        None => (TodoAction::Squash, subject.strip_prefix("squash! ")?),
    };
    while let Some(inner) = rest
        .strip_prefix("fixup! ")
        .or_else(|| rest.strip_prefix("squash! "))
    {
        rest = inner;
    }
    Some((action, rest))
}

/// Does `target` name this row: its subject, or a prefix of its id at least
/// four digits long?
fn names(row: &TodoLine, target: &str) -> bool {
    row.subject == target
        || (target.len() >= 4
            && target.chars().all(|c| c.is_ascii_hexdigit())
            && row
                .commit
                .to_string()
                .starts_with(&target.to_ascii_lowercase()))
}

/// The todo git runs, and the messages its helper hands back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GitTodo {
    /// The file `GIT_SEQUENCE_EDITOR` copies over git's.
    pub(crate) text: String,
    /// The commit git is on when it asks → the message to answer with.
    pub(crate) messages: Vec<(Oid, String)>,
}

/// His rows, then each dependent's commits as picks, with an `update-ref`
/// line after every branch's tip but the top one (ADR-0047 §2: git rebases
/// the top branch and moves it itself).
///
/// Messages are keyed by the commit git is on when it asks, which is the
/// last line of `rebase-merge/done` (verified against git 2.54: git writes
/// the full id there as the todo gave it):
/// - a reword row: git asks on that row;
/// - a squash group: git asks once, on the group's last row, when the group
///   holds a squash. A group whose message was written but that holds only
///   fixups gets its last fixup written as `squash` so git asks — the
///   message given replaces git's either way, so the two words then differ
///   in nothing but that git asks. Its kept row goes as `pick` even when it
///   was marked reword: the group's one message is the reword.
/// - a pick with a message of its own goes as `reword`.
///
/// An `edit` row keeps its word; a message written for one with nothing
/// melded into it is not used — he amends at the stop.
///
/// `message_of` gives a row's whole message, for a squash group with no
/// message written ([`default_message`]). `home` is his branch's full name;
/// `followers` each dependent's full name and commits, bottom first;
/// `forks` the forks that follow, each replayed inline after its row
/// ([`ForkSection`], ADR-0059 §3). No forks → the todo is the one before
/// ADR-0059, byte for byte.
pub(crate) fn git_todo(
    todo: &[TodoLine],
    messages: &BTreeMap<usize, String>,
    message_of: &dyn Fn(usize) -> String,
    home: &str,
    followers: &[(String, Vec<TodoLine>)],
    forks: &[ForkSection],
) -> GitTodo {
    let groups = squash_groups(todo);
    let mut asked: Vec<(Oid, String)> = Vec::new();
    for group in &groups {
        if let Some(last) = asks_on(group, todo, messages) {
            let message = messages
                .get(&group.kept)
                .cloned()
                .unwrap_or_else(|| default_message(group, message_of, todo));
            asked.push((todo[last].commit, message));
        }
    }
    for row in ungrouped(&groups, todo.len()) {
        if let Some(message) = messages.get(&row)
            && matches!(todo[row].action, TodoAction::Pick | TodoAction::Reword)
        {
            asked.push((todo[row].commit, message.clone()));
        }
    }
    GitTodo {
        text: todo_text(todo, &git_words(todo, messages), home, followers, forks),
        messages: asked,
    }
}

/// Each of his rows' word as git is handed it ([`git_todo`]'s rules): a
/// squash group that asks has its last row written `squash` and its kept
/// row `pick`; a row of no group with a message of its own is `reword`.
pub(crate) fn git_words(todo: &[TodoLine], messages: &BTreeMap<usize, String>) -> Vec<TodoAction> {
    let mut words: Vec<TodoAction> = todo.iter().map(|line| line.action).collect();
    let groups = squash_groups(todo);
    for group in &groups {
        if let Some(last) = asks_on(group, todo, messages) {
            words[last] = TodoAction::Squash;
            if words[group.kept] == TodoAction::Reword {
                words[group.kept] = TodoAction::Pick;
            }
        }
    }
    for row in ungrouped(&groups, todo.len()) {
        if messages.contains_key(&row)
            && matches!(todo[row].action, TodoAction::Pick | TodoAction::Reword)
        {
            words[row] = TodoAction::Reword;
        }
    }
    words
}

/// The row git asks for a squash group's message on — its last, since
/// `melded` is newest first — when it holds a squash or a message was
/// written for it; `None` when git is not to ask.
fn asks_on(
    group: &SquashGroup,
    todo: &[TodoLine],
    messages: &BTreeMap<usize, String>,
) -> Option<usize> {
    let has_squash = group
        .melded
        .iter()
        .any(|&row| todo[row].action == TodoAction::Squash);
    if !has_squash && !messages.contains_key(&group.kept) {
        return None;
    }
    group.melded.first().copied()
}

/// The rows in no squash group, in order.
fn ungrouped(groups: &[SquashGroup], rows: usize) -> impl Iterator<Item = usize> {
    let mut grouped = vec![false; rows];
    for group in groups {
        grouped[group.kept] = true;
        for &row in &group.melded {
            grouped[row] = true;
        }
    }
    (0..rows).filter(move |&row| !grouped[row])
}

/// His rows with the words git runs them by, then the `update-ref` lines
/// and each dependent's picks; each fork's section right after the row it
/// follows (rows counted his first, then the dependents', as git runs them).
fn todo_text(
    todo: &[TodoLine],
    words: &[TodoAction],
    home: &str,
    followers: &[(String, Vec<TodoLine>)],
    forks: &[ForkSection],
) -> String {
    let mut text = String::new();
    let mut actions = 0;
    for (line, word) in todo.iter().zip(words) {
        push_line(&mut text, *word, line);
        push_sections(&mut text, forks, actions);
        actions += 1;
    }
    if !followers.is_empty() {
        text.push_str(&format!("update-ref {home}\n"));
    }
    for (index, (name, picks)) in followers.iter().enumerate() {
        for line in picks {
            push_line(&mut text, TodoAction::Pick, line);
            push_sections(&mut text, forks, actions);
            actions += 1;
        }
        if index + 1 < followers.len() {
            text.push_str(&format!("update-ref {name}\n"));
        }
    }
    if actions == 0 {
        // An empty todo is "nothing to do" to git, which refuses it. A rebase
        // with nothing to replay (the branch is behind) still moves the
        // branch to `onto`: git's own todo for it is `noop`.
        text.push_str("noop\n");
    }
    text
}

/// The label a fork's section marks the line with: `graft-fork-<n>`, `n`
/// its 1-based place in [`RewriteParams::forks`](super::RewriteParams).
/// `progress` reads the prefix back.
pub(super) const FORK_LABEL: &str = "graft-fork-";

/// Every fork section that follows `row`, in order. A fork with commits of
/// its own: `label`, its picks with an `update-ref` after each branch's tip,
/// `update-ref` for the fork, `reset` back to the line. One with none: its
/// `update-ref` alone, which records `HEAD` where it is. git writes every
/// such ref only when the rebase finishes, so an abort leaves each where it
/// was (`tests/it/fork_follows.rs`).
fn push_sections(text: &mut String, forks: &[ForkSection], row: usize) {
    for (index, fork) in forks.iter().enumerate() {
        if fork.after_row != row {
            continue;
        }
        if fork.picks.is_empty() {
            text.push_str(&format!("update-ref {}\n", fork.branch.name));
            continue;
        }
        let label = format!("{FORK_LABEL}{}", index + 1);
        text.push_str(&format!("label {label}\n"));
        for pick in &fork.picks {
            push_line(text, TodoAction::Pick, pick);
            for inside in fork.inside.iter().filter(|tip| tip.oid == pick.commit) {
                text.push_str(&format!("update-ref {}\n", inside.name));
            }
        }
        text.push_str(&format!("update-ref {}\n", fork.branch.name));
        text.push_str(&format!("reset {label}\n"));
    }
}

/// `<word> <full id> <subject>`.
fn push_line(text: &mut String, action: TodoAction, line: &TodoLine) {
    text.push_str(&format!(
        "{} {} {}\n",
        action.word(),
        line.commit,
        line.subject
    ));
}
