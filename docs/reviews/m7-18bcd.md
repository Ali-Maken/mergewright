# Review: m7-power-tools 18b, 18c, 18d — blame and trace git reads, second round

Reviewed at fixed hash `58419879`, in a separate worktree with its own `CARGO_TARGET_DIR`, one cargo command at a time.
Scope: 18b (merge `79b801f5`), 18c (merge `aeee9040`, ADR-0050), 18d (merge `ecb41b6b`). Each diff read as
`git diff <merge>^1 <merge>`.

**Counts: Critical 0 · Major 1 · Minor 7 · Questions 1.**

Known and not refiled: M7/boundary, M7/20eg-q1, the 20d boundary-line note, M7/18r (the first review's Minors,
including Minor 5, the dead `code: None` arm at `trace/log.rs:140-143`).

## Gates (at `58419879`)

| Check | Result |
|---|---|
| `cargo fmt --check` | ok |
| `cargo clippy --all-targets -- -D warnings` | ok (52 s) |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | ok |
| `cargo nextest run --workspace` | **4061/4061**, 1 skipped; 441 s test time, 515 s wall with build. A second run: 4061/4061, 464 s |
| `cargo test --workspace --doc` | 3/3 |
| Heavy tier, release perf run | **not run**, on the conductor's instruction (the previous review ran both an hour earlier) |
| Fuzzing | not run. The parsers are unchanged apart from moves (18b), so `blame_porcelain`, `line_log` and `follow_log` were not re-fuzzed |

- Five slowest tests: `undo_cancel::cancel_at_every_step_boundary_is_clean` 27.7 s;
  `hints::tests::a_tail_hint_gains_its_verb_only_once_the_whole_list_is_drawn` 26.5 s;
  `hints::tests::no_hint_loses_ground_as_the_room_for_it_grows` 26.2 s;
  `hints::tests::the_bar_never_draws_a_fragment_of_itself` 24.7 s; `hints::tests::the_bare_keys_are_a_tail_not_a_scatter`
  22.0 s. None of them is from this group. This group's slowest test is `unreached::unreached_matches_rev_list_all_on_candidates`
  at 1.2 s.
- Peak memory: the largest process reached 1.05 GB RSS (`/usr/bin/time -l` over the nextest run).
- `target/` (own dir) after all of the above: 4.2 GB.

### On git 2.38.5, the floor

I used the 2.38.5 binary that 18b built under `$TMPDIR/oldg238`, first on `PATH`. I ran
`cargo nextest run -p graft-git -E 'test(/blame|trace|switch_refs|unreached/)'`: **85/86 pass**. The one failure is
`blame::a_skipped_line_is_marked_skipped`, which comes from ticket 18. See Major 1.

Every test this group added passes on 2.38.5: the six 18c shallow tests, both 18d tests, both `@{-1}` tests, M15, M23
and all 13 `unreached` tests.

Probes by hand, on both 2.38.5 and 2.54:
- `rev-list --stdin` with a `--not` line: 2.38.5 fails `fatal: options not supported in --stdin mode` (exit 128),
  2.54 accepts it. `^<rev>` lines, including a missing id under `--ignore-missing`, work on both. This confirms 18b's
  Major 2 claim.
- `@{-1}` with a branch and a tag both named `b`: `rev-parse --verify --quiet --symbolic-full-name` exits 0 with
  nothing on stdout and `error: refname '@{-1}' is ambiguous` on stderr, and `check-ref-format --branch @{-1}` prints
  `b`. Same on both versions. Confirms 18b.
- `blame.ignoreRevsFile=no-such-file`: `--ignore-revs-file=` fails `fatal: could not open object name list`, and
  `--no-ignore-revs-file` blames. Same on both. 18d probed 2.54 only, so this confirms its fix on the floor.
- A depth-2 clone, blame line 18: 2.38.5 names `0a27627 Priya` with `boundary`, and `log -L18,29` lists
  HEAD then the boundary. This matches 18c's 2.54 probe, so the boundary filter sees the same input on the floor.

## Tick spot-check (a false tick is Critical)

I checked **20 of 22 ticks. All 20 held.**

- **18b, 8 of 8 checked:**
  - Major 1: I read the test. The red line (`src/blame/tests.rs:207` at `79b801f5`) is the `panic!`.
  - Major 2: probe plus the 2.38.5 run.
  - Major 3: the p95 line is at `docs/perf/trace-readers.txt:33`, and the script exists.
  - Major 4: line counts at `79b801f5` are 358/192/55/323/134, against the 359/192/55/323/134 claimed. Off by one, and
    under 400 either way.
  - The mutation tick: all five tests exist, and on reading each one fails under its mutation. I did not apply the
    mutations: the conductor's rule is no source edits.
  - `@{-1}`: probe plus the 2.38.5 run.
  - Minor 4: I read the test.
  - Gates: re-run at HEAD.
- **18c, 8 of 10 checked:**
  - The six test ticks: every test exists and passes on 2.54 and 2.38.5.
  - The depth-6 oracle: holds on reading.
  - ADR-0050: its text matches the spec's Part 1.
  - Not checked: the MAP.md tick and the gates tick.
  - `an_unreadable_shallow_file_is_an_error` says itself that its red was not observed. That is honest, and the spec
    did not mark it red-first.
- **18d, 4 of 4 checked.** I reproduced the red with a probe on both versions. The comment at `blame.rs:245-248` now
  says what git does.

**Red-first claims:** 18d's red is reproduced independently. 18b's `@{-1}` red and its `--not` red are reproduced by
probe. The other red claims are consistent with the code on reading but were not re-run.

**Oracles:**
- Independent of the code under test: git's own `switch -` (the `@{-1}` tests); plain `git blame` without `-M` as the
  M15 premise; `rev-parse HEAD~1` as the boundary id (18c); `rev-parse --git-path shallow` (the worktree test); git's
  `log -L` in the clone (depth 6).
- The two truncation tests call the helper directly rather than the reader (Minor 3).

## Critical

None.

## Major

### 1. On git 2.38, blame's porcelain has no `ignored` line, so lines blame skipped are never marked. The hand-written "2.38" sample says it does
- **Where:**
  - `crates/graft-git/src/blame/porcelain.rs:127-128`: the marks come only from `ignored`/`unblamable` porcelain lines.
  - `crates/graft-git/tests/it/blame.rs:220`: `a_skipped_line_is_marked_skipped` fails on 2.38.5 (`left: Plain`,
    `right: Skipped`).
  - `crates/graft-git/src/blame/tests.rs:12-55`: `PORCELAIN_2_38`, written by hand, carries `unblamable`.
- **Observed:** the session fixture, `-c blame.markIgnoredLines=true … blame --porcelain -M -w
  --ignore-revs-file=.git-blame-ignore-revs -L30,31 HEAD`.
  - 2.54 writes `ignored` under line 31's header.
  - 2.38.5 writes nothing there.
  - The human format shows `^?3f6c31` on both, so git 2.38 still skips the line. Only the porcelain omits the mark.
  - Not verified: `unblamable` on 2.38, and which git version first writes these lines.
- **Why it matters:**
  - At the stated floor (`version.rs`, 2.38), the blame place never draws a skipped or unblamable mark. The `i` line
    can still say the list is applied. He sees lines credited to an older commit with nothing saying why (H1).
  - The "2.38" oracle asserts the opposite of real 2.38. M7/18r's Minor 12 called it unverified; it is now verified
    wrong.
  - Not from this group: this is ticket 18's code. I found it by running this group's reader suites on the floor, as
    asked.
- **Fix (a decision, through the conductor):**
  1. Find the first git version that writes `ignored` in porcelain.
  2. Then either:
     - raise the floor to that version; or
     - on older git, read the marks from the human format's `?`/`*`; or
     - say on the `i` line that marks need git ≥ X.
  3. Re-capture the 2.38 sample from the real 2.38.5 binary under `$TMPDIR/oldg238`.

## Minor

1. **18d knock-on: with a configured ignore-revs file git cannot open, blame fails and the blame place has no way out.**
   - **Reader side:**
     - `crates/graft-git/src/blame.rs:238-249`: both `true` arms leave the configured `blame.ignoreRevsFile` to git.
     - Probed on 2.38.5 and 2.54: git fails with `fatal: could not open object name list`. That holds whether or not
       the root has `.git-blame-ignore-revs`, because git appends ours after the configured one and still opens both.
   - **Place side:**
     - `crates/graft/src/jobs/detail.rs:159` makes the failure `BlameFailure::Failed`.
     - Then `i` does nothing: `crates/graft/src/update/place/keys.rs:66-68` returns when `ignore_line()` is `None`, and
       `:406-408` makes it `None` for any body but `Ready`.
   - **Why it matters:** re-running with `--no-ignore-revs-file` would work, but the only key that does that is dead
     exactly then. That fails H9 (help recover from errors).
   - 18d's reader behaviour, surfacing git's error, is right. The gap is the place.
   - **Fix (spec call):** let `i` re-run on this failure. For example, keep `i` live on `Failed` when the options had
     `ignore_revs: true`, and name `i` in the failure line. Then a test that presses `i` from the failure.
2. **Blame's too-large error reads as a parse failure.**
   - `crates/graft-git/src/blame.rs` `read_porcelain` returns `RunError::Unparsable("git blame's output passed the
     runner's 64 MiB cap")`, and `Unparsable`'s `Display` (`crates/graft-git/src/run.rs:149`) prefixes `git's output
     did not parse as expected:`.
   - `jobs/detail.rs:159` shows it verbatim, so the words contradict themselves: it parsed, it is too big.
   - The same class: `trace/function.rs` `too_large`, `trace/log.rs:136`.
   - **Fix:** a `RunError::TooLarge { what }` with its own words, or `BlameError::TooLarge`. Change the siblings with it.
3. **Both truncation tests call the helper, not the reader, so bypassing the helper survives.**
   - `a_blame_cut_at_the_output_cap_is_an_error_not_a_short_file` (`src/blame/tests.rs:189-209`) calls
     `read_porcelain`. Replacing `read_porcelain(&output)?` in `blame()` with `parse_porcelain(&output.stdout)` keeps
     it green.
   - The same holds for `enclosing_function_refuses_output_cut_at_the_cap` (`src/trace/tests.rs`) against
     `function.rs:88-96`.
   - The ticket allowed "a cut porcelain sample", so the tick holds; the gap is the call site.
   - **Fix:** one test through `blame()` with a lowered cap, if the runner's cap can be set per invocation. If it can't,
     a doc line saying the call site is unguarded.
4. **`switch_refs` keeps a flag it no longer uses, and the docs disagree about it.**
   - `crates/graft-git/src/switch_refs.rs:86` passes `--symbolic-full-name`, but only the exit code is read (`:92`).
   - The module doc (`:9`) and `has_previous`' doc (`:102`) name the command without the flag; MAP.md `:111` names it
     with the flag.
   - **Fix:** drop the flag. Exit 0/1 is the same without it; re-check on 2.38.5, whose binary is still under
     `$TMPDIR/oldg238`. Or keep the flag and say why.
5. **An unreadable `shallow` file hides git's own failure.**
   - `crates/graft-git/src/trace/log.rs:145` reads the boundaries before the `code != 0` branch (`:146`).
   - When git failed and the `shallow` file is also unreadable, he sees `couldn't read …/shallow` instead of git's
     reason.
   - This is rare. It is ordering, not a swallow.
   - **Fix:** read the boundaries lazily: on the failure path, fall back to git's reason when the read fails.
6. **Ticket 21 (history place) doesn't carry 18c's new inputs.**
   - `.scratch/m7-power-tools/issues/21-history-place.md` has no mention of shallow, `GitStops`, zero rows or
     `TraceError::Shallow`.
   - 18c made each of these possible:
     - a depth-1 clone gives **zero rows** and `GitStops(SHALLOW_END)` (`a_depth_one_clone_traces_no_rows_and_says_shallow`);
     - `TraceError::Shallow` is new (`trace/mod.rs:123-129`).
   - 18c's Touches said "no consumer yet", which was correct, but nobody carried it forward. This is the connection
     CLAUDE.md's knock-on rule exists for.
   - **Fix:** add to 21's spec: the place draws an empty history with the end row alone, with no cursor-on-row-0
     assumption; `Shallow` is drawn as a failure; and one test for each.
7. **Stale gap row, and a budget nothing runs.**
   - `KNOWN_GAPS.md:863` (M7/18a, "Blame on a long file is unmeasured") is answered: 18b measured p95 1 481 ms
     (`docs/perf/trace-readers.txt:33`), and ADR-0050 decided on it. Close it to `docs/gaps/closed.md`.
   - ADR-0050's budget says "the milestone review's release run reads `blame-longfile p95`". But TESTING.md's heavy
     tier and release perf section (`TESTING.md:161-180`) never names `scripts/m7-perf-blame-longfile.sh`, and the
     reviewer's release command doesn't run it. So the 3 000 ms regression line has no runner.
   - **Fix:** one line in TESTING.md next to the release perf run.

## Questions

- **Q1. Can a dropped boundary row eat the cap's probe row?**
  - `line_history`/`file_history` ask git for `--max-count=cap+1` (`trace/log.rs:60`, `:100`), a count that includes
    boundary rows. `finish` then compares the filtered count to `cap` (`:171`).
  - Suppose one of the cap+1 rows fetched is a boundary that is not the oldest row, and git has more rows past it. Then
    the end reads `SHALLOW_END` where it should read `Capped`.
  - That needs uneven boundaries (`--shallow-since`, a `fetch --deepen` on one branch). Not verified whether git
    produces that shape, so it is a question, not a defect.
  - If it can: ask git for `cap + 1 + |boundaries|` rows, or compare against `fetched > cap` before filtering.

## Knock-ons on the readers' callers (checked)

- **Blame place** (`crates/graft/src/jobs/detail.rs:145-186`, `update/place/keys.rs`):
  - `BlameError` is matched with a catch-all (`:159`), so nothing breaks.
  - The cap error reads badly (Minor 2), and the ignore-revs failure is a dead end (Minor 1).
  - `SHALLOW_END` is reused in `keys.rs:192` (20d/20e), which is consistent with the trace.
- **History place:** not built. No app caller of `line_history`/`file_history`/`TraceError` (grep over
  `crates/graft/src`, `graft-ui/src`, `graft-ops/src`). Its ticket lacks 18c's inputs (Minor 6).
- **`unreached`:** one caller, `crates/graft-ops/src/journal_list.rs:218`. Signature unchanged; the workspace suite is
  green.
- **`switch_refs`, `enclosing_function`:** no caller outside graft-git (grep).

## Rule 3 in the new code

- No `let _`, `unwrap()`/`expect()` or empty `Err(_)` in the non-test lines this group added.
- Two `unwrap_or_default()` in `trace/follow.rs`: moved code, not new, and already filed (review 18 Minor 9, M7/18r).
- The new swallows are handled:
  - `shallow_boundaries`: `NotFound` means not shallow, and any other failure is an error.
  - `has_previous`: exit 1 means none, and any other exit is an error.
  - `marked_diff`: `None` means too large, not a stop. That is correct per `run.rs:430` and `:491`.

## Gap rows

Ready to paste into KNOWN_GAPS.md.

```
| M7/18bcd-major1 | **git 2.38's blame porcelain writes no `ignored` line: skipped lines get no mark at the floor** (`blame/porcelain.rs:127-128`; `tests/it/blame.rs:220` `a_skipped_line_is_marked_skipped` fails on 2.38.5 — `Plain` vs `Skipped`). The hand-written `PORCELAIN_2_38` (`src/blame/tests.rs:12-55`) carries `unblamable`, the opposite of real 2.38 for `ignored`. First git version writing the lines, and `unblamable` on 2.38: unverified. Decide: raise the floor, read the human format's `?`/`*` on old git, or say so on the `i` line; re-capture the sample from `$TMPDIR/oldg238`. Source: `docs/reviews/m7-18bcd.md` Major 1 | **broken** | M7 |
| M7/18bcd-m1 | **A configured ignore-revs file git can't open: blame fails and `i` is dead** (`blame.rs:238-249` leaves it to git, which is fatal on 2.38.5 and 2.54; `place/keys.rs:66-68`, `:406-408` make `i` a no-op on `Failed`). No way to re-run with `--no-ignore-revs-file` (H9). Spec call. Source: review m7-18bcd Minor 1 | **broken** | M7 |
| M7/18bcd-m2 | **Blame's too-large error reads `git's output did not parse as expected: … passed the runner's 64 MiB cap`** (`run.rs:149` `Unparsable` Display; `blame.rs` `read_porcelain`; shown verbatim by `jobs/detail.rs:159`). Same in `trace/function.rs` `too_large`, `trace/log.rs:136`. Source: review m7-18bcd Minor 2 | **broken** | M7 |
| M7/18bcd-m3 | **The truncation tests call the helpers, not the readers** (`src/blame/tests.rs:189-209`, `src/trace/tests.rs` `enclosing_function_refuses_output_cut_at_the_cap`): bypassing `read_porcelain`/`blob_content` at the call site stays green. Source: review m7-18bcd Minor 3 | **unverified** | M7 |
| M7/18bcd-m4 | **`switch_refs` passes `--symbolic-full-name` and reads only the exit code** (`switch_refs.rs:86`, `:92`); module doc `:9` and `:102` omit the flag, MAP.md `:111` has it. Source: review m7-18bcd Minor 4 | **broken** (docs disagree) | M7 |
| M7/18bcd-m5 | **An unreadable `shallow` file hides git's own failure reason** (`trace/log.rs:145` reads boundaries before the `code != 0` branch at `:146`). Source: review m7-18bcd Minor 5 | **unverified** (not observed) | M7 |
| M7/18bcd-m6 | **Ticket 21 (history place) lacks 18c's inputs:** zero rows + `GitStops(SHALLOW_END)` on a depth-1 clone, and `TraceError::Shallow` (`trace/mod.rs:123-129`). Add to 21's spec with a test each. Source: review m7-18bcd Minor 6 | **unverified** (no consumer yet) | M7 |
| M7/18bcd-m7 | **ADR-0050's blame budget (3 000 ms) has no runner:** `scripts/m7-perf-blame-longfile.sh` is not in TESTING.md's release perf section (`TESTING.md:161-180`). Also close M7/18a (`KNOWN_GAPS.md:863`), answered by 18b + ADR-0050. Source: review m7-18bcd Minor 7 | **broken** (budget unrun) | M7 |
| M7/18bcd-q1 | **Question: can a dropped boundary row eat the cap's probe row?** `--max-count=cap+1` counts boundary rows (`trace/log.rs:60`, `:100`); `finish` compares the filtered count (`:171`), so uneven boundaries could read `SHALLOW_END` where `Capped` is true. Whether git produces that shape is unverified. Source: review m7-18bcd Q1 | **unverified** | M7 |
```
