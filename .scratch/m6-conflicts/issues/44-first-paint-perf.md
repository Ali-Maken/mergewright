# first-paint-perf: ticket 21's perf step

Status: done
Blocked by: none.
Was: split by the conductor 2026-09-24 from ticket 21 (Remaining step 3).
Build with: **Sonnet**.
Needs: none.
Crates: none (a script, and `docs/perf/`).

## Scope
In: write `.scratch/m6-conflicts/perf-m6-21.sh`, modelled on `.scratch/m6-conflicts/perf-m6-07.sh`. It runs the release `heavy::budgets` and writes the `first_paint_us` lines and the summary to `docs/perf/m6-21.txt`. Measurement never runs in the helper's context (CLAUDE.md): run the script and read only its summary lines.

Out: any code change. A budget that fails → report it with the number; don't fix.

## Acceptance (each tick names its test, at tick time)
- [x] `docs/perf/m6-21.txt` written by the script; the summary lines quoted in NOTES, with pass/fail against each budget — test: the release `heavy::budgets` run named there. **Ticked: `cargo test --release --features heavy-tests -p graft --test it -- heavy::budgets` via `.scratch/m6-conflicts/perf-m6-21.sh`.**

## NOTES

Script run 2026-09-24, own CARGO_TARGET_DIR; summary lines only read:
- `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 401 filtered out; finished in 3.38s`
- `first_paint_us: 5 runs, max 126075 us, 0 over the 150000 us budget` → **pass** (first paint <= 150 ms)
- `peak_rss: max 112 MiB, 0 at/over the 300 MiB budget` → **pass** (peak RSS < 300 MiB)
- `exit 0, wall 45 s`
No scout: no crate code touched; read heavy.rs budget constants directly (crates/graft/tests/it/heavy.rs:30-33). No compaction.
