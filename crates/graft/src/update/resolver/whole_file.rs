//! A whole-file shape's card (m6-conflicts/05 scope 1): `UD`/`DU`/`AU`/
//! `UA`/`DD`, or a binary `UU`/`AA`. No blocks and no markers, so `o`/`t`
//! resolve the whole path — a git write, never an fs write like a block
//! take (design note, hard choice 3).

use graft_core::job::JobId;
use graft_git::status::UnmergedHow;
use graft_ops::operation::{Operation, ResolveWholeParams, Side};
use graft_ui::conflict_resolver::SideNames;

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
    /// Whether `d` can start this card over: ops' answer, asked when it
    /// opened (`jobs::conflict::OpenedFile::start_over`).
    pub start_over: crate::jobs::conflict::StartOver,
}

/// The keys a whole-file card offers, per shape (m6-conflicts/05 scope 1).
/// `binary` overrides `how`: a binary `UU`/`AA` has both sides on disk
/// nowhere near each other (git leaves ours' bytes, or nothing, depending on
/// platform) and is resolved exactly like the two text shapes' own
/// `checkout --ours|--theirs` path, so it reads the same as they do.
///
/// A side is named by its one name (`t keeps theirs · feature`,
/// [`super::side_names`]); words about the file itself (`keeps the file`,
/// `deletes it`) stay as they are.
#[must_use]
pub fn whole_file_keys(how: UnmergedHow, binary: bool, names: &SideNames) -> String {
    let (ours, theirs) = (names.ours.as_str(), names.theirs.as_str());
    if binary {
        return format!("o keeps {ours}  t keeps {theirs}");
    }
    match how {
        UnmergedHow::DeletedByThem => "o keeps the file  t deletes it".to_owned(),
        UnmergedHow::DeletedByUs => format!("o deletes it  t keeps {theirs}"),
        UnmergedHow::AddedByUs => format!("o keeps {ours}  t drops it"),
        UnmergedHow::AddedByThem => format!("o drops it  t keeps {theirs}"),
        UnmergedHow::BothDeleted => "o or t removes it".to_owned(),
        UnmergedHow::BothModified | UnmergedHow::BothAdded => {
            format!("o keeps {ours}  t keeps {theirs}")
        }
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
    if let Some(words) = super::write_refusal(state) {
        state.status = Status::sticky(words);
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
        keys: Some(super::names::side_keys(state)),
    };
    true
}
