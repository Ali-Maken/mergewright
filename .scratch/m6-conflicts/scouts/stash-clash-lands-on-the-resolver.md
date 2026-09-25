# Scout 72 — stash clash lands on the resolver (main @ fc3a0f47)

## What exists
- Op: `Operation::StashApply/StashPop {index, commit}` `graft-ops/src/operation.rs:64,71`; plan `stash::plan_at("pop"|"apply", index, true)` :993-994; pins `[commit]` for Pop/Drop `apply/record.rs:313`. App builds it `graft/src/update/stash.rs:449`.
- Apply path `graft-ops/src/apply.rs:193-262`: `check` → `Refused` :198; `run_plan` :230; `stopped_on_conflict` :361-377 returns None unless `op.starts_in_progress()` (merge/rebase/cp/revert only) — so a clashed pop falls to :250-254: `EntryStatus::Aborted{reason}` + `failed.into_error` (i.e. `Git`). No index read on the failure branch specific to stashes (could not confirm what `finish` records as `index_after` on Aborted).
- Journal constructors: `apply/record.rs:50 begin`, `:101`; `journal/lines.rs:114,135-136`; `apply/fold.rs:134`; plus ~25 test fixtures (graft-ops `journal/tests.rs:29-110,471`, `undo/tests.rs:51`, graft `update/tests.rs:4862`, `operate.rs:1043`, `undo.rs:654,1189`, `journal.rs:322`, `view/journal.rs:246,334`, `view/tests.rs:2583-2634`, `remote/tests.rs:843,1223,1293`, `remote/tests/lease.rs:227`, `banner/tests.rs:57`, `resolver/tests/words_and_c.rs:22`, `jobs/undo.rs:617`, `tests/it/conflict_interim.rs:972`, `undo_by_key.rs:217`). A `..Default` or field would touch each.
- `Refusal::Unmerged` words `operation.rs:550-553` ("…in the shell first (Graft resolves conflicts in a later milestone)"); raised `:907`. Surfaces via 67's `refusal_status` `graft/src/update/operate.rs:954`, called `:928` and `preview.rs:648` (`PreviewFailure::Refused` `preview.rs:590`).
- Conflicted arm `update/operate.rs:799-843`: sets `live_entries = vec![entry]` :808, `stopped = Some(Stop{origin: ThisRun, again, kind, paths_at_stop})` :816, `banner_focus`, then `banner::land_in_resolver` :842 (`OpenedFrom::Banner`, test :2265).
- `Stop::kind: graft_git::state::InProgress` `update/state.rs:728` — a Stop cannot exist without an InProgress today.
- Close: `update/resolver/resolved.rs:141-155 close_after_last` — stash-pop case, `resolver = None`, status `every_file_done` — lands on the **graph**. `every_file_done_parts` :295-305 drops `c` when no banner. `finish_owed` `resolver.rs:221`, cleared `done.rs:115`, `answers.rs:224`, `status_read.rs:91`.
- `side_names` `update/resolver/names.rs:39`; stash arm `None if state.resolver.is_some()` → `SideRoles::StashPop`, ours = HEAD branch, theirs **None** (~:70) — no `stash@{n}` source anywhere.
- Journal read in app: `update/journal.rs:16,32,353` (`JournalRead`, only when `J` loads — could not confirm it is read after each write/at launch). `relaunch.rs:94` uses `live_entries`.
- Tests on today's stash clash: `graft/tests/it/resolver/stash_pop.rs:12,74` (no banner, closes resolver); captures with "every file done": `docs/shots/m6/m6-07-all-resolved-*`, `fix-every-file-done-footer-120x40`, `fix-finish-*`, `m6-15-finish-row-after-esc-*`, contact sheets 1-3, `prototype-journeys.txt`. Walks: `tests/it/m5_walk/{conflict_journeys,resolved_operations,resolving}.rs`, `resolver/file_list.rs`.

## Keys
None new; resolver `o/t/⏎/esc`, `u`, `S p/⏎` as today.

## Mismatches
1. Ticket cites `resolved.rs:295-305`; file is `update/resolver/resolved.rs`, and the landing is `close_after_last` :141-155, not :295.
2. Landing via `OpenedFrom::Pane` — the only landing helper today uses `Banner` (`operate.rs:842`); pop goes through the Failed arm, not Conflicted.
3. Stop line needs a Stop with no `InProgress` — `state.rs:728` forbids it; new shape needed.
4. `live_clash(... stashes)` from `RefBadges::stashes` plus journal entries: app only holds `live_entries` (Conflicted only) and `J`'s lazy read.
5. Q9 radar `is_stash`: not checked.

## 71 overlap
71 holds `context_menu.rs`/`verbs.rs`, graft-ui; none of 72's files. No overlap found.

## Model
Opus; size warrants splitting app half (touches operate, state, resolver, relaunch, stash).
