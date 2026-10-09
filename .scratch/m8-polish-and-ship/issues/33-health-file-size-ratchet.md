# health — file-size ratchet: every .rs over 400 lines listed, with its count

Status: draft (planner, from `docs/reviews/m8-architecture.md` "KNOWN_GAPS … disposition")

Build with: Sonnet. It writes a new test file and adds one `mod` line, and touches no behaviour. The planner sees no need for a scout; ADR-0035 is the conductor's call.

Needs: none. It can run at any time. If it runs before a `health-split-*` ticket, that ticket lowers or removes its rows.
Blocked by: none (conductor, from Needs)

## Why
- PROMPT §6.2 caps files at about 400 lines.
- KNOWN_GAPS `M8/files-over-400-unrowed` says roughly 100 production and 80 test files are over the cap, and **nobody has counted**.
- No check stops a file from growing.
- This ticket gives an exact list and stops growth. It does not fix the files.

## Read first
- `docs/reviews/m8-architecture.md` § "KNOWN_GAPS `M8/files-over-400-unrowed`".
- `crates/graft/MAP.md`: grep `seam_rule`. That test is the shape to copy: a workspace-reading test under `crates/graft/tests/it/`.
- `crates/graft/tests/it/main.rs`: the `mod` list only.

## Scope
In:
- A new `crates/graft/tests/it/file_size.rs`.
- A new data file `crates/graft/tests/it/over_400.txt`. One row per file: `<lines> <path from repo root>`, sorted by path. A `#` comment heads the file and gives the rule.
- The test walks every `.rs` under `crates/*/src` and `crates/*/tests`.
  - Lines are counted as newline bytes, which is what `wc -l` reports.
  - It skips `target/`, and `prototypes/` is never under `crates/`.
- One `mod file_size;` line.

Out:
- Splitting or trimming any file.
- Any threshold other than 400.
- Counting inline `#[cfg(test)]` blocks separately.

## Rules the test enforces (independent checks, no judgement)
1. A file over 400 lines that has no row → fail. The message names the path and its count.
2. A row whose file now has **more** lines than its row says → fail. The message is "grew from N to M".
3. A row whose file has **fewer** lines → fail. The message is "shrank to M: lower the row". This keeps the list exact, so lowering a count is a one-line edit and raising one shows in the diff.
4. A row whose file is at or under 400 lines, or no longer exists → fail with "remove the row".
5. The test prints one summary line: `over_400: <p> production, <t> test files` (stderr; `nextest` shows it with `--no-capture`). "Test" means a path under a `tests/` directory or named `tests.rs`.

## Signatures
None public. Test fns:
- `fn every_file_over_400_lines_is_listed_with_its_count()`
- `fn the_size_list_has_no_stale_rows()`

## Acceptance
- [ ] With the list as generated, both tests pass (test: the two named above).
- [ ] Rule 1. Adding 401 blank lines to a scratch `.rs` file makes `every_file_over_400_lines_is_listed_with_its_count` fail with that path. Done by hand and reverted, and the failure line is quoted in NOTES.
- [ ] Rule 2. Raising one listed file by a line fails with "grew". Done by hand and reverted, and the line is quoted in NOTES.
- [ ] Rule 4. Deleting a row's file fails `the_size_list_has_no_stale_rows`, checked with a temp copy of the list. Quoted in NOTES.
- [ ] NOTES gives the summary line's two numbers. The conductor puts them in the KNOWN_GAPS row.

## Touches
- `crates/graft/tests/it/main.rs`: one `mod` line.
- The `health-split-*` tickets: each must edit `over_400.txt`.
- Every later ticket that grows an over-size file must now edit the list. That is intended.
- 21b-6's allow guard: a new test file may need its lint prelude, so check it.
- KNOWN_GAPS row: the conductor updates it.
- No production code.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
About 25k tokens.
