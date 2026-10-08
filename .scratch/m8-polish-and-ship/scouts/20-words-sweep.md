# Scout: 20 words-sweep (read-only, HEAD b5c5cbc0)

Blockers: 10b, 10e, 10f, 10i, 10k, 10l, 14 done; **10 "lands with 10c"** (10c done with deviations); **10h partial** (captures box); 10g, 10j partial.

## Per item
1. **Looks done, contradicts ticket.** `view/detail.rs:177-185` passes `compare_state(&state.detail,…)`; `:217-226` fills `files/added/removed` from `detail.files`; ladder `graft-ui/src/detail_pane/header.rs:50-53`; test `detail_pane/tests.rs:1349,1358` expects `· 2 files +10 −4`. Could not confirm it draws in the app (no run).
2. Open. `header.rs:124-127` `evil merge: clean (N files hand-resolved)`; separate `EvilUnchecked` → `evil merge: unchecked` at `:121`. Line moved (ticket says :55).
3. Open. No blind-spot line in TESTING.md (grep "blind spot|both sides touched" hits only `mistake_radar/probes.rs:165-167`).
4. Open. `AfterState` `apply/error.rs:222-252`: no `BackupUnread`.
5. Clause code moved: `update/undo.rs:790-805, 842-878` (ticket: 645-652). `J to inspect` spelled from `graph.journal` `:805`. Cut behaviour not checked.
6. Open by grep: no path-naming refusal found in `snapshot/raw.rs`; could not confirm the refusal site.
7. Open. `view/stash.rs:76-83` own `age` (no `now` guard); `view/journal.rs:335-345` guarded copy.
8. Open. `header.rs:217` `format!("Contained in: {}", names.join(", "))` with empty names still drawn.
9. Open. Header `this file +3 −1` (history snaps `graft-ui/src/history/snapshots/*`); no `these lines` string in code.
10. Moved: `update/preview/plan_lines.rs:90-101` totals (`commits replayed`), `:118`, `:125-146` per-ref `N new commits`. Not verified whether counts overlap.
11. Open by grep: no `refused · nothing written` anywhere; journal mark `RowMark::Undone` `view/journal.rs:194`.
12. Open. `apply/lost_edit.rs:101-104` still `paths.len()`.
13. Open. Doc `apply/error.rs:43-45, 52`. `apply.rs:419` not re-read; `kept:` only at `apply.rs:175` by grep — could not confirm the :419 site still exists.
14. Open. Words `apply/error/words.rs:25` (moved from `error.rs:301-305`).
15. Moved/changed: reason now `push rejected: <remote> has commits you don't` (`apply/remote.rs:314`); split-at-` · ` comment `:299-301`. Cut not tested here.
16. **Partly done, contradicts ticket.** `p pull first` gone in ops: now `p pull, then m or r` + `P force with lease` (`apply/remote.rs:304-316`). Stale doc comment still says `p pull first` at `update/remote/in_flight.rs:228-229` and `graft/MAP.md:261`. Spec's `p fetch and choose` not present.
17. Open. `update/operate/failure.rs:247` `words.reason.width()` (moved from :195); whether `reason` is sanitised there not confirmed.
+ Askpass: `apply/remote.rs:131` `cleanup.push(AfterState::AskPass(error))` (ticket says :128).

## Captures
Old strings in `docs/shots/` (e.g. `Contained in:`, `evil merge: clean`): m2-01..04, m2-10, m2-13, m2-17, m2-detail-*, m5-lane-cap.
