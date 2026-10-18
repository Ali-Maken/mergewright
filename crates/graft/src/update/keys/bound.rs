//! The table of every action the app's key handlers answer, split out of
//! `keys` (review m6-amend-16 M3): a claim the tests check against the hint
//! tables, not code the dispatch runs.

use graft_ui::keymap::ActionId;

/// Every action this build binds on the graph screen and the panes, boxes
/// and places its footer hints from, by its row in
/// `graft_ui::keymap::defaults()` (ticket m8/10: actions, not key strings).
///
/// Before m8/10 each line was a `(key, verb)` pair, because a key string
/// alone could not tell `M map` from `M mergetool` (m6-03 review, Critical
/// 2). An action can: `graph.map` and `operation.mergetool` are two rows, so
/// a hint naming the wrong one names an action this list does or does not
/// hold. `update::tests` walks it three ways:
///
/// - every action a drawn hint names must be in here
///   (`every_key_the_bar_draws_is_answered`), and every hint drawn live must
///   also be *answered* — its chords pressed — by the handler with focus;
/// - every action only a `HintState::Planned` hint names must **not** be in
///   here (`every_planned_hint_is_still_unbound`);
/// - and every action **here** must be drawn live in some context, or be
///   named in `update::tests::NEVER_HINTED` with the reason it never is
///   (`every_bound_action_is_hinted_live_or_is_named_here`).
///
/// `table_check` holds each line to a row of `defaults()`. Each line names
/// the dispatcher it is true because of: this list is a claim about the
/// code, and the tests only compare it with the hint tables.
#[must_use]
pub(crate) fn bound_actions() -> &'static [ActionId] {
    BOUND_ACTIONS
}

const fn id(name: &'static str) -> ActionId {
    ActionId(name)
}

/// [`bound_actions`]' list. A `const`, not the function's own body: it is a
/// table, and a table is not a function long enough to want splitting.
const BOUND_ACTIONS: &[ActionId] = &[
    // Global (keymap §2.2): `graph_key`, the panes, the places.
    id("app.quit"),      // `is_quit`
    id("app.down"),      // `motion_for` → `move_cursor`; every list's cursor
    id("app.up"),        // the same
    id("app.top"),       // `motion_for`
    id("app.bottom"),    // `motion_for`
    id("app.page_down"), // `motion_for`; the review's `finish_key` → `page`
    id("app.page_up"),   // the same
    id("app.refresh"),   // `is_refresh` → `refresh_key`
    id("app.undo"),      // `is_undo_key` → `undo::undo_key`
    id("app.redo"),      // `is_undo_key`, redo
    // `Esc`: a stale status (`key_event`), every overlay's and box's close,
    // a drag's cancel, the finish summary's and the places' back.
    id("app.back"),
    id("app.menu"),       // `is_context_menu_key` → `context_menu::open_selected`
    id("app.help"),       // `palette::open_key`: the bar's `? keys` doorway (m8/11b)
    id("app.palette"),    // `palette::open_key`: the doorway when `?` is unbound
    id("app.find"),       // `search::search_key`; the blame place's find prompt
    id("app.next_match"), // `place::keys::step_find`
    id("app.prev_match"), // the same
    // `Enter`: `detail::keys` opens the pane / the changes / the resolver;
    // `place::keys::enter`; the text fields' own `Enter` (create, keep,
    // the finish subject's commit).
    id("app.open"),
    // `Tab`: `detail::keys` focus cycle, `banner::take_focus`, the history
    // place's panes, the finish summary's focus, the name prompt's toggle,
    // the rewrite editor's `after` below 120 columns.
    id("app.next_pane"),
    // `Shift+Tab`: `banner::take_focus` / `hand_the_keyboard_back` (the
    // cycle run backwards, m8/20d), `detail::keys`; the graph bar's
    // `⇧⇥ banner` (20d-b). Like `Tab`, matched as the raw key, not by action.
    id("app.prev_pane"),
    // The graph (`graph_key`, keymap §2.3).
    id("graph.status"),       // `is_screen_toggle`
    id("graph.renderer"),     // `is_tier_cycle`
    id("graph.lanes"),        // `is_lane_toggle`
    id("graph.focus"),        // `is_focus_toggle`
    id("graph.copy_sha"),     // `is_copy_key` → `copy_selected_sha`
    id("graph.compare"),      // `is_compare_key` → `toggle_compare_selected`
    id("graph.commit"),       // `is_commit_key` → `commit_editor::open`
    id("graph.amend"),        // `is_commit_key`, amending; `amend_at_edit_stop`
    id("graph.take"),         // `write_key` → `preview::take_key`
    id("graph.merge"),        // `write_key` → `preview::history_key`, `Merge`
    id("graph.rebase"),       // `write_key` → `preview::history_key`, `Rebase`
    id("graph.pull"),         // `is_remote_key` → `remote::pull_key`
    id("graph.push"),         // `is_remote_key` → `remote::push_key`
    id("graph.fetch"),        // `is_remote_key` → `remote::fetch_key`
    id("graph.fold"),         // `view_change` → `ViewChange::Fold`
    id("graph.first_parent"), // `view_change` → `ViewChange::FirstParent`
    id("graph.hide_stash"),   // `view_change` → `ViewChange::Stashes`
    id("graph.branch"),       // `is_name_key` → `name_prompt::open_from_graph`
    id("graph.tag"),          // `is_name_key`, tagging
    id("graph.stash"),        // `stash::stash_key`: opens the overlay
    id("graph.journal"),      // `journal::journal_key`: opens the browser
    id("graph.map"),          // `map::map_key`
    id("graph.overview"),     // `strip::strip_key`
    id("graph.switch"),       // `picker::picker_key`: opens the branch picker
    id("graph.rewrite"),      // `is_rewrite_key` → `open_rewrite`
    // The changes pane and the diff (`detail::keys`).
    id("changes.stage"), // `changes_pane_write_key`; a `CONFLICTS` row's `resolved`
    id("changes.discard"), // the same, `operate::request_discard`
    id("changes.stage_all"), // the same, `operate::stage_all`
    id("changes.edit"),  // the same, `$EDITOR` at the cursor's line
    id("changes.mergetool"), // the same, on an unmerged path
    id("diff.shrink"),   // while the pane is shown (ADR-0028)
    id("diff.grow"),     // the same
    id("diff.ignore_whitespace"), // `place::keys::rerun`, `-w` toggled
    id("diff.blame"),    // `place::b_key`; the history place's `blame here`
    id("diff.file_history"), // `place::f_key`, on a file
    id("diff.fold_dir"), // a directory row of the file list
    // The operation banner, while it has focus (`banner::banner_key`).
    id("operation.continue"),  // `continue_pressed`
    id("operation.skip"),      // `skip_pressed`
    id("operation.abort"),     // `abort_pressed`
    id("operation.mergetool"), // `open_mergetool`
    id("operation.edit"),      // `$EDITOR` on the conflicted file
    // Inside the resolver (`resolver::resolver_key`).
    id("resolver.take_ours"),   // `take_side`, `Take::Ours`
    id("resolver.take_theirs"), // the same, `Take::Theirs`
    id("resolver.take_both"),   // the same, `Take::Both`
    id("resolver.all_ours"),    // `take_all_key`
    id("resolver.all_theirs"),  // the same
    id("resolver.file_ours"),   // `take::settle_key`
    id("resolver.file_theirs"), // the same
    id("resolver.base"),        // `toggle_base`
    id("resolver.start_over"),  // `restart_box`
    id("resolver.next_block"),  // `first_open`: the first open clash
    id("resolver.prev_block"),  // `walk_blocks`, backward
    id("resolver.step_prev"),   // `step_clash`, backward, in this file
    id("resolver.step_next"),   // the same, forward
    id("resolver.mark_done"),   // `mark_resolved`: `git add` on the open file
    id("resolver.finish"),      // `finish_pressed` → `banner::continue_pressed`
    id("resolver.review"),      // `review::review_pressed`: a merge's review
    id("resolver.edit"),        // `$EDITOR` on the open file
    id("resolver.mergetool"),   // `open_mergetool`
    // The overlays (`map`, `journal`, `stash`).
    id("map.seek"),       // `map::seek_selected`: the graph jumps to the tip
    id("journal.revert"), // `journal::enter`: undoes/redoes the entry
    id("stash.apply"),    // `stash::operate_on_selected`, keeping the entry
    id("stash.pop"),      // the same, popping
    id("stash.drop"),     // `stash::stash_key`
    id("stash.branch"),   // `stash::shown_key` → `name_prompt::open_name_prompt`
    // The branch picker (`picker::shown_key`): a text field, keys fixed.
    id("picker.switch"), // `picker::enter` → `switch::switch_into`
    id("picker.show"),   // `picker::show_on_graph` → `map::seek_to`
    // The confirm box (`confirm::confirm_key`).
    id("confirm.left"),   // the cursor between choices
    id("confirm.right"),  // the same
    id("confirm.choose"), // the choice under the cursor
    // The acting button (`PendingConfirm::yes`); also the finish list's
    // commit (`finish_key` → `commit`, P12) and the rewrite editor's start
    // (`row_key` → `after::start`) — the one confirm key (m8/09c, D3).
    id("confirm.yes"),
    id("credential.send"), // `credential::credential_key`: the answer to git
    // The finish summary (`finish::finish_key`).
    id("finish.toggle_diff"), // `finish_key` → `toggle_diff`
    // Its `⌃E` now acts only in the rewrite message field
    // (`rewrite_editor::keys::field`), whose `$EDITOR` hint names this row.
    id("finish.edit_outside"),
    id("commit.commit"), // the commit editor's `control_key` → `commit`
    // The blame and history places (`place::place_key`, `history_key`).
    id("blame.parent"),        // `place::keys::caret`
    id("blame.line_history"),  // `place::keys::history_of_lines`
    id("blame.heatmap"),       // `place_key`: the heatmap
    id("blame.ignore_revs"),   // `place::keys::rerun`, the ignore file toggled
    id("history.next_change"), // `history_keys::next`
    id("history.prev_change"), // the same, backward
    // The rewrite editor (`rewrite_editor::rewrite_key`).
    id("rewrite.toggle_changes"), // `row_key` → `toggle_changes`
    id("rewrite.pick"),           // `row_key` → `mark`
    id("rewrite.reword"),         // `row_key` → `open_field`
    id("rewrite.edit"),           // `row_key` → `mark`
    id("rewrite.squash"),         // `row_key` → `mark`
    id("rewrite.fixup"),          // `row_key` → `mark`
    id("rewrite.drop"),           // `row_key` → `mark`
    id("rewrite.move_up"),        // `row_key` → `move_row`
    id("rewrite.move_down"),      // the same
];
