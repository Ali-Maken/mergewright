# G-rename 2b — every environment name is `TRIB_*`

Status: done
Blocked by: 27a
Build with: **Sonnet** — a token rename (`GRAFT_` → `TRIB_`) held by one grep test; no behaviour changes.
Needs: 27a merged (same test files; sequential, not concurrent).

## Read first
- 27's `## Decisions` (env bullet); `scouts/27-rename-sweep.md` `## Env vars`
- `crates/graft/MAP.md` `## cli` (`ASKPASS_SOCKET_VAR`), `## trace`; `crates/graft-fixtures/MAP.md` `UPDATE_VAR`;
  `crates/graft-ops/MAP.md` `askpass_test_helper`; `crates/graft-core/MAP.md` `on_disk`

## Decisions (spec-only pass)
- **Every `GRAFT_` token → `TRIB_`**, identifier or string, in `crates/**` (src, tests, examples, benches),
  `scripts/**`, `.scratch/*/pty-walk*.py`, `crates/*/MAP.md`, `ARCHITECTURE.md`. Why whole-token: Rust consts
  named `GRAFT_*` were deleted in 26, so what's left is env names; a plain absence test then holds it. 45 names,
  ~109 files (scout). The old names are not read (nothing shipped under them).
- **Lower-case code names stay** (`is_graft_internal`, `graft_*` paths) — crate-name class, 27's KNOWN_GAPS row.
- **One source for the askpass socket name.** Today two literals: `graft::cli::ASKPASS_SOCKET_VAR` (`cli.rs:50`) and
  `graft-git/src/run.rs:356`. Move the const to `graft_core::on_disk` (both crates depend on graft-core; ADR-0057
  §2 groups env names with on-disk names); `graft::cli` re-exports it; graft-git `run.rs` and graft-ops
  `askpass_test_helper.rs:14` use it.
- History docs (logs, reviews, ADRs, KNOWN_GAPS, closed gaps, field notes, design notes, past tickets' text) keep
  `GRAFT_`. README/TESTING.md: 27c. `.claude/**` is the conductor's: NOTES lists its `GRAFT_` lines, not edited.
- Captures that print a `GRAFT_` name (scout: 10 files): TestBackend ones regenerate here; the rest go to 27d's list.

## Scope
In: the token rename over the files above; the askpass const move; MAP/ARCHITECTURE lines; TestBackend captures
whose text changes; the grep test. Out: README, TESTING.md, script prose (27c); pty captures (27d); history docs.

## Signatures
```rust
// graft-core::on_disk
pub const ASKPASS_SOCKET_VAR: &str = "TRIB_ASKPASS_SOCKET";
// graft::cli
pub use graft_core::on_disk::ASKPASS_SOCKET_VAR;
```
Values change, names unchanged: `UPDATE_VAR` = `"TRIB_UPDATE_SHOTS"` (graft-fixtures), trace `TRIB_PERF_TRACE`,
`TRIB_NOW`, `TRIB_PRACTICE_EPOCH`, `TRIB_FIXTURES_DIR`, `TRIB_INIT_TARGET`, and the rest of the scout's 45.

## Acceptance (each box: the test named at tick time)
- [ ] `crates/graft/tests/it/rename_names.rs::no_old_env_name` — no `GRAFT_` in `crates/**` (any file),
      `scripts/**`, `.scratch/*/pty-walk*.py`, `crates/*/MAP.md`, `ARCHITECTURE.md`; allow-list empty or each entry
      with a reason.
- [ ] `graft-git` spawns askpass with `TRIB_ASKPASS_SOCKET` and the binary answers it: the existing askpass tests
      (named) pass; grep shows one definition of the string (`on_disk.rs`), quoted.
- [ ] An old name is not read: test that `GRAFT_NOW` set alone leaves the session clock at wall time (or the
      nearest existing env test, named, with the old name).
- [ ] Captures: `TRIB_UPDATE_SHOTS=1 cargo nextest run`, then a run without — green. `scripts/shot-drift.py --base
      <27a's commit>` summary quoted (`.scratch/m8-polish-and-ship/27b-shot-drift.txt`); drift-files 0 expected
      (a name swap of equal-or-shorter length) — any drift listed in NOTES.
- [ ] `27-shots-left.txt` re-run (27a's rule) and committed — 27d's input.
- [ ] NOTES lists `.claude/**`, CLAUDE.md and PROMPT.md lines naming a `GRAFT_` var (for the conductor).

## Touches
graft-core `on_disk` (+ MAP); graft-git `run.rs` (+ MAP); graft-ops helper bin (+ MAP); graft `cli`, `trace`,
`session`, `places/practice`, every test/example reading or setting an env var (+ MAP); graft-fixtures `cache.rs`,
`UPDATE_VAR` (+ MAP); `ARCHITECTURE.md`; `scripts/**`; pty-walk scripts; TestBackend captures. Each checked by grep or
test, named in the report.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` —
workspace-wide. Capture runs write to files; read summary lines only.

## Size
One context, ~45k tokens (wide but sed-shaped; read files only where a match is not a plain token).

## Done (2026-10-10)
- [x] No `GRAFT_` left — `crates/graft/tests/it/rename_names.rs::no_old_env_name` (bytes, >100 files, allow-list empty).
- [x] Askpass — `graft::it askpass_mode::binary_answers_over_socket`, `…binary_exits_1_with_nothing_on_stdout_when_cancelled`, `…binary_exits_2_naming_the_error_when_graft_is_gone`, `graft-git run::tests::askpass_env_is_set`; one string: `crates/graft-core/src/on_disk.rs:71: pub const ASKPASS_SOCKET_VAR: &str = "TRIB_ASKPASS_SOCKET";` (grep `"TRIB_ASKPASS` over crates: that line only).
- [x] Old name not read — `graft::it clock_pin::the_old_now_name_is_not_read` (old name = `1.5`, launches, exit 0).
- [x] Captures — update run then plain run: 6295 run, all pass after MAP-line fix (map_lines_are_short re-run green). Drift: `changed 7 · name-only 7 · drift-files 0 · png 0` (`.scratch/m8-polish-and-ship/27b-shot-drift.txt`).
- [x] `27-shots-left.txt` re-run: 443 (unchanged).
- [x] `.claude/**`, CLAUDE.md, PROMPT.md: `grep -rl GRAFT_` finds none.

## NOTES
- Scout used: `scouts/27-rename-sweep.md`. No compaction.
- Also: `docs/man/trib.1` regenerated (env lines); `over_400.txt` raised: run.rs 683→684 (import), clock_pin.rs 422→443 (new test).
- `.patch` files under `scripts/` renamed in-place too (in scope as `scripts/**`); they may no longer apply to old trees.
- Env names stay literals per name (no shared `TRIB_` prefix const: Rust can't `concat!` a const); a later rename is a sed, held by the grep test.
