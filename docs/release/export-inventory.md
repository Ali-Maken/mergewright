# Export inventory — public repo (one fresh commit)

Written 2026-10-18 by `researcher` against HEAD `2bd3209c`. Read-only survey; nothing deleted.
Method: `git ls-files` (tracked only — `target/` 28G and `.claude/worktrees` 13G are untracked and never exported), `git grep`, `du -sh`.

**Blocker found:** there is **no LICENSE file** at top level (`ls -la | grep -i licen` → nothing). A public repo needs one. Which licence? (question for the user)

## 1. Top level

| Path | Class | Note |
|---|---|---|
| `crates/` (1848 files, 21M) | SHIP | the app |
| `Cargo.toml`, `Cargo.lock` | SHIP | |
| `.cargo/config.toml`, `.config/nextest.toml`, `clippy.toml`, `rustfmt.toml` | SHIP | gates use them |
| `.github/workflows/` | SHIP | CI; references no `scripts/` file (`grep -o 'scripts/…'` on workflows → empty) |
| `.gitignore` | SHIP | |
| `README.md`, `CHANGELOG.md` | SHIP | CHANGELOG has 13 "graft" hits |
| `ARCHITECTURE.md`, `CONTEXT.md` | SHIP | contributor docs |
| `CAPABILITIES.md` | SHIP | read by `mergewright-capabilities` (question: confirm it is the input file) |
| `DESIGN.md` | SHIP? | index into `docs/design/`; ship if `docs/design` ships |
| `TESTING.md` (264K) | SHIP? | test checklist, but carries "Times I left" process log — question: split? |
| `packaging/homebrew` | SHIP | |
| `fuzz/` (8.1M, 11 tracked files) | SHIP | `fuzz/corpus` is 8.0M on disk — check how much is tracked |
| `PROMPT.md` | PRIVATE | spec brief, process |
| `CLAUDE.md` | PRIVATE | agent rules |
| `PROGRESS.md`, `KNOWN_GAPS.md` (348K) | PRIVATE | |
| `prototypes/` (804K) | PRIVATE | |
| `.scratch/` (943 files, 11M) | PRIVATE | |
| `.claude/` (11 tracked files) | PRIVATE | |
| `target/` | n/a | untracked |

## 2. `docs/` one level down

| Path | Class | Note |
|---|---|---|
| `docs/man/mwr.1` | SHIP | man page |
| `docs/shots/` (6.9M) | SHIP (mostly) | goldens: 95 files under `crates/` reference `docs/shots` (`git grep -c 'docs/shots' -- crates \| wc -l`), e.g. `crates/mergewright-fixtures/src/capture.rs`, `crates/mergewright-graph/tests/it/raster.rs`. STALE subset in §4 |
| `docs/capabilities/` | SHIP | parsed by `crates/mergewright-capabilities/src/parse.rs` (references `docs/shots`; question: confirm it reads this dir) |
| `docs/generated/capabilities.html` | SHIP? | generated output; ship only if README links it |
| `docs/design/` | SHIP? | keymap + screens; useful to contributors |
| `docs/adr/` (436K) | SHIP? | decisions; many name the codename (0057: 36 hits, 0045: 30, 0047: 14, 0026: 11, 0015: 11, 0013: 10) |
| `docs/release/` | PRIVATE | build/matrix logs, plan.txt, this file |
| `docs/log/` (816K) | PRIVATE | |
| `docs/reviews/` (2.5M) | PRIVATE | |
| `docs/perf/` (3.8M) | PRIVATE | measurement output |
| `docs/research/` (384K) | PRIVATE | |
| `docs/design-notes/` (572K) | PRIVATE | |
| `docs/agents/` | PRIVATE | |
| `docs/gaps/closed.md` | PRIVATE | |
| `docs/AUTHORSHIP.md`, `docs/field-notes.md`, `docs/failure-recurrence.md`, `docs/rules-rationale.md`, `docs/naming-research.md` | PRIVATE | |
| `docs/.DS_Store` | STALE | macOS junk (on disk; check if tracked) |

## 3. `scripts/`, `prototypes/`, `.scratch/`, `.claude/`

- `prototypes/*` (ease-mocks, graph-spike, heap-profile-27b/-59, heap-slope-hk02, m6-conflict-journey, m6-conflict-look, m7-power-tools, m8-polish-and-ship): all PRIVATE.
- `.scratch/*` (codename-rename, ease-pass, housekeeping, m0…m8): all PRIVATE.
- `.claude/agents/*` (9 files), `settings.json`, `skills/conduct/`: PRIVATE. `worktrees/`, `scheduled_tasks.lock`: untracked.
- `scripts/` (72 tracked, 440K): CI uses none. Each script's references counted from SHIP files only (excluding `docs/log`, `docs/reviews`, `.scratch`, `PROGRESS`, `KNOWN_GAPS`, `docs/perf`):
  - SHIP: `scripts/lib/` (machine-state.sh + fixtures), `scripts/testdata/`, `check-scripts.sh`, `check-sandbox.py`, `check-sandbox-test.py`, `mwr_sandbox.py` (imported by other sandbox scripts — question: by which shipped one?), `contact-sheet.sh`, `shot-drift.py`, `shot-drift-test.py`, `release-local.sh`, `cargo-install-check.sh`, `git-matrix.sh`, `linux-fast-tier.sh`, `fuzz-run.sh`, `memory-curve.sh`, `clone-git-git.sh`, `formula.py`, `m8-accept.sh` (current acceptance walk). Question: confirm each by reading — classed by name + reference count, not by reading every script.
  - PRIVATE (one-off milestone perf/debug, output lives in private `docs/perf`): all `m1-*`…`m7-*`, `m8-pace/switch-refs/trace-*/19e-*`, `47-*`, `41-*`, `fix-scroll-*`, `scroll-*`, `heap-slope.sh`, `perf-palette*`, `toolchain-bench.sh`, `ease-ahead-read.sh`, `context-cost.sh`, `esc-digest.py`, the three `.patch` files.
  - STALE (referenced only from private history, `git grep -lw <stem>`):
    - `47-debug-rss.sh` → only `.scratch/m7-power-tools/issues/47-…`, `50-…`
    - `fix-scroll-deflate-cost.sh` → only `.scratch/m7-power-tools/issues/45-…`
    - `m2-fix-1c-evil-audit.sh` → only `KNOWN_GAPS.md`, `docs/log/m2.md`, `docs/reviews/m2-…`
    - `m2-fix-perf.sh`, `m2-fix-perf-1b.sh` → only `docs/log/m2.md`, `docs/perf/m2-review-fix-*`, `docs/reviews/m2-…`
    - `m3-06-fix-perf.sh` → only `docs/perf/m3-06-review-fix.txt`
    - `m8-19e-tour-walk-repeat.sh` → only `.scratch/m8-…/19e-…`

## 4. STALE captures under `docs/shots/`

Test: basename not found in any tracked file outside `docs/shots` (`git grep -h -o '<name>.(txt|png|tsv|py)'`), and its stem (size suffix stripped) not found in `crates/` or `scripts/` (`git grep -qF <stem> -- crates scripts`). 32 files, 212K. Heuristic — question: a test that builds names by format string could miss; `scripts/shot-drift.py` is the authoritative check, not run here.

- Numbered M1: `22-raster-criss-cross-merge-on-dark.png`, `22-raster-foxtrot-merge-on-dark.png`, `25-iterm2-capture.esc-digest.txt`, `25-kitty-resize-check.esc.txt`, `33-kitty-check.esc-digest.txt`, `33-kitty-check.esc.txt`
- M2: `m2-05-feature-branches-x-j-j-x-80x24-unicode.txt`, `m2-13-unicode-names-enter-80x24-unicode.txt`, `m2-15-feature-branches-m-{100x30-unicode,160x40-unicode,80x24-ascii,80x24-unicode}.txt`
- M7: `m7/fix-scroll-contact.md`
- M8 prototype frames (18): `m8-proto-1a…1e`, `2a,2b,2c,2e,2f,2g`, `3b,3e,3l`, `4b,4c,4d,4e` — prototype output, PRIVATE in any case
- `docs/shots/FROZEN.md` flagged by the test but it is an index doc ("Year-age captures no test or script re-draws") — **not** stale; keep, and the files it lists are deliberately frozen.

Removed layout: `docs/shots/m6/fix-three-panes-{180x50,187x50,200x50}.txt` — referenced only by private files (`git grep -l fix-three-panes` → `.scratch/m6-conflicts/…`, `.scratch/ease-pass/issues/11b-…`, `.scratch/m7-…`, `KNOWN_GAPS.md`, `docs/design-notes/m6-conflicts.md`). No crate/script/shipped doc. STALE. Whether the rest of `docs/shots/m6` (1.9M) still matches the post-ease-pass resolver: not checked — question.

## 5. "graft" in SHIP files

`git grep -il graft -- crates | wc -l`: **315 files under `crates/`** mention it. Top counts (`git grep -ic`):

| File | Hits |
|---|---|
| `crates/mergewright-ops/tests/it/rename/review.rs` | 33 |
| `scripts/scroll-pixel-memory.patch` (PRIVATE) | 29 |
| `crates/mergewright-ops/tests/it/rename.rs` | 28 |
| `crates/mergewright-ops/tests/it/rename/generations.rs` | 24 |
| `crates/mergewright/src/config/folder/tests.rs` | 22 |
| `crates/mergewright-core/src/on_disk/tests.rs` | 19 |
| `crates/mergewright/tests/it/rename_names.rs` | 18 |
| `crates/mergewright-ops/tests/it/rewrite_oracle.rs` | 18 |
| `crates/mergewright-ops/tests/it/rename/generations/windows.rs` | 17 |
| `crates/mergewright-ops/tests/it/rename/more.rs` | 15 |
| `crates/mergewright-git/src/repo/tests.rs` | 15 |
| `crates/mergewright-core/src/on_disk.rs` | 13 |
| `CHANGELOG.md` | 13 |
| `crates/mergewright/tests/it/rename_docs.rs` | 12 |
| `crates/mergewright-git/tests/it/snapshot_reads.rs` | 12 |
| `crates/mergewright-ops/tests/it/aborted_fetch.rs` | 11 |
| `crates/mergewright-fixtures/src/lost_work.rs` | 11 |
| `docs/capabilities/m3-…md` / `m1-the-graph.md` | 17 / 10 |

Many are likely the deliberate legacy-name migration (`rename*` tests, `on_disk.rs` reading old `.graft` dirs) — question: which are intentional migration code vs leftovers. Also capture names `35-graft-self-*`, `m2-17-graft-self-*`, `43-graft-self-varied.txt` in `docs/shots`.

## 6. Biggest dirs (`du -sh`, on disk)

`target/` 28G, `.claude/` 13G (worktrees, untracked) — neither exported. Tracked: `crates/` 21M, `docs/` 16M (`shots` 6.9M, `perf` 3.8M, `reviews` 2.5M, `log` 816K, `design-notes` 572K, `adr` 436K, `research` 384K), `.scratch/` 11M, `fuzz/` 8.1M, `prototypes/` 804K, `scripts/` 440K, `KNOWN_GAPS.md` 348K, `TESTING.md` 264K. Inside shots: `m6` 1.9M, `m7` 716K, `ease-pass` 392K, `m8` 156K.

## 7. Secrets / personal data (SHIP side)

`git grep -nE '/Users/[a-z]|devuser|devuser|BEGIN .*PRIVATE|ghp_|AKIA|@gmail'` excluding `.scratch .claude docs/log docs/reviews`:

- `crates/mergewright-fixtures/src/capture/machine.rs:321` — `"devuser"` in a list of account names to scrub/test. Personal username in shipped code — question: rename to a neutral name?
- `crates/mergewright/src/update/commit_editor/mask_tests.rs:184` — `-----BEGIN OPENSSH PRIVATE KEY-----` header: test fixture for secret masking; check the body is fake.
- `crates/mergewright/src/update/credential/tests.rs:366` — `https://ali@github.com` in a prompt string; first name only, test data.
- `crates/mergewright-git/src/mailmap/tests.rs:18` — `Rui M <reza@example.com>`; example.com, but "Rui M" echoes the user's email name — question: neutralise?
- All other emails are `@example.com`/`.invalid`; `git@github.com` strings are protocol text.
- No `/Users/` absolute path in any tracked SHIP file (grep empty). No `.pem/.key/.env` files tracked.
- Not scanned: binary files (PNG metadata), `fuzz/corpus` contents, git author identities (moot: fresh commit sets its own author).

## 8. Proposed export allow-list (globs)

```
Cargo.toml
Cargo.lock
.cargo/config.toml
.config/nextest.toml
clippy.toml
rustfmt.toml
.gitignore
.github/workflows/**
README.md
CHANGELOG.md
ARCHITECTURE.md
CONTEXT.md
CAPABILITIES.md
LICENSE*                      # to be added
crates/**
fuzz/Cargo.toml
fuzz/Cargo.lock
fuzz/README.md
fuzz/fuzz_targets/**
packaging/**
docs/man/**
docs/shots/**                 # minus STALE list below
docs/capabilities/**
scripts/lib/**
scripts/testdata/**
scripts/{check-scripts.sh,check-sandbox.py,check-sandbox-test.py,mwr_sandbox.py,contact-sheet.sh,shot-drift.py,shot-drift-test.py,release-local.sh,cargo-install-check.sh,git-matrix.sh,linux-fast-tier.sh,fuzz-run.sh,memory-curve.sh,clone-git-git.sh,formula.py,m8-accept.sh}
# user decides: DESIGN.md + docs/design/**, docs/adr/**, TESTING.md, docs/generated/**, fuzz/corpus/**
```

Then: `cargo` gates in a clean checkout of the export (not run here) — any missing file a test reads shows up there.

## 9. STALE delete list (for approval; delete in this repo or only omit from export?)

```
docs/shots/22-raster-criss-cross-merge-on-dark.png
docs/shots/22-raster-foxtrot-merge-on-dark.png
docs/shots/25-iterm2-capture.esc-digest.txt
docs/shots/25-kitty-resize-check.esc.txt
docs/shots/33-kitty-check.esc-digest.txt
docs/shots/33-kitty-check.esc.txt
docs/shots/m2-05-feature-branches-x-j-j-x-80x24-unicode.txt
docs/shots/m2-13-unicode-names-enter-80x24-unicode.txt
docs/shots/m2-15-feature-branches-m-*.txt          (4)
docs/shots/m6/fix-three-panes-*.txt               (3)
docs/shots/m7/fix-scroll-contact.md
docs/shots/m8-proto-*.txt (the 18 in §4; or move to prototypes/)
scripts/47-debug-rss.sh
scripts/fix-scroll-deflate-cost.sh
scripts/m2-fix-1c-evil-audit.sh
scripts/m2-fix-perf.sh
scripts/m2-fix-perf-1b.sh
scripts/m3-06-fix-perf.sh
scripts/m8-19e-tour-walk-repeat.sh
docs/.DS_Store
```
Before deleting any shot: run `scripts/shot-drift.py` to confirm no test re-writes it.

## Conductor rulings (2026-10-18)

- `DESIGN.md` + `docs/design/`: **SHIP** (contributors need the keymap and screens).
- `TESTING.md`: **split** at cleanup — the checklist ships, "Times I left" stays private.
- `docs/generated/capabilities.html`: SHIP only if README links it.
- `docs/adr/`: **PRIVATE** (codename-heavy decision history).
- `mask_tests.rs:184` key: checked by eye — the body is the generic `openssh-key-v1` header plus a fake line, not a real key. Ships; GitHub secret scanning may still flag it.
- Personal names in tests (`"devuser"` at `capture/machine.rs:321`, "Rui M" at `mailmap/tests.rs:18`): replace with neutral names at cleanup.
- STALE captures: confirm with `scripts/shot-drift.py` before deleting; the list then goes to the user once.
- **LICENSE files missing** — the licence step adds them (MIT OR Apache-2.0).
