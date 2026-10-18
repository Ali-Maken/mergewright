# Scout 06s — split update/ files (at 2352d87f, after 06h merge)

Paths under `crates/mergewright/src/update/`. No cargo run.

## Sizes now
| file | lines | inline tests | non-test |
|---|---|---|---|
| state.rs | 1318 | none | 1318 |
| keys.rs | 474 (ticket: 487) | none; tests in keys/tests.rs etc. | 474 |
| selection.rs | 639 | `mod tests` 257-639 | 256 |
| preview.rs | 429 | none (`mod tests;` :429 → preview/tests.rs 2152) | 429 |

## state.rs
- `AppState` struct :31-522 (**492 lines on its own**, 100+ fields), `Screen` :526, `impl AppState` :545-807 and :938-1048, `GraphOverlay` :808, `Harness` :834-884, `Banner` :885, `Quit` :903, `Stop` :1049, `ClashState` :1069, `RefsRead` :1082, `IndexRead` :1118 (pub(crate)), `HeadLanding` :1138, `StashOnlyMarks` :1160, `Status` :1187, `impl Status` :1270 (`done` :1281, `done_with_keys` :1290), `Size` :1313.
- Re-exported by `update/mod.rs:84-87` (`pub use state::{AppState, Banner, CHROME_ROWS, ClashState, GraphOverlay, Harness, HeadLanding, Quit, RefsRead, Screen, Size, StashOnlyMarks, Status, Stop}`); 206 files say `state::`. Moving a type keeps callers working if mod.rs re-exports from the new home.
- 06h landed: `graph_mode` field :431.

## keys.rs
- dispatch `key_event` :183-305, `before_any_key` :167; per-family fns :65-162, :306-435; `apply_view_change` :436 (ticket: 447); `remote_key` :450; `move_cursor` :463.
- `keys/` holds: acknowledge 47, actions 82, **bound 178 (`#[cfg(test)]`, keys.rs:16-17; doc: "a claim the tests check against the hint tables, not code the dispatch runs")**, commit 39, edit_stop 69, escape 70, predicates 159, refresh 65, view_change 208, table_check 40 (test), tests 298, edit_stop_tests 249, live_c_tests 579, tab_words_tests 218.

## selection.rs
- `move_from_graph` :35, `reconcile` :81, follow cluster `FollowKey`/`follow_*`/`pane_follows` :98-197 (re-exported mod.rs:83), `selected_merge` :198, `followed` :213. Outside callers: context_menu/tests, detail/ops, detail/state, keys, mod, resolver/tests, state.

## preview.rs
- `CannotStop` :109-154, `PreviewSubject` :155, `PreviewRequest` :182, `PreviewFailure` :194, `Planned` :206, `PreviewState` :220-324, helpers :325-373, `open_preview` :374-427.

## Contradictions with the ticket
1. **state.rs cannot reach <400 with `AppState` whole in it**: the struct is 492 lines (:31-522). Moving every other type still leaves >400.
2. `facts_pending` (:68) and `fold_counts` (:502) are **fields**, not types; nothing to move to `update::facts`/`update::fold_count` except their `FoldCount` type, already in fold_count.rs.
3. No `update::status` module exists; `Status` words are drawn in `view/status.rs` (other layer).
4. `resident_slice` **does not exist** (grep of src: no match). `preview/slice.rs` is itself 431 and in over_400.txt:176 — moving into it grows it.
5. `bound.rs` is test-only; "a toggle like 07's `#` goes there" would put dispatch code in a `#[cfg(test)]` module. 07's `bound.rs:78` cites a test table.
6. selection.rs non-test is 256: moving tests to `tests.rs` alone gets <400 and the ticket's rule permits it (non-test not over 400).
7. Line drift: keys.rs 487→474, `apply_view_change` 447→436; `Status` 1188→1187, `done` 1282→1281.
- Path-string pins: none in tests for these four (seam_rule.rs:107 uses a fixture path). Comments cite `update/keys.rs` (palette.rs:347 `:125`, focus_keys.rs:210, finish/choices.rs:331, journal_put_back.rs:120) — stale-able, not failing.
- Could not confirm: whether 11b/12b diffs touch state.rs (not merged here).

## Build with
Sonnet for keys/selection/preview (plain moves, re-exports cover callers). state.rs needs a decision first (contradiction 1) — Opus or conductor ruling.
