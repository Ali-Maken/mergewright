# Time budgets fail in release only; every review runs a release perf run

Decision (user, 2026-09-13, during m1-the-graph/30c): **input-to-frame
(16 ms) and first-paint (150 ms) budgets fail only in a `--release` build.**
The debug heavy tier still runs those tests and prints the numbers
(`debug build, budget recorded not asserted`). **Peak RSS (300 MB, measured
by `getrusage`) fails in both builds.**

PROMPT.md §7.1's heavy-tier command gains a second, release run. TESTING.md
"The heavy tier" and `.claude/agents/reviewer.md` step 2 carry it:

```sh
cargo test --features heavy-tests -- --test-threads=2
cargo test --release -p graft --features heavy-tests --test it -- frames:: heavy::budgets_hold --test-threads=1 --nocapture
```

## Why

- PROMPT.md's budgets are claims about the binary users run. Debug code is
  2–5x slower, and not uniformly: m1/30c measured the pixel tier at 72 ms
  p99 debug against 15 ms release, and 30d's first paint with 50k refs at
  494 ms debug against 57 ms release. A debug assert either fails forever
  or needs a looser, invented limit.
- Local-only release runs get skipped. Making the release run a standing
  review step keeps the budgets checked at every milestone.
- Debug regression guards with their own limits (`many_refs.rs`, 1.5 s /
  200 ms) stay as they are: they are not PROMPT.md budgets.

## Measurement caveat: CPU power state

The frame numbers depend on the CPU's power state (m1/30c part b, Apple
Silicon build machine). A 60 Hz paced scroll is light load; the first
heavier burst meets a slow CPU:

- The script's slowest key is always **the first key that sends a row
  request**, wherever it falls: key 37 (default script), 47 (10 `k` first),
  57 and 97 (20 and 60 no-op keys first). It is not the first key after idle
  and not arbitrary. Release: 12.5–21.4 ms over 7 runs.
- Its cost is three frames back to back: the key's frame, a layout
  progress frame, and the rows' frame. The `j` frames before it take
  6–9 ms; after it 3–5 ms.
- **With one unrelated thread spinning on another core** the same key took
  6.5–6.8 ms (3 runs), p99 6.6–7.0 ms (from 11.7–15.0 ms), and the scattered
  16.3–17.4 ms `PageUp` misses (keys 371–419) disappeared. Nothing in Graft
  changed between the runs, so the variance is the CPU's, not the code's.
- The first key after ≥ 0.5 s idle before the scroll missed as well
  (17.0–24.8 ms; 16.0 ms with the spinner).

How the release assertion treats this (exclude the named key, hold the CPU
busy while measuring, or cut the per-key cost until a cold CPU fits) is
**open**: m1-the-graph/30c's handoff asks the user.

## Applied: rows on hand (m1-the-graph/30f, 2026-09-13)

Not a new decision; this rule applied to one more check. `frames::`'s
rows-on-hand check (no key drawn before its viewport rows arrived) depends
on pace as the time budgets do: whether layout stays two screens ahead of
`PageDown` held at 60 Hz (~2,400 rows/s) is a question of how fast the
worker runs. The first debug run failed it (pixel 50k: 42 keys, pixel 100k:
19, Unicode 50k: 1); 7 release runs never did. It fails in release and is
printed in debug (`rows on hand recorded not asserted: N of M keys`), so a
regression stays visible. The script's pace was not slowed for debug.
