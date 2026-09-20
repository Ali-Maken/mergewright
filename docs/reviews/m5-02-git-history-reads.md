# Review: M5 group 02 — git history reads

Range `3b9d855..2826aa5`, `crates/graft-git` only. Ticket
`.scratch/m5-history-and-ghost-preview/issues/02-git-history-reads.md`.

**What was verified and how.** Read the ticket, the design note's G4 block and
Hard choice 5, `crates/graft-git/MAP.md`, then the diff crate by crate and the
three test files whole. No cargo command run (conductor's instruction: two
builders hold concurrent worktrees); gate numbers below are the conductor's
from the merge. Four claims about real git behaviour that the findings rest on
were checked with plain `git` in a throwaway `/tmp` repository on git 2.54.0
(Apple Git-157): `merge-tree --name-only` path quoting, `--write-tree` object
writes, the bad-id exit shape, and `rebase-apply`/detached-HEAD state files.
Heavy tier, release perf run, fuzzing and benchmarks: **not run** (no cargo).

Gates (conductor, workspace-wide at merge): fmt ok · clippy ok · 1842 tests
run, 1842 passed, 23 skipped · doctests ok.

## Tick spot-check

4 ticks, all 4 checked, 3 held, 1 false.

| Tick | Result |
|---|---|
| 1. Every function has its integration test | **False for `first_marker_line`**: no test anywhere (`grep -rn first_marker_line crates` finds only the definition and MAP). The seven named tests exist and prove what they say. |
| 2. `forecast_merge` never `Err` for "cannot forecast" | Held. `forecast_is_not_checkable_never_an_error` (old git + bad id) and the unit test (three version gates) both exist and assert `NotCheckable`. Bad id = exit 1 + empty stdout confirmed on git 2.54. |
| 3. Row 455 relabel + test | Held. `poisoned_sink_is_not_spawn` asserts `Err(RunError::Sink(_))` on the second run of a cloned invocation. |
| 4. MAP updated; NOTES records version and `merge-tree` shapes | Held (MAP has one inaccuracy, m4 below). |

## Critical

### C1. False tick: `first_marker_line` has no test

`.scratch/m5-history-and-ghost-preview/issues/02-git-history-reads.md:57` ticks
"Every function above has the integration test named in piece 6, green".
`first_marker_line` (`history_reads.rs:376`) is in the Signatures block and has
no test at all — not in `tests/it/history_reads.rs`, not in
`history_reads/tests.rs`. It is the smallest function in the group; the rule is
"a false tick is Critical, whatever its size" (CLAUDE.md). It matters more than
its size because the untested function has a real bug (M2) that a test with a
non-ASCII fixture would have caught, and group 09 binds `e` to it.

Fix: untick, add the tests listed under M2, retick naming them.

## Major

### M1. `forecast_merge` returns C-quoted paths for any non-ASCII or special filename

`history_reads.rs:298-324`. The command omits `-z`, so `merge-tree --name-only`
prints conflicted paths through git's C-quoting: verified on git 2.54, a
conflict in `café.txt` comes back on stdout as `"caf\303\251.txt"` — quotes and
octal escapes included — and the code turns that literal text into the
`PathBuf`. Any path with a non-ASCII byte, a `"`, a `\` or a control character
is wrong, and the shape also depends on the user's `core.quotepath`. The
existing test only uses `shared.txt`. `changed_paths` two functions down
already uses `-z` for exactly this reason, so the file is inconsistent with
itself.

Fix: add `GitArg::Flag("-z")` and split stdout on NUL. Verified: with `-z` the
tree id line is NUL-terminated too (`<tree>\0café.txt\0`), so the same split
handles both lines. Add a test with a conflicting `café.txt` (and one with a
space) asserting the exact `PathBuf`.

### M2. `first_marker_line` fails on any non-UTF-8 byte before the marker

`history_reads.rs:376-384`. `BufRead::lines()` yields `Err(InvalidData)` for a
line that is not valid UTF-8. A conflicted file with a Latin-1 byte in a
comment, or any binary-ish content, above the first `<<<<<<<` makes the call
return `Err` rather than the marker's line; group 09's `e` then either shows an
io error for a file that plainly has a marker or, if the caller falls back to
`None`, opens the editor at the top and claims "already clean". Also each line
is allocated as a `String`; a file with no newlines is read whole.

Fix: `BufReader::split(b'\n')` and `line.starts_with(b"<<<<<<< ")` — bytes, no
decoding. Tests: marker on line 3 → `Some(3)`; no marker → `None`; a `0xE9`
byte on line 1 and the marker on line 2 → `Some(2)`; CRLF file → still found.

### M3. "Nothing writes" is false: every forecast writes objects into the repository

`history_reads.rs:6-7` (module doc) and the design note's Hard choice 5 both
say the read layer never writes. `git merge-tree --write-tree` writes the
result tree and every merged or conflicted blob into `.git/objects` — that is
how it can print a tree id. Verified: one forecast on a one-file conflict took
`git count-objects` from 9 to 11. A 50-commit rebase preview (design note
`FORECAST_CAP`) can leave hundreds of unreachable loose objects per preview,
pruned only by a future `gc`. Not user-data loss, but an invariant group 07
plans against, a disk cost nobody tracks, and a wrong claim in a doc comment.

Fix, in two steps: (1) now — correct the module doc and MAP (`forecast_merge`
writes unreachable objects; nothing else writes), and add a KNOWN_GAPS row
(broken: limitation, not unverified). (2) worth trying in group 07 or a fixup:
run the forecast with `GIT_OBJECT_DIRECTORY=<temp dir>` and
`GIT_ALTERNATE_OBJECT_DIRECTORIES=<repo objects dir>` through
`GitInvocation::env`, then delete the temp dir — git reads through the
alternate and writes only into the temp dir. Verify against a repository that
has its own `objects/info/alternates` and against a linked worktree before
relying on it.

### M4. `RunError::Spawn` reused for "git answered, graft-git could not read it"

`history_reads.rs:49` (`merge_base`) and `history_reads.rs:119`
(`commits_between`) wrap a parse failure as `RunError::Spawn(io::Error::other(..))`.
`Spawn`'s Display is "could not run git: …" — git ran fine. This is the same
mislabel class the ticket's row-455 piece just fixed for `Sink`, reintroduced
two functions later, and it is in the error type group 07 now matches on. The
builder flagged it in NOTES as a deliberate trade against surface; the trade is
wrong for a dependency five groups build on.

Fix: a `history_reads::ReadError { Run(RunError), Parse(ParseError) }` like
`diff::DiffError`, or a `RunError::Unrecognised { command, what }` variant if
the conductor prefers one runner-level bucket. Either way the message must not
say git could not run.

## Minor

- **m1.** `run.rs:114-117` — `RawOutput::code` doc says `None` for "a stop, or
  the output cap". A stop is `Err(RunError::Stopped)` (`wait`, `run.rs`), never
  `Ok`; only the output cap yields `None`. Fix the sentence.
- **m2.** `history_reads.rs:319` — the exit-1 first line is accepted as a tree
  id when merely non-empty. Parse it with `Oid::from_hex` so an unrecognised
  exit-1 shape becomes `NotCheckable`, not `Conflicts` with garbage paths.
- **m3.** `history_reads.rs:332` — for `code: None` (output cap hit) the reason
  is `first_line`'s fallback "git gave no reason"; say "merge-tree output
  exceeded the cap". Also document on `Forecast::NotCheckable` that the string
  is raw git stderr and must go through the display layer's sanitiser
  (KNOWN_GAPS M4/06: sanitisation is convention, not type).
- **m4.** `MAP.md:111` says `parse_rev_list` is "a `cargo-fuzz` target";
  `fuzz/fuzz_targets/` has `parse_file_list`, `parse_porcelain_v2`,
  `parse_unified` only. Add the target (a few lines, three siblings to copy) or
  say "shaped for one".
- **m5.** `state.rs:127-129` — `head_name` doc says "the full ref
  (`refs/heads/x`)". For a rebase started from a detached HEAD git writes the
  literal `detached HEAD` (verified). Document it; group 09 draws this string.
- **m6.** `rebase-apply` backend (`next`/`last`) untested — unverified, not
  broken (files confirmed present on git 2.54). `git rebase --apply main` in
  the existing state test is cheap.
- **m7.** `history_reads.rs:311` reaches into `commit_graph::first_line` for a
  stderr helper. It is a runner concern; move it to `run` and call it from
  both.
- **m8.** `run/exec.rs:36-42` — `Event.poisoned_sink: bool` rides beside
  `read: io::Result`, a flag qualifying an error that two places must keep in
  step. `enum DrainError { Io(io::Error), PoisonedSink(io::Error) }` says it
  in the type.
- **m9.** `run.rs:448` — `Ok(Ending::Capped(output)) => Ok(Ending::Capped(output))`
  identity arm. `let ending = Self::wait(..)?; if let Ending::Exited(..) =
  &ending { join stdin }` reads plainer.
- **m10.** `history_reads.rs:104` — `GitArg::Word("--date=raw")` for a static
  flag; `Word` is documented "caller-built". `Flag` fits.
- **m11.** `history_reads.rs` is 398 lines, at the ~400 cap before any fix
  above lands. `first_marker_line` is a fs read, not a git read; it can live
  in `state` next to `in_progress_detail` or its own small module.
- **m12.** `PathBuf` vs the crate's byte-path convention — builder flagged.
  The design note's G4/G5 signatures use `PathBuf` consistently, so leave it;
  note the non-unix `bytes_to_pathbuf` is lossy (fine for the targets).
- **m13.** `changed_paths_matches_diff_tree_name_only_and_caps` hardcodes the
  expected paths rather than reading `git diff-tree` back as the tick's
  "proven by reading git back" and the test name say. Equivalent proof for two
  files; rename or read git back.
- **m14.** KNOWN_GAPS row 455 still reads "broken (label) / unverified". The
  label half is fixed by this group; the `WouldOverwrite` multi-path half is
  untouched. Conductor: split the row, close the label half.
- **m15.** `poisoned_sink_is_not_spawn` leaves the first run's failure mode
  unasserted (the comment says so). Fine, but the drain thread's panic prints
  a backtrace into test output; a `std::panic::set_hook` guard or
  `#[should_panic]`-free quieting would keep the fast tier's output clean.

## Answers to the conductor's questions

- **`run_raw` safety.** No error swallowed: `spawn` failures, `Stopped`, a
  signal death (`Signalled`) and the stdin-thread failures all still `Err`;
  only git's own exit code stops being judged. The one caller
  (`forecast_merge`) distinguishes exit 0 / exit 1 with a tree line / exit 1
  with empty stdout / anything else. The "exit 1 + empty stdout = git refused"
  reading is correct on git 2.54 (verified). Two gaps: the tree line is not
  validated (m2) and the exit-1 path list is mis-parsed for non-ASCII names
  (M1).
- **Chained-forecast limitation recorded?** Yes, where a caller sees it:
  `Forecast`'s doc comment (`history_reads.rs:242-249`) and MAP's
  `forecast_merge` line both say per commit, not chained, "may conflict" never
  "will". Adequate.
- **Do the five signatures hold up as a dependency?** Shapes match the design
  note exactly; borrowing is plain (`&RepoHandle`, `Oid` by value, `&dyn
  Stop`). Nothing forces a caller to swallow: `merge_base`/`mergetool` fold
  git's exit 1 into `Ok(None)` on purpose, `forecast_merge` folds "cannot
  check" into `NotCheckable(reason)` per spec, `in_progress_detail` folds
  missing/unparsable state files into `None` per ticket. The error *label* is
  the weak point (M4), and `Conflicts.paths` is wrong for real-world filenames
  (M1). `changed_paths` has no `Stop`, per its signature; fine for a bounded
  `diff-tree`.

## For group 07 (do not wait on fixes to know this)

- `forecast_merge` writes objects (M3).
- `Conflicts.paths` is currently C-quoted for non-ASCII (M1); after the fix it
  is raw bytes as `PathBuf`.
- `NotCheckable(String)` is raw git stderr; sanitise before drawing.
- `InProgressDetail::head_name` may be the literal `detached HEAD`.
