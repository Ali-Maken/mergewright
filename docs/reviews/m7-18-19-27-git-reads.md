# Review — m7-power-tools group: 18 trace-readers, 19 lost-commit-cost, 27 graft-git-run-fixes, 10a switch-refs-read

Reviewer, 2026-09-26. **Fixed hash: `62ace7b0`** (merges 0d42b5c0 = 18, 60340f33 = 19, 93c10f57 = 27, 473a7dbc = 10a).
Each merge was read against its first parent, and the code at 62ace7b0. `main` moved during the review (20a, 03a,
now `9702680e`). The findings below that name `blame.rs` / `switch_refs.rs` were rechecked at `9702680e`: all still
there. The `blame.rs` line numbers are 62ace7b0's; 20a shifted them by +3.
27's item 1a (branch `m7-27-1a-stop-after-exit`) was not reviewed, as instructed.

**Counts: Critical 0 · Major 4 · Minor 14.**

## Gates (run at 62ace7b0, own worktree + own `CARGO_TARGET_DIR`, one command at a time)

| Check | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass (27's item 2 holds) |
| `cargo nextest run` (run 1) | **3773/3773 pass**, 1 skipped. Nextest summary 228.5 s (280 s wall, with the build) |
| `cargo nextest run` (run 2, `--status-level pass`) | 3772/3773. **1 fail:** `graft-fixtures remote::tests::http_remote_frees_its_port_on_drop` ("port 57046 still accepts after drop", `crates/graft-fixtures/src/remote/tests.rs:313`). It passed in run 1. The load average was 12–14 from concurrent builders, so another process probably took the ephemeral port. Not this group's code. **Unverified as a flake; not in KNOWN_GAPS.** |
| `cargo test --doc` | pass |
| Heavy tier, whole workspace (`--features heavy-tests -- --test-threads=2`) | **Aborted at ~35 min**, past 7.1's 10 min budget. graft lib 1320/1320 had passed; it was inside `graft::it` `frames::*` in a debug build. Load average 7–14 from other builders. **No verdict on the heavy tier as a whole.** |
| Heavy tier, graft-git only (`-p graft-git --features heavy-tests -- --test-threads=2`) | 404/404 pass, 33 s |
| Release perf run (`frames:: heavy::budgets_hold`) | **Not run.** The conductor's brief says: no measurement in this context. The perf claims below were read from `docs/perf/`. |
| Fuzz, 60 s each (`cargo +nightly fuzz run <t> -- -max_total_time=60`) | `blame_porcelain` 448 885 runs, `line_log` 427 865, `follow_log` 523 354, `parse_unified` 488 352 (27 changed its header parser): **no crash** |

- Fast suite: 228.5 s by nextest's summary, **over 7.1's 3-minute line**. The machine was shared with other builders
  (load 7–14), so this is not a clean measurement.
- 5 slowest (run 2):
  - `diff_fixture::parse_unified_matches_numstat_for_every_shape` 17.8 s
  - `conflict_ops::whole_file_shapes_resolve_like_git` 17.2 s
  - `undo_cancel::cancel_at_every_step_boundary_is_clean` 16.0 s
  - `m5_walk::every_scenario::merge_resolved_with_the_keys` 14.3 s
  - `layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded` 14.0 s
- Peak RSS of the nextest run: 1.04 GB (`/usr/bin/time -l`).
- Size of `target/`: the shared one is 12 GB; the review's own is 7.7 GB, plus 869 MB for fuzz.

## Tick spot-check (a false tick is Critical)

| Ticket | Checked | Held | How |
|---|---|---|---|
| 18 | 11 | 11 | Named tests present and passing (run 2 lists each), plus the mutations below. Details after the table. |
| 19 | 5 | 5 | Details after the table. |
| 27 | 4 | 4 | Details after the table. 1a is correctly left unticked. |
| 10a | 10 | 10 | Details after the table. |

How each was checked:

- **18:**
  - Read the oracle: real `git blame --line-porcelain` at 3 revs plus the working tree.
  - Killed by mutation:
    - `a_skipped_line_is_marked_skipped`: M16;
    - `ignore_revs_false_clears…`: M20;
    - `range_at_moves_with_the_code`: M19;
    - `trace_capped_says_how_many`: M21;
    - `whole_file_of_a_merge…`: M22;
    - `map_to_head_shifts_past_a_removal`: M18.
  - The "red first" claim on `a_rust_method_is_its_fn_not_the_impl` reproduced: with the Graft attributes file
    suppressed (M24), it fails.
  - The perf numbers match `docs/perf/trace-readers.txt`.
  - The fuzz targets build and run.
- **19:**
  - The oracle is computed apart from the reader (`tests/it/unreached.rs:108-148`).
  - The builder's mutation claims reproduced: HEAD dropped (M10) and older stash entries dropped (M12b) each fail
    their test.
  - The runs and limits tests catch M11 and M12.
  - The summary line matches the perf file.
  - MAP entry: its text read against the code.
- **27:**
  - 1b: M13 fails `a_stop_asked_before_run_spawns_nothing`.
  - 2: the heavy clippy line above passes.
  - 3a and 3b: M14 fails both named tests.
- **10a:**
  - All 9 named tests exist and pass.
  - MAP section read against the code.
  - Its red-first claim could not be checked after the fact.

Several ticks carry disclosed deviations: 18's `blame_honours_stop` and `stop_kills_git_and_keeps_the_rows`, and 18's
hand-written 2.38 samples. They are disclosed at the tick, so they are **not** false ticks. They appear below where
they leave something unverified.

## Mutation check (the conductor asked, for 10a especially)

Each mutation was one source edit in the review worktree. The test filter was the module's tests
(`cargo nextest run -p graft-git -E 'test(/<module>/)'`), and each edit was reverted after its run. **Caught** means
at least one test failed.

| # | Mutation | Result |
|---|---|---|
| M1 | 10a: any non-zero `@{-1}` exit → `Ok(None)` (`switch_refs.rs:82`) | **survived** |
| M2 | 10a: symref rows kept | caught (2 tests) |
| M3 | 10a: `%(authordate)` for `%(committerdate)` | caught |
| M4 | 10a: a malformed record is skipped (`continue`), not `Err` (`switch_refs.rs:119`) | **survived** |
| M5 | 10a: ref name decoded lossily | caught |
| M6 | 10a: `refs/remotes` dropped | caught |
| M7 | 10a: upstream never read | caught |
| M10 | 19: `HEAD` not held | caught |
| M11 | 19: no folding of candidates into runs | caught |
| M12 | 19: `under` counts double | caught (3) |
| M12b | 19: only `stash@{0}` held | caught |
| M13 | 27: no pre-spawn stop check in `run` | caught |
| M14 | 27: no mixed-quote branch | caught (2) |
| M15 | 18: blame without `-M` | **survived** |
| M16 | 18: `markIgnoredLines=false` | caught |
| M17 | 18: the "shallow" end reads as `Start` (`log.rs:153-157`) | **survived** |
| M18 | 18: `map_to_head` removal boundary `<=` → `<` | caught |
| M19 | 18: a function ends only at a *shallower* header | caught (4) |
| M20 | 18: `ignore_revs: false` does not clear config | caught |
| M21 | 18: cap off by one | caught |
| M22 | 18: `whole_file` diffs against the last parent | caught |
| M23 | 18: the working-tree on-disk binary check ignored (`blame.rs:268-282`) | **survived** |
| M24 | 18: no Graft attributes file | caught (3) |

- **10a: 5 of 7 caught.** Its two gaps are exactly the two error rules of its spec: "any other git failure → Err",
  and a bad record being an error. So its tests catch a wrong answer, but not a swallowed failure.
- 18: 9 of 12 caught.
- 19: 4 of 4 caught.
- 27: 2 of 2 caught.

## Critical

None.

## Major

### 1. A blame cut at the runner's 64 MiB cap is returned as a whole blame (rule 3)
- **Where:** `crates/graft-git/src/blame.rs:225-229` (`9702680e`: `:228-232`).
- **What's wrong:** `blame` takes `output.stdout` and never looks at `output.truncated`.
  - The runner's contract is that a capped read "is still a successful one" (`run.rs:26-28`).
  - `parse_porcelain` ends cleanly when the cut falls inside a text line. The last entry's `\t…` line ends that entry
    (`blame.rs:401-403`), and the loop ends at end of input (`:386`).
  - Most bytes of porcelain are text lines, so most cuts land there. The result is `Ok(Blame)` with the tail of the
    file missing and the last line's text shortened, and nothing tells the user.
  - When the cut lands in a header, it is a `Parse` error saying git's output "made no sense", which is also wrong.
- **Why it matters:** the blame screen would show a short file as if it were complete. The trigger is a generated or
  vendored file of roughly ~1M lines, which exists in monorepos.
- **Siblings in the same group already do this right:** `trace/log.rs:123`, `trace/whole.rs:64` and
  `history_reads/unreached.rs:114`.
- **Fix:**
  - `if output.truncated { return Err(BlameError::Run(RunError::Unparsable(…))) }`, or a `TooLarge` variant.
  - Test it with `output_cap` lowered through a test seam, or with a unit test on a cut porcelain sample plus the
    flag.

### 2. `unreached` puts `--not` on `--stdin`, which git before 2.42 refuses. The floor is 2.38. **Unverified**
- **Where:** `crates/graft-git/src/history_reads/unreached.rs:97` (`input.push_str("--not\nHEAD\n")`).
- **What's wrong:**
  - `rev-list --stdin` learned pseudo-options such as `--not` in git 2.42 (per the 2.42 release notes, from memory).
  - Before 2.42, `read_revisions_from_stdin` dies with "options not supported in --stdin mode".
  - The project's floor is 2.38 (`PROMPT.md:51`, `version.rs:5`).
  - The builder probed git 2.54 only (MAP.md `history_reads`: "checked, git 2.54").
- **Not observed:** this machine has only git 2.54 (`/usr/bin/git` and the CLT git, both Apple Git-157). It is filed
  as Major because, if true, `J`'s lost-commit scan fails outright on 2.38–2.41. It is **unverified**, so the first
  step is to check it.
- **Why it matters:** "you cannot lose work" (M3's promise) depends on this scan. It would fail loudly
  (`couldn't look for lost commits: …`), not silently.
- **Fix:** it is cheap and version-proof. Send the negatives as `^HEAD` and `^<stash-entry>` lines on stdin, not
  `--not` + lines. Every git version takes `^<rev>` on stdin. Verify on a 2.38 build (a container, or a
  `brew`-extracted older git) before closing.

### 3. The blame p95 (14.3 ms) measures a one-line file, so blame's cost is unmeasured (rule 1)
- **Where:** `docs/perf/trace-readers.txt:5` ("trunk.txt: 1 lines"). Ticket 18 NOTES says so itself.
- **What's wrong:** blame's cost is walking back until every line is attributed. A single line that HEAD rewrote is
  attributed at HEAD, so the walk is one commit. The 14.3 ms is spawn plus parse, not blame.
  - `line_history`'s 252.6 ms and `file_history`'s 18.5 ms are real deep-history numbers. Both are capped at 1000.
  - The tick discloses the gap ("See NOTES on what this does not measure"), so it is not a false tick. But the
    ticket's question, "over 1 000 ms for blame → ask", is still unanswered. The p95 figure being quoted as "blame
    p95" is not evidence.
- **Why it matters:** blame is the screen he opens on large files daily. The research (§4.10) is why it runs async
  at all.
- **Fix:** measure blame on a long file with interleaved history:
  - either a synthetic file of several thousand lines, where each of N commits touches a scattered subset;
  - or a real large file (for example git.git's `diff.c`), from the script into `docs/perf/`.
  - Add a summary line `blame-longfile p95=…`.

### 4. Two new files are over the 400-line limit (PROMPT.md 6.2)
- **Where:** `crates/graft-git/src/blame.rs` (598 lines at 62ace7b0, 606 now), `crates/graft-git/src/trace/log.rs`
  (454).
- **Why it matters:** 6.2 is a hard rule, and these are Major by the review's own terms.
- **Fix:**
  - `blame.rs`: move the porcelain parser (`parse_porcelain` … `record_commit`, ~170 lines) to `blame/porcelain.rs`,
    and delete its private `unquote` (Minor 13).
  - `log.rs`: split the `--follow` reader (`parse_follow_records`, `FollowFiles`, `parse_follow_files`, `stat`, ~115
    lines) into `trace/follow.rs`.

## Minor

1. **`@{-1}` ambiguous with a tag reads as "detached": the error is swallowed (rule 3).**
   - Where: `crates/graft-git/src/switch_refs.rs:83-86`.
   - Observed (git 2.54): a throwaway repo with branch `b` and tag `b`; after `b → c`,
     `rev-parse --verify --quiet --symbolic-full-name @{-1}` prints nothing, **exits 0**, and writes
     `error: refname '@{-1}' is ambiguous` to stderr. The code maps "exit 0 + empty" to "previous was detached", so
     `previous: None`, while `git switch -` would go to `b`.
   - Branches and tags that share a name (`v1.2`) happen on teams.
   - Fix: `git check-ref-format --branch @{-1}` gives the name git's own `-` uses. Observed: it prints `b` in the
     ambiguous case and a full id when the previous checkout was detached. Keep it only if `refs/heads/<name>` is in
     the listing just read, which covers the deleted case.
2. **10a's error rules have no test** (M1 and M4 survived).
   - "Any other git failure → Err" is untested: `switch_refs.rs:92-100`.
   - "A bad record is an error" is untested: `switch_refs.rs:118-120`.
   - Fix:
     - one test where `@{-1}` fails for another reason, for example a corrupt `HEAD` reflog line, asserting `Err`;
     - one unit test on `parse_listing` with a six-field record.
3. **18's stop tests never stop a running git.**
   - `blame_honours_stop` (`tests/it/blame.rs:290`), `stop_kills_git_and_keeps_the_rows` (`tests/it/trace.rs:302`),
     `unreached_honours_stop` (`tests/it/unreached.rs:335`) and `switch_refs_honours_stop`
     (`tests/it/switch_refs.rs:204`) all pass an `AtomicBool` that is already true.
   - Since 27's 1b, `run` refuses before spawning (`run.rs:333-338`), so these four now test that pre-check only.
   - The kill path is proven only by the runner's own tests (`run.rs` `a_stopped_command_is_killed_and_reaped`,
     `tests/it/run.rs` `a_stop_ends_a_long_log`). That is adequate, since every read goes through `run`. But 18's
     tick text "(the child is killed)" describes nothing these tests do.
   - Fix: reword the ticks, or add one `SecondPoll`-style test per read.
4. **Enclosing function: output not checked for truncation, and a capped diff reported as `Stopped`.**
   - `crates/graft-git/src/trace/function.rs:93-94` uses `cat-file`'s stdout without checking `truncated`.
   - `function.rs:146` maps `code: None` to `RunError::Stopped`. In `run_raw`, `code: None` comes only from the
     output cap (`run.rs` `supervise_raw`, `Ending::Capped`). So a file whose marked diff passes 64 MiB (≈500k lines)
     is reported as stopped by the user.
   - Fix: check `truncated` on both and return `Unparsable`/too large.
5. **Dead branch with a misleading comment.**
   - Where: `crates/graft-git/src/trace/log.rs:128-132`. `raw.code == None` implies `truncated`, which `:123` has
     already returned on.
   - Its comment says "the stop", but a stop arrives as `Err(Stopped)` (`:120`).
   - Fix: delete it, or make it `unreachable`-free by matching `(truncated, code)` once.
6. **The shallow-history end is untested** (M17 survived).
   - Where: `crates/graft-git/src/trace/log.rs:155-157` (`GitStops("…a shallow clone?")`).
   - Fix: add one test on `git clone --depth 2` of the session fixture.
7. **`-M` in blame is untested** (M15 survived).
   - Where: `blame.rs:204`. The fixture has no moved lines, and the oracle passes `-M` itself.
   - Fix: add a moved block to the fixture, or to a copy in the test.
8. **The working-tree on-disk binary check is untested** (M23 survived).
   - Where: `blame.rs:268-282`. `binary_is_refused_before_spawning` uses a file that is binary in HEAD too.
   - Fix: add a test with a text file that is binary on disk, and one with the reverse.
9. **A commit with no file record gets an empty path.**
   - Where: `crates/graft-git/src/trace/log.rs:365`, `files.path.map(…).unwrap_or_default()`. Example: a merge that
     `--follow` lists without a diff.
   - An empty `PathBuf` stands in for "no path". It is not an error, but downstream reads it as the root.
   - Fix: `Option<PathBuf>`, or skip the row, with a comment.
10. **Records are split on `\x01` (`log.rs:198-216`).**
    - A `\x01` byte in a line inside an `-L` diff, or in a subject, splits a record. It fails loudly (`Parse`), not
      silently.
    - Fix: frame the header with a sentinel git cannot emit in content, for example `%x00%x00` at line start, or
      count lines.
11. **Stale perf row.**
    - Where: `docs/perf/lost-commit-cost.txt:38` reads `found=WRONG` for method (b). Ticket 19 NOTES says the example
      was fixed after the run to print `not-judged`.
    - Anyone reading the file sees "WRONG".
    - Fix: edit the line by hand with a note, or re-run (b) on fewer samples.
12. **The "2.38" parser samples are hand-written but named as if captured.**
    - Where: `src/trace/tests.rs:17,38` (`LINE_LOG_2_38`, `FOLLOW_LOG_2_38`) and `src/blame/tests.rs:12`
      (`PORCELAIN_2_38`). The module docs say so, but the test names (`the_2_38_…`) read as real 2.38 output.
    - The parsers are **unverified against a real 2.38**.
    - Fix: rename them to `hand_written_…`, or capture from a real 2.38.
13. **Duplicated helpers.**
    - `blame.rs:536` `unquote` duplicates `diff/unified/primitives.rs:125`.
    - `trace/log.rs:321` `find` duplicates `primitives.rs:100`.
    - Fix: use the `diff` ones (both are `pub(crate)`).
14. **Unmeasured cost and an unclear definition** (questions, no defect claimed):
    - (a) `switch_refs`' `for-each-ref` parses every tip commit (`%(subject)`, `%(authorname)`, `%(committerdate)`,
      `switch_refs.rs:48-49`). On the 50k-ref monorepo that the 19 measurement modelled, the picker's open cost is
      unmeasured, and so is its 64 MiB-cap behaviour (`switch_refs.rs:64` does not check `truncated`).
    - (b) The held set in `unreached` (`unreached.rs:101-111`) leaves out other worktrees' `HEAD`s. A commit checked
      out detached in a linked worktree shows as lost. That matches the spec's list, but is it what he means by lost?
    - (c) 27's 1b test sleeps 2 × 500 ms (`tests/it/run.rs:121`), which adds 1 s to the fast suite.

## Rule 3: every swallow in the new reads

- **Fine, with a comment:**
  - `log.rs:139` (parse of a failed run's stdout dropped; git's reason is returned);
  - `log.rs:202`, `unreached.rs:155` (`split` always yields);
  - `blame.rs:281` `.unwrap_or(binary)` (a real default: no on-disk row).
- **Fine, handled:** `blame.rs:342` (unreadable ignore file). It is passed to git, which reports it.
- **Findings:** Major 1 (blame truncation); Minor 1 (`@{-1}` stderr); Minor 4 (function-range truncation and
  cap-as-stop); Minor 9 (empty path); Minor 14a (`switch_refs` truncation).

## Oracle strength against real git

- Strong:
  - blame: `--line-porcelain` line for line, 3 revs plus the working tree, and per-line paths;
  - `line_history`: ids against `git log -L`;
  - 27's header: git's own `diff -M` bytes;
  - `unreached`: a separately computed `rev-list` plus stash entries.
- Weak:
  - `file_history` is checked against hard-coded fixture ids, not `git log --follow` output;
  - `enclosing_function` is checked against hand-read expectations, which is unavoidable;
  - the 2.38 samples are hand-written (Minor 12).

## Stop behaviour of the reads

- **Every spawn goes through `run`/`run_raw`.** Since 27's 1b (`run.rs:333-338`, `:357-360`), a stop already asked
  spawns nothing, and a stop mid-run kills and reaps (the runner's tests).
- **Multi-spawn reads check the stop between spawns** through that pre-check:
  - blame: 2–3 spawns;
  - `enclosing_function`: up to 4;
  - `whole_file`: 2–3;
  - `unreached`: 2, plus its own redundant check at `unreached.rs:81`;
  - `switch_refs`: 2.
- **The stop shapes differ, as the specs fixed them:**
  - `BlameError::Stopped`;
  - `TraceEnd::Stopped` (an `Ok`);
  - `RunError::Stopped`;
  - `DiffError::Run(Stopped)`.
- **One mislabel:** Minor 4. A cap is reported as a stop.
- **The 1a race is still open on these reads:** a stop that lands after git has exited still reads `Stopped`. For
  read-only reads that is harmless, since the caller asked to stop. 27b carries it for writes.

## 27 in particular

- **1b:**
  - Minimal and at the cause: a pre-spawn check in both `run` and `run_raw`.
  - Its test uses a real `update-ref`, and M13 makes it fail.
  - The runner's unit test was rewritten so that it still exercises the kill: `supervise` is called directly with a
    set `AtomicBool`.
- **3a:**
  - Correct: the bare path cannot hold `"` because git quotes it.
  - The quoted-first case was already handled.
  - Fuzzed 60 s through `parse_unified`, with no crash.
- **2:** confirmed clean.

## Pipeline

- These are reads with no screen of their own.
- The design note has Need, Prior art (`docs/research/blame-line-history.md`, `reflog.md`) and Journeys (2b "Journey
  counts"). Step counts belong to the screen groups (20a/b, J) and were not counted here.
