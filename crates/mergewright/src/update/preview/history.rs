//! The graph's two history keys, `m` and `r`, and the operation each builds.

use mergewright_ui::keymap::{ActionId, Keymap};

use super::{
    AppState, Command, JobId, KeyCode, KeyPress, Oid, Operation, PreviewSubject, RebaseParams,
    RefBadges, RefLabel, Source, Status, Tip, open_preview, slice,
};

/// The two history keys the graph pane answers directly
/// (`docs/design/02-keymap.md` §2.3: `m` merge selected branch into current,
/// `r` rebase current onto selection).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryKey {
    /// `m`.
    Merge,
    /// `r`.
    Rebase,
}

/// Is this `m` or `r`?
#[must_use]
pub fn is_history_key(key: KeyPress) -> Option<HistoryKey> {
    if key.modifiers.control || key.modifiers.alt {
        return None;
    }
    match key.code {
        KeyCode::Char('m') => Some(HistoryKey::Merge),
        KeyCode::Char('r') => Some(HistoryKey::Rebase),
        _ => None,
    }
}

/// `m` or `r`: build the operation the selection names and open its preview.
///
/// Nothing runs here. Both keys end in a picture, and only `Confirm` in that
/// picture runs anything (PROMPT.md §1.7) — which is also why every refusal
/// below is a sentence rather than a beep: the user pressed a key that does
/// something, and is owed the reason it did not.
pub fn history_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    which: HistoryKey,
    next_id: &mut impl FnMut() -> JobId,
) {
    let verb = match which {
        HistoryKey::Merge => "m",
        HistoryKey::Rebase => "r",
    };
    match history_operation(state, which, verb) {
        Ok(op) => {
            open_preview(
                state,
                commands,
                PreviewSubject::Operation {
                    op: Box::new(op),
                    opened_by: verb,
                },
                false,
                next_id,
            );
        }
        Err(reason) => {
            state.status = Status::Sticky {
                reason,
                keys: None,
                kind: mergewright_ui::graph_screen::ProblemKind::Notice,
            };
        }
    }
}

/// The operation `m` or `r` names from where the cursor is, or the sentence
/// saying what is missing.
pub(super) fn history_operation(
    state: &AppState,
    which: HistoryKey,
    verb: &str,
) -> Result<Operation, String> {
    // A `p` that could not fast-forward offered these two keys and named
    // the upstream they act on. While that line is on screen they mean
    // *that* branch, wherever the cursor is — the user is answering the
    // sentence they are reading, not pointing at a row (piece 9). It comes
    // first for exactly that reason: no row is read, so no row can refuse.
    if let Some((tracking, tip)) = state.remote.diverged_upstream.as_ref() {
        let refs = state
            .refs
            .as_deref()
            .ok_or_else(|| format!("{verb} is waiting for the refs"))?;
        let current =
            current_branch(refs).ok_or_else(|| format!("{verb} needs a branch checked out"))?;
        let source = Source::Branch(crate::update::remote::short_tracking(tracking).to_owned());
        return Ok(build(which, current, source, *tip));
    }

    if state.on_working_tree {
        return Err(format!("{verb} needs a commit row"));
    }
    let refs = state
        .refs
        .as_deref()
        .ok_or_else(|| format!("{verb} is waiting for the refs"))?;
    let selected = state
        .history
        .as_ref()
        .and_then(crate::history::History::selected)
        .ok_or_else(|| format!("{verb} needs a commit row"))?
        .commit
        .id;
    // A stash's commit is git's internal record of set-aside edits, not
    // something to merge or rebase onto (m8-polish-and-ship/14).
    if refs.is_stash(selected) {
        return Err(stash_isnt_a_branch(&state.keymap));
    }
    let current =
        current_branch(refs).ok_or_else(|| format!("{verb} needs a branch checked out"))?;
    let source = source_at(refs, selected);
    if current.oid == selected
        && matches!(source, Source::Branch(ref name) if *name == short_name_of(&current.name))
    {
        return Err("already on it".to_owned());
    }
    Ok(build(which, current, source, selected))
}

/// The operation itself, once the source and the tip it is pinned to are
/// settled — the same shape whether they came from the cursor's row or from
/// the diverged upstream a `p` named.
pub(super) fn build(which: HistoryKey, current: Tip, source: Source, tip: Oid) -> Operation {
    match which {
        HistoryKey::Merge => Operation::Merge(mergewright_ops::operation::MergeParams {
            source,
            into: current,
            source_tip: tip,
            ff: mergewright_ops::operation::FastForward::Default,
        }),
        HistoryKey::Rebase => Operation::Rebase(RebaseParams::new(current, source, tip)),
    }
}

/// The branch `HEAD` is on, as a [`Tip`]: full ref name and the oid the
/// preview is pinned to. `None` when `HEAD` is detached or unborn — the
/// state both keys refuse in words.
pub(in crate::update) fn current_branch(refs: &RefBadges) -> Option<Tip> {
    let head = refs.head()?;
    refs.on(head).iter().find_map(|label| match label {
        RefLabel::Branch {
            name,
            current: true,
            ..
        } => Some(Tip {
            name: format!("refs/heads/{}", String::from_utf8_lossy(name)),
            oid: head,
        }),
        _ => None,
    })
}

/// What the cursor's row names on a command line: the first local branch on
/// it, else the first remote-tracking branch, else the commit itself.
///
/// A name beats an id because a name is what the user sees on the row and
/// what a reflog will record; the id is the honest answer when the row
/// carries no branch at all.
pub(in crate::update) fn source_at(refs: &RefBadges, oid: Oid) -> Source {
    let labels = refs.on(oid);
    let local = labels.iter().find_map(|label| match label {
        RefLabel::Branch { name, .. } => Some(String::from_utf8_lossy(name).into_owned()),
        _ => None,
    });
    if let Some(name) = local {
        return Source::Branch(name);
    }
    let remote = labels.iter().find_map(|label| match label {
        RefLabel::Remote(name) => Some(String::from_utf8_lossy(name).into_owned()),
        _ => None,
    });
    remote.map_or(Source::Commit(oid), Source::Branch)
}

/// `refs/heads/main` → `main`.
pub(in crate::update) fn short_name_of(full: &str) -> String {
    String::from_utf8_lossy(&slice::short_of(full)).into_owned()
}

/// `a stash isn't a branch · ⏎ opens it`: `⏎` on the graph's stash row
/// opens the stash box. Unbound, the fact alone.
pub(crate) fn stash_isnt_a_branch(keys: &Keymap) -> String {
    keys.after("a stash isn't a branch", ActionId("app.open"), "opens it")
}
