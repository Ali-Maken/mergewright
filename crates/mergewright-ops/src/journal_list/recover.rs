//! What `⏎` on a row of the list puts back (ADR-0046 §3; the note's `⏎`
//! table and J1).
//!
//! Pure over the list, the journal and [`RepoFacts`]: the refs it compares
//! are the ones [`recover_scope`] names, read by the app into `facts.refs`
//! (a name missing there reads as absent, as everywhere `facts.refs` is
//! read). A put-back never moves a branch that moved on since the list was
//! read: it offers a new branch at the commit instead, and says why.

use mergewright_core::journal::{JournalEntry, OperationKind};
use mergewright_core::oid::Oid;

use super::steps::{branch_short, checkout_names, is_hex_id};
use super::words::shell_command;
use super::{HeldBy, JournalList, ListRow, LostCommit, OutsideStep};
use crate::operation::{
    FactsScope, NoPutBack, RecoverParams, RecoverTarget, Refusal, RepoFacts, short,
};

/// How many `-restored` names are tried: `<b>-restored`, then `-2` to this.
const RESTORED_TRIES: usize = 9;

/// What `⏎` on a row does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recover {
    /// Put it back: recover-op's operation, previewed and confirmed with `y`.
    Run(RecoverParams),
    /// Nothing can be put back in place — the branch moved on since, or no
    /// branch held it — so a new branch at the commit is offered instead:
    /// J's status row says `why`, and `b` opens the name prompt with `name`.
    OfferBranch {
        /// The commit.
        at: Oid,
        /// The name to prefill, free now.
        name: String,
        /// Why, in words: `main moved on since (now 5f6e7d8)`.
        why: String,
    },
}

/// What `⏎` on `rows[row]` puts back.
///
/// `entries` are the ones the list was built from (a Mergewright row's words and
/// values); `facts.refs` must hold the names [`recover_scope`] gives.
///
/// # Errors
/// A [`Refusal`] when the row has no put-back: a Mergewright row
/// ([`NoPutBack::Mergewright`], the app's revert-to is its `⏎`), a shell switch
/// ([`NoPutBack::Switch`]), a step that made its branch
/// ([`NoPutBack::Created`]), a commit a branch holds again
/// ([`Refusal::HeldAgain`]), or no `-restored` name free
/// ([`Refusal::RefExists`]).
pub fn recover_for(
    list: &JournalList,
    row: usize,
    entries: &[JournalEntry],
    facts: &RepoFacts,
) -> Result<Recover, Refusal> {
    match list.rows.get(row) {
        None => Err(Refusal::NoPutBack(NoPutBack::Nothing)),
        Some(ListRow::Mergewright(_)) => Err(Refusal::NoPutBack(NoPutBack::Mergewright)),
        Some(ListRow::Outside(step)) => outside_step(step, facts),
        Some(ListRow::Lost { event, commit }) => lost_commit(list, *event, commit, entries, facts),
        Some(ListRow::LostEdit { event, edit }) => Ok(Recover::Run(RecoverParams {
            target: RecoverTarget::Files {
                entry: edit.entry.clone(),
                backup_tree: edit.backup_tree,
                paths: edit.paths.clone(),
            },
            because: because_of(list, *event, entries, None),
        })),
    }
}

/// The refs [`recover_for`] compares for `rows[row]`: the branch (and the
/// remote-tracking branch and tag of that name, which it never makes again),
/// and the `-restored` names it may offer. The app reads them into
/// `facts.refs`.
pub fn recover_scope(list: &JournalList, row: usize) -> FactsScope {
    let refs = match list.rows.get(row) {
        Some(ListRow::Outside(step)) if checkout_names(&step.record.subject).is_none() => {
            match branch_short(&step.record.refname) {
                Some(name) => branch_scope(name),
                None => restored_scope(None),
            }
        }
        Some(ListRow::Lost { commit, .. }) => match &commit.held_by {
            HeldBy::Branch { name, .. } => branch_scope(name),
            HeldBy::Head => restored_scope(None),
        },
        _ => Vec::new(),
    };
    FactsScope {
        remotes: false,
        refs,
    }
}

/// A branch's own ref, the names it could be confused with, and its
/// `-restored` names.
fn branch_scope(name: &str) -> Vec<String> {
    let mut refs = vec![
        format!("refs/heads/{name}"),
        format!("refs/remotes/{name}"),
        format!("refs/tags/{name}"),
    ];
    refs.extend(restored_scope(Some(name)));
    refs
}

fn restored_scope(base: Option<&str>) -> Vec<String> {
    restored_names(base)
        .map(|name| format!("refs/heads/{name}"))
        .collect()
}

/// `<b>-restored`, `<b>-restored-2`, …; `restored`, `restored-2`, … when no
/// branch held it.
fn restored_names(base: Option<&str>) -> impl Iterator<Item = String> {
    let stem = base.map_or_else(|| "restored".to_owned(), |name| format!("{name}-restored"));
    (1..=RESTORED_TRIES).map(move |try_number| {
        if try_number == 1 {
            stem.clone()
        } else {
            format!("{stem}-{try_number}")
        }
    })
}

/// The first `-restored` name no branch has.
fn free_restored(base: Option<&str>, facts: &RepoFacts) -> Result<String, Refusal> {
    let mut last = String::new();
    for name in restored_names(base) {
        if branch_now(&name, facts).is_none() {
            return Ok(name);
        }
        last = name;
    }
    Err(Refusal::RefExists { name: last })
}

fn branch_now(name: &str, facts: &RepoFacts) -> Option<Oid> {
    facts
        .refs
        .get(&format!("refs/heads/{name}"))
        .copied()
        .flatten()
}

/// Why `name` must not be made as a branch: a remote-tracking branch or a
/// tag has it. A detached checkout of `origin/main` reads like a branch in
/// the logs; `refs/heads/origin/main` would then shadow the remote one for
/// every later `git rebase origin/main` (review m7-22, M1).
fn not_a_branch(name: &str, facts: &RepoFacts) -> Option<String> {
    let has = |prefix: &str| {
        facts
            .refs
            .get(&format!("{prefix}{name}"))
            .copied()
            .flatten()
            .is_some()
    };
    if has("refs/remotes/") {
        Some(format!("{name} is a remote branch, not one of yours"))
    } else if has("refs/tags/") {
        Some(format!("{name} is a tag, not a branch"))
    } else {
        None
    }
}

fn offer(at: Oid, base: Option<&str>, why: String, facts: &RepoFacts) -> Result<Recover, Refusal> {
    Ok(Recover::OfferBranch {
        at,
        name: free_restored(base, facts)?,
        why,
    })
}

fn moved_on(name: &str, now: Oid) -> String {
    format!("{name} moved on since (now {})", short(now))
}

/// `$`: the ref back to before the step.
fn outside_step(step: &OutsideStep, facts: &RepoFacts) -> Result<Recover, Refusal> {
    let record = &step.record;
    if let Some((from, _)) = checkout_names(&record.subject) {
        let back_to = if is_hex_id(from) {
            from.chars().take(7).collect()
        } else {
            from.to_owned()
        };
        return Err(Refusal::NoPutBack(NoPutBack::Switch { back_to }));
    }
    let branch = branch_short(&record.refname);
    let Some(old) = record.old else {
        let name = branch.unwrap_or("HEAD").to_owned();
        return Err(Refusal::NoPutBack(NoPutBack::Created { name }));
    };
    let because = format!("the shell's {}", shell_command(&record.subject));
    let Some(name) = branch else {
        let why = "only HEAD's log has this step, no branch's".to_owned();
        return offer(old, None, why, facts);
    };
    let target = match branch_now(name, facts) {
        None => match not_a_branch(name, facts) {
            Some(why) => return offer(old, None, why, facts),
            None => RecoverTarget::Recreate {
                name: name.to_owned(),
                at: old,
            },
        },
        Some(now) if now == old => {
            return Err(Refusal::HeldAgain {
                commit: old,
                by: name.to_owned(),
            });
        }
        Some(now) if Some(now) == record.new => RecoverTarget::MoveRef {
            name: name.to_owned(),
            to: old,
            expected: now,
        },
        Some(now) => return offer(old, Some(name), moved_on(name, now), facts),
    };
    Ok(Recover::Run(RecoverParams { target, because }))
}

/// `✗`: what held the commit, back on it.
fn lost_commit(
    list: &JournalList,
    event: usize,
    commit: &LostCommit,
    entries: &[JournalEntry],
    facts: &RepoFacts,
) -> Result<Recover, Refusal> {
    let tip = commit.run.tip;
    let HeldBy::Branch { name, .. } = &commit.held_by else {
        let why = "no branch held it, only HEAD".to_owned();
        return offer(tip, None, why, facts);
    };
    let because = because_of(list, event, entries, Some(&commit.held_by));
    let left = left_at(list, event, name, entries);
    let target = match branch_now(name, facts) {
        None => match not_a_branch(name, facts) {
            Some(why) => return offer(tip, None, why, facts),
            None => RecoverTarget::Recreate {
                name: name.clone(),
                at: tip,
            },
        },
        Some(now) if now == tip => {
            return Err(Refusal::HeldAgain {
                commit: tip,
                by: name.clone(),
            });
        }
        // The event left the branch where it is now. `None` (a switch away
        // left it on the tip itself) is the arm above.
        Some(now) if Some(now) == left => RecoverTarget::MoveRef {
            name: name.clone(),
            to: tip,
            expected: now,
        },
        Some(now) => return offer(tip, Some(name), moved_on(name, now), facts),
    };
    Ok(Recover::Run(RecoverParams { target, because }))
}

/// Where the event left the branch: its record's new value, or the entry's
/// recorded after-value. `None` when the event did not move that branch —
/// a switch away from it — so it was left on the lost tip itself.
fn left_at(list: &JournalList, event: usize, name: &str, entries: &[JournalEntry]) -> Option<Oid> {
    let full = format!("refs/heads/{name}");
    match list.rows.get(event)? {
        ListRow::Outside(step) if step.record.refname == full => step.record.new,
        ListRow::Mergewright(id) => entries
            .iter()
            .find(|entry| entry.id == *id)?
            .refs_after
            .get(&full)
            .copied()
            .flatten(),
        _ => None,
    }
}

/// The event in words, for the journal's `put main back (<because>)`.
fn because_of(
    list: &JournalList,
    event: usize,
    entries: &[JournalEntry],
    held_by: Option<&HeldBy>,
) -> String {
    match list.rows.get(event) {
        Some(ListRow::Outside(step)) => {
            if matches!(held_by, Some(HeldBy::Branch { gone: true, .. })) {
                "the shell's branch -D".to_owned()
            } else {
                format!("the shell's {}", shell_command(&step.record.subject))
            }
        }
        Some(ListRow::Mergewright(id)) => match entries.iter().find(|entry| entry.id == *id) {
            Some(entry) => mergewright_words(entry),
            None => format!("journal entry {}", id.as_str()),
        },
        _ => "an earlier step".to_owned(),
    }
}

/// `undo of "commit x"` for an undo (whose description is `undo: commit x`),
/// `redo of "…"` for a redo, and the entry's own description otherwise.
fn mergewright_words(entry: &JournalEntry) -> String {
    let (verb, prefix) = match entry.kind {
        OperationKind::Undo => ("undo", "undo: "),
        OperationKind::Redo => ("redo", "redo: "),
        _ => return entry.description.clone(),
    };
    let of = entry
        .description
        .strip_prefix(prefix)
        .unwrap_or(&entry.description);
    format!("{verb} of \"{of}\"")
}
