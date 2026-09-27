# 18f blame-floor-gate — blame's git < 2.50 path tested on every run; `blame.rs` split; review 18e-20f's cheap Minors

Status: done
Blocked by: none.
Crates: graft-git only (`blame.rs`, `blame/*.rs` new and moved, their tests, graft-git `MAP.md`). No graft, no graft-ui:
`blame`'s public signature and every public path are unchanged; no word the app writes changes. Needing another crate →
commit at the seam, report `blocked: needs <crate>` naming the signature, stop.
Build with: **Opus**. A test seam into a decision path: the risk is a seam that tests a copy of the path instead of the
path. The acceptance's named mutations settle that on the diff; the split is a pure move. Not Fable: nothing
structural outside graft-git's private module layout, and every wrong answer shows red on git 2.54.
Scout: none. The review names every site with its line (`docs/reviews/m7-18e-20f.md` Major 1, Major 2, Minors 1–4).
Re-read each site before editing; line numbers are at `b79f6351` and move with the split.
Needs: none.
Split by: planner, 2026-09-27, from review `docs/reviews/m7-18e-20f.md` (gap rows M7/18e20f-major1, -major2, -m1…-m4).

## Ruling (Major 1): the floor path is forced by a test seam, in the fast tier; no new gate command
**The Auto path's two git-dependent inputs become knobs**, so the whole git < 2.50 branch (the list read, the version
gate, the second spawn, the merge) runs on whatever git is on `PATH`, in every `cargo nextest run`:
- `GitFrom::Given(v)`: condition 4 sees `v`, not `system_git()`.
- `PorcelainMarks::Dropped`: the porcelain's marks are read as git < 2.50 leaves them: every parsed mark `Plain`,
  `porcelain_has_marks` false. On 2.38 that is a no-op. That 2.38 leaves them `Plain` is pinned by 18e's
  `the_2_38_porcelain_leaves_the_skipped_lines_plain` on the checked-in 2.38.5 bytes.
- **The knobs replace inputs, never the decision.** `blame` calls `blame_with(…, &Knobs::real())`; the tests call the
  same `blame_with` with other knobs. No branch anywhere reads "is this a test".

What the seam does **not** cover on every run: a live spawn of the real 2.38 binary with our flags. Its bytes are
covered by the checked-in 2.38.5 captures (`src/testdata/git-2.38/`, unit tests, every run). The live run is a
**milestone-review** step, not a gate: for the conductor (below), a TESTING.md row. **No CLAUDE.md gate changes, so
nothing needs the user.**

Rejected:
- **A floor-git tier as the gate** (review option c alone). It needs a self-built 2.38.5 on every laptop, and it runs
  only when someone remembers. The review's own mutation shows the cost: green on 2.54, red only on 2.38.5.
- **Faking `system_git` through `PATH`/env.** It tests the environment, not the decision, and can't make 2.54's
  porcelain drop its marks.

## Major 2: split `blame.rs` (607 lines → ≤ 400), plain early return at `:379`
Pure moves, public paths unchanged (`pub use` where MAP names an item), in this order until `blame.rs` ≤ 400:
1. `blame/marks_pass.rs`: the second pass: `needs_marks_pass` (from `marks.rs`), `marks_pass_wanted`, `configured_list`,
   `merge_marks` (`blame.rs:355-425`), the human-format spawn, and Minor 2's version cache.
2. `blame/invocation.rs`: `Format`, `blame_invocation` (`:427-497`); both passes build their command here.
3. `blame/knobs.rs`: `Knobs`, `MarksFrom`, `GitFrom`, `PorcelainMarks` (everything a test can set, in one place).
4. Still over 400 → the binary/untracked refusal (`diff-tree --numstat` check) → `blame/refusal.rs`.

`:379`: replace `needs_marks_pass(opts.ignore_revs, porcelain_has_marks, true, None)` with
`if !opts.ignore_revs || porcelain_has_marks { return Ok(false); }`. `needs_marks_pass` keeps **one** production
call, with the four real values, once the list and version are read.

Tests follow their code: `blame/tests.rs` is 510 lines; the marks and second-pass tests (including `:428-510`) move to
`blame/marks/tests.rs` and `blame/marks_pass/tests.rs`. Every file this ticket touches ends ≤ 400 lines.

## Folded in (review 18e-20f; all in the files above)
- **Minor 1, marks merged by position with only the count checked.** `parse_marks` returns each line's id prefix
  (git prints it after the marks, one hex shorter per mark, review Minor 1). `merge_marks` checks every line: the
  porcelain line's full id must start with the prefix; an all-zero prefix matches `commit: None` (not committed yet;
  check the zero form on the 2.38 and 2.54 edges testdata). A mismatch → `BlameError::Changed { first: n, second: n }`
  (same shape: graft's tests construct it, conductor merge fix `6ed55d2f`). Doc the fields: "the passes' line counts;
  equal when a line's commit differed".
- **Minor 2, the version read on every blame.** Cache it per process: the first successful read of `system_git()` is
  kept (`OnceLock`); an unreadable one is **not** cached (the pass is taken, 18e's safe side, and the next blame reads
  again). A lost `set` race holds the same answer: ignore its `Err` with a comment saying so (rule 3). What stays:
  the **first** read per process still can't be stopped (bounded by `version.rs`'s own deadline). Rejected: passing
  graft's `state.system_git` through `BlameOptions` (public shape change, graft's crate).
- **Minor 3, "no list, no spawn" is false.** Wording only; the spawn stays (git's own config reader is the oracle for
  `include`/`includeIf`; gix could disagree and silently skip the pass). `MAP.md:36-37` and the comment at `:371` say:
  "With no root file, one `git config --get blame.ignoreRevsFile` spawn per blame learns whether a list is configured
  (when the porcelain carried no mark). The version is one `git --version` per process, cached."
- **Minor 4, the second pass's cap refusal untested.** `Knobs::marks_cap` caps only the second pass; test below.

Not folded: **Minor 5** (graft `jobs/detail.rs:159`; part done by `6ed55d2f`). **Minors 6, 7** (graft tests → 20j).
**Minor 8** and the floor-git review row (TESTING.md, conductor). **Q1, Q2** (questions).

## Read first
- `docs/reviews/m7-18e-20f.md` §Verified (first five bullets), §Major 1, §Major 2, Minors 1–4.
- `.scratch/m7-power-tools/issues/18e-blame-floor-git.md` §Ruling (the four conditions), §Signatures, NOTES.
- `.scratch/m7-power-tools/issues/18b-git-reads-review-fixes.md` NOTES "2.38.5 build" (only if `$TMPDIR/oldg238` is gone).
- `crates/graft-git/MAP.md`: `blame`, `version`, `run` (`run_raw`, `RawOutput::code`, `output_cap`).
- `ARCHITECTURE.md`: the module rules, before moving files.

## Signatures (exact for new items; moved private fns keep today's shape except what's named here)
```rust
// graft-git::blame::knobs (crate-private)
pub(crate) enum MarksFrom { Auto, HumanFormat }             // unchanged
pub(crate) enum GitFrom { System, Given(GitVersion) }        // Given: the version condition 4 sees
pub(crate) enum PorcelainMarks { Read, Dropped }             // Dropped: every porcelain mark read as Plain
pub(crate) struct Knobs {
    pub cap: usize,          // porcelain pass output cap
    pub marks_cap: usize,    // second pass output cap (Minor 4)
    pub marks: MarksFrom,
    pub git: GitFrom,
    pub porcelain_marks: PorcelainMarks,
}
impl Knobs { pub(crate) fn real() -> Knobs; }   // runner's default cap on both, Auto, System, Read: what `blame` passes

// graft-git::blame::marks (pub, was Result<Vec<LineMark>, ParseError>)
pub struct MarkedLine { pub mark: LineMark, pub id_prefix: String }   // hex as git printed it after the marks
pub fn parse_marks(bytes: &[u8]) -> Result<Vec<MarkedLine>, ParseError>;

// graft-git::blame::marks_pass (crate-private)
pub(crate) fn needs_marks_pass(ignore_revs: bool, porcelain_has_marks: bool, list_in_effect: bool,
                               git: Option<&GitVersion>) -> bool;           // moved, unchanged
pub(crate) fn configured_list(repo: &RepoHandle, stop: &dyn Stop) -> Result<bool, BlameError>;
pub(crate) fn merge_marks(lines: &mut [BlameLine], marks: Vec<MarkedLine>) -> Result<(), BlameError>;
// marks_pass_wanted: today's parameters + `git: &GitFrom`.
```
If `configured_list` or `merge_marks` differs today only in borrowing or parameter order, keep today's and say so in
NOTES. `blame`, `blame_with`, `BlameError`, `LineMark`, `BlameLine`: signatures unchanged. Existing `Knobs { .. }`
literals become `Knobs { <field>: …, ..Knobs::real() }`.

## Acceptance (name the verifying test at tick time; standard run = git 2.54 on `PATH`)
Tests in `blame/marks_pass/tests.rs` unless said. Each "as old git" test uses `GitFrom::Given(GitVersion::MINIMUM)` +
`PorcelainMarks::Dropped` and asserts **both** the marks **and** the second-pass count (18e's test-only counter), so a
seam that stops dropping can't pass. Green on first run is expected (the code exists); red is shown by the mutations.
- [x] `as_old_git_a_configured_list_alone_takes_the_pass` (green 2.54 and 2.38.5; red on 2.54 with `Some(0) => Ok(false)`: left 0 right 1): `blame.ignoreRevsFile` set, no root file → pass count 1,
      the skipped line `Skipped`. **Red on 2.54** with `configured_list`'s `Some(0) => Ok(false)` (review mutation, `:414`).
- [x] `as_old_git_the_root_file_takes_the_pass`: root `.git-blame-ignore-revs` → count 1, `Skipped`.
- [x] `as_old_git_an_unblamable_line_is_unblamable` (repo rebuilt in the unit test; `tests/it` can't reach `blame_with`): 18e's unblamable repo → `[Plain, Unblamable, Plain]`, count 1.
- [x] `as_old_git_no_list_is_no_pass`: count 0, all `Plain`.
- [x] `as_old_git_ignore_revs_off_is_no_pass`: `ignore_revs: false` with a list → count 0.
- [x] `at_the_marks_version_no_pass`: `Given(GitVersion::BLAME_PORCELAIN_MARKS)`, `Dropped`, a list → count 0.
- [x] **Mutations, each applied and reverted, on 2.54** (reds per mutation in NOTES; tests named there) (say which tests went red): `if wanted && false` at the pass
      (`:324`); `Dropped` ignored; `<` → `<=` in `needs_marks_pass`'s version test; `Some(0) => Ok(false)`.
- [x] `configured_list_reads_set_unset_and_refuses_a_broken_config` (unit): set → `true`, unset (exit 1) → `false`, a
      broken `.git/config` → `Err`. Record git's exit code for the broken file in NOTES (the review says 3; observe).
- [x] Minor 1 (in `blame/marks/tests.rs`; red with the prefix check removed: `merge_marks_refuses_a_line_whose_commit_differs`, `merge_marks_matches_zero_ids_to_uncommitted_lines`; `parse_marks_reads_the_leading_run_only` now asserts the prefixes): `merge_marks_refuses_a_line_whose_commit_differs` (same count, one prefix from another commit →
      `Changed`) · `merge_marks_matches_zero_ids_to_uncommitted_lines` · `merge_marks_accepts_a_boundary_prefix`
      (`^`-shortened). Unit, in `blame/marks/tests.rs` or beside `merge_marks`. Red: the prefix check removed.
      `parse_marks` units updated for `MarkedLine`; `parse_marks_refuses_garbage_without_panicking` still green.
- [x] Minor 2 (red with the cache removed): `the_system_version_is_read_once_per_process`: `GitFrom::System`, `Dropped`, a list; two `blame_with`
      calls, one version read (test-only counter). Red: the cache removed. (nextest runs each test in its own process.)
- [x] Minor 4 (red with `if output.truncated && false` in `read_marks`): `a_second_pass_cut_at_its_cap_is_too_large`: `MarksFrom::HumanFormat`, `cap` default, `marks_cap` small
      → `Err(TooLarge { cap: marks_cap })`. Red: the second pass's cap refusal bypassed.
- [x] Major 2 (counts and grep in NOTES; `needs_marks_pass_holds_only_when_all_four_do` is the table test): `wc -l` in NOTES for `blame.rs` and every file created or moved: each ≤ 400. `grep -n 'needs_marks_pass('`
      in `src/`: one production call, no literal argument, plus the table test.
- [x] Existing blame tests all green (filtered suite 196/196 on 2.54, incl. `a_list_in_effect_takes_the_pass_only_on_a_git_without_porcelain_marks`; `git diff -M` shows the moves), moved tests unchanged in body (`git diff -M` shows moves).
- [x] Filtered suite (**196/196 on 2.38.5, 196/196 on 2.54**) `cargo nextest run --workspace -E 'test(/blame|trace|switch_refs|unreached/)'` **on 2.38.5**
      (`$TMPDIR/oldg238/inst/bin` first on `PATH`; rebuild per 18b NOTES if gone) and on 2.54: counts for both.
- [x] MAP.md (edited and read back, `crates/graft-git/MAP.md:26-43`): `blame` file list (new files), `parse_marks`/`MarkedLine`, `Knobs` and its
      fields, Minor 3's wording, the version cache, `Changed`'s equal-count case.
- [x] Gates workspace-wide (fmt clean, clippy clean, nextest 4150/4150 (1 skipped), `test --doc` ok) (2.54 on `PATH`): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
      `cargo nextest run`, `cargo test --doc`.

## Touches (check each; NOTES says how)
- **`parse_marks` callers** in every crate and `/fuzz` (MAP: no fuzz target): grep. Only graft-git expected.
- **`Knobs {` literals** in graft-git (src tests, `tests/it`, `examples/`): grep; each gains `..Knobs::real()`.
- **`BlameError::Changed`** outside graft-git (graft's `6ed55d2f` test): shape unchanged; grep shows it compiles as is.
- **`needs_marks_pass`** callers: grep; the table test moves with it.
- **`blame(` callers** (`examples/blame_longfile.rs`, `examples/trace_perf.rs`, `tests/it/blame.rs`, graft's blame
  job): signature unchanged.
- **Perf:** `docs/perf/18e.txt` not re-run. The cache removes spawns after the first; claim nothing faster.
- **graft, graft-ui, captures, keymap, journal, undo:** none. `LineMark` drawing unchanged.
- **For the conductor:** closes M7/18e20f-major2, -m1, -m3, -m4; closes -major1 with the TESTING.md row below;
  narrows -m2 to "the first `git --version` per process can't be stopped (bounded by `version.rs`'s deadline)".
  TESTING.md, milestone-review tier, **floor git** row: build 2.38.5 per 18b NOTES into `$TMPDIR`; with it first on
  `PATH`, `cargo nextest run --workspace -E 'test(/blame|trace|switch_refs|unreached/)'`; all pass. Fold with Minor 8
  (M7/18bcd-m7: `scripts/m7-perf-blame-list.sh` on the floor git in the release perf run).

## Size
~60k. One context. Do the split first, gates green, commit at that seam; then the seam and tests. If it compacts,
split: **18f-a** the split, `:379`, knobs and the Major 1 tests; **18f-b** Minors 1, 2, 4 and the docs.

## Rules
Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`. The 2.38.5 binary stays under
`$TMPDIR`; never install it.

## NOTES (builder, 2026-09-27)
- **Scout:** none (ticket says so; the review names every site). No compaction.
- **Commits:** `f29c5549` the split + `:379` (pure moves; gates green then: nextest 4138/4138, doc ok); `18a4b260` knobs, prefix merge, version cache; last commit: tests re-homed, MAP, this ticket.
- **Signatures:** as specified. `configured_list(repo, stop)` and `merge_marks(lines, marks)` keep today's order. New crate-private `read_marks` in `marks_pass.rs` is the human-format spawn (with its cap refusal and 18e's pass counter). `needs_marks_pass` is no longer re-exported from `blame` (only `marks_pass` and its tests call it).
- **Mutation reds on git 2.54** (each applied with `sed`, filtered suite run `--no-fail-fast`, reverted with `git checkout`):
  - `Some(0) => Ok(false)` (`configured_list`): red `as_old_git_a_configured_list_alone_takes_the_pass` (left 0, right 1), `configured_list_reads_set_unset_and_refuses_a_broken_config`.
  - `if wanted && false` (`blame_with`): red `as_old_git_the_root_file_takes_the_pass`, `as_old_git_a_configured_list_alone_takes_the_pass`, `as_old_git_an_unblamable_line_is_unblamable`, `a_second_pass_cut_at_its_cap_is_too_large`. The oracle `the_human_format_marks_match_the_porcelain_marks` stayed **green** under this mutation (with the pass skipped, its "human" marks are 2.54's porcelain marks). The review says the oracle catches it; I did not observe that.
  - `Dropped` ignored (`if false && knobs.porcelain_marks == Dropped`): red `as_old_git_the_root_file_takes_the_pass`, `as_old_git_a_configured_list_alone_takes_the_pass`, `as_old_git_an_unblamable_line_is_unblamable`, `at_the_marks_version_no_pass`, `the_system_version_is_read_once_per_process`.
  - `<` → `<=` in `needs_marks_pass`: red `needs_marks_pass_holds_only_when_all_four_do`, `at_the_marks_version_no_pass`.
  - Prefix check removed (`true ||`): red `merge_marks_refuses_a_line_whose_commit_differs`, `merge_marks_matches_zero_ids_to_uncommitted_lines`.
  - Cache removed (`.get().filter(|_| false)`): red `the_system_version_is_read_once_per_process`.
  - Second-pass cap check bypassed: red `a_second_pass_cut_at_its_cap_is_too_large`.
- **Broken `.git/config`:** `git config --get blame.ignoreRevsFile` exits **128** on both 2.38.5 and 2.54 (observed; the test prints it), not 3 as the review said. The test asserts `Exited { status > 1 }`.
- **Zero id:** both edges samples print the all-zero id unshortened (40 digits, no marks) for the uncommitted line; `^` lines have 39 digits, `^?`/`*?` 38.
- **Version cache under `cargo test`:** the counter is per thread, the `OnceLock` per process; the test asserts ≤ 1 read on the first blame and 0 on the second, and that the cache is filled, so it holds under both runners.
- **2.38.5 runs:** `$TMPDIR/oldg238/inst/bin` first on `PATH` (the oracle test printed "git 2.38.5 writes no porcelain marks", confirming which git ran): 196/196. 2.54: 196/196.
- **`wc -l`:** `blame.rs` 363; `blame/marks_pass.rs` 205; `blame/invocation.rs` 79; `blame/knobs.rs` 73; `blame/refusal.rs` 85; `blame/marks.rs` 77; `blame/tests.rs` 302; `blame/marks_pass/tests.rs` 374; `blame/marks/tests.rs` 175. All ≤ 400.
- **`grep -n 'needs_marks_pass(' src/`:** `blame/marks_pass.rs:37` (definition), `:73` (the one production call: `opts.ignore_revs, porcelain_has_marks, list_in_effect, git.as_ref()`), and the table test in `blame/marks_pass/tests.rs`.
- **Moved tests:** bodies unchanged except where `parse_marks` now returns `MarkedLine` (`marks_only(..)` wraps it; the count test pushes a `MarkedLine`) and `Knobs` literals gained `..Knobs::real()`. `marks_merge_by_position_and_a_count_mismatch_is_changed` lives in `blame/marks/tests.rs` beside the other merge tests, to keep `marks_pass/tests.rs` ≤ 400.
- **Touches:**
  - `parse_marks` callers: `grep -rn parse_marks crates fuzz` → graft-git `blame` only; no fuzz target.
  - `Knobs {` literals: graft-git `src/blame/tests.rs` (2) and `src/blame/marks_pass/tests.rs`; each uses `..Knobs::real()`. None in `tests/it` or `examples/` (grep).
  - `BlameError::Changed`: graft `jobs/detail.rs:200`, `jobs/detail/blame_tests.rs:213`; shape unchanged; workspace clippy and nextest green.
  - `blame(` callers: signature unchanged; examples and `tests/it/blame.rs` compile (clippy `--all-targets`).
  - Perf: `docs/perf/18e.txt` not re-run; no speed claim.
  - graft, graft-ui, captures, keymap, journal, undo: not touched (diff is graft-git `src/blame*`, `MAP.md`, this ticket).
