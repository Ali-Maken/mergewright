//! The keys the hint tables name, as table rows (ticket m8/10): each is
//! the action a hint's key does, so the bar spells it from his keymap and a
//! remap moves the hint with the key. One constant per key a table draws;
//! a two-key hint (`j/k`) is two actions in one slice.
//!
//! `Fixed` keys are a text field's: the field passes every press as typed
//! (ADR-0055, `graft::update::edge`), so its keys are their defaults
//! whatever his config says — `esc close` in the picker is `Esc` even with
//! `app.back` moved.

use super::HintKey::{self, EveryChord, Fixed, Key, Word};
use crate::keymap::ActionId;

const fn key(id: &'static str) -> HintKey {
    Key(ActionId(id))
}

const fn fixed(id: &'static str) -> HintKey {
    Fixed(ActionId(id))
}

// §2.2 Global.
pub(super) const OPEN: &[HintKey] = &[key("app.open")];
pub(super) const BACK: &[HintKey] = &[key("app.back")];
pub(super) const NEXT_PANE: &[HintKey] = &[key("app.next_pane")];
pub(super) const QUIT: &[HintKey] = &[key("app.quit")];
pub(super) const UNDO: &[HintKey] = &[key("app.undo")];
pub(super) const FIND: &[HintKey] = &[key("app.find")];
pub(super) const MENU: &[HintKey] = &[key("app.menu")];
pub(super) const HELP: &[HintKey] = &[key("app.help")];
pub(super) const PALETTE: &[HintKey] = &[key("app.palette")];
pub(super) const DOWN_UP: &[HintKey] = &[key("app.down"), key("app.up")];
pub(super) const NEXT_MATCH: &[HintKey] = &[key("app.next_match")];
pub(super) const PREV_MATCH: &[HintKey] = &[key("app.prev_match")];
pub(super) const NEXT_PREV_MATCH: &[HintKey] = &[key("app.next_match"), key("app.prev_match")];

// §2.3 Graph.
pub(super) const COMMIT: &[HintKey] = &[key("graph.commit")];
pub(super) const AMEND: &[HintKey] = &[key("graph.amend")];
pub(super) const REWRITE: &[HintKey] = &[key("graph.rewrite")];
pub(super) const SWITCH: &[HintKey] = &[key("graph.switch")];
pub(super) const TAKE: &[HintKey] = &[key("graph.take")];
pub(super) const MERGE: &[HintKey] = &[key("graph.merge")];
pub(super) const REBASE: &[HintKey] = &[key("graph.rebase")];
pub(super) const BRANCH: &[HintKey] = &[key("graph.branch")];
pub(super) const TAG: &[HintKey] = &[key("graph.tag")];
pub(super) const COMPARE: &[HintKey] = &[key("graph.compare")];
pub(super) const LANES: &[HintKey] = &[key("graph.lanes")];
pub(super) const STASH: &[HintKey] = &[key("graph.stash")];
pub(super) const FETCH: &[HintKey] = &[key("graph.fetch")];
pub(super) const PULL: &[HintKey] = &[key("graph.pull")];
pub(super) const PUSH: &[HintKey] = &[key("graph.push")];
pub(super) const MAP: &[HintKey] = &[key("graph.map")];
pub(super) const FOCUS: &[HintKey] = &[key("graph.focus")];
pub(super) const OVERVIEW: &[HintKey] = &[key("graph.overview")];
pub(super) const FOLD: &[HintKey] = &[key("graph.fold")];
pub(super) const FIRST_PARENT: &[HintKey] = &[key("graph.first_parent")];
pub(super) const HIDE_STASH: &[HintKey] = &[key("graph.hide_stash")];
pub(super) const COPY_SHA: &[HintKey] = &[key("graph.copy_sha")];
pub(super) const JOURNAL: &[HintKey] = &[key("graph.journal")];
pub(super) const STATUS: &[HintKey] = &[key("graph.status")];
pub(super) const RENDERER: &[HintKey] = &[key("graph.renderer")];

// §2.3b–c confirm box.
pub(super) const CONFIRM_YES: &[HintKey] = &[key("confirm.yes")];
pub(super) const CONFIRM_MOVE: &[HintKey] = &[key("confirm.left"), key("confirm.right")];
pub(super) const CONFIRM_CHOOSE: &[HintKey] = &[key("confirm.choose")];

// §2.4–2.5 Changes, diff.
pub(super) const STAGE: &[HintKey] = &[key("changes.stage")];
pub(super) const STAGE_ALL: &[HintKey] = &[key("changes.stage_all")];
pub(super) const DISCARD: &[HintKey] = &[key("changes.discard")];
pub(super) const EDIT: &[HintKey] = &[key("changes.edit")];
pub(super) const MERGETOOL: &[HintKey] = &[key("changes.mergetool")];
pub(super) const RESIZE: &[HintKey] = &[key("diff.shrink"), key("diff.grow")];
pub(super) const WHITESPACE: &[HintKey] = &[key("diff.ignore_whitespace")];
pub(super) const BLAME: &[HintKey] = &[key("diff.blame")];
pub(super) const FILE_HISTORY: &[HintKey] = &[key("diff.file_history")];
pub(super) const FOLD_DIR: &[HintKey] = &[key("diff.fold_dir")];

// §2.6 Resolver.
pub(super) const OURS: &[HintKey] = &[key("resolver.take_ours")];
pub(super) const THEIRS: &[HintKey] = &[key("resolver.take_theirs")];
pub(super) const BOTH: &[HintKey] = &[key("resolver.take_both")];
pub(super) const ALL_SIDES: &[HintKey] = &[key("resolver.all_ours"), key("resolver.all_theirs")];
pub(super) const FILE_SIDES: &[HintKey] = &[key("resolver.file_ours"), key("resolver.file_theirs")];
pub(super) const BASE: &[HintKey] = &[key("resolver.base")];
pub(super) const START_OVER: &[HintKey] = &[key("resolver.start_over")];
pub(super) const NEXT_BLOCK: &[HintKey] = &[key("resolver.next_block")];
pub(super) const PREV_BLOCK: &[HintKey] = &[key("resolver.prev_block")];
pub(super) const RESOLVER_FINISH: &[HintKey] = &[key("resolver.finish")];
pub(super) const MARK_DONE: &[HintKey] = &[key("resolver.mark_done")];
pub(super) const RESOLVER_EDIT: &[HintKey] = &[key("resolver.edit")];
pub(super) const RESOLVER_MERGETOOL: &[HintKey] = &[key("resolver.mergetool")];

// §2.6a Banner.
pub(super) const CONTINUE: &[HintKey] = &[key("operation.continue")];
pub(super) const SKIP: &[HintKey] = &[key("operation.skip")];
pub(super) const ABORT: &[HintKey] = &[key("operation.abort")];
pub(super) const BANNER_MERGETOOL: &[HintKey] = &[key("operation.mergetool")];
pub(super) const BANNER_EDIT: &[HintKey] = &[key("operation.edit")];

// §2.6b Finish summary.
/// `y` from the files: the one confirm key (§2.3b), borrowed by `Finish`.
pub(super) const FINISH_COMMIT: &[HintKey] = &[key("confirm.yes")];
pub(super) const TOGGLE_DIFF: &[HintKey] = &[EveryChord(ActionId("finish.toggle_diff"))];

// §2.7 Rewrite editor.
/// `y`: the one confirm key (§2.3b), borrowed by `Rewrite`.
pub(super) const REWRITE_START: &[HintKey] = &[key("confirm.yes")];
pub(super) const REWRITE_CHANGES: &[HintKey] = &[key("rewrite.toggle_changes")];
pub(super) const PICK: &[HintKey] = &[key("rewrite.pick")];
pub(super) const REWORD: &[HintKey] = &[key("rewrite.reword")];
pub(super) const REWRITE_EDIT: &[HintKey] = &[key("rewrite.edit")];
pub(super) const SQUASH: &[HintKey] = &[key("rewrite.squash")];
pub(super) const FIXUP: &[HintKey] = &[key("rewrite.fixup")];
pub(super) const DROP: &[HintKey] = &[key("rewrite.drop")];
pub(super) const MOVE_ROW: &[HintKey] = &[key("rewrite.move_up"), key("rewrite.move_down")];

// §2.8 Blame, §2.17 History.
pub(super) const PARENT: &[HintKey] = &[key("blame.parent")];
pub(super) const LINE_HISTORY: &[HintKey] = &[key("blame.line_history")];
pub(super) const HEATMAP: &[HintKey] = &[key("blame.heatmap")];
pub(super) const IGNORE_REVS: &[HintKey] = &[key("blame.ignore_revs")];
pub(super) const NEXT_PREV_CHANGE: &[HintKey] =
    &[key("history.next_change"), key("history.prev_change")];

// Text fields: their keys are their defaults (module docs).
pub(super) const FIELD_ENTER: &[HintKey] = &[fixed("app.open")];
pub(super) const FIELD_ESC: &[HintKey] = &[fixed("app.back")];
pub(super) const FIELD_TAB: &[HintKey] = &[fixed("app.next_pane")];
pub(super) const FIELD_EDITOR: &[HintKey] = &[fixed("finish.edit_outside")];
pub(super) const FIELD_COMMIT: &[HintKey] = &[fixed("commit.commit")];
pub(super) const PICKER_SWITCH: &[HintKey] = &[fixed("picker.switch")];
pub(super) const PICKER_SHOW: &[HintKey] = &[fixed("picker.show")];
pub(super) const CREDENTIAL_SEND: &[HintKey] = &[fixed("credential.send")];

// Words: what the hand does that is not a key (`NOT_A_BINDING`).
pub(super) const TYPE: &[HintKey] = &[Word("type")];
pub(super) const RELEASE: &[HintKey] = &[Word("release")];
pub(super) const PICKER_ROWS: &[HintKey] = &[Word("↑/↓")];
