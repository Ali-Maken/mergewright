# Review: m8-polish-and-ship 49, package `mergewright`, command `mwr`

Commit `3f15befa` against base `f11ac645` (merged at `7ce123a0`). Ticket:
`.scratch/m8-polish-and-ship/issues/49-mergewright-package-and-command.md`.

**Not run:** cargo (the conductor's instruction: gates and a builder are running). Gate results are the builder's
(`6402 passed, 15 skipped`), not observed here. **Run here:** `bash -n` on the 20 touched `.sh` files (0 failed) and
`ast.parse` on the 29 touched `.py` files (0 failed), both read from `3f15befa`. The builder's NOTES say 30 `.py`; I count
29 in `git diff --name-only --diff-filter=AMR` (the count is off by one, but no script fails).
Every other claim below was checked with `git grep`/`git show` at `3f15befa`.

## Acceptance spot-check

I checked all 10 boxes and all 10 held. No box is ticked falsely.

| Box | Check | Held |
|---|---|---|
| metadata / package test | `rename_names.rs:17` `package_and_bin_follow_the_table` asserts `CARGO_PKG_NAME == CURRENT.dir()` and the bin stem `== COMMAND`; `Cargo.toml` `[lib] graft`, `[[bin]] mwr` | yes |
| `--version` / usage | `cli.rs` `version_and_usage_lines_start_with_the_command` asserts `"mwr"` | yes |
| man page | `docs_tests.rs:18` reads `docs/man/mwr.1`; `git ls-tree` shows only `mwr.1` | yes |
| crash test | `crash.rs:400` uses `CURRENT_DIR` (`= CURRENT.dir()`, `on_disk.rs:183`) and `COMMAND`. The name was already in place at base, so nothing was renamed here | yes |
| `scripts_run_the_command` extended | scans `docs/shots/*.py`, built from `older()`. It holds as worded, but it misses the case below (Major 1) | yes (weak) |
| `no_old_package_spec` | `rename_names.rs:412`. My `git grep` for live files finds only the ones NOTES lists as records | yes |
| sandbox | `scripts/mwr_sandbox.py` exists and `trib_sandbox.py` is gone; no importer left (`git grep trib_sandbox` is empty) | yes |
| syntax | I re-ran it: 0 failed | yes |
| captures | `mwr-pkg-shot-drift.txt` matches the summary in NOTES | yes |
| `27-shots-left` | 463 lines in the file. Its regex is judged in Minor 1 | yes |

## Critical

None.

## Major

1. **Two capture scripts still run the stale binary, and their docstrings now say they don't.**
   - **Where:** `docs/shots/39-capture.py:15` (`binary = os.path.join(…, "target", "release", "graft")`) and
     `docs/shots/40-capture.py:19` (same). In this commit, line 1 of each was edited to read "drive target/release/mwr".
   - **Why it matters:** this is the exact failure `scripts_run_the_command` exists to stop. A `target/release/graft` left
     from any build before 27a runs old code, and nothing fails. The guard (`rename_names.rs:362`) only matches the
     strings `target/…/<name>`, `/release/<name>` and `/debug/<name>`. A path built with `os.path.join` with separate
     segments gets past it.
   - **Fix:**
     - Point both scripts at `"mwr"`. The sandbox `GRAFT_BIN` in 37 is the env ticket's job.
     - Make the guard also catch a quoted `"release"`/`"debug"` segment followed by an earlier generation's command, for
       example `"(release|debug)",\s*"(graft|trib)"` built from `older()`.

2. **Past tickets and a scout were rewritten by the `CARGO_BIN_EXE_trib → mwr` sweep.** The ticket says
   `.scratch/*/issues/` are records and keep the old names.
   - **Where:**
     - `27a-rename-command-and-app-name.md:47`. It now says 27a turned `CARGO_BIN_EXE_graft` into `CARGO_BIN_EXE_mwr`;
       27a produced `_trib`.
     - `27a…:70`.
     - `28-accept-m8.md:73`. A ticked box now names a binary that did not exist when it was ticked.
     - `scouts/27-rename-sweep.md:4`. It says adding `[[bin]] name="trib"` yields `CARGO_BIN_EXE_mwr`, which is false.
     - `49-mergewright-package-and-command.md:27`. The spec now reads "`CARGO_BIN_EXE_mwr` → `CARGO_BIN_EXE_mwr`".
   - **Why it matters:** the history now says what didn't happen (rule 1 and the record rule). Anyone tracing the rename
     through 27a reads a false command.
   - **Fix:** restore those five lines from `f11ac645`. Add `.scratch/*/issues/` and `.scratch/*/scouts/` exclusions to
     any future sed sweep.

3. **`ARCHITECTURE.md:233-234` still says "`app` is the binary, and its package is simply `graft`".**
   - **Why it matters:** the package is now `mergewright` and the binary `mwr`. ARCHITECTURE.md must match the code
     (PROMPT 6.1). No later ticket owns this line: 51 touches only ARCHITECTURE env lines, and 55's scope leaves out
     ARCHITECTURE.md. The ticket's Touches lists only MAP lines.
   - **Fix:** one sentence, "its package is `mergewright` (dir `crates/graft`, lib `graft`, bin `mwr`)". Either add it to
     a 49 follow-up or fold it into 55's scope.

4. **`rename_names.rs` passed the 400-line cap (451) and was added to the ratchet instead of being split.**
   - **Where:** `over_400.txt:276`. PROMPT 6.2 counts file size as readability, and this brief rates readability Major.
   - **Fix:** move `no_old_package_spec`, `shot_scripts` and `pty_walk_scripts` into `tests/it/rename_names/package.rs`
     (or `rename_scripts.rs`), and drop the row.

## Minor

1. **`27-shots-left` (443 → 463): the two counts can't be compared, and NOTES says the old regex was not recorded. It
   was.**
   - `27a…:130` records it: "443 files still match `(?i)graft`".
   - The new run uses `grep -rlwE "graft|Graft|trib|Tributary"`. That is word-bounded and case-sensitive, so it misses
     `GRAFT_…`, and it adds `trib`/`Tributary`. The diff shows 20 additions and 0 removals. That fits "newer captures
     now name an earlier generation", but under a different rule.
   - **My judgement:** acceptable as input to 56, because 56 filters to `docs/shots/m8*` anyway. The NOTES claim is
     wrong, though.
   - **Fix:** write the regex as the file's first line, and build it from `Generation::older()` names case-insensitively
     so that 50, 51 and 56 re-run the same rule.

2. **The man page path is stale in live docs.**
   - **Where:** `README.md:51` (`man ./docs/man/trib.1`) and `README.md:184-185`. Line 185 was edited in this commit and
     its regenerate command still writes `> docs/man/trib.1`: anyone following it creates a stray file and the
     staleness test stays red. Also `TESTING.md:3233,3236`, the J17 row. The user's `[ ] His read` step is still open,
     and `man ./docs/man/trib.1` → "No such file".
   - **Owner:** 55 owns "man page name" in both files, so this is temporary.
   - **Fix:** change the three path references now. They are path knock-ons of the `git mv`, not wording.

3. **The release scripts still run `trib`.**
   - **Where:** `scripts/cargo-install-check.sh:42` (`$tmp/cargo-home/bin/trib --version`) and
     `scripts/release-local.sh:56` (`$logs/trib`). Both fail after this commit.
   - **Owner:** 53 owns both and re-runs them, but 49's NOTES and Touches don't mention them, and the guard doesn't see
     them because neither path has `target/`/`release/`/`debug/` in it.
   - **Fix:** note the hand-off in 49's NOTES.

4. **Rule 3, in tests: a renamed or missing path silently empties a guard.**
   - **Where:** `rename_names.rs:273` (`every_file` returns on any `read_dir` error) and `rename_names.rs:436` (any read
     error is `continue`d, including `NotFound` for the five named files at `:420`).
   - **Why it matters:** if `docs/capabilities` or `.config/nextest.toml` moves, the guard still passes and checks
     nothing.
   - **Fix:** skip only `ErrorKind::InvalidData`, and assert that the named files exist.

5. **The guard is narrower than the sweep.**
   - `no_old_package_spec` doesn't cover the non-walk `.scratch/m1-the-graph/pty-*.py` drivers this commit swept, nor
     the forms `--package=graft` and `-pgraft`.
   - **Fix:** widen the glob to `.scratch/*/pty-*.py`, and add the `=` form.

6. **`Generation::CURRENT.command()` is spelled inline in three crates.**
   - **Where:** `graft-ops/src/snapshot/plumbing.rs:101`, `raw.rs:372` and `graft-term/src/input/gate.rs:133`.
   - **Fix:** add `pub const COMMAND: &str = Generation::CURRENT.command();` beside `CURRENT_DIR` in `on_disk.rs`, and
     have `cli::COMMAND` re-export it.

7. **An unrelated status change came in without being mentioned.**
   - **What:** `CAPABILITIES.md`'s M5-22 changed from broken to works (broken 5 → 4).
   - **Why it's legitimate:** the source has said `status: works` since `c3a47536`, and the generated file was stale at
     base.
   - **The problem:** NOTES doesn't mention it, so the user sees a status flip inside a rename commit.

8. **`README.md:71-74` doesn't agree with itself.** It reads "`trib --help` prints it:" followed by `usage: mwr`. This
   fixes itself when 55 lands.

9. **NOTES has no `Touches:` line** (CLAUDE.md "Running work"), and no statement on whether a scout was used. The
   ticket's Brownfield line covers the scout, but the report should still say so.

## Questions (rule 7: no line proves a defect)

- **Q1.** `docs/shots/37-pty-drive.py:10-11` and `docs/shots/40-capture.py:24-25` write
  `<config_home>/graft/config.toml` to force the pixel tier. `mwr_sandbox` seeds `mergewright/`.
  - **What I couldn't confirm:** whether the app reads `graft/config.toml` under a sandboxed `XDG_CONFIG_HOME`, through
    the 48 migration or otherwise. If it doesn't, these captures run in the wrong tier.
  - **Status:** this was there before this commit (base used `tributary/`). Who checks it: 56 (recaptures).
