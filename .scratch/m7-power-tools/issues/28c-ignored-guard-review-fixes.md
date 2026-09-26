# 28c ignored-guard-review-fixes — review 28's Majors and fixable Minors

Status: ready
Blocked by: none (06c merged).
Crates: graft-ops (`in_the_way.rs` and its split, `operation.rs` StashInTheWay words, tests), plus docs: ADR-0049's table and `docs/design/02-keymap.md:425`.
Build with: Opus (unchanged from 28; git writes).
Scout: none. The review cites every site with file:line. Re-read each one before editing, because 06b and 06c touched graft-ops.
Split by: conductor, 2026-09-26, from `docs/reviews/m7-28-ignored-guard.md`. No new decision: each fix is the review's.

## Scope (each at the cause, red first)
- **M1** (§M1): pull diffs `HEAD` against the tracking tip (`in_the_way.rs:210`). Diff the incoming side instead, from the merge base to the tip. Amend ADR-0049's table to say so, in an amendment line dated today.
- **M2** (§M2): a case-only rename on a case-insensitive disk is refused and called "(ignored)" (`:87-91`, `:607`).
- **M3** (§M3): `in_the_way.rs` is 718 lines; bring it under 400. Split the two functions over ~60 lines. Replace the runtime word template with plain formatting. No behaviour change.
- **m1 / M7/28skip** (§m1): model git's skip of unchanged picks, so that `R` and `a` which leave the untracking commit unchanged are not refused.
- **m4** (§m4): when any of `StashInTheWay`'s paths are ignored, the refusal says `(ignored)` and gives the next step, in the same words as every other kind. Edit the keymap line at `02-keymap.md:425`.
- **m5** (§m5): time the read for a switch on the 100k synthetic fixture, with a script writing `docs/perf/m7-28.txt`. Read only its summary line. If it is over budget, list the index only when `on_disk` found candidates.
- **m6** (§m6): split the test file into `in_the_way/{one_shot,stash,pull,replay}.rs`, each ≤ 400 lines. Add one absorb case.

## Acceptance
- [ ] M1: the review's probe as a test, red first; plain git's "Already up to date" case passes.
- [ ] M2: the case-only rename probe as a test, red first (skip it where the disk is case-sensitive, and say so).
- [ ] M3: `wc -l` for each new file, all ≤ 400; the words unchanged, shown by existing tests.
- [ ] m1: the review's `R` probe and `a`-into-first-commit as tests, red first; the refusing case still refuses.
- [ ] m4: a stash put-back over an ignored file, test names the words.
- [ ] m5: the summary line from `docs/perf/m7-28.txt` quoted in NOTES.
- [ ] m6: `wc -l` per test file; the absorb case named.
- [ ] Gates workspace-wide.

## Touches
Every caller of the guard (apply, preview, the engine), the pull path used by 03b, and ADR-0049. Name how each was checked.
