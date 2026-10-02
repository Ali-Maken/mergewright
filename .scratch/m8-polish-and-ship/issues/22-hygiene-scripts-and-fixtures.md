# G-hygiene 2 — scripts, fixtures and captures fit to publish

Status: partial (8 of 10 items; 2 partial/open — see NOTES)
Build with: Sonnet (conductor, after scout `scouts/22-hygiene-scripts.md`)
Blocked by: none
Needs: none. Files: `scripts/`, `.scratch/*/pty-walk-*.py` (capture writers), `crates/graft-fixtures`, `fuzz/`,
`docs/shots/*.esc`. Runs beside the scroll lane (disjoint).

## Read first
- CLAUDE.md "Gates" (captures ≤ 200 KB, raw terminal streams never committed)
- `crates/graft-fixtures/MAP.md`; `crates/graft/MAP.md` `ghost`/`capture` test helpers (`check_shot`)

## Items (from the open rows; fix decided)
1. **The ref-states fixture's `HEAD` isn't where its builder says** (builder does `checkout --orphan fresh`; built
   fixture has `HEAD`→`main`). Fix the build's final steps / cache key; a test asserts `HEAD`.
2. **Compaction has no corpus shape and no snapshot** → a `ManyLanes` corpus shape; snapshots at 256/16/`NO_COLOR`.
3. **`scripts/pty-walk-m6.py` `Run::end` has the unread-pty wait that hung m5-11** (graft blocks in `write`, misses
   SIGTERM) → drain the pty until exit, as the m5 script's fix does; same for every `pty-walk-*.py`.
4. **Fuzz targets ran far past `-max_total_time=60`** (`line_log` 590 s, `parse_file_list` 397 s,
   `parse_porcelain_v2` 1001 s) → add `-timeout=10` per input in the fuzz runner script; if a slow input is found,
   save it under `fuzz/slow/` and file it (NOTES).
5. **Walk m6-06 depends on the history sort finishing under 300 ms** (`update/commit_graph.rs:23-25`) → the walk sets
   the offer state explicitly instead of racing the threshold.
6. **Question: `GRAFT_UPDATE_SHOTS=1` stops after the first PNG in `write_shots`** (graft-graph tests; not
   reproduced) → reproduce; fix or close with the run as evidence.
7. **`scripts/fix-scroll-emphasis-cost.sh:11` refers to the raster path deleted in 46b** → point at the current path
   or delete the script (say which).
8. **shellcheck findings in perf scripts**: 20× SC2164 (`cd` without `|| exit`), 2× SC2034 (`m4-review-perf.sh:13`,
   `:15`), 1× SC2296 (`47-debug-idle.sh:38`) → fixed; `scripts/check-scripts.sh` runs shellcheck over `scripts/`.
9. **Pty captures reproduce only on this machine** (the header sizes itself to the real scratch path's width) → a fixed
   scratch location (`$TMPDIR/graft-capture/<scenario>`), not a wider placeholder; regenerate twice, byte-identical.
10. **Three `.esc` escape-stream captures are committed** (`docs/shots/{24d-kitty-check,25-iterm2-capture,
    33-kitty-check}.esc`, ~132 KB each). Planner ruling: the Gates rule means no protocol bytes in the tree. Replace
    each with a `.txt` digest (each APC command's header fields, payload size and sha256, in order); update the
    references in M1 tickets, `docs/capabilities/m1-the-graph.md`, `docs/log/m1.md`; delete the `.esc` files.

## Acceptance
- [ ] One test or script output per item, named at tick (10 ticks) — 8 ticked, 2 not:
  - [x] 1 — `graft-fixtures` `it::corpus::each_ref_shape_leaves_head_where_its_builder_says` (passes). **No defect**: the orphan `fresh` step is `unborn_branch` (`shapes/refs.rs:184`), whose built HEAD is `refs/heads/fresh`; `ref_states` ends `checkout_main` (`refs.rs:143`), HEAD→`main` by design. Test pins all three ref shapes.
  - [ ] 2 — **partial.** `Shape::ManyLanes` (in new `Shape::WIDE`, not `ALL`) + layout snapshot `graft-graph` `it::corpus::many_lanes_lays_out_as_snapshotted` (15 columns). 256/16/`NO_COLOR` tier snapshots **not done**: they live in graft-ui tests (held by another builder) → `blocked: needs graft-ui`.
  - [x] 3 — m3/m4/m6 `Run::end` now pump the pty until exit with a 10 s `EXIT_GRACE` then SIGKILL (m5's shape). Verified: `python3 -m py_compile` on all four; **walks not run** (unverified at runtime).
  - [x] 4 — `scripts/fuzz-run.sh` (`-timeout=10`, slow inputs to `fuzz/slow/<target>-*`). Run `scripts/fuzz-run.sh 20 line_log parse_file_list parse_porcelain_v2` → `docs/perf/fuzz-run.txt`: 23 s wall each against 20 s (line_log 237 s includes the nightly build). No slow input found in 20 s; the 60 s full run is a review job.
  - [x] 5 — `pty-walk-m6.py` `relaunch` writes `.git/graft/commit-graph-offer` = `declined` before opening the real binary (graft-git `commit_graph::offer_file`), so the offer never races `SLOW_SORT`. **Walk not run** (unverified at runtime).
  - [x] 6 — reproduced by line: `raster.rs` `check_png` asserted `!updating` after writing each PNG, so the first panic ended `write_shots`. Now collects into `Regenerated` and fails once at the end. Run `GRAFT_UPDATE_SHOTS=1 cargo nextest run -p graft-graph raster::write_shots` → "regenerated 10 capture(s)" (all 10), `git status docs/shots` no PNG changed; plain run green.
  - [x] 7 — closed, no change: `fix-scroll-emphasis-cost.sh:11` is prose; the script runs `emphasis_layer::emphasis_cost`, which exists (`crates/graft-graph/tests/it/emphasis_layer.rs:517`). No raster path in the script.
  - [x] 8 — 20× SC2164 → `|| exit 1`; SC2034 `m4-review-perf.sh:13` was a false read of the SC2296 at `:15` (`${((…))}` → `$((…))`, now uses `t0`); `47-debug-idle.sh:38` unused `i` → `_`. `scripts/check-scripts.sh` (shellcheck `-x -S warning`) prints `check-scripts: ok`.
  - [ ] 9 — **not done** (handoff below).
  - [x] 10 — `scripts/esc-digest.py` → `docs/shots/{24d-kitty-check,25-iterm2-capture,33-kitty-check}.esc-digest.txt` (43, 3 and 49 commands; header fields, payload size, sha256, in order); `.esc` removed; refs updated in `.scratch/m1-the-graph/issues/{24d,25,33}-*.md` and `iterm2_capture.py`. `docs/capabilities/m1-the-graph.md` and `docs/log/m1.md` name none (grep); `docs/log/m5.md:1116` and m5/41 are history, left.
- [x] `git ls-files 'docs/shots/*.esc'` → `0`; `find docs/shots -size +200k` → empty.
- [x] `scripts/check-scripts.sh` → `check-scripts: ok`, exit 0.

## NOTES (builder, 2026-10-02)
- Scout used (`scouts/22-hygiene-scripts.md`).
- Item 9 handoff: the scratch repo is made by the walk tests' `prepare_capture` in `crates/graft/tests/it/{m3_walk.rs,m4_walk/run.rs,m5_walk/run.rs}` (graft crate), not by the python. Next step: give `graft-fixtures` a `copy_into(shape, dir)` and have each `prepare_capture` use `/tmp/graft-capture/<scenario>` under `--capture`. **Ticket's `$TMPDIR` does not fix it**: macOS `$TMPDIR` is `/var/folders/<random>/T/`, per-user length; `/tmp` is the same everywhere (check how `capture::machine` redacts `/private/tmp`). Then regenerate each walk twice, byte-compare. Touches graft tests → conductor to route.
- Item 10 digest: the 25 file holds 3 commands — the kitty capability query (`a=q`) then the 2 iTerm2 bands; the `.esc.txt` decodes that already existed are kept beside the digests.
- Not added: `docs/shots/*.esc` to `.gitignore` (outside my set) — would stop a re-commit.

## Touches
- Every pty-walk script and the captures they write (re-shot only if bytes change — list); the M1 docs naming the
  `.esc` files; `nothing_a_commit_would_carry_names_this_machine` (must stay green); G-rename's list (scratch dir name).

## Scout questions
- Which capture writers derive layout from the scratch path's length (file:line)?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide;
plus `scripts/check-scripts.sh`.

## Size
One context, ~50k tokens.
