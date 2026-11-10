# Scout 11 — codename hits (main @ 121c167c)

Command: `git grep -n -i graft -- . ':!docs/log' ':!docs/reviews' ':!docs/adr' ':!.scratch' ':!KNOWN_GAPS.md' ':!PROGRESS.md' ':!prototypes' ':!docs/gaps'` → **5660 lines**. Shipped set approximated by D1's private list; `export-inventory.md` §8 globs **not applied** (could not confirm exact set — e.g. `docs/perf`, `docs/AUTHORSHIP.md`, `.claude`, `CLAUDE.md` counted below).

## By area
docs 4405 (shots 3764, design-notes 364, research 102, perf 81, capabilities 51, AUTHORSHIP 15) · crates: ui 396, ops 276, app 257, git 88, core 41, fixtures 40, graph 25, term 2, 2 binary · scripts 54 · TESTING.md 15 · .claude 15 · CHANGELOG 13 · ARCHITECTURE 8 · fuzz 6 · CONTEXT 6 · CLAUDE.md 3 · CAPABILITIES 3 · PROMPT 2 · README 1 · clippy.toml 1 · Cargo.toml 1.

## By class (regex heuristics; classes overlap, not exclusive)
- `.snap` `source:` header: 71 lines.
- Fixture identity `Graft Fixture/Test/Perf`: 155.
- Temp-dir/temp-file prefix (`"graft-`, `.graft-tmp`, `graft-raw`): ~70.
- Env vars: 21 — `GRAFT_UPDATE_SHOTS` 10, `GRAFT_ASKPASS_SOCKET` 4, `GRAFT_FRAME_NO_IDLE_ADVANCE` 2, `GRAFT_PERF_TRACE` 2, `GRAFT_REAL_REPO` 2, `GRAFT_BIN` 1, `GRAFT_FRAME_TRACE` 1.
- `fn …graft…` names: 7. Comment lines (`//`/`#` led): 615.
- Capture file names under docs/shots containing graft: 15 lines' paths. `.snap` file names: 0.
- Git's own word (`grafted`/`--graft`/`info/grafts`): 3 — keep.
- Doc text: ~4435 lines in shipped docs (mostly docs/shots capture bodies).
- String written into git/disk: not separately classed — could not confirm without per-line read.

## Old-generation (drop per user ruling 2026-10-25, ticket 11b): **316 lines in crates**
Top: ops `tests/it/rename/review.rs` 33, `tests/it/rename.rs` 28, `tests/it/rename/generations.rs` 24, `…/generations/windows.rs` 17, `…/more.rs` 15, `…/setup.rs` 10, `…/minors.rs` 10, `src/rename/tests.rs` 9, `tests/it/aborted_fetch.rs` 8; app `src/config/folder/tests.rs` 22, `tests/it/rename_names.rs` 18, `tests/it/rename_docs.rs` 12; core `src/on_disk/tests.rs` 19, `src/on_disk.rs` 13; git `src/repo/tests.rs` 7. (Matched by path/word `on_disk|rename|migrat|Generation::Graft|refs/graft|folder|set_aside_gen`; aborted_fetch/repo tests may be false positives — unconfirmed.)

## Existing guards
- `crates/mergewright/tests/it/rename_names.rs:221` `no_old_message_literal`: production-file string literals only, flags `Generation::of_message` older than CURRENT; skips `mergewright-core/src/on_disk.rs` (:230). Name literal check at :263-271 with `ALLOWED` prefix list (:41), incl. all of `mergewright-fixtures/` (:48). Depends on `Generation` table → dropping old-generation code (11b) breaks these guards.
- `rename_docs.rs:91` `old_name_lines`: scans only README.md, CHANGELOG.md top entry, `scripts/**`, words graft/tributary/trib_ (:86), minus `ALLOWED_FILES`. Does **not** scan crates comments, docs/, .snap, TESTING.md.
- Also present: `rename_opens.rs`, `rename_status.rs`, `rename_names/scripts.rs` (not read).

## insta
Cargo.lock:1662-1663 insta **1.48.0**. `runtime.rs:937-941`: on `SnapshotUpdate::Force` a *passing* snapshot is rewritten via `update_snapshot(new_snapshot)` → headers (incl. `source:`) regenerated. Whether `source:` takes the current path verified only by reading, not run.
