//! After the commit (C13, C14, C15): the words `esc` keeps, the commit just
//! made and the bar it earns, `u` giving its message back, and the push
//! count the header and the graph's top line show.

use graft_core::journal::{HeadRef, JournalEntry};
use graft_core::oid::Oid;
use graft_ops::operation::{Include, Operation};
use graft_ui::graph_screen::hints::RowBar;

use super::{AppState, Draft, Status, give_back, redo_took_back};
use crate::refs::Upstream;

/// The commit just made, while it is the last thing that happened: the
/// cursor resting on it shows [`RowBar::JustCommitted`], and `u` taking it
/// back gives its message back as the draft.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LastCommit {
    /// The new `HEAD`.
    pub commit: Oid,
    /// The subject and body just written.
    pub message: Draft,
}

/// `Some(RowBar::JustCommitted)` while the cursor rests on the commit just
/// made.
#[must_use]
pub fn landed_bar(last: Option<&LastCommit>, cursor: Option<Oid>) -> Option<RowBar> {
    let last = last?;
    (cursor == Some(last.commit)).then_some(RowBar::JustCommitted)
}

/// A write landed clean. A plain commit becomes [`LastCommit`] and says
/// `Committed N files to <branch>.`; every other write clears it (an amend
/// included: its bar is the row's own). `None`: not a commit, the caller's
/// words stand.
pub fn commit_landed(
    state: &mut AppState,
    launched: Option<&Operation>,
    entry: &JournalEntry,
) -> Option<Status> {
    state.last_commit = None;
    let Some(Operation::Commit(params)) = launched else {
        return None;
    };
    let commit = new_head(entry)?;
    let mut message = Draft::default();
    let text = params.message.replace("\r\n", "\n");
    match text.split_once('\n') {
        Some((subject, rest)) => {
            subject.trim_end().clone_into(&mut message.subject);
            rest.trim_matches('\n').clone_into(&mut message.body);
        }
        None => text.trim_end().clone_into(&mut message.subject),
    }
    state.last_commit = Some(Box::new(LastCommit { commit, message }));
    let files = match &params.include {
        Include::Chosen(chosen) => chosen.len(),
        // Not built by this screen (`params` always chooses for a commit);
        // counted from what the last read found staged.
        Include::Staged => state
            .working_tree
            .as_deref()
            .map_or(0, |status| status.staged().count()),
    };
    let noun = if files == 1 { "file" } else { "files" };
    let onto = branch_of(entry).map_or_else(String::new, |name| format!(" to {name}"));
    let keys = state
        .keymap
        .key_verbs(&[("app.undo", "undo"), ("graph.push", "push")]);
    Some(Status::Done {
        what: format!("Committed {files} {noun}{onto}."),
        also: None,
        keys: (!keys.is_empty()).then(|| keys.replace("  ", " · ")),
    })
}

/// The commit `HEAD` ended on, as the entry recorded it.
fn new_head(entry: &JournalEntry) -> Option<Oid> {
    match entry.head_after.as_ref()? {
        HeadRef::Detached(oid) => Some(*oid),
        HeadRef::Branch(name) | HeadRef::Unborn(name) => entry.refs_after.get(name).copied()?,
    }
}

/// The branch `HEAD` is on after the entry, short (`main`).
fn branch_of(entry: &JournalEntry) -> Option<&str> {
    match entry.head_after.as_ref()? {
        HeadRef::Branch(name) | HeadRef::Unborn(name) => {
            Some(name.strip_prefix("refs/heads/").unwrap_or(name))
        }
        HeadRef::Detached(_) => None,
    }
}

/// An undo landed (Q9 a): when it moved a ref off the commit just made,
/// that commit's message goes to the `c` slot ([`give_back`]) and the
/// commit stops being the last thing that happened. An undo that left
/// every ref on it (a stage taken back) keeps both as they are (08s m11).
pub fn undo_landed(state: &mut AppState, entry: &JournalEntry) {
    let Some(last) = state.last_commit.as_deref() else {
        return;
    };
    let moved_off = entry.refs_before.iter().any(|(name, before)| {
        *before == Some(last.commit) && after(entry, name) != Some(last.commit)
    });
    if !moved_off {
        return;
    }
    let Some(last) = state.last_commit.take() else {
        return;
    };
    give_back(&mut state.drafts, last.message.clone());
    state.drafts.given_back = Some(last);
}

/// A redo landed: it never takes a commit back, and one that re-creates
/// the commit `u` gave the words of lets go of them while unedited (m1).
pub fn redo_landed(state: &mut AppState, entry: &JournalEntry) {
    state.last_commit = None;
    let Some(given) = state.drafts.given_back.as_deref().map(|given| given.commit) else {
        return;
    };
    let recreated = entry.refs_after.iter().any(|(name, after)| {
        *after == Some(given) && entry.refs_before.get(name).copied().flatten() != Some(given)
    });
    if recreated {
        redo_took_back(&mut state.drafts, given);
    }
}

/// Where the entry left ref `name`; `None` when it deleted or never had it.
fn after(entry: &JournalEntry, name: &str) -> Option<Oid> {
    entry.refs_after.get(name).copied().flatten()
}

/// `N commit(s) to push` while `HEAD`'s branch is ahead of its upstream;
/// `not pushed yet` when it has none (`None` with `with_unpushed` off: the
/// graph's top line says only the count). `None` when level, or before
/// the first refs read.
#[must_use]
pub fn push_words(state: &AppState, with_unpushed: bool) -> Option<String> {
    let refs = state.refs.as_deref()?;
    let (_, _, _, upstream) = refs.branches().find(|(_, _, current, _)| *current)?;
    match upstream {
        Upstream::Apart(counts) if counts.ahead > 0 => {
            let noun = if counts.ahead == 1 {
                "commit"
            } else {
                "commits"
            };
            Some(format!("{} {noun} to push", counts.ahead))
        }
        Upstream::None if with_unpushed => Some("not pushed yet".to_owned()),
        Upstream::Apart(_) | Upstream::InSync { .. } | Upstream::None => None,
    }
}

#[cfg(test)]
mod tests;
