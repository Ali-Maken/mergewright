# 12g key-37-cause — name the cause of M1's 17–18 ms key 37 (review m7-12 Major 2)

Status: blocked — measuring runs need the machine quiet (stray process, see NOTES)
Blocked by: nothing to build. **The measuring runs** wait for a quiet machine: no other cargo running (conductor
confirms), and not overlapping 12e's perf rev.
Crates: **none at HEAD.** Works in its own worktree at M1's commit `10138177` (detached, own `CARGO_TARGET_DIR`); that
worktree is throwaway and never merged. At HEAD it adds only `scripts/m7-key37.sh`, `scripts/m7-key37.patch`,
`docs/perf/m7-12g-key-37.txt`, and one "Amended" paragraph in ADR-0021. So it runs while 20k holds graft.
Build with: Opus, dispatched as `debugger` (`diagnosing-bugs` is debugger-only). No product fix: the code under test is
M1's and is gone. The deliverable is a measured cause, or `not proven` after 3 ruled-out hypotheses (debugger rule 3).
Not Fable: nothing structural rides on it.
Scout: none — review m7-04 Major 2 and ADR-0021 `:29-56` hold everything known; there is no HEAD code to scout.
Split by: planner, 2026-09-27. Owns KNOWN_GAPS M7/04-M2.

## Why a ticket of its own, not 12f or 12e
- It shares no file with either: its code is at `10138177`, theirs at HEAD. Groups are files shared (ADR-0029).
- It needs no graft slot, so it isn't queued behind 20k.
- Its done-condition is a cause. As a box on a perf ticket it has been carried four milestones; the user said
  "don't push it a fifth time" (PROGRESS.md :102, quoted in review m7-04 `:68`).

## Read first
- `docs/reviews/m7-04-slow-key.md` Major 2 (`:67-88`), Minor 2 (`:113-117`).
- `docs/reviews/m7-12.md` Major 2 (`:164-192`).
- `docs/adr/0021-time-budgets-fail-in-release-only.md` `:29-56`.
- `git show 10138177:crates/graft/tests/it/frames.rs`, only `:440-480` first (pacing and latency origin), then what
  those lines call.

## What is known (ADR-0021, reviews 04 and 12)
- Key 37 = the first key that sends a row request, wherever it falls (keys 37/47/57/97). Not the first key after idle.
  Release 12.5–21.4 ms over 7 runs; three frames back to back (key, layout progress, rows).
- With a spinner thread on another core: 6.5–6.8 ms.
- The M1 harness paced with `recv_timeout(wait)` and measured **from the due tick** (`frames.rs:456,466-468`), which
  counts up to 8.35 ms of the harness's own oversleep. Since 04b, latency runs from the send.
- Today the key no longer fetches rows (m1/30c), so it cannot be measured at HEAD.

## Scope
**In** — on `10138177`, release, the M1 default script:
1. Reproduce: M1 harness as it was, ≥ 5 runs. Record key 37's latency per run. If it doesn't reproduce above 12 ms,
   stop and say so — that is a finding, not a failure.
2. Split key 37 into its parts, same runs, by a patch to the **harness only** (`scripts/m7-key37.patch`):
   the send time and the due tick per key (→ harness slack), and per-frame draw/update time for key 37's three frames
   (→ Graft's work). No change to anything under `crates/*/src`.
3. The same with ADR-0021's spinner control, ≥ 3 runs (→ CPU power state share).
4. Hypotheses, each confirmed or ruled out by a number from these runs: H1 harness slack; H2 CPU power state; H3 Graft's
   three back-to-back frames. Parts must add up to the observed latency within 1 ms per run, or the remainder is named
   "unexplained" with its size.

**Out** — HEAD code; the idle keys (04 explained them); key 1 (12 fixed it); any change to today's budget or harness.

## Rules
- Measurement never in the helper's context: `scripts/m7-key37.sh` creates the worktree, applies the patch, runs,
  writes `docs/perf/m7-12g-key-37.txt` with a `SUMMARY` line per hypothesis. Read SUMMARY lines only.
- The file records `uptime` before and after every run (review m7-12 Minor 4). Every run stays in the file. The
  conclusion uses runs at 1-minute load < 3 and says how many; a loaded run is marked, never rerun into green.
- Touches: at HEAD, only the four files above. ADR-0021 amendment replaces review m7-04 Minor 2's missing paragraph;
  check `rg -n 'key 37' docs/ KNOWN_GAPS.md` and list, for the conductor, every line that now needs the result
  (log m7.md :32/:67, KNOWN_GAPS M7/04-M2). Don't edit KNOWN_GAPS or the log.

## Acceptance (each tick names its SUMMARY line or command, inline, at tick time)
- [ ] Reproduced (or not) on `10138177`: runs, key 37 min/median/max — cite the SUMMARY line.
- [ ] H1 harness slack: per-run slack on key 37 — cite the line; confirmed / ruled out.
- [ ] H2 CPU power state: spinner runs vs plain — cite the line; confirmed / ruled out.
- [ ] H3 Graft's three frames: per-frame time — cite the line; confirmed / ruled out.
- [ ] Parts sum to the observed within 1 ms per run, or the remainder named with its size — cite the line.
- [ ] `bash scripts/m7-key37.sh` re-runs end to end from a clean checkout (second run's SUMMARY lines cited).
- [ ] ADR-0021 "Amended (m7/12g)" paragraph: the cause in one sentence, each share in ms, the perf file path.
- [ ] One line for the user at the M7 stop, in his terms: "The slow key you saw in M1 was …", with the numbers.

## Gates
No HEAD Rust changes, so the cargo gates don't apply at HEAD; `bash -n scripts/m7-key37.sh`. The throwaway worktree is
removed at the end (say so).

## Size
Small in code (a harness patch, a script); long in wall time (release build at M1, ~15 runs). One context.

## NOTES (debugger, 2026-09-27) — blocked before the measuring runs

No scout: the ticket names none and there is no HEAD code to scout. Used `diagnosing-bugs`.

**Stop 1 — the machine was not quiet.** pid 41125, a debug `it` test binary from the removed worktree
`.claude/worktrees/agent-ae58ad1336bb58fb5` (`cargo test -q -p graft --test it blame_place`, parent a claude shell),
ran at 100% of one core for 5 h+ the whole session. `sample` shows it looping in
`blame_place::capture_blame_shallow_187x50` → `m7_walk::blame::open_on_line_22` → `recv_timeout` + `update`: a test
that never ends. One busy core is ADR-0021's spinner condition, so every "plain" run is a spinner run and H2 cannot
be told apart. Pausing it was refused by the permission system (not my process). Load at start 4.45 (1-min).
**Needs:** someone to end pid 41125 (and find why that walk never ends — likely a hang bug of its own), then
`bash scripts/m7-key37.sh`.

**Stop 2 — the ticket's commit is not where key 37 was measured.** `10138177` differs from `b5380399` (the commit that
recorded ADR-0021's key 37 numbers, m1/30c part b seam 1) in 21 `crates/*/src` files (30b, 30d, 30c part a `2bf6bf3e`
"a row request lays out only the rows it names", 35c) — `git diff --stat 10138177 b5380399 -- crates/`. Measured: at
`10138177` the first key to ask for rows is **75**, not 37 (smoke, 2 runs, 7.1 / 9.6 ms). At `b5380399` it is 37.
`b5380399` itself changed only tests and docs, so its src is the code that produced the 12.5–21.4 ms. The script
defaults to `M1=b5380399` and says why.

**Built and checked (smoke only, not evidence):**
- `scripts/m7-key37.patch`: harness only (`tests/it/frames.rs`): wake hand-off times, `KEY37`/`NEAR`/`KEYS` lines,
  `KEY37_SPIN=1` spinner. Parts sum to the latency by construction: smoke max |latency − parts| = 2 µs.
- `scripts/m7-key37.sh`: worktree, patch, release build, PLAIN=6/SPIN=3 interleaved, `uptime` + busy processes
  per run, `SUMMARY passN` lines; runs at load ≥ 3 or beside a busy process marked `*`, left out. `bash -n` ok;
  ran end to end once (PLAIN=1 SPIN=1) → `docs/perf/m7-12g-key-37.txt` pass 1, marked SMOKE, 0 quiet runs.
- Smoke shape, both runs with the stray core busy (so warm-CPU numbers): key 37 = 9.0–10.4 ms = harness oversleep
  5.6–7.0 ms + three frames 3.4 ms (key 1.2, progress 1.1, rows 1.1) + worker wait 0 (answer already queued).
  All keys' oversleep p50 4.5 ms, max 7.7 ms. Suggests H1 + H3 carry a warm key 37 and H2 carries the rest to
  17–18 ms, but that needs the quiet runs: **not a finding yet.**

**Acceptance:** none ticked. ADR-0021 not amended (no measured cause yet). Worktree and target dirs removed.

Touches: at HEAD only `scripts/m7-key37.sh`, `scripts/m7-key37.patch`, `docs/perf/m7-12g-key-37.txt`, this ticket.
`rg -n 'key 37' docs/ KNOWN_GAPS.md` still to be listed with the result.
