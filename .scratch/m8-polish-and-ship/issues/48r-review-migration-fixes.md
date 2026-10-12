# 48r: review fixes for ticket 48 (names table + on-disk move)

Status: ready
Blocked by: none (48 merged at f11ac645)
Build with: **Opus**. The on-disk move: a wrong fix deletes his backups or journal lines unread.
Brownfield: ticket 48 NOTES and `scouts/26-rename-on-disk.md` serve as the scout; the review names every line.
Needs: none. Files: graft-ops `rename/*` and their tests, graft `config/load.rs` (minor 7), MAP.md. Runs beside 49/50 only
if their file lists stay disjoint from these.

## Read first
- `docs/reviews/m8-mergewright-48.md`, whole (Major 1–2, Minor 1–7)
- Ticket 48: Decisions, Part B, NOTES

## Scope (from the review; conductor's routing)
- **Major 1 (data loss):** which generation is scratch is inferred from markers that exist, never recorded or re-checked
  (`strays.rs:75-80`, `scratch.rs:35-49`, `:84-105`, `locks.rs:27-31`). Record the source in the marker (the review's
  suggestion, e.g. `switched tributary`) or an equivalent the builder argues for in NOTES; scratch is deleted only when
  the record says it is scratch. Test-first: each trigger (crash mid stray pass; lock file not yet created) as a
  real-repo test that loses a backup or journal line before the fix.
- **Major 2:** tests for the uncovered windows: Mergewright `switched` from a Tributary source; `switched` with
  scratch; a stray pass cut short; `CleanedUp` beyond `it/rename.rs:242`.
- **Minor 1, 2, 4, 5, 7:** fix as the review says, each with a test where it is behaviour. Minor 2 (stray pass stuck on
  two generations' refs with the same rest) test-first.
- **Minor 3 (question):** answer by test whether `check_journals` must read kept-aside journals; fix only if it loses
  something.
- **Minor 6:** reword the two tick texts in ticket 48.

## Acceptance (each box: the test named at tick time)
- [ ] Major 1 trigger A (crash mid stray pass) test: red before, green after; nothing lost; `git fsck` clean.
- [ ] Major 1 trigger B (lock file not yet there) test: red before, green after.
- [ ] Major 2: one named test per window listed above.
- [ ] Minor 2 test: red before, green after.
- [ ] Minor 1, 4, 5, 7: test or grep named per item.
- [ ] Minor 3: test named, outcome stated.
- [ ] Every older-generation scenario from 48 still green.

## Touches
Callers of `settle_names`; journal reading (kept-aside); config folder; status words; MAP.md; ticket 48 NOTES.
