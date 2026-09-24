# review-fix-amend-31: `checking the files…` strands when the read fails; `Closed` leaves the resolver open; sizes

Status: done
Blocked by: none (31 merged `12e8ac20`). Runs before 20 Part B (same crate).
Was: filed by the conductor 2026-09-24 from `docs/reviews/m6-amend-31.md` Major 1, Major 2, and the reviewer's answer on `resume_answered` (promoted: same hole 31 fixed for `Merged`, still open for `Closed`).
Build with: **Opus** (low).
Needs: none.
Crates: graft.

## Read first
- `docs/reviews/m6-amend-31.md`: Major 1, Major 2, "Your questions" (`resume_answered`) — each names file and line.
- `crates/graft/MAP.md` (`update::status_read`, `update::resolver::resolved`, `update::finish::answers`); `codebase-design` for the split.

## Scope
In:
1. **Major 1.** The notice can strand at `checking the files…`: `update/mod.rs:671` (`WorkingTree(Err)`) and `:940` (`job_failed`) call `answered` but not `abandon`; the notice is set at `update/resolver/resolved.rs:150` and `update/banner.rs:416`. Failing test first (the owed read fails, and the job fails — two cases). Fix at those call sites: a failed or lost read ends the checking line in words, never leaves it.
2. **`Closed` hole.** `update/finish/answers.rs:149-150`: an `Incomplete` or `Conflicted` answer leaves the resolver open, as `Merged` did before 31. Failing test first, then close it the way 31 closed `Merged` — or, if the resolver *should* stay for one of them (e.g. new conflicts to resolve), say which with the reason and test that.
3. **Major 2.** `resolved_advance` 91 lines (`resolved.rs:61-151`), `update/resolver.rs` 435: split, pure moves, to ~50-line functions and ~400-line files where a seam exists; MAP.md; no seam → the count in NOTES.

Out: review-31's Minors (→ KNOWN_GAPS).

## Acceptance (each tick names its test, at tick time)
- [x] M1: failing tests first (read `Err`, job failed), red on unchanged code — tests: `update::resolver::tests::words_and_c::{a_failed_read_clears_the_notice, a_failed_read_job_clears_the_notice}` (both red at the `notice == None` assert before the fix)
- [x] M1: neither leaves `checking the files…`; each ends in words — tests: same two, green: notice `None`, status `Failed("status broke")` / `Failed(".. git died")`
- [x] Closed: failing test first — test: `update::resolver::tests::finish_here::{an_incomplete_commit_from_the_resolver_closes_it, a_new_stop_from_the_resolver_opens_its_own_resolver}` (red: resolver still open / stale path)
- [x] Closed: `Incomplete` / `Conflicted` land where stated — tests: same two, green: `Incomplete` → summary and resolver closed; `Conflicted` with paths → summary closed, a fresh resolver on `fresh.txt`
- [x] M2: splits, counts in NOTES, MAP.md — gates green (fmt, clippy, nextest 2967/2967)
- [x] Gates workspace-wide: fmt, clippy `-D warnings`, nextest 2967 passed, doctests pass

## NOTES
- Status: done. No scout: the review names every line.
- M1 fix (cause): `update/mod.rs` `WorkingTree(Err)` and `job_failed` now call `status_read::abandon` when `answered`; the failure still goes on the status line after. Comments corrected.
- Closed fix (cause): `finish/answers.rs::resume_answered` sets `state.resolver = None` for every non-`Kept` answer (moved above the `Done` check). `Conflicted` with new paths then opens its own resolver via `land_in_resolver`; the resolver should not stay for it — the stale one is on the old stop's file.
- M2: `resolved_advance` 91 → 22 lines (guard, record, dispatch); new `next_not_done` 16, `to_next` 15, `close_after_last` 13, `stay_after_last` 22. `finish_pressed`, `escape`, `close` moved to new child `resolver/leave.rs` (64). `resolver.rs` 435 → 388; `resolved.rs` 197 → 225. Pure moves.
- Compacted: no.
