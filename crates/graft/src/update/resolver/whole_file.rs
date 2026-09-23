//! A whole-file shape's card (m6-conflicts/05 scope 1): `UD`/`DU`/`AU`/
//! `UA`/`DD`, or a binary `UU`/`AA`. No blocks and no markers, so `o`/`t`
//! resolve the whole path — a git write, never an fs write like a block
//! take (design note, hard choice 3).

use graft_core::job::JobId;
use graft_git::status::UnmergedHow;
use graft_ops::operation::{Operation, ResolveWholeParams, Side};

use crate::update::events::Command;
use crate::update::state::{AppState, Status};

/// A whole-file shape's own card: no blocks, no markers, `o`/`t` resolve the
/// whole path (m6-conflicts/05).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WholeFileShape {
    /// The shape the status read.
    pub how: UnmergedHow,
    /// git calls the working-tree file binary.
    pub binary: bool,
    /// What is on disk under the card — its first lines only, capped
    /// (`jobs::conflict::CARD_LINES`) — or `None` when there is nothing
    /// there to show (`BothDeleted`, or a side that removed it).
    pub on_disk: Option<crate::jobs::conflict::OnDisk>,
}

/// The keys a whole-file card offers, per shape (m6-conflicts/05 scope 1).
/// `binary` overrides `how`: a binary `UU`/`AA` has both sides on disk
/// nowhere near each other (git leaves ours' bytes, or nothing, depending on
/// platform) and is resolved exactly like the two text shapes' own
/// `checkout --ours|--theirs` path, so it reads the same as they do.
#[must_use]
pub fn whole_file_keys(how: UnmergedHow, binary: bool) -> &'static str {
    if binary {
        return "o keeps ours  t keeps theirs";
    }
    match how {
        UnmergedHow::DeletedByThem => "o keeps the file  t deletes it",
        UnmergedHow::DeletedByUs => "o deletes it  t keeps theirs",
        UnmergedHow::AddedByUs => "o keeps yours  t drops it",
        UnmergedHow::AddedByThem => "o drops it  t keeps theirs",
        UnmergedHow::BothDeleted => "o or t removes it",
        UnmergedHow::BothModified | UnmergedHow::BothAdded => "o keeps ours  t keeps theirs",
    }
}

/// `o`/`t` on a whole-file card: resolve the path whole, through the same
/// `Operate`/`Stage` channel `⏎` uses — a whole-file resolution *is* a git
/// write (design note, hard choice 3), never an fs write like a block take.
///
/// `false` when there is no whole-file card open, so the caller falls
/// through to the ordinary per-block [`super::take::take_side`].
#[must_use]
pub(super) fn whole_file_take(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    side: Side,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    let (how, path) = {
        let Some(open) = state.resolver.as_ref() else {
            return false;
        };
        let Some(whole) = open.whole_file.as_ref() else {
            return false;
        };
        (whole.how, open.path.clone())
    };
    if state.operating.is_some() {
        state.status = Status::sticky("a write is already running");
        return true;
    }
    crate::update::operate::launch(
        state,
        commands,
        Operation::ResolveWholeFile(ResolveWholeParams { path, side, how }),
        next_id,
    );
    let operating = state.operating;
    if let Some(open) = state.resolver.as_mut() {
        open.staging = operating;
    }
    true
}

/// `b` on a whole-file card: refused in words — there is no "both" for a
/// path with no blocks to keep both sides of.
///
/// `false` when there is no whole-file card open, so the caller falls
/// through to the ordinary per-block [`super::take::take_side`].
#[must_use]
pub(super) fn whole_file_both_refused(state: &mut AppState) -> bool {
    let Some(open) = state.resolver.as_ref() else {
        return false;
    };
    let Some(whole) = open.whole_file.as_ref() else {
        return false;
    };
    state.status = Status::Sticky {
        reason: format!("no both for a {} file", whole.how.words()),
        keys: Some("o ours  t theirs".to_owned()),
    };
    true
}
