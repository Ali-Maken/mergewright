# Scout — g06 app-answers-beat-rereads

Explore, sonnet, 55 tool uses, 217 s, 2026-09-22. Read-only; conductor wrote this file from the reply. Conductor spot-checked two claims: `fetch_key`'s silent refusal (`update/remote.rs:294-296`) and `mod.rs:537` as the only non-test writer of `state.in_progress` (`view/resolver.rs:361,375` are inside `#[cfg(test)]` from :271). Both held.

## Structural cause of bugs 2 and 3
`Command::ReadWorkingTree` / `Product::WorkingTree` and `Product::Refs` carry **no `JobId`** (`events.rs:98,206,410`; routed at `mod.rs:200-203` matching `_` for the id). Nothing can tell a stale working-tree re-read from a fresh one. The JobId-on-state pattern exists elsewhere: `state.operating != Some(id)` (`operate.rs:738`, `undo.rs:169`, `banner.rs:567`), `state.remote.reading != Some(id)` (`remote.rs:255`).

## Bug 1 — re-push lease from memory (gap row 557)
`repush_for`, `update/remote.rs:630-663`: `lease.expected` from `facts(state).head_upstream.tracking_tip` (:636-640) — live `state.remotes`, written only by `remotes_arrived` (:250-275) from a background `ReadRemotes`. `source_commit` correctly reads `target.refs_before` (:649-654). `JournalEntry.refs_after: BTreeMap<String, Option<Oid>>` exists (`graft-core/src/journal.rs:318,336,338`) and is never read here. The `reading` guard (:255) drops superseded reads, not a fresh read outrunning the journal's answer.

## Bug 2 — `u` right after `--continue` refused (row 569) — the window is real
`banner::resumed` (`update/banner.rs:561-577`) on `Operated::Done` clears `banner_focus` and sets `Status::Done` (:574-575) but never touches `state.in_progress`. Only writer: `mod.rs:537` in `took_fact`'s `WorkingTree(Ok)` arm, fed by the id-less re-read scheduled by `refresh_scope`/`refresh_key` (`operate.rs:961-987`). `in_progress_refusal` (`update/undo.rs:103-109`) → `banner::live` reads `state.in_progress` (`banner.rs:64-67`) → refused until the re-read lands. Precedent for the fix shape: `banner/tests.rs:411-435` `finishing_clears_the_focus` — its doc comment says `banner_focus` is cleared "before any re-read lands" on purpose. Same reasoning, not applied to `in_progress`.
- Two separate `Done` paths: `banner::resumed` (:573-577) and `operate::operated` (`operate.rs:752-763`). A shared `operation_ended` touches both.

## Bug 3 — relaunch lands on the conflict (Decision 2a)
`launch_into_live`, `update/journal.rs:145-163`: sets `banner_focus = true` and Sticky `"<kind> still in progress from a previous run"`, keys `LIVE_KEYS = "c continue  a abort"` (:134). No `⏎ resolve`, no detail pane, no cursor. Called from `took_fact` on `first_read` only (`mod.rs:545-550`), guarded by `live_entries.is_empty()` (:146) and `banner_focusable` (:154).
- Existing test `crates/graft/tests/it/conflict_interim.rs:761` `relaunch_into_a_live_conflict` pins today's shape (keys `"c continue  a abort"`, no pane/cursor assertion) — **must be updated**, not only added beside.

## Bug 4 — `fetch_key` swallows its refusal (row 481)
`update/remote.rs:289-303`: `if operation_in_flight(state) { return true; }` — no status. `pull_key` (:307-324) same guard sets `Status::Failed(why)`. No test covers `fetch_key`'s refusal.

## Row 505 — `Operated::Conflicted` has no entry id
`jobs/operate.rs:60-80`: fields `description, kind, paths, stalled` — no `EntryId`. Built from `ApplyError::Conflicted { entry, .. }` / `ResumeFailed { entry, .. }` at :310-324, which keep only `entry.description`. Nothing downstream references it. Real work, not a close-the-row.

## Routing
`update/*` push `Command`s → `event_loop/running.rs::carry_out` (:160-203) handles terminal-owning ones synchronously, rest to `jobs.dispatch` (`jobs/pool.rs`) → `AppEvent::Produced(JobId, Product)` → `update::mod::answer` (`mod.rs:164-412`).
