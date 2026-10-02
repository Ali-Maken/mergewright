# G-hygiene 2 — scripts, fixtures and captures fit to publish

Status: draft
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Sonnet**: each item has its fix named.
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
- [ ] One test or script output per item, named at tick (10 ticks).
- [ ] `git ls-files 'docs/shots/*.esc'` empty; no file in `docs/shots/` over 200 KB (`find` output quoted).
- [ ] `scripts/check-scripts.sh` exits 0.

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
