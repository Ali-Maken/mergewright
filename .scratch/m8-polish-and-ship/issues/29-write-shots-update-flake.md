# 29 — `raster::write_shots` red under `GRAFT_UPDATE_SHOTS=1`

Status: done (debugger, 2026-10-09). Gap row `M8/write-shots-update-fails`.
Build with: debugger (diagnosing-bugs).

## Cause — not a race, not test order: deterministic, in the test's own checker

`check_png` in `crates/graft-graph/tests/it/raster.rs` (pre-fix lines 844-889, at `e4733b87`):

- :845-848 — when updating, it **wrote the PNG first**, then :849 read it back. The comparison was the new picture
  against itself, so it could never see drift.
- :887-889 — when updating, it pushed **every** path into `Regenerated`, whatever the comparison said.
- :894 (`Regenerated::finish`) — red if the list was non-empty.

So under `GRAFT_UPDATE_SHOTS=1` the test was red on every run with all 10 PNGs listed as "regenerated", even when
not one byte changed. Every other capture goes through `graft_fixtures::capture::check_raw`
(`crates/graft-fixtures/src/capture.rs:180-189`), which reads the committed file **before** rewriting and errs only
on a difference — so a no-drift update run is green everywhere except here.

Reproduced before the fix, alone, 3 of 3 runs red, `git status` clean afterwards (nothing rewritten):

```
GRAFT_UPDATE_SHOTS=1 cargo nextest run --workspace -E 'test(=raster::write_shots)'
  FAIL ... panicked at crates/graft-graph/tests/it/raster.rs:894:9: regenerated 10 capture(s): ...
```

Ruled out: race on shared files (the only other test naming a `22-raster-*` PNG, `gallery.rs:1258`, writes into a
tempdir; the test fails run alone); test order (same).

Unverified: 20c's "passed on two reruns". With the pre-fix code a run with `GRAFT_UPDATE_SHOTS=1` cannot pass, so
those reruns were most likely run without the variable. Not confirmed — 20c's exact commands weren't available.

## Fix (cause)

`check_png` reads the committed PNG before writing, compares pixels against that, and records only real drift
(missing/unreadable, size, pixels) in `Drifted` (renamed from `Regenerated`). Drift is red whether or not the run
rewrote the file; no drift is green in both modes — the same contract as `graft_fixtures::capture`. Side effect:
a non-update run now reports every drifted PNG at once instead of panicking at the first.

`updating` is now a parameter (the caller passes `graft_fixtures::updating()`), which is the seam the tests use.

## Boxes

- [x] No-drift regeneration is green — `raster::check_png_regenerating_an_unchanged_capture_is_green` (red before
  the fix, green after).
- [x] Drift while regenerating is red and the file is rewritten —
  `raster::check_png_regenerating_a_drifted_capture_rewrites_it_and_is_red`.
- [x] Drift without regenerating is red and the file is left alone —
  `raster::check_png_drift_without_regenerating_is_red_and_writes_nothing`.
- [x] Missing capture is red even when created — `raster::check_png_missing_capture_is_red_even_when_created`.
- [x] Original symptom gone — `raster::write_shots` under `GRAFT_UPDATE_SHOTS=1`: 3 of 3 green, no files changed.
  Also checked by hand: copied `22-raster-octopus-merge.png` over `22-raster-foxtrot-merge.png` → plain run red
  (size), update run red and file restored byte-identical, rerun green.

## Touches

- Callers of `check_png`/`save`: only `write_shots` and `write_thread_shot` in the same file.
- Other writers of these PNGs: `grep save_png|updating()` across `crates/` — only this file. `gallery.rs:1258` uses
  a tempdir.
- Captures: none changed (`git status docs/shots` clean).
- Scout: none — one function in one test file, found by the failure message's line.
