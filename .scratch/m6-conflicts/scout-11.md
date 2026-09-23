# Scout 11 — preview verb confirms (`m m`, `r r`)

## What exists
- `preview_key` `crates/graft/src/update/preview.rs:377-411`. Modal while `state.preview` is Some. `Ctrl+C` passes (388). `Left` clamps down (399); `Right|Tab|BackTab` clamp up (400-405); `Enter` → `enter` (406); `Escape` → `close_preview` (407); **everything else swallowed** (409). So today `m`/`r` inside the preview do nothing.
- `enter` `preview.rs:414-433`: `!fits` → return (419); cursor 0 or `!acts()` → `close_preview` (427); else `confirm_preview`.
- `PreviewState::planning()` `preview.rs:205` (Idle|Pending); `acts()` `:210`; `planned()` `:196`.
- `confirm_preview` `preview.rs:571`: takes the preview; for `Operation{op, opened_by}` calls `operate::launch_unstoppable(..., *op, ..., Some(opened_by), ...)` (590-601) — the same boxed `op`, not rebuilt.
- `PreviewSubject::Operation { op, opened_by: &'static str }` `preview.rs:100-107`. Openers: `history_key` sets `"m"`/`"r"` (`preview.rs:1052-1063`); context menu `"."` (`context_menu.rs:312-314`); **name prompt `"b"`** (`name_prompt.rs:309-315`). Journal opens `RevertTo` (other variant).
- `is_history_key(key: KeyPress) -> Option<HistoryKey>` `preview.rs:1029-1037`; refuses ctrl/alt.
- Widget footer: `const FOOTER = "←→ choose  ⏎ act  esc cancel"` `crates/graft-ui/src/preview_box.rs:42`, drawn `:215-220` via `border_row`. `PreviewBox` fields `:50-71` (`title, lines, commands, …, planning`); no `again`. Built at `preview.rs:664` (planning) and `:690`. Tests: `graft-ui/src/preview_box/tests.rs` (`mod tests` `:265`).
- Main footer while preview open: `HintContext::Preview` (`view/rows.rs:59-63`).
- `BOUND_ACTIONS` `update/keys.rs:754`; `m merge`/`r rebase` rows exist for graph (`keys.rs:765-766`), none for the preview.
- Keymap: `docs/design/02-keymap.md:218` §2.3b allocates `m`/`r` = Confirm in preview (P1). Claimed, consistent.

## Contradictions
1. Ticket writes `is_history_key(opened_by)`; the fn takes a `KeyPress` and returns `Option<HistoryKey>` (`preview.rs:1029`), `opened_by` is `&str`. Needs a string compare or a new helper.
2. Ticket names `.` and journal as the non-`m/r` openers; `b` (`name_prompt.rs:315`) also opens an `Operation` preview — not mentioned (falls under "gets nothing new" by the m/r rule, but unlisted).
3. Notice `still predicting · wait for the picture`: no such text anywhere; no notice channel from `preview_key` confirmed (Status has no Notice variant, `update/state.rs:648-700`). Could not confirm where the footer notice (`footer.rs:80` `view.notice`) is fed from.

## Build with
Sonnet — two crates (`graft`, `graft-ui`), small, pattern exists.
