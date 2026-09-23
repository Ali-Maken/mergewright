# Review — m6-conflicts 09: ops conflict set, finish summary, `ContinueWith`

Group review, not milestone. Range `2184642..2e43a0a` (fixed hash, read-only). Ticket
`.scratch/m6-conflicts/issues/09-ops-conflict-set-and-summary.md`. Reviewer: Opus, 2026-09-23.

**Verdict: pass. 0 Critical · 0 Major · 9 Minor (2 of them questions for the conductor).**

## Gates (run one at a time, workspace-wide, on `main` at `cd31d1d`; its crates are byte-identical to `2e43a0a`: `git diff --stat 2e43a0a HEAD` touches only `.scratch/`, `docs/`)

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo nextest run` | pass — 2701 run, 2701 passed, 1 skipped; 91.3 s including the incremental build; peak RSS 272 MB (`/usr/bin/time -l`) |
| `cargo test --doc` | pass |

The heavy tier, release perf run and fuzzing were skipped on the conductor's instruction; they run at the milestone review.

## Acceptance ticks — all 8 checked against their test bodies, 8 held

| Tick | Test read | Held? |
|---|---|---|
| merge lists resolved files | `conflict_set.rs:19` — `graft_stage` a.txt, `[b.txt, a.txt]`, `recorded` | yes |
| relaunch lists the same | `conflict_set.rs:32` + `conflict_fixtures.rs:114` (a new handle and new `read_facts` on each read) | yes |
| each kind | `conflict_set.rs:117` — 5 kinds listed, stash pop `[a.txt]`/`false`, plus the 4 extras named | yes |
| summary splits | `finish.rs:32` — a.txt Theirs, b.txt Combined, clean.txt from `git diff --cached --numstat`, into/from/message | yes |
| refuses unmerged / merge only | `finish.rs:155`, `finish.rs:166` | yes |
| edited message | `finish.rs:237` — hook refuses → `ResumeFailed{Merge}`, MERGE_MSG bytes and mtime unchanged; then `Committed`, `%B` == `git stripspace --strip-comments`, `HEAD^2` == feature, MERGE_MSG gone | yes, but the command assertion is weaker than the tick says (Minor 7) |
| continue unchanged | `finish.rs:302`; `git diff --stat 2184642..2e43a0a -- history_ops.rs chained_backup.rs shell_begun_merge.rs` is empty | yes |
| rebase refused before anything runs | `finish.rs:322` — refusal, words, `state_of`, every ref (backups included), still rebasing | yes |

## Builder's deviations (ticket NOTES) — each judged

1. **`-F -` (stdin) instead of `-F <scratch>/MSG`** — **acceptable.** It is the same `commit -F` route, and `MERGE_MSG` is never written (hard choice 3 holds). A hook failing before git reads stdin is safe: `graft-git/src/run.rs:458` treats `BrokenPipe` as "git's exit status decides", and `finish.rs:237` exercises it. With no temp file there is nothing to leave behind. The design note's interface line still says `<scratch>/MSG` (`docs/design-notes/m6-conflicts.md:542`); see Minor 9.
2. **Extra error variants** (`ConflictSetError::{Facts, Journal, Snapshot}`, `SummaryError::{Conflicts, Diff}`) — **acceptable.** Without them each failure would carry the wrong label, which rule 3 forbids. MAP.md records them.
3. **(b) and (c) read only while git has an operation in progress** — **acceptable.** A stale `MERGE_MSG` describes no operation now in progress. The reasoning is in the doc comment at `conflict/set.rs:67-69`, and `nothing_in_progress_is_an_empty_unrecorded_set` covers it.
4. **A pull's `from` is `main of <url>`** — **acceptable.** It is a fair reading of "kept whole", `a_shell_merge_names_its_branch_and_a_pull_its_source` pins it, and `into <branch>` is dropped because the summary names that branch itself.
5. (Scout contradiction) **`Resume` loses `Copy`, and the non-arm edit at `graft/src/update/banner/tests.rs:729`** — **acceptable.** It is test code only, it is declared in NOTES, and the non-`Copy` payload forces it. The two app arms (`banner.rs:464`, `runner.rs:650`) are the only behaviour-free edits in `graft`.

## Git-data safety of `ContinueWith`

Checked in `apply/resume.rs:165-217`:
- The kind check (`resume.rs:233-235`) runs before the journal lookup, the backup chain and git. The rebase test proves no ref moved.
- The unmerged check covers `ContinueWith` (`resume.rs:242`). git's own `commit` would refuse too.
- The backup is chained before git runs (`resume.rs:174`), the same as `Continue`.
- There is no `-a` and no pathspec, so only the index is committed. This matches what `merge --continue` does, since it runs `git commit` itself. The same hooks run on both paths.
- If a hook refuses, the entry stays `Pending` through `still_pending`, and a second attempt succeeds on the same entry (tested).
- `MERGE_MSG` is never written. The test proves it by bytes and mtime.

Residual risk: Minor 1 and Minor 2.

## Critical

None.

## Major

None.

## Minor

1. **`ContinueWith` loses user lines that start with `#`.** `apply/resume.rs:183` (`--cleanup=strip`). The summary's `message` has already had its comment lines removed (`finish.rs` → `merge_msg::without_comments`). If the user then types a line starting with the comment character, such as `#123 fixes login`, git silently deletes it from the commit. That is a small loss of the user's own text. The spec asked for `strip`, and git's editor flow behaves the same way, so this is not a build defect. **Question for the conductor:** should the finish ticket (a) send `--cleanup=whitespace`, since comments are already stripped, or (b) warn when a body line starts with the prefix? Whichever is chosen, record it in the finish-screen ticket.
2. **An empty message is not refused up front.** `apply/resume.rs:180-188`. If `ContinueWith { message }` is empty after stripping, git runs anyway and then exits with "Aborting commit due to empty commit message". That becomes `ResumeFailed`, carrying git's stderr as its message. It is safe, because the merge stays live and the entry `Pending`, but the words are git's and not Graft's, and no test covers it. Suggested fix: refuse before chaining a backup, using the existing `Refusal::EmptySubject` (`operation.rs`), and add a test.
3. **Question — for a multi-step Graft rebase, (c) returns conflicts from earlier, already-committed steps.** `conflict/set.rs:153` walks the whole backup chain of the live entry, and the same entry stays live across steps. `apply/fold.rs:108` chains each Stage to it, and `apply/resume.rs:174` chains it again before each `--continue`. So step 2's set also holds the files step 1 conflicted on. `MERGE_MSG` (b) is per step. This follows the spec, which says "any `IndexState::Unmerged` on the live entry's backup chain", and only `operation_conflicts` sees it, because `finish_summary` is merge-only. It is **unverified**: I read the code path, but I did not run a two-step rebase. The file-list ticket will then show step-1 files as `Done` during step 2. Conductor: decide whether a rebase's set should be bounded to the current step, for example by the chain links newer than the last resume, before `fix-resolver-file-list` builds on it.
4. **A catch-all error arm would mislabel a future failure as "stopped".** `conflict/set.rs:147` (`_ => ConflictSetError::Facts(FactsError::Stopped)`). This is true today, because `live_entry` (`apply/resume.rs:79-125`) returns only `Stopped` or `Facts`. But once `live_entry` gains another error variant, that error will surface silently as "stopped", which is the rule-3 pattern waiting to happen. Suggested fix: match `ApplyError::Stopped` explicitly and let the compiler flag new variants, or have `live_entry` return `FactsError` directly.
5. **Two ops-layer reads do unbounded git work with no stop.** `finish.rs:141` (`let stop = &()`) and `conflict/set.rs:153` run one `ls-tree`/`ls-files` per chain link, plus a full staged diff with numstat. The signatures follow the spec, which gave no `Stop` parameter, so this is not a defect. But the doc comment's reason ("every read here is one short `git` call") is inaccurate, because the chain walk is O(steps). The app must call both from a job, never on the UI thread (the design lists `Command::ReadConflictSet` / `ReadFinishSummary`, which is correct). Suggested fix: add `stop: &dyn Stop` when the app ticket wires them, or correct the comment.
6. **The `by_git` mapping is untested.** `finish.rs:186-206`. `Renamed { from }`, `TypeChanged`, `Copied → Added` and binary → 0/0 are mapped without a test. The builder states this in NOTES under "Not verified", so it is not a false tick. Also, `Unmerged | Untracked → Modified` (`finish.rs:198`) turns an impossible case into a plausible value instead of an error. The comment explains why it cannot happen; acceptable, but noted. Suggested fix: add one fixture where the merge brings in a rename and a binary file.
7. **The tick names an exact command, but the test checks only a prefix.** `tests/it/finish.rs:284` asserts `starts_with("git commit --cleanup=strip -F")`, while the tick says the recorded command is `git commit --cleanup=strip -F -`. Suggested fix: `assert_eq!(…last(), Some("git commit --cleanup=strip -F -"))`.
8. **Readability** (PROMPT.md 6.2):
   - `apply/resume.rs:177` has a closure named `flag` whose parameter is also named `flag`.
   - `close_resume(…, aborting: bool, …)` (`resume.rs:285`) is a bare boolean flag; an `enum Ending { Abort, Commit }` would read better at the call site.
   - `finish.rs:276` `records(stdout, pick)` takes a closure that returns a field *index*, which is clever where two plain parsers (`ls-files -s`, `ls-tree`) would be clearer.
   - `results` (`finish.rs:212`) is a vague name; `resolved_as` would say what it computes.
   - `path_of` is duplicated verbatim in `conflict/merge_msg.rs:135` and `finish.rs:323`; move it into one crate-private helper.
9. **Docs lag the code.**
   - `crates/graft-ops/MAP.md:145` still says ``Resume` (enum) — `Continue`, `Skip`, `Abort``, while the new row just above it adds `ContinueWith`. Merge the two rows.
   - `docs/design-notes/m6-conflicts.md:542` still says `-F <scratch>/MSG`. If the conductor accepts deviation 1, update the note so that P4's `FinishMerge` ticket does not build the temp-file variant.
   - `core.commentString`, the git ≥ 2.45 multi-character form, is not read (`merge_msg.rs:43-58`, NOTES). If that is set, (b) misses its list and `recorded` falls back to (c) alone. It is a real but rare gap, and it belongs in `KNOWN_GAPS.md` as **unverified**, not broken. The conductor files it.

## Not checked

- Display sanitisation of `into`/`from`/`message`/paths: they are raw bytes here by design, and the design's `FinishView` takes `SanitizedText`. The finish-screen ticket must sanitise them; nothing in this group draws them.
- Heavy tier, release perf, fuzz: deferred to the milestone review, per the conductor.
- Behaviour from the app: there is no caller yet, as NOTES says.
