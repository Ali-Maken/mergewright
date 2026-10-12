# 49r: review fixes for ticket 49 (package and command)

Status: done
Blocked by: none (49 merged at 7ce123a0)
Build with: **Sonnet**. Script paths, record restores, one test split; held by grep tests.
Brownfield: ticket 49 NOTES serve as the scout; the review names every line.
Needs: none. Before 50 (same test files: `rename_names.rs`).

## Read first
- `docs/reviews/m8-mergewright-49.md`, whole
- Ticket 49: Decisions, NOTES

## Scope
- **Major 1:** `docs/shots/39-capture.py:15`, `40-capture.py:19` run `target/release/graft` → the `mwr` binary. Extend
  `rename_names::scripts_run_the_command` (`rename_names.rs:362`) to catch paths built with `os.path.join` (red first).
- **Major 2:** restore the records the sweep rewrote (`27a:47,70`, `28-accept-m8.md:73`, `scouts/27-rename-sweep.md:4`,
  `49:27`) to their original text; a record says what was true then. Add them to the sweep's records allow-list.
- **Major 3:** `ARCHITECTURE.md:234`: the shipping package is `mergewright`, crates stay `graft-*` (ADR-0057 §9).
- **Major 4:** split `rename_names.rs` under 400 lines; remove its over_400 row.
- **Minors:** fix each as the review says, or say in NOTES why not.
- Ticket 49 NOTES: correct "the original regex isn't recorded" (it is, `27a:130`).

## Acceptance (each box: the test named at tick time)
- [x] `scripts_run_the_command` red on the old 39/40 scripts, green after: `rename_names::scripts::scripts_run_the_command`
      (red run listed `39-capture.py:15`, `40-capture.py:19`); matcher pinned by `rename_names::scripts::the_segment_test_reads_joined_paths`.
- [x] Records grep: the five sites read as before 49. `git diff f11ac645 -- issues/27a… issues/28… scouts/27…` is empty;
      `49:27` reads `` `CARGO_BIN_EXE_trib` → `CARGO_BIN_EXE_mwr` `` (its pre-49 text).
- [x] `over_400.txt` has no `rename_names.rs` row (now 271 lines + `rename_names/scripts.rs`); `file_size` tests green.
- [x] Every minor: fixed with test/grep named, or reason in NOTES (below).

## Touches
Capture scripts; records allow-list; ARCHITECTURE.md; over_400.txt.

## NOTES (builder, 2026-10-12)
Scout: none; the review names every line (ticket's Brownfield line).
- Split: `rename_names.rs` keeps package/bin and string-literal tests; `rename_names/scripts.rs` holds the env-name,
  scripts and package-spec guards.
- Major 2 allow-list: `no_old_package_spec`'s doc lists `.scratch/*/scouts/` beside `.scratch/*/issues/` as records.
  No test sweeps `CARGO_BIN_EXE_*`, so for future seds the exclusion is that doc line, not a test.
- Minor 1: not fixed. `27-shots-left.txt` is 56's input and 56 re-runs it; the regex (from `older()`, case-insensitive,
  written as line 1) belongs there.
- Minor 2: fixed. README.md:51,184,185 and TESTING.md:3233,3236 → `docs/man/mwr.1` (grep `trib\.1`: only TESTING:3248,
  the dist row, 53's).
- Minor 3: hand-off to 53: `scripts/cargo-install-check.sh:42`, `scripts/release-local.sh:56` still run `trib`.
- Minor 4: fixed. `every_file` panics on an unreadable dir; `read_text` skips only `InvalidData`; the five named files
  are asserted to exist (in `no_old_package_spec`).
- Minor 5: fixed. `no_old_package_spec` reads `.scratch/*/pty-*.py`, and adds `-pgraft`, `--package=graft`.
- Minor 6: not fixed: graft-ops/graft-term are outside this ticket's files, and 48r holds graft-ops concurrently.
- Minor 7: CAPABILITIES M5-22 broken→works in 49 was a stale generated file at base (source `works` since c3a47536).
- Minor 8: README:71-74 wording, 55 owns. Minor 9: this NOTES has Touches and scout lines.
- Ticket 49 NOTES corrected (`27a:130` records the original rule).
- Touches: capture scripts 39/40 (guard test); records 27a/28/scout/49 (git diff); ARCHITECTURE.md:233 (read back);
  README/TESTING man paths (grep); over_400.txt (file_size tests). Not run: the 39/40 capture scripts themselves.
- No compaction.
