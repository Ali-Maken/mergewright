# Scout 03 — settle work at rest (HEAD 261c6be0)

## Where per-row work starts today
- `update(state, event)` is pure, one event at a time (`update/mod.rs:130`); every event ends in `settle` (`mod.rs:135,142` → `mod.rs:179-207`). No frame/rest input exists; `update` has no `rested` parameter.
- Loop: `take` runs `update` then `carry_out` **immediately** (`event_loop/running.rs:340-355`), before any draw. So jobs start per key event, not per frame. Draw is separate (`running.rs:261-262`, `395-409`, Pacer gates via `may_draw`).
- `j`: `keys.rs:438-449 move_cursor` → `selection::move_from_graph` (`selection.rs:35`) → `History::move_cursor`; emits `ShowRows` only when the window must shift (`keys.rs:444`).

## Q1: what one `j` emits
Pane **shown** (`settle` → `selection::follow_selection`, `selection.rs:106`):
- `detail.follow` (`detail/ops.rs:60`) → `load` (`ops.rs:237`): `CancelJob` for pending files list and diff (`ops.rs:259-266`), then `ListFiles` (`ops.rs:249`) + `ContainedIn` if uncached (`ops.rs:250-255`). **`ContainedIn` is never cancelled** on the next `j` — no CancelJob for it.
- `probe_evil` (`ops.rs:179`): on a merge row (non-stash, `selection.rs:136-147`) with EvilMerge severity on: `CancelJob` old pending probe (`ops.rs:197-198`) + `EvilProbe` (`ops.rs:211`). Stale answers dropped by id (`ops.rs:229`).
Pane **hidden**: `followed` returns nothing (`selection.rs:152`), **but `probe_evil` still runs** — it has no `shown` check (`ops.rs:185-212`); comment at `ops.rs:176` claims "nothing is probed when the pane is hidden" — contradicts code; `selection.rs:113-118` says probe follows selection regardless.
Other settle calls (`radar::request_context/force_push/duplicates`, `journal::request`, `remote::*`, `detail::open_explain`, `mod.rs:188-202`) not shown to depend on cursor — could not confirm each is cursor-independent.

## Contradictions with the ticket
- **Emphasis (thread) is not a job.** Computed in the view each frame: `view/entries.rs:40 thread` → `graft_graph::ancestry::Ancestry::of` (`graft-graph/src/ancestry.rs:44`), called twice per frame (`view/rows.rs:58`, `view/strips.rs:224`). "Jobs for emphasis… cancelled/generation" has nothing to attach to; deferring it means changing view/strip (graphics.rs also mentions emphasis — **overlap with 04**).
- "Rest = frame with no motion queued": update runs before draw, per event; rest has to come from the loop, not `update`.
- "Detail keeps last rested commit, `reading…` muted": today `load` overwrites `spec` and sets `Pending` → body draws `reading…` (`view/detail.rs:263,289`); last commit is not kept.
- Results for a left row: files/diff/evil already dropped by id via `Loading::Pending(id)`; `ContainedIn` keyed by oid — could not confirm stale handling.

## Q2: per-frame work scaling with refs/commits
- `view/strip.rs:38-42` rebuilds a `HashSet` of **all** search hits every frame (KNOWN_GAPS:414, still true).
- `Ancestry::of` scales with resident window rows (`ancestry.rs:45`), ×2 per frame. Window size bound not confirmed.
- `radar::refresh_ref_findings` is gated on change (`mod.rs:196-199` comment); not verified.

## File set
`update/mod.rs` (settle), `update/selection.rs`, `update/detail/ops.rs`, `update/detail/state.rs`, `update/keys.rs`, `update/mouse.rs` (clicks, `ShowRows` 196/216), `update/search.rs`, `update/events*` (if new event), `event_loop/running.rs`, `event_loop/tests/pace.rs`, `view/detail.rs`, possibly `view/entries.rs`/`rows.rs`/`strips.rs` (emphasis), `tests/it/held_key.rs`, `held_key_bound.rs`, `event_loop/tests/pixel_scroll.rs`, `MAP.md`.
Overlap: **04** if emphasis touches `event_loop/graphics.rs`. **14**: none of the above is `update/history.rs`/`preview.rs`/`clash.rs`, but `update/history.rs:68` also emits `ShowRows` — don't touch. `journal::request`/`place.rs` (blame) share `settle` only.
