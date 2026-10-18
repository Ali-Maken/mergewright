//! The behavioural sweep's swallow list (m8/09d Major 3; split from
//! `sweep_tables` for size): each place's catch-all, cited once, and every
//! row it swallows. Paths are under `crates/graft/src/update/` unless
//! named; lines as read on 2026-10-07.

use graft_ui::keymap::ContextId;

pub(super) const R_PREVIEW: &str =
    "preview/keys.rs:69-70 the preview swallows every key but its own and ^C";
pub(super) const R_CONFIRM: &str =
    "confirm.rs:192-196 the confirm box swallows every key but its own and ^C";
/// Ticket 11, decision 7: no palette over a box that waits for an answer.
pub(super) const R_CONFIRM_PALETTE: &str = "palette.rs:153 the palette refuses over a confirm box, which \
     then swallows the key (confirm.rs:192-196)";
pub(super) const R_FINISH: &str =
    "finish/keys.rs:31,58 the merge review swallows keys it does not own";
pub(super) const R_RESOLVER: &str =
    "resolver.rs:380-383, resolver/scroll.rs:152 the resolver is modal";
pub(super) const R_BANNER: &str =
    "banner/focus_keys.rs:167-173 the focused banner swallows keys it does not own";
pub(super) const R_REWRITE: &str =
    "rewrite_editor/keys.rs:158,185 the rewrite editor swallows keys it does not own";
pub(super) const R_BLAME: &str =
    "keys.rs:124-129 place_claim; place/keys.rs:44,78 blame swallows the rest";
pub(super) const R_HISTORY: &str =
    "keys.rs:124-129 place_claim; place/history_keys.rs:25,53 history swallows the rest";
pub(super) const R_PICKER: &str =
    "picker.rs:153-155 the picker field swallows keys it does not own";
pub(super) const R_EDITOR: &str =
    "commit_editor/keys.rs:154,275 the commit editor swallows keys it does not own";
pub(super) const R_CREDENTIAL: &str =
    "credential.rs:325-326,383-385 the credential field swallows the rest";
pub(super) const R_MENU_CLOSE: &str =
    "context_menu.rs:155 closes the menu, as F20 does (edge.rs:50)";

/// `(place, id, why)`: the place answers the key itself and does nothing
/// with it — a box or a modal place that swallows what is not its own.
/// The Map, the stash list and the journal are not here: what they swallow
/// is out of their scope or in `ContextId::stands_down` (m8/09f), checked
/// by `sweep::every_row_a_place_stands_down_is_idle_there`.
#[rustfmt::skip]
pub(super) const SWALLOWED_BY_DESIGN: &[(ContextId, &str, &str)] = &[
    // m8/09e: were DEFECTS; ruled idle.
    (ContextId::Diff, "graph.map", "§2.3a :279 Map opens from the graph pane; M in the detail pane is the mergetool's (§2.4 :308)"),
    (ContextId::Preview, "app.undo", R_PREVIEW),
    (ContextId::Preview, "app.redo", R_PREVIEW),
    (ContextId::Preview, "app.refresh", R_PREVIEW),
    (ContextId::Preview, "app.quit", R_PREVIEW),
    (ContextId::Preview, "app.down", R_PREVIEW),
    (ContextId::Preview, "app.up", R_PREVIEW),
    (ContextId::Preview, "app.top", R_PREVIEW),
    (ContextId::Preview, "app.bottom", R_PREVIEW),
    (ContextId::Preview, "app.page_down", R_PREVIEW),
    (ContextId::Preview, "app.page_up", R_PREVIEW),
    (ContextId::Preview, "app.find", R_PREVIEW),
    (ContextId::Preview, "app.next_match", R_PREVIEW),
    (ContextId::Preview, "app.prev_match", R_PREVIEW),
    (ContextId::Preview, "app.menu", R_PREVIEW),
    (ContextId::Confirm, "app.help", R_CONFIRM_PALETTE),
    (ContextId::Confirm, "app.palette", R_CONFIRM_PALETTE),
    (ContextId::Confirm, "app.undo", R_CONFIRM),
    (ContextId::Confirm, "app.redo", R_CONFIRM),
    (ContextId::Confirm, "app.refresh", R_CONFIRM),
    (ContextId::Confirm, "app.quit", R_CONFIRM),
    (ContextId::Confirm, "app.down", R_CONFIRM),
    (ContextId::Confirm, "app.up", R_CONFIRM),
    (ContextId::Confirm, "app.top", R_CONFIRM),
    (ContextId::Confirm, "app.bottom", R_CONFIRM),
    (ContextId::Confirm, "app.page_down", R_CONFIRM),
    (ContextId::Confirm, "app.page_up", R_CONFIRM),
    (ContextId::Confirm, "app.find", R_CONFIRM),
    (ContextId::Confirm, "app.next_match", R_CONFIRM),
    (ContextId::Confirm, "app.prev_match", R_CONFIRM),
    (ContextId::Confirm, "app.menu", R_CONFIRM),
    (ContextId::Diff, "diff.fold_dir", "detail/keys.rs:371-375 space claimed with no effect in the diff"),
    (ContextId::Diff, "app.open", "detail/keys.rs:130-134 ⏎ claimed, nothing left to open"),
    (ContextId::Resolver, "app.next_pane", R_RESOLVER),
    (ContextId::Resolver, "app.prev_pane", R_RESOLVER),
    (ContextId::Resolver, "app.refresh", R_RESOLVER),
    (ContextId::Resolver, "app.quit", R_RESOLVER),
    (ContextId::Resolver, "app.find", R_RESOLVER),
    (ContextId::Resolver, "app.menu", R_RESOLVER),
    (ContextId::Banner, "app.refresh", R_BANNER),
    (ContextId::Banner, "app.down", R_BANNER),
    (ContextId::Banner, "app.up", R_BANNER),
    (ContextId::Banner, "app.top", R_BANNER),
    (ContextId::Banner, "app.bottom", R_BANNER),
    (ContextId::Banner, "app.page_down", R_BANNER),
    (ContextId::Banner, "app.page_up", R_BANNER),
    (ContextId::Banner, "app.find", R_BANNER),
    (ContextId::Banner, "app.next_match", R_BANNER),
    (ContextId::Banner, "app.prev_match", R_BANNER),
    (ContextId::Banner, "app.menu", R_BANNER),
    (ContextId::Finish, "app.redo", R_FINISH),
    (ContextId::Finish, "app.refresh", R_FINISH),
    (ContextId::Finish, "app.quit", R_FINISH),
    (ContextId::Finish, "app.top", R_FINISH),
    (ContextId::Finish, "app.bottom", R_FINISH),
    (ContextId::Finish, "app.find", R_FINISH),
    (ContextId::Finish, "app.next_match", R_FINISH),
    (ContextId::Finish, "app.prev_match", R_FINISH),
    (ContextId::Finish, "app.menu", R_FINISH),
    // ease-pass/12: the review's message is read-only — no field to open
    // in $EDITOR, none to move between.
    (ContextId::Finish, "finish.edit_outside", R_FINISH),
    (ContextId::Finish, "app.next_pane", R_FINISH),
    (ContextId::Finish, "app.prev_pane", R_FINISH),
    (ContextId::Rewrite, "app.next_pane", "rewrite_editor/keys.rs:181-182,185 Tab swallowed at >=120 cols"),
    (ContextId::Rewrite, "app.prev_pane", R_REWRITE),
    (ContextId::Rewrite, "app.undo", R_REWRITE),
    (ContextId::Rewrite, "app.redo", R_REWRITE),
    (ContextId::Rewrite, "app.refresh", R_REWRITE),
    (ContextId::Rewrite, "app.quit", R_REWRITE),
    (ContextId::Rewrite, "app.top", R_REWRITE),
    (ContextId::Rewrite, "app.bottom", R_REWRITE),
    (ContextId::Rewrite, "app.page_down", R_REWRITE),
    (ContextId::Rewrite, "app.page_up", R_REWRITE),
    (ContextId::Rewrite, "app.find", R_REWRITE),
    (ContextId::Rewrite, "app.next_match", R_REWRITE),
    (ContextId::Rewrite, "app.prev_match", R_REWRITE),
    (ContextId::Rewrite, "app.menu", R_REWRITE),
    (ContextId::Picker, "app.prev_pane", R_PICKER),
    (ContextId::Picker, "app.refresh", R_PICKER),
    (ContextId::Picker, "app.page_down", R_PICKER),
    (ContextId::Picker, "app.page_up", R_PICKER),
    (ContextId::Blame, "diff.word_diff", R_BLAME),
    (ContextId::Blame, "diff.side_by_side", R_BLAME),
    (ContextId::Blame, "diff.less_context", R_BLAME),
    (ContextId::Blame, "diff.more_context", R_BLAME),
    (ContextId::Blame, "diff.shrink", R_BLAME),
    (ContextId::Blame, "diff.grow", R_BLAME),
    (ContextId::Blame, "diff.blame", R_BLAME),
    (ContextId::Blame, "diff.file_history", R_BLAME),
    (ContextId::Blame, "app.next_pane", R_BLAME),
    (ContextId::Blame, "app.prev_pane", R_BLAME),
    (ContextId::Blame, "app.undo", R_BLAME),
    (ContextId::Blame, "app.redo", R_BLAME),
    (ContextId::Blame, "app.menu", R_BLAME),
    (ContextId::History, "diff.word_diff", R_HISTORY),
    (ContextId::History, "diff.less_context", R_HISTORY),
    (ContextId::History, "diff.more_context", R_HISTORY),
    (ContextId::History, "diff.shrink", R_HISTORY),
    (ContextId::History, "diff.grow", R_HISTORY),
    (ContextId::History, "diff.file_history", R_HISTORY),
    (ContextId::History, "diff.fold_dir", R_HISTORY),
    (ContextId::History, "app.prev_pane", R_HISTORY),
    (ContextId::History, "app.undo", R_HISTORY),
    (ContextId::History, "app.redo", R_HISTORY),
    (ContextId::History, "app.top", R_HISTORY),
    (ContextId::History, "app.bottom", R_HISTORY),
    (ContextId::History, "app.find", R_HISTORY),
    (ContextId::History, "app.menu", R_HISTORY),
    (ContextId::CommitEditor, "app.refresh", R_EDITOR),
    (ContextId::CommitEditor, "app.page_down", R_EDITOR),
    (ContextId::CommitEditor, "app.page_up", R_EDITOR),
    (ContextId::Credential, "app.next_pane", R_CREDENTIAL),
    (ContextId::Credential, "app.prev_pane", R_CREDENTIAL),
    (ContextId::Credential, "app.refresh", R_CREDENTIAL),
    (ContextId::Credential, "app.page_down", R_CREDENTIAL),
    (ContextId::Credential, "app.page_up", R_CREDENTIAL),
    (ContextId::Menu, "app.back", R_MENU_CLOSE),
    (ContextId::Menu, "app.menu", R_MENU_CLOSE),
];
