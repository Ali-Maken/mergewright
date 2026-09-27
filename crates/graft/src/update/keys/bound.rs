//! The table of every `(key, verb)` the graph screen binds, split out of
//! `keys` (review m6-amend-16 M3): a claim the tests check against the
//! hint tables, not code the dispatch runs.

/// Every `(key, verb)` this build actually binds on the graph screen and
/// the panes its footer hints from, spelled exactly as
/// `graft_ui::graph_screen::hints`'s tables spell them (`⏎` for `Enter`,
/// `j/k` for either move key, `⌃A` for `Ctrl+A`).
///
/// The **verb** is half the claim: `M` is bound, but to the Map view, so a
/// hint reading `M mergetool` is as dead as one on an unbound key — which
/// is exactly what the key-only version of this list let through (m5-03
/// review, Critical 2 and Major 6). `update::tests` walks it three ways:
///
/// - every hint `hints_for` draws must be in here
///   (`every_key_the_bar_draws_is_answered`), and every hint drawn live must
///   also be *answered* by the handler that has focus;
/// - every `HintState::Planned` entry in the hint tables must **not** be in
///   here (`every_planned_hint_is_still_unbound`), so the group that binds
///   `m merge` cannot add it here without also flipping the hint to live;
/// - and every line **here** must be drawn live in some context, or be named
///   in `update::tests::NEVER_HINTED` with the reason it never is
///   (`every_bound_action_is_hinted_live_or_is_named_here`). Until m5/33 the
///   comment here said "both ways" and meant two walks that both started
///   from the hint tables: a key bound in a handler and listed here but put
///   in no table at all went through every sweep untouched.
///
/// Each line names the dispatcher it is true because of. Keep them in step:
/// this list is a claim about the code below it, and the tests only compare
/// it with the hint tables.
#[must_use]
pub fn bound_actions() -> &'static [(&'static str, &'static str)] {
    BOUND_ACTIONS
}

/// [`bound_actions`]' list. A `const`, not the function's own body: it is a
/// table, and a table is not a function long enough to want splitting.
const BOUND_ACTIONS: &[(&str, &str)] = &[
    // `graph_key`, this file.
    ("q", "quit"),         // `is_quit`
    ("F2", "status"),      // `is_screen_toggle`
    ("F3", "renderer"),    // `is_tier_cycle`
    ("+", "lanes"),        // `is_lane_toggle`
    ("F", "focus"),        // `is_focus_toggle`
    ("y", "sha"),          // `is_copy_key` → `copy_selected_sha`
    ("x", "compare"),      // `is_compare_key` → `toggle_compare_selected`
    ("c", "commit"),       // `is_commit_key` → `commit_editor::open`
    ("a", "amend"),        // `is_commit_key`, amending
    ("m", "merge"),        // `write_key` → `preview::history_key`, `Merge`
    ("r", "rebase"),       // `write_key` → `preview::history_key`, `Rebase`
    ("p", "pull"),         // `is_remote_key` → `remote::pull_key`
    ("P", "push"),         // `is_remote_key` → `remote::push_key`
    ("f", "fetch"),        // `is_remote_key` → `remote::fetch_key`
    ("space", "fold"),     // `view_change` → `ViewChange::Fold`
    ("1", "first-parent"), // `view_change` → `ViewChange::FirstParent`
    ("H", "hide stash"),   // `view_change` → `ViewChange::Stashes`
    ("j/k", "move"),       // `motion_for` → `move_cursor`
    ("g/G", "top/bottom"), // `motion_for`
    ("PgUp/PgDn", "page"), // `motion_for`
    ("⌃R", "refresh"),     // `is_refresh` → `refresh_key`
    ("u", "undo"),         // `is_undo_key` → `undo::undo_key`
    ("U", "redo"),         // `is_undo_key`, redo
    ("esc", "dismiss"),    // `key_event`: clears a `Failed`/`Sticky` status
    ("b", "branch"),       // `is_name_key` → `name_prompt::open_from_graph`
    ("t", "tag"),          // `is_name_key`, tagging
    (".", "more"),         // `is_context_menu_key` → `context_menu::open_selected`
    (".", "explain"),      // the same menu's `Explain` item on a marked row
    // The panes and overlays (`pane_and_overlay_key`).
    ("⏎", "open"),      // `detail::keys`: opens the pane on the cursor row
    ("⏎", "changes"),   // the same key on the uncommitted-changes row
    ("<>", "resize"),   // `detail::keys`, while the pane is shown (ADR-0028)
    ("⇥", "graph"),     // `detail::keys`: cycles focus back to the graph
    ("⇥", "banner"),    // `banner::take_focus`: back into a live operation's banner
    ("⇥", "diff"),      // `detail::keys`: from the file list to the diff (m6-conflicts/70)
    ("space", "stage"), // `detail::keys::changes_pane_write_key`
    ("d", "discard"),   // the same, `operate::request_discard`
    ("⌃A", "all"),      // the same, `operate::stage_all`
    ("e", "edit"),      // the same, `$EDITOR` at the cursor's line
    // The operation banner, while it has focus (`banner::banner_key`).
    // `e edit` and `⇥ graph` above are the same keys in another
    // context; these four are the banner's own.
    ("c", "continue"),  // `banner_key` → `continue_pressed`
    ("s", "skip"),      // `banner_key` → `skip_pressed`
    ("a", "abort"),     // `banner_key` → `abort_pressed`
    ("M", "mergetool"), // `banner_key` → `open_mergetool`
    // The conflict keys, on the banner and on a `CONFLICTS` row of the
    // changes pane (m6-conflicts/04).
    ("⏎", "resolve"), // `banner_key`, `detail::keys` → `resolver::open_resolver`
    ("space", "resolved"), // `detail::keys` → `operate::stage_selected` on an unmerged path
    // Inside the resolver (`resolver::resolver_key`).
    ("B", "base"),       // `toggle_base`
    ("d", "start over"), // `restart_box`
    // `o`/`t` read as the operation's role words (`SideRoles`, ADR-0043 §4):
    // one row per word the bar can draw.
    ("o", "yours"),     // `take_side`, `Take::Ours`
    ("o", "upstream"),  // the same, in a rebase
    ("o", "branch"),    // the same, edits set aside clashing (m6-conflicts/84)
    ("t", "theirs"),    // the same, `Take::Theirs`
    ("t", "yours"),     // the same, in a rebase and a set-aside clash
    ("t", "picked"),    // the same, in a cherry-pick
    ("t", "reverting"), // the same, in a revert
    ("t", "stashed"),   // the same, after a stash pop
    ("b", "both"),      // the same, `Take::Both`
    // `Ctrl+O`/`Ctrl+T`, every open block (m6-conflicts/16): `take_all_key`.
    ("⌃O/⌃T", "all"),
    ("n", "next"), // `walk_blocks`, forward
    ("N", "prev"), // the same, backward
    ("⏎", "done"), // `mark_resolved`: `git add` on the open file
    // `O`/`T`: settle the whole file, then the next (m6-conflicts/17).
    ("O/T", "file"), // `take::settle_key`
    ("j/k", "file"), // `files::step_file`, `↓`/`↑` too
    // `c` once every file is done (m6-conflicts/31, P8): a merge's summary.
    // Elsewhere it is `c continue`, the banner's row above.
    ("c", "finish"),   // `finish_pressed` → `banner::continue_pressed`
    ("S", "stash"),    // `stash::stash_key`: opens the overlay
    ("J", "journal"),  // `journal::journal_key`: opens the browser
    ("M", "map"),      // `map::map_key`
    ("O", "overview"), // `strip::strip_key`
    ("C", "switch"),   // `picker::picker_key`: opens the branch picker
    ("/", "search"),   // `search::search_key`, before `key_event`
    // Inside the three overlays, which hint their own lists.
    ("⏎", "seek"),    // `map::seek_selected`: the graph jumps to the tip
    ("⏎", "revert"),  // `journal::enter`: undoes/redoes the entry
    ("⏎", "apply"),   // `stash::operate_on_selected`, keeping the entry
    ("p", "pop"),     // the same, popping
    ("d", "drop"),    // `stash::stash_key`
    ("b", "branch"),  // `stash::shown_key` → `name_prompt::open_name_prompt`
    ("esc", "close"), // either overlay's own `Esc`
    // Dragging a branch badge (`drag::drag_mouse`, `drag::drag_key`).
    // `release` is the mouse button, named as a key so the hint bar can
    // offer it: the gesture ends with the button, not with `⏎`.
    ("release", "drop on a row"),
    ("esc", "cancel"), // `drag_key`: `Esc` drops the badge, doing nothing
    // The three modal boxes (rulings 07b, Q1): `esc cancel` above is theirs
    // too — `confirm_key`'s `Esc` arm, `name_prompt_key` and
    // `credential_key` each close on `Esc`.
    ("←/→", "move"), // `confirm::confirm_key`: the cursor between choices
    ("⏎", "choose"), // `confirm::confirm_key`: the choice under the cursor
    // `confirm::confirm_key`'s `y` arm: the acting button, named by its own
    // label (`PendingConfirm::yes`, m6-conflicts/50) — one per `ConfirmYes`.
    ("y", "push"),
    ("y", "drop"),
    ("y", "discard"),
    ("y", "delete"),
    ("y", "skip"),
    ("y", "abort"),
    ("y", "start over"),
    ("y", "undo"),
    ("y", "redo"),
    ("⏎", "create"), // `name_prompt::name_prompt_key` → `create`
    ("⇥", "toggle"), // `name_prompt::name_prompt_key`: check out after
    ("⏎", "send"),   // `credential::credential_key`: the answer to git
    // The finish summary (`finish::finish_key`, m6-conflicts/19, 59). `j/k
    // move` above is its cursor too (`move_cursor`), `↓`/`↑` too; `PgUp/PgDn
    // page` above moves it a page (`files_key` → `page`, keymap §2.1's
    // never-hinted navigation); `⇧⇥` steps the focus back (`prev`).
    ("y", "commit"),     // `files_key` → `commit` (P12)
    ("⏎", "commit"),     // `subject_key` → `commit`
    ("⌃⏎", "commit"),    // `control_key` → `commit`, from any focus
    ("⏎/space", "diff"), // `files_key` → `toggle_diff`
    ("⇥", "message"),    // `finish_key`: files → subject
    ("⇥", "body"),       // the same, subject → body
    ("⇥", "files"),      // the same, body → files
    ("⌃E", "$EDITOR"),   // `control_key` → `edit_outside`
    ("esc", "back"),     // `finish_key` → `escape`; the blame place's `escape`
    // The blame place (`place::place_key`, m7-power-tools/20). `j/k move`,
    // `g/G`, `PgUp/PgDn` above are its cursor too; `esc back` pops it.
    ("B", "blame"),                  // `detail::keys` → `place::b_key`
    ("⏎", "that commit"),            // `place::keys::enter`
    ("^", "before it"),              // `place::keys::caret`
    ("L", "history of these lines"), // `place::keys::history_of_lines` (m7-power-tools/21)
    ("/", "find"),                   // `place_key`: opens the find prompt
    ("n N", "match"),                // `place::keys::step_find`
    ("space", "ages"),               // `place_key`: the heatmap
    ("w", "whitespace"),             // `place::keys::rerun`, `-w` toggled
    ("i", "ignore list"),            // `place::keys::rerun`, the ignore file toggled
    ("esc", "back to blame"),        // `detail::keys::place_key` → `place::show_again`
    // The history place (`place::history_keys::history_key`,
    // m7-power-tools/21b). `⏎ that commit` and `esc back` above are its too.
    ("j/k", "older·newer"),     // `history_keys::step`, the list focused
    ("j/k", "scroll"),          // `history_keys::step`, the file pane focused
    ("⇥", "file"),              // `history_key`: focus to the file pane
    ("⇥", "list"),              // `history_key`: focus back to the list
    ("n/N", "next change"),     // `history_keys::next`
    ("B", "blame here"),        // `history_keys::blame_here`
    ("F", "history"),           // `detail::keys` → `place::f_key`, on a file
    ("esc", "back to history"), // `place::esc_goes_back` → the hidden history place
];
