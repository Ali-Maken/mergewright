# 18b git-reads-review-fixes — review 18/19/27/10a's Majors and mutation gaps

Status: done (see NOTES: Major 3 over budget, one new finding)
Blocked by: none (18, 19, 27, 10a merged)
Crates: graft-git only (`blame.rs`, `trace/log.rs`, `history_reads/unreached.rs`, `switch_refs.rs`, their tests),
plus a perf script and `docs/perf/trace-readers.txt`. Another crate needed → commit at the seam, report `blocked:
needs <crate>` with the signature.
Build with: Opus (git reads with oracles against real git; one perf run).
Scout: none. The review cites every site with file:line at `62ace7b0`; re-read each site before you edit it.
Split by: conductor, 2026-09-26, from `docs/reviews/m7-18-19-27-git-reads.md`. No new decision.

## Scope
Each fix is the one the review states, at the cause. Read the review's section for each.
- **Major 1** (blame ignores `truncated`): return an error, as the siblings do (`trace/log.rs:123`). A test through a
  lowered cap or a cut porcelain sample, red first.
- **Major 2** (`--not` on `--stdin`; floor git 2.38): **verify first**. Try a git 2.38–2.41 (a brew-extracted or
  source-built older git under `$TMPDIR`, never installed system-wide). Whatever it shows, send negatives as `^<rev>`
  lines, which every version takes. State in NOTES what you verified on which version, or that you could not.
- **Major 3** (blame perf unmeasured): a long file with interleaved history, measured from a script into
  `docs/perf/trace-readers.txt` as `blame-longfile p95=…`. Read summary lines only. Over 1 000 ms → report it, don't
  tune.
- **Major 4** (files over 400 lines): split `blame.rs` (porcelain parser → `blame/porcelain.rs`, delete its private
  `unquote`, Minor 13) and `trace/log.rs` (the `--follow` reader → `trace/follow.rs`). No behaviour change.
- **Mutation gaps** (§Mutation check): a test that fails under each surviving mutation: M1 and M4 (10a's two error
  rules), M15 (blame without `-M`), M17 (shallow end reads as `Start`), M23 (working-tree binary check).
- **10a `@{-1}` with a same-named tag** reads as detached (the review saw it on real git; see its Minor). Fix at the
  cause, with an oracle test, red first.

- **Minor 4** (enclosing function: output not checked for truncation; a capped diff reported as `Stopped`): the
  same rule-3 class as Major 1; fix it with it, and a test.

Not in scope: other Minors (they go to KNOWN_GAPS).

## Acceptance
- [x] Major 1: `blame::tests::a_blame_cut_at_the_output_cap_is_an_error_not_a_short_file` — red first (panicked
  at `src/blame/tests.rs:207`, the cut read as `Ok`), green after `read_porcelain` checks `truncated`.
- [x] Major 2: verified on git **2.38.5** (source-built under `$TMPDIR`, not installed) and 2.54: before the fix all
  13 `unreached::*` tests failed on 2.38.5 with `fatal: options not supported in --stdin mode`; after (`^HEAD`,
  `^<stash entry>` lines) all 13 pass on both. The scan finding the same commits: `tests/it/unreached.rs`'s oracle
  tests, e.g. `unreached::lost_ancestors_fold_into_one_run_with_under`, `unreached::a_commit_only_a_stash_holds_is_held`,
  `unreached::an_older_stash_entry_holds_too`, `unreached::a_detached_head_holds`.
- [x] Major 3: `blame-longfile p95=1481.4` in `docs/perf/trace-readers.txt`, written by
  `scripts/m7-perf-blame-longfile.sh` (example `crates/graft-git/examples/blame_longfile.rs`). **Over 1 000 ms:
  reported, not tuned** — see NOTES.
- [x] Major 4: `blame.rs` 359, `blame/porcelain.rs` 192 (+ `blame/ignore_revs.rs` 55, moved to get `blame.rs`
  under 400), `trace/log.rs` 323, `trace/follow.rs` 134; blame's private `unquote` and log's `find` deleted for
  `diff`'s. No behaviour change: crate suite 403/403 right after the move (commit 770b93a5). **Not** "no file in
  `src` over 400": 13 older files are (NOTES).
- [x] M1 `switch_refs::tests::a_git_failure_reading_the_previous_checkout_is_an_error`; M4
  `switch_refs::tests::a_record_of_six_fields_is_an_error`; M15 `blame::a_block_moved_within_the_file_keeps_its_author`;
  M17 `trace::tests::a_log_ending_on_a_change_not_an_add_is_not_a_start`; M23
  `blame::working_tree_binary_is_the_file_on_disk`. Each seen red with its mutation applied, and green reverted.
- [x] `@{-1}` with a same-named tag: `switch_refs::a_previous_branch_sharing_a_tags_name_is_still_the_branch` (red
  first: `None`; oracle is git's own `switch -`), plus `switch_refs::a_deleted_previous_with_a_same_named_tag_is_none`
  (red first: read `Some(refs/tags/t)`). Both pass on 2.38.5 and 2.54.
- [x] Minor 4: `trace::tests::enclosing_function_refuses_output_cut_at_the_cap` — red first (`src/trace/tests.rs:190`).
- [x] Gates workspace-wide: fmt ok, clippy `--all-targets -D warnings` ok, nextest 3808/3808 (1 skipped, 445 s
  under load), `test --doc` ok.

## Touches
MAP.md (graft-git) lines for moved code; every caller of `blame`, `unreached`, `switch_refs` (grep). Name how each
was checked.

## NOTES (builder, 2026-09-26)
No scout: the review cites every site with file:line; each was re-read before editing. No compaction.

**Touches, and how checked:**
- Callers (grep of `graft_git::blame|trace|switch_refs|unreached` outside the crate): `unreached` is called by
  `graft-ops/src/journal_list.rs:218` — signature unchanged, workspace nextest covers it. `blame`, `trace`,
  `switch_refs`: no caller outside `graft-git` (tests and examples only). No public signature changed.
- `diff` module: `unquote`/`find` now re-exported `pub(crate)` from `diff::unified` (no public change).
- MAP.md: blame/trace file lines, `blame`'s cap rule, `enclosing_function`'s cap rule, `switch_refs`' three
  commands, `unreached`'s `^<rev>` stdin.
- Fuzz targets (`blame_porcelain`, `line_log`, `follow_log`): same public fns, re-exported at the same paths; not
  re-fuzzed. Blame's quoted paths now go through `diff`'s `unquote`, which reads `\NNN` the same for anything git
  writes (both unquote tests and the 2.38 quoted-path samples pass).

**Major 3 — over budget, not tuned (ask).** `blame-longfile p95=1481.4` ms, release, 10 runs, git 2.54: a
synthetic 5 000-line file, 500 commits each rewriting ~100 lines (one line in 50, a different residue each commit),
every 10th line untouched since the first commit so blame walks all 500. Default options (`-M -w`). Machine load
during the run was not controlled (other builders may have been running). Blame runs async, but 1.5 s is what he
waits before the blame screen fills on such a file. Not measured: `-M` / `-w` off, or a real file (git.git `diff.c`).

**New finding (behaviour, not fixed — needs a decision).** On a real shallow clone the trace does **not** reach the
"shallow" arm: `git clone --depth 2` of the session fixture, then `line_history` on 18–29 ended as
`Start { added_by: <Priya's commit> }` — the shallow boundary, which git diffs against nothing, so its hunk is all
adds. Tom added those lines. The row comes from `trace/log.rs:156` (`Some((row, true)) => Start`); the `(_, false)`
arm at `:157` that says "a shallow clone?" is not what a shallow clone produces. Observed with a test written for
Minor 6 (red; removed, since fixing it is a new decision). Likely fix: read `<common_dir>/shallow` and turn a `Start`
at a listed boundary into `GitStops`; the same applies to `file_history` (the boundary's raw status is `A`).
M17's test is therefore a unit test on `finish` with a log ending on a change, which is the input that arm handles.

**M1's test.** The review's suggested trigger (a corrupt `HEAD` reflog line) does not make `rev-parse @{-1}` fail:
on 2.38.5 and 2.54 a garbage line is skipped, and an unreadable reflog or one that is a directory gives exit 1,
i.e. "no previous" (so an unreadable reflog reads as no previous — unverified whether that ever matters). No
repository state found where `for-each-ref` succeeds and `rev-parse @{-1}` exits non-0/1, so M1 is killed by a unit
test on the exit-code rule (`has_previous`), not by real git.

**`@{-1}` design.** `rev-parse --verify --quiet @{-1}` still decides "is there a previous" (exit 1 = none, else
Err); the name comes from `check-ref-format --branch @{-1}` (what `switch -` uses), kept only if
`refs/heads/<name>` is listed. `check-ref-format` alone can't tell "no previous" from a failure (both exit 128), so
it was not used alone. Costs one more spawn when a previous exists. The `None`-code arm now says `Unparsable`
(output cap) instead of `Signalled`: `run_raw` gives no code only at the cap (`run.rs:430`, stop is `Err` at `:491`).

**Not done (out of scope, for KNOWN_GAPS):** Minor 14a (`switch_refs` does not check `for-each-ref`'s
`truncated`, `switch_refs.rs` `switch_refs`). Files in `crates/graft-git/src` still over 400 lines, none from this
group: `status.rs` 494, `run/exec.rs` 495, `history_reads.rs` 504, `diff/read.rs` 515, `diff/unified.rs` 521,
`run/askpass.rs` 526, `run.rs` 799, and tests files `status/tests.rs`, `repo/tests.rs`, `diff/patch/tests.rs`,
`diff/read/tests.rs`, `history/tests.rs`, `conflict/tests.rs`.

**2.38.5 build:** `git-2.38.5.tar.gz` from kernel.org, `make NO_GETTEXT NO_OPENSSL NO_CURL NO_EXPAT NO_PERL
NO_PYTHON NO_TCLTK`, prefix under `$TMPDIR`; tests run with it first on `PATH`. Not installed anywhere else.
