# Scout 10 merge-preview (main 2610bce9)

## How the merge preview is built today
- Open: `update/preview.rs:387` `open_preview` sets `PreviewState` (`:238`, fields subject/resident/job/plan/rows/ghost_index/undrawable/cursor/refused…) and pushes `Command::PlanPreview`. `PreviewSubject::Operation { op, opened_by: &'static str }` (`:175-181`).
- Job: `jobs/preview.rs:41` `run_preview` reads facts, calls `graft_ops::preview::plan_as_run(&handle,&git,op,…)` (`:63`). One path for every op kind and `RevertTo`.
- Forecast: **in graft-ops.** Merge plan `graft-ops/src/preview/kinds.rs:107` `plan.conflicts = forecast_one(repo,git,None,into,source,stop)`; `forecast_one` = `landing.rs:128-143` wrapping `history_reads::forecast_merge` → `ConflictForecast::{Clean,Likely{paths},NotChecked(why)}`. `forecast_one` is also used by cherry-pick/revert (`kinds.rs:157,213`), stash (`stash.rs:206`), put-back (`put_back.rs:124`). → crate set must include graft-ops, or the merge forecast must be read app-side outside `plan_as_run` (conductor decides).
- Box words: `update/preview/plan_lines.rs:271` `conflict_line`: Clean → `no conflicts expected`; Likely → `may conflict: <3 paths>`; NotChecked → `conflicts: not checked (<why>)`.
- Widget: `graft-ui/src/preview_box.rs:50` `PreviewBox { title, lines: Vec<SanitizedText>, commands, planning, confirm_key, again, keep, back_only, choices }`. **No body enum** — body is `lines` + `commands`. Buttons via `with_choices` (`update/preview/boxes.rs:111` `[Cancel, <label>]`; `:42,:64,:152`). Ghost rows: `PreviewState.rows`/`ghost_index`, drawn by `view::preview`.

## GitVersion
- App holds none. `jobs/preview.rs:51-53` calls `system_git()?.version()` per plan. (A cached `SYSTEM_VERSION` OnceLock exists only in `jobs/rewrite.rs:160-180`.)

## Refusals / unrelated
- `PreviewFailure::Refused` (`jobs/preview.rs:67`) → `answer.rs:58` `refused()` (`:157`) **closes the box** and puts the refusal on the **status line** (`operate::refusal_status`). Only stash put-back stays in the box (`answer.rs:53-55`, `refused=true`).
- Merge write `graft-ops/src/operation/history.rs:738-754`: `merge --no-edit --ff|--ff-only|--no-ff -m <subj> <src>` — **no `--allow-unrelated-histories`**; `y` on unrelated → git refuses (stop/failed path; not traced).
- `UNRELATED = "the two branches share no history"` (`graft-git/src/history_reads/merge_check.rs:19`); today's `forecast_merge` NotCheckable words for unrelated: could not confirm.

## Key routing
- `edge.rs:41` `keymap.translate(ContextId::Preview, chord)` runs **first**, then `dispatch` → `keys.rs:198 boxes_key` → `preview/keys.rs:20 preview_key`.
- `keys.rs:41-46` confirm first: `confirm_key` (`:78`) hard-matches `Char('y')`, or `m`/`r` via `is_history_key` (`history.rs:23`) when equal to `opened_by`.
- `:67` `KeyCode::Enter => enter(...)` (choice under cursor, `:192`); `:68` `KeyCode::Escape => close_preview` — **hard match, not `app.back`** (footer *spells* `app.back`). `←/→/Tab` move choice (`:60-66`). `_ => {}` swallows — **j/k never reach a move today**; `stands_down` (`keymap.rs:227`) only lists Map/Stash/Journal.
- Opened by `m`: `opened_by == "m"` (`history.rs:47`); `footer_keys` `keys.rs:176-178` adds `preview.confirm_merge`. Drag-drop (`drag.rs:252`) and `.` menu (`context_menu/verbs.rs:259`) open with `opened_by: "."` (`context_menu.rs:350`) — **not "m"**. Palette merge: could not confirm its `opened_by`.
- `Preview` borrows `confirm.yes` (`keymap.rs:204`) ✓.

## Footer / bottom bar
- `preview_box/footer.rs:33` `footer_label`: parts from keymap via `key_verb` (`:11-14`), **joined by two spaces** (`:17-23`), e.g. `y confirm  m merge  ←/→ choose  ⏎ act  esc cancel`. Not ` · `.
- Bottom bar empty with preview: `graph_screen/hints/filters.rs:20` `HintContext::Preview => (Vec::new(), None)` ✓.

## Contradictions with the ticket
1. Hints path `hints/context.rs:97-99` → actual `graft-ui/src/graph_screen/hints/context.rs:133` (variant) and `filters.rs:20` (empty bar).
2. `preview_box.rs:28` is `pub(crate) use footer::footer_label` (re-export), fine; but `PreviewBody { Plan(..) }` — no such type; body is `PreviewBox.lines`.
3. "Refused → the box says why (today's)": today a merge refusal closes the box → status line (`answer.rs:58,157-172`).
4. "`esc` = the preview's existing cancel route" / `app.back`: it is a hard `KeyCode::Escape` match (`keys.rs:68`); remaps arrive via translate's default press.
5. Touches says drag-drop merge is "opened by `m`": it is `opened_by "."`, so `⏎`→list / `m m` would not apply there.
6. Ticket says forecast may sit in app: it is graft-ops (`kinds.rs:107`).
7. Footer separator ` · ` vs existing two-space join (all previews).
