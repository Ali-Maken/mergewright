//! After the commit (C13, C14, C15): the words `esc` keeps, the commit just
//! made and the bar it earns, `u` giving its message back, and the push
//! count the header and the graph's top line show.

use graft_core::journal::{HeadRef, JournalEntry};
use graft_core::oid::Oid;
use graft_ops::operation::{Include, Operation};
use graft_ui::graph_screen::hints::RowBar;

use super::{AppState, Status};
use crate::refs::Upstream;

/// The words `esc` keeps from the message line (C15), and the message `u`
/// gives back after a commit (Q9 a).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Draft {
    /// The subject line.
    pub subject: String,
    /// The body.
    pub body: String,
    /// `Some(head)` when kept from an amend of `head`: it resumes on `a`
    /// while `HEAD` is still that commit, never on `c`.
    pub amending: Option<Oid>,
}

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

/// An undo landed (Q9 a): when it took back the commit just made, that
/// commit's message becomes the draft the next `c` opens with. Any other
/// undo leaves the draft alone. Either way the commit is no longer the
/// last thing that happened.
pub fn undo_landed(state: &mut AppState, entry: &JournalEntry) {
    let Some(last) = state.last_commit.take() else {
        return;
    };
    let took_it_back = entry
        .refs_before
        .values()
        .any(|at| *at == Some(last.commit));
    if took_it_back {
        state.commit_draft = Some(Draft {
            amending: None,
            ..last.message
        });
    }
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
