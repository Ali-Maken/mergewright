# 18e blame-floor-git — skipped and unblamable marks on git 2.38; the reader-side Minors of review 18bcd

Status: draft
Blocked by: none.
Crates: graft-git only (`blame.rs`, `blame/porcelain.rs`, a new `blame/marks.rs`, `version.rs`, `trace/log.rs`,
`trace/function.rs`, `trace/mod.rs`, `switch_refs.rs`, their tests, `examples/`, graft-git `MAP.md`). No graft, no
graft-ui: no word the app writes changes (the new `Display`s below are graft-git's, shown verbatim by the app). Needing
another crate → commit at the seam, report `blocked: needs <crate>` naming the signature, stop.
Build with: **Opus**. A new read-only reader and a version gate, pinned by oracles on two real gits (2.38.5 and 2.54).
The decision is made here; nothing structural. Not Fable: a wrong answer shows as a red test on one of the two gits.
Scout: none. The review names every site with its line (`docs/reviews/m7-18bcd.md` Major 1, Minors 2–5). Re-read each
site before editing.
Needs: none.
Split by: planner, 2026-09-27, from review `docs/reviews/m7-18bcd.md` Major 1 (gap rows M7/18bcd-*).

## Ruling (Major 1)
**Floor stays 2.38 (`PROMPT.md:51`). Marks are right on every supported git: on a git whose porcelain writes no mark
lines, blame reads the marks from a second `git blame` in the human format**, which puts `?` (skipped) and `*`
(unblamable) at the line's start. Seen: `^?3f6c31` on 2.38.5 for the line its porcelain left unmarked (review §Major 1,
"Observed"). Not yet seen: `*` on 2.38.5 (the unblamable test below settles it).

Rejected:
- **Raise the floor.** The first git writing porcelain marks is after 2.38.5 (observed) and may be recent (not
  verified; step 1). The rewrite engine relies on 2.38's `update-ref` (ADR-0047), not on anything newer. A higher floor
  cuts users for one mark. Spec change, needs the user, for less than the second pass gives.
- **Degrade in words** (`marks need git ≥ X` on the `i` line). He still sees lines credited to an older commit with
  nothing on the line saying why (H1). Words can't point at which lines.

Cost, and why it's bounded: the second pass runs only when all of these hold (a pure predicate, tested by table):
1. `opts.ignore_revs` is true (with `false` we pass `--no-ignore-revs-file`, so git marks nothing);
2. the porcelain carried no `ignored`/`unblamable` line (a git that writes them has answered);
3. a list is in effect: we passed the root `.git-blame-ignore-revs`, **or** `blame.ignoreRevsFile` is set in config
   (a repo with no list pays nothing, on any git). Read the config without a spawn if `RepoHandle` offers it (MAP.md);
   else `git config --get blame.ignoreRevsFile` via `run_raw`, exit 1 = unset, any other non-zero = error;
4. the system git is older than `GitVersion::BLAME_PORCELAIN_MARKS`, or its version can't be read (`system_git`, one
   `git --version` spawn, only when 1–3 hold). An unreadable version takes the second pass: slower, still right.
   Comment it at the site; that is the safe side, not a swallow.

### Steps
1. **Find `BLAME_PORCELAIN_MARKS`**: the first git release whose `builtin/blame.c` writes `ignored`/`unblamable` in
   porcelain. Method: kernel.org release tarballs (as 18b fetched 2.38.5), grep `builtin/blame.c` per minor version,
   bisect (~5 downloads, no build); or git's `Documentation/RelNotes`. Name the source (URL or tarball + line) in the
   const's doc. **Can't establish it → set it to 2.54.0**, the lowest seen writing them, and say so in NOTES: that errs
   slow (a needless second pass between the true version and 2.54), never wrong.
2. **Re-capture** `PORCELAIN_2_38` (`src/blame/tests.rs:12-55`) from the real 2.38.5 binary (`$TMPDIR/oldg238`; gone →
   rebuild per 18b NOTES "2.38.5 build"). Its test now asserts what 2.38 gives: `Plain` on the skipped line from the
   porcelain alone. Capture the human-format output of the same command on 2.38.5 and 2.54 as `HUMAN_2_38`,
   `HUMAN_2_54`, checked in as bytes. The capture command goes in a comment above each.
3. **The second pass**: the same options as the porcelain run (`-M`, `-w`, the ignore-revs flag, rev, path) plus
   `-l -s`, and pinned configs: `-c blame.markIgnoredLines=true -c blame.markUnblamableLines=true
   -c blame.blankBoundary=false -c blame.coloring=none -c color.ui=never`. Read 2.38.5's `builtin/blame.c` `emit_other`
   for anything else printed before the marks; name each pinned config with its blame.c line in NOTES.
4. **`parse_marks`** reads only each line's leading run of `^`/`*`/`?` (the hash that follows is hex, so content can't
   be mistaken for it). One output line per final line, in order. `*` and `?` together map as the porcelain reader maps
   both lines together; state which, and both readers agree.
5. **Merge**: the marks replace `BlameLine::mark` by position. A different line count → `BlameError::Changed`
   (possible only for `Rev::WorkingTree`: the file changed between the two reads).
6. **Order**: sequential is fine if step 7's p95 is under ADR-0050's 3 000 ms. Over it → run the two spawns
   concurrently, measure again, say which in NOTES. Never drop the pass to meet the budget.
7. **Measure** (script, not the helper's context): `blame-longfile` on 2.38.5 with a list in effect that skips ≥1 line,
   to `docs/perf/18e.txt`; the helper reads the summary line. The example needs a list → add it to
   `examples/blame_longfile.rs` as a mode; the 2.54 no-list number is unchanged.

## Folded in (review 18bcd; all graft-git, same or adjacent files)
- **Minor 2, too-large reads as a parse failure.** `BlameError::TooLarge { cap }` and `TraceError::TooLarge { cap }`,
  each with its own one-line `Display` (below). Every site: `blame.rs` `read_porcelain` (and the new pass),
  `trace/log.rs:136`, `trace/function.rs` `too_large`. `enclosing_function` now returns `TraceError` (no caller outside
  graft-git at `58419879`; 21 is updated for it).
- **Minor 3, truncation tests bypass the call site.** A crate-private `blame_with(…, knobs)` that `blame` calls with the
  runner's default cap; the test calls it with a small cap on the fixture, through the real spawn. Same shape for
  `enclosing_function`. Check: replacing `read_porcelain(&output)?` in the body with `parse_porcelain(&output.stdout)`
  turns the new test red (say how you checked: applied and reverted, or on reading).
- **Minor 4, `switch_refs` flag.** Drop `--symbolic-full-name` (`switch_refs.rs:86`); exit 0/1 is what's read (`:92`).
  Run the existing `@{-1}` tests on 2.38.5 and 2.54. Module doc `:9`, `:102` and MAP.md `:111` say the same command.
- **Minor 5, shallow read hides git's reason.** `trace/log.rs:145-146`: when git exited non-zero, git's reason is the
  error; a failed `shallow` read doesn't replace it (comment why that is safe: git's failure is the cause he can act on).

Not folded: **Minor 1** (needs graft: `i` on `Failed`; a place decision → conductor, e.g. 20i). **Minor 6** (written
into 21's spec). **Minor 7** (TESTING.md + closing M7/18a: conductor's docs). **Q1** (a question; unverified shape).

## Read first
- `docs/reviews/m7-18bcd.md` §Gates "On git 2.38.5", §Major 1, Minors 2–5.
- `.scratch/m7-power-tools/issues/18b-git-reads-review-fixes.md` NOTES "2.38.5 build".
- `crates/graft-git/MAP.md`: `blame`, `trace`, `version`, `run` (`output_cap`, `run_raw`, `RunError`), `switch_refs`.
- `docs/adr/0050-blame-waits-for-git.md` (the budget).

## Signatures (exact)
```rust
// graft-git::version
impl GitVersion {
    /// First git whose `blame --porcelain` writes `ignored` / `unblamable` lines. Source: <URL or tarball:line>.
    pub const BLAME_PORCELAIN_MARKS: GitVersion = /* step 1 */;
}
// graft-git::blame (blame/marks.rs)
/// Marks from `git blame -l -s` (human format), one per output line, in final-line order.
pub fn parse_marks(bytes: &[u8]) -> Result<Vec<LineMark>, ParseError>;
pub(crate) fn needs_marks_pass(ignore_revs: bool, porcelain_has_marks: bool, list_in_effect: bool,
                               git: Option<&GitVersion>) -> bool;
pub(crate) enum MarksFrom { Auto, HumanFormat }   // HumanFormat: force the pass (the 2.54 oracle test)
pub(crate) struct Knobs { pub cap: usize, pub marks: MarksFrom }
pub(crate) fn blame_with(repo: &RepoHandle, rev: &Rev, path: &Path, opts: &BlameOptions, stop: &dyn Stop,
                         knobs: &Knobs) -> Result<Blame, BlameError>;
// graft-git::blame::BlameError gains:
TooLarge { cap: usize },   // "this file's blame is over <cap in MiB> MiB, too large to show"
Changed { first: usize, second: usize },   // "the file changed while blame read it; open blame again"
// graft-git::trace::TraceError gains:
TooLarge { cap: usize },   // "too large to trace: git's output is over <cap in MiB> MiB"
pub fn enclosing_function(repo: &RepoHandle, at: Oid, path: &Path, line: u32, stop: &dyn Stop)
    -> Result<Option<LineRange>, TraceError>;   // was RunError
```
`blame`'s public signature is unchanged. `parse_marks` gets a fuzz target beside `blame_porcelain`'s (written, not run;
fuzzing is the milestone review's). If that target lives outside graft-git's directory, say so and skip it.

## Acceptance (name the verifying test at tick time; "on both" = run with 2.38.5 first on `PATH`, then 2.54)
- [ ] `blame::a_skipped_line_is_marked_skipped` (`tests/it/blame.rs:220`, assertion unchanged) passes **on both**.
      Red first on 2.38.5 (the review saw `left: Plain`).
- [ ] New, on both: `an_unblamable_line_is_marked_unblamable`: a test-local repo where an ignored commit added lines.
      Red first on 2.38.5, or say why not.
- [ ] Oracle on 2.54: `the_human_format_marks_match_the_porcelain_marks`: `MarksFrom::HumanFormat` vs git's porcelain
      marks, line for line on the session fixture with the list; asserts ≥1 `Skipped` so it isn't vacuous.
- [ ] Unit: `PORCELAIN_2_38` re-captured (all `Plain` on the skipped line); `HUMAN_2_38`/`HUMAN_2_54` parse to the
      skipped line; `^?`, `^*`, `*?` prefixes; a last line with no newline (count holds); a line count mismatch →
      `Changed`; garbage → `Err`, never a panic.
- [ ] `needs_marks_pass` table: each of conditions 1–4 alone flips it; `None` version → true.
- [ ] No list in effect → one blame spawn on 2.38.5 (test through `blame_with`, or say how it's observed).
- [ ] Minor 2: tests for `BlameError::TooLarge` and `TraceError::TooLarge` through the readers (small cap); the
      `Display`s above, one line each.
- [ ] Minor 3: the call-site tests; how the bypass was shown red.
- [ ] Minor 4: the `@{-1}` tests on both; docs and MAP.md agree.
- [ ] Minor 5: `a_failed_log_with_an_unreadable_shallow_file_reports_gits_reason` (shallow is a directory, bad rev).
- [ ] `docs/perf/18e.txt`: p95 on 2.38.5 with a list, against 3 000 ms; sequential or concurrent, and why.
- [ ] graft-git suites filtered as the review ran them (`-E 'test(/blame|trace|switch_refs|unreached/)'`) **on
      2.38.5**: all pass; give the count.
- [ ] MAP.md: `blame` (both commands, the predicate), `version` (the const), `BlameError`/`TraceError` variants,
      `enclosing_function`'s error, `switch_refs`' command.
- [ ] Gates workspace-wide: fmt, clippy `--all-targets -D warnings`, nextest, `test --doc`.

## Touches (check each; NOTES says how)
- **`BlameError`/`TraceError` new variants:** grep both outside graft-git. An exhaustive `match` → `blocked: needs
  <crate>`, naming the arm. The review saw a catch-all at `crates/graft/src/jobs/detail.rs:159`: the new words reach
  the blame place verbatim. Say what it now shows for each.
- **`enclosing_function`'s error type:** grep callers in every crate (none at `58419879`); 21's spec is updated.
- **Tests or captures asserting `did not parse as expected` for blame/trace:** grep; update and name them.
- **`blame(` callers:** `examples/blame_longfile.rs`, `examples/trace_perf.rs`, `tests/it/blame.rs`, the blame place.
- **ADR-0050 budget:** measured above; over it → report, don't edit the ADR.
- **Captures, keymap, journal, undo:** none (no drawing, read-only). Show the grep for `LineMark` in graft/graft-ui:
  the draw already handles `Skipped`/`Unblamable`, so nothing new is drawn, only drawn on 2.38 too.
- **For the conductor:** closes gap rows M7/18bcd-major1, -m2, -m3, -m4, -m5, and M7/18r's Minor 12 (the hand-written
  2.38 sample).

## Size
~55k. One context. If it compacts, split: **18e-a** Ruling steps 1–7; **18e-b** the folded Minors.

## Rules
Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`. The 2.38.5 binary stays under
`$TMPDIR`; never install it.
