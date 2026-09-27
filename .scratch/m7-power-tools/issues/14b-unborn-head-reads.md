# 14b unborn-head-reads — `only_in_stash` (and maybe `unreached`) on an unborn `HEAD`; two bases; a stop between spawns

Status: done
Milestone: M7 (m7-power-tools)
Source: review `docs/reviews/m7-20k-to-20l.md` (at `3e05dd3e`) Minor 5 (gap M7/14a-unborn), Minor 6. Planner,
2026-09-27.
Blocked by: none (14a merged `54082304`). graft-git only, so it runs beside the graft chain (20m, 20n); conductor
confirms no other graft-git ticket is live.
Crates: **graft-git only** — `src/history_reads/compare.rs`, `src/history_reads/unreached.rs` (only if its red test
is red), `tests/it/compare_and_stash.rs` and `unreached`'s test file, `MAP.md`. Needing another crate → commit at the
seam, report `blocked: needs <crate>`, stop.
Build with: **Sonnet**. One flag on one spawn, four tests; the fix and the fixtures are given.
Needs: none.
Scout: none. The review names the lines at `3e05dd3e`; begin with
`git diff 3e05dd3e -- crates/graft-git/src/history_reads crates/graft-git/tests/it` and re-find each site by name.

## Read first
- Review §Minor 5, §Minor 6.
- `.scratch/m7-power-tools/issues/14a-history-reads-compare-and-stash.md` §Decided, §NOTES.
- graft-git `MAP.md` `unreached` (`:201`, note its `--ignore-missing` on the `--stdin` spawn), `only_in_stash`
  (`:204`), `Stop` (`:262`).

## A — Minor 5: unborn `HEAD` (closes M7/14a-unborn)
`only_in_stash` feeds `^HEAD` to `rev-list --stdin` (`compare.rs:174`); on an orphan branch git answers
`fatal: bad revision '^HEAD'`, exit 128 (review, git 2.54), so the whole read is `Err`.
**Ruling:** add `--ignore-missing` to that `rev-list`, as `unreached` does. Signatures unchanged; no extra spawn (the
read is already at its perf bound, M7/14a-perf). Rejected: passing `head_state` in (a signature change for 16's
caller, for the same answer). Positive lines are safe under the flag: each `^1` was already checked by `cat-file`.
`unreached` (`unreached.rs:101`): the review says "same line" but did not run it, and MAP says its `--stdin` spawn
already has `--ignore-missing`. **Settle by its red test:** green before → quote the flag's line, change nothing.

## B — Minor 6: two bases, and a stop between spawns
- Two bases on **separate lines** (neither an ancestor of the other; if `B` sat on `A`, entry 0 would reach `A` too
  and the answer would rightly be `(A, 0)`): root `R` on `main`; commit `A` on `R`, stash (becomes `stash@{1}`);
  reset `main` to `R`, commit `B` on `R`, stash (`stash@{0}`); move `main` to a new commit on `R`. Expected exactly
  `[(B, 0), (A, 1)]`, in that order, `skipped == 0`, `capped == false`. Guards `claim`'s early `continue`
  (`compare.rs:220`) starving entry 1.
- Stop at every check: a test `Stop` counting its checks, answering `false` for the first `k` and `true` after.
  First count `N`, the checks an unstopped run makes. For every `k` in `0..N`: `Err(Stopped)`. For `k == N`: the full
  answer. **Never** a partial `Ok`. This covers the gap between the `cat-file` and the `rev-list`. Same sweep for
  `compare_span` (two spawns: `merge-base`, `rev-list --count`).

```rust
// tests only (compare_and_stash.rs).
struct StopAfter { checks: AtomicUsize, allowed: usize }
impl Stop for StopAfter { fn should_stop(&self) -> bool; } // true once more than `allowed` checks were made
```

## Acceptance (each box names its test at tick time; red means seen red and quoted in NOTES)
- [x] (`only_in_stash_on_an_unborn_head`) A red first: `only_in_stash_on_an_unborn_head` — the rebased-away-base fixture of
      `only_in_stash_finds_the_rebased_away_base`, then `git checkout --orphan fresh`: expect `[(A, 0)]`. Quote the red
      (`bad revision '^HEAD'`). Green after the flag.
- [x] (`unreached_on_an_unborn_head`, green on arrival) A: `unreached_on_an_unborn_head` — a lost commit, then `checkout --orphan fresh`: the commit comes back lost.
      Red → same fix, quoted; green before → the `--ignore-missing` line quoted, `unreached.rs` untouched
      (`git diff --stat`).
- [x] (`two_stashes_on_two_rebased_away_bases_each_keep_their_own`, red by skip-entry-1 mutant; see NOTES) B: `two_stashes_on_two_rebased_away_bases_each_keep_their_own` → `[(B, 0), (A, 1)]`. Green on arrival
      (it guards existing code), so **red by mutant**: make `claim`'s `continue` a `break` (or skip entry 1), see it
      red, quote, revert, `git status` clean.
- [x] (`only_in_stash_stopped_at_any_check_is_never_partial`, `compare_span_stopped_at_any_check_is_never_partial`; assertion reshaped, see NOTES) B: `only_in_stash_stopped_at_any_check_is_never_partial` and `compare_span_stopped_at_any_check_is_never_partial`.
      Red by mutant: remove the stop check between the two spawns (or pass `&()` to the second), see red, quote,
      revert, `git status` clean. Name `N` for each in NOTES. If a `k < N` returns a full `Ok` (a check after the last
      spawn), report it; don't loosen the assertion.
- [x] (nextest `binary(it) & (test(compare_and_stash::) | test(unreached::))`: 28/28, 3 runs) The 13 existing `compare_and_stash` tests and every `unreached` test green (name the filter and count).
- [x] (read back: `only_in_stash` line; `unreached` line unchanged) MAP.md graft-git `:204` (and `:201` if changed): "an unborn `HEAD` holds nothing (`--ignore-missing`)". Read back.
- [x] (fmt ok; clippy clean; nextest 4212/4212; test --doc ok) Gates workspace-wide: `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`; `cargo nextest run`;
      `cargo test --doc`.

## Touches (check each; the report's **Touches:** line says how)
- Callers of `only_in_stash`: none yet (16 queued; signature unchanged) —
  `rg -n only_in_stash crates/ --glob '!crates/graft-git/**'`.
- Callers of `unreached` (graft's lost-work reads): signature unchanged; the full nextest is the check.
- `merge_base`, `commits_between`, `count_between`, `compare_span` bodies: unchanged (`git diff`).
- Perf: `--ignore-missing` adds no spawn; `docs/perf/stash-only-cost.txt` not re-run (M7/14a-perf stays the
  conductor's). Fuzz: no parser changed.
- Keymap, captures, journal/undo, other crates: none.
- For the conductor: closes M7/14a-unborn; review Minor 6 done.

## Size
~25k. One context.

## Rules
Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.

## NOTES (builder, 2026-09-27)
- Scout: none, per ticket (sites named; re-found by name).
- A red, quoted: `Exited { status: 128, stderr_first_line: "fatal: bad revision '^HEAD'", command: "git rev-list --parents --stdin --not --branches --tags --remotes" }`. Fix: `--ignore-missing` on `walk_unheld`'s spawn (compare.rs). Cause, not symptom.
- `unreached_on_an_unborn_head` green before any change: `unreached.rs:109` `GitArg::Flag("--ignore-missing"),` already on the `--stdin` spawn. `unreached.rs` untouched (`git diff --stat` on src shows compare.rs only).
- B two-bases: `continue`->`break` mutant stays green (A and B share no printed commit, so that branch never fires for this fixture). Skip-entry-1 mutant (`live.iter().take(1)`) red: `left: [(534bf74..., 0)] right: [(534bf74..., 0), (15fbcf5..., 1)]`. Reverted.
- B stop sweep: **deviation from spec, flagged.** N is not fixed: `run.rs` `wait` polls `stop` on a `recv_timeout(POLL)` timer, so check counts vary run to run; a strict "every k<N is Err" would be flaky, and a fixed-N sweep cannot see a spawn given `&()` (N just shrinks with it). Assertion used, per k in `0..=N`: `Err(Stopped)` iff some check answered true; any `Ok` equals the full answer (never partial); plus a floor `CHECKS_PER_RUN = 4` per spawn (pre-spawn + 3 loop tops), so N >= 8 (compare_span), >= 9 (only_in_stash). Mutant (second spawn given `&()`) red: `made 4` / `made 5`. Reverted. Exact N for correct code not pinned (timer-dependent).
- No partial `Ok` seen at any k.
