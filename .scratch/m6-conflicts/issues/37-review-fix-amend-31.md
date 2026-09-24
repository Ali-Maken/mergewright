# review-fix-amend-31: `checking the files…` strands when the read fails; `Closed` leaves the resolver open; sizes

Status: ready
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
- [ ] M1: failing tests first (read `Err`, job failed), red on unchanged code — tests:
- [ ] M1: neither leaves `checking the files…`; each ends in words — tests:
- [ ] Closed: failing test first — test:
- [ ] Closed: `Incomplete` / `Conflicted` land where stated — tests:
- [ ] M2: splits, counts in NOTES, MAP.md — gates green
- [ ] Gates workspace-wide: fmt, clippy `-D warnings`, nextest, doctests

## NOTES
