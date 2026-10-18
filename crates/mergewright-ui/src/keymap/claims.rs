//! What each place claims of other places' rows (`ContextId::borrows`)
//! and the `Global` rows a modal overlay answers with nothing
//! (`ContextId::stands_down`), each list with the line in graft that
//! claims or swallows them. Split from `keymap.rs` for size (m8/09f).

use super::ActionId;

// The rows each place answers itself (`ContextId::borrows`), with the
// line in graft that claims them.

// `context_menu_key` claims j/k, the arrows, ⏎, esc and `.`
// (graft `update/context_menu.rs:133-161`); every other key
// closes the menu and falls through.
pub(super) const MENU_BORROWS: &[ActionId] = &[
    ActionId("app.down"),
    ActionId("app.up"),
    ActionId("app.open"),
    ActionId("app.back"),
    ActionId("app.menu"),
];

// `detail_key` (graft `update/detail/keys.rs:123-167` ⏎, esc,
// ⇥/⇧⇥; `:285`, `:326` j/k, PgDn/PgUp, g/G in files and diff),
// before `graph_key`.
pub(super) const PANE_BORROWS: &[ActionId] = &[
    ActionId("app.down"),
    ActionId("app.up"),
    ActionId("app.page_down"),
    ActionId("app.page_up"),
    ActionId("app.top"),
    ActionId("app.bottom"),
    ActionId("app.open"),
    ActionId("app.back"),
    ActionId("app.next_pane"),
    ActionId("app.prev_pane"),
];

// `map_key` (graft `update/map.rs:159-172`): esc, j/k, ⏎. Ahead of it
// (`update/keys.rs:309`, the overlays first while the graph has focus)
// `journal_key` opens J (`update/journal/keys.rs:29`) and `stash_key`
// opens the stash list (`update/stash.rs:175`).
pub(super) const MAP_BORROWS: &[ActionId] = &[
    ActionId("app.down"),
    ActionId("app.up"),
    ActionId("app.open"),
    ActionId("app.back"),
    ActionId("graph.stash"),
    ActionId("graph.journal"),
];

// `stash::shown_key` (graft `update/stash.rs:204-218`): esc, j/k, ⏎;
// ahead of it `journal_key` opens J (`update/keys.rs:309`,
// `update/journal/keys.rs:29`).
pub(super) const STASH_BORROWS: &[ActionId] = &[
    ActionId("app.down"),
    ActionId("app.up"),
    ActionId("app.open"),
    ActionId("app.back"),
    ActionId("graph.journal"),
];

// `journal::shown_key` (graft `update/journal/keys.rs:83-108`):
// esc, j/k, n/N, ⏎.
pub(super) const JOURNAL_BORROWS: &[ActionId] = &[
    ActionId("app.down"),
    ActionId("app.up"),
    ActionId("app.open"),
    ActionId("app.back"),
    ActionId("app.next_match"),
    ActionId("app.prev_match"),
];

// `place_claim` (graft `update/keys.rs:124-130`) ahead of
// everything; blame's keys `update/place/keys.rs:54-77`.
pub(super) const BLAME_BORROWS: &[ActionId] = &[
    ActionId("app.down"),
    ActionId("app.up"),
    ActionId("app.page_down"),
    ActionId("app.page_up"),
    ActionId("app.top"),
    ActionId("app.bottom"),
    ActionId("app.find"),
    ActionId("app.next_match"),
    ActionId("app.prev_match"),
    ActionId("app.open"),
    ActionId("app.back"),
];

// `place_claim`, then `update/place/history_keys.rs:29-52`.
pub(super) const HISTORY_BORROWS: &[ActionId] = &[
    ActionId("app.down"),
    ActionId("app.up"),
    ActionId("app.page_down"),
    ActionId("app.page_up"),
    ActionId("app.next_pane"),
    ActionId("app.open"),
    ActionId("app.back"),
];

// The `Global` rows each modal overlay answers with nothing
// (`ContextId::stands_down`), with the line in graft that swallows them.
// Graph search stands down under every overlay: `/`, `n`, `N` need the
// graph shown (graft `update/search.rs:20-28`, `graph_is_shown`; m8/09e).

// `map.rs:176`: the Map swallows every plain key it does not own.
pub(super) const MAP_STANDS_DOWN: &[ActionId] = &[
    ActionId("app.next_pane"),
    ActionId("app.prev_pane"),
    ActionId("app.quit"),
    ActionId("app.top"),
    ActionId("app.bottom"),
    ActionId("app.page_down"),
    ActionId("app.page_up"),
    ActionId("app.find"),
    ActionId("app.next_match"),
    ActionId("app.prev_match"),
    ActionId("app.menu"),
];

// `stash.rs:224,226`: the stash list swallows every plain key it does not
// own. It passes ⇥/⇧⇥ to the detail pane (`stash.rs:199`), so those stay.
pub(super) const STASH_STANDS_DOWN: &[ActionId] = &[
    ActionId("app.quit"),
    ActionId("app.top"),
    ActionId("app.bottom"),
    ActionId("app.page_down"),
    ActionId("app.page_up"),
    ActionId("app.find"),
    ActionId("app.next_match"),
    ActionId("app.prev_match"),
    ActionId("app.menu"),
];

// `journal/keys.rs:111,113`: J swallows every plain key it does not own.
// Its own find answers `/` (`journal/find.rs`), and `n`/`N` step its lost
// rows (`JOURNAL_BORROWS`), so those stay.
pub(super) const JOURNAL_STANDS_DOWN: &[ActionId] = &[
    ActionId("app.quit"),
    ActionId("app.top"),
    ActionId("app.bottom"),
    ActionId("app.page_down"),
    ActionId("app.page_up"),
    ActionId("app.menu"),
];
