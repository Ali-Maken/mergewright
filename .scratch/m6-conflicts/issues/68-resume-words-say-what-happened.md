# resume-words-say-what-happened: an abort reads `aborted: … ✓`, a final skip says it skipped

Status: draft
From: `docs/reviews/m6-usability-final.md` N2 (:112-123).
Build with: (conductor, after the scout)
Scout: `.scratch/m6-conflicts/scouts/resume-words-say-what-happened.md` — brief below.
Needs: none. Shares `view/status.rs` reading with `a-box-owns-the-bottom-row` (same group B; either order, rebase on the
first).
Crates: graft (`update/banner.rs` `resumed`/`resumed_done`, tests, walks).
Group: **B** (the bottom row tells the truth), with `a-box-owns-the-bottom-row`.

## Read first
- `docs/reviews/m6-usability-final.md` N2 (:112-123).
- `crates/graft-ops/MAP.md` :165-167 (`Resume`, `resume` closes **that same entry**: `Committed` / `Aborted { reason }` /
  `Conflicted`; the shell-merge route to `AbortMerge`), :51 (`AbortMergeParams`, its `describe`).
- `crates/graft-core/MAP.md` :51 (`EntryStatus`), :56 (`JournalEntry`).
- `crates/graft/MAP.md` :306-308 (`continue_pressed`, `resume`, `resumed`), :676 (`done_line`).
- `.scratch/m6-conflicts/usability-final-plan.md`.

## The cause (rule 7, lines at `1b1995cc`; the scout re-reads them on main)
- `close_resume` returns the operation's own entry for an abort; only its status changes, to `Aborted`
  (`graft-ops/src/apply/resume.rs:318-374`).
- `resumed_done` writes `Status::done(applied.entry.description)` for every resume, abort included
  (`graft/src/update/banner.rs:762-770`). Observed: `a y` on a relaunched `rebase main onto fix` stop aborted it, and the
  line read `rebase main onto fix ✓`.
- So the words are chosen without looking at what happened. The fix reads what happened: the entry's status, and for a
  skip the `Resume` that was sent (a final skip closes `Committed`, like a continue).

## Decided here: the words (one function, one table)
| Resume sent | Entry closed as | Status line |
|---|---|---|
| `Continue` / `ContinueWith` | `Committed` | `<description> ✓` (unchanged) |
| `Abort`, operation begun in Graft | `Aborted { .. }` | `aborted: <description> ✓` |
| `Abort`, merge begun in the shell (`AbortMerge` entry) | `Committed` | `<description> ✓` — it already reads `abort merge of <name>`; no double word |
| `Skip` that finished the operation | `Committed` | `<description> ✓ · last commit skipped` |

- The `✓` stays on an abort: the abort he asked for succeeded, and a status must not read as an error when nothing is
  wrong (CLAUDE.md). The prefix is what makes it true.
- A skip or continue that stops again (`Conflicted`) and a failed resume (`Failed`) are untouched.
- Landing is unchanged: `resumed`'s `Done` arm (graph focused, `land_on_head`).

## Scout brief (≤1 page, code not intent)
1. Re-read `banner.rs:740-780` on main: `resumed`, `resumed_done`; what they get (`JobId`, `jobs::operate::Operated`) and
   whether the `Resume` that was sent is known there (kept on `AppState` when `resume` queues `Command::Resume`, or
   carried back in the answer). Name the field or say there is none.
2. What `applied.entry` is on each arm: the op's own entry (Graft-begun) vs the `AbortMerge`/`FinishMerge` entry
   (shell-begun). Its `kind` and `status` on each.
3. What status a final `--skip` closes with (expect `Committed`): an existing ops test that shows it, or say none.
4. Every other place that words a closed resume (the resolver's notice row via `ReturnTo::Resolver`, `undo::land_after_undo`,
   the journal browser's row for an `Aborted` entry): does any of them also say `✓` for an abort?
5. Captures and TESTING.md lines that show an abort's line: `grep -rn "✓" docs/shots/m6 | grep -i "abort\|rebase .* onto"`.

## Scope
**In:** `resumed_words` and its one caller; carrying the `Resume` to the answer if scout Q1 finds it absent; tests and
the capture.
**Out:** `graft-ops` (the entry and its status are right); the journal browser's own wording (report scout Q4's answer;
fix only the status line); counting several skips.

## Signatures
```rust
// graft, update/banner.rs — the one wording of a resume that closed its entry. `resumed_done` calls it; nothing else
// builds these words.
pub(crate) fn resumed_words(
    how: &graft_ops::apply::Resume,
    entry: &graft_core::journal::JournalEntry,
) -> String;
// If scout Q1 finds the sent `Resume` unknown at the answer: the in-flight record `resume` writes gains it, e.g.
// `resuming: Option<graft_ops::apply::Resume>` on the state `resume` already sets. Name the real place in NOTES.
```

## The failing test first
`update::banner` tests: `aborting_a_rebase_does_not_read_as_done` — a Graft-begun rebase stop, `a y`, the answer
`Done` with the entry `Aborted`: the status line reads `aborted: rebase main onto fix ✓`. Red today
(`rebase main onto fix ✓`).

## Touches (each entry, and how the builder checks it)
- **`resumed_done`**, its only construction of the words. Check: `grep -n "Status::done(applied" crates/graft/src` → the one
  call goes through `resumed_words`.
- **The shell-merge abort** (m6-conflicts/57): words unchanged. Check: its table row, and
  `resolver::shell_begun::a_on_the_banner_aborts_a_shell_merge` green with its capture byte-identical.
- **Other wordings of a closed resume** (scout Q4): each named in NOTES with whether it says `✓` for an abort; only the
  status line is changed here.
- **Captures** showing an abort's or final skip's line (scout Q5): each regenerated by its test, or `git diff --exit-code`.
- **TESTING.md** lines with the old line: listed in NOTES for the acceptance ticket.
- **Journal/undo:** none; the entry is not touched. **Keymap:** none.
- **MAP.md:** graft `update::banner` (`resumed_words`; the `Resume` on the in-flight record if added).

## Acceptance (name the verifying test at tick time)
- [ ] `aborting_a_rebase_does_not_read_as_done`, red then green.
- [ ] `the_resume_words_say_what_happened`: the table above, one row each, including the shell-merge abort row.
- [ ] Real repo (`tests/it`): `a_relaunched_rebase_abort_reads_aborted` — the review's path: relaunch into a stopped
      rebase, `a y`; `git status` shows no rebase, and the line reads `aborted: rebase main onto fix ✓`.
- [ ] Real repo: `a_final_skip_says_it_skipped` — a two-commit rebase stopping on the last; `s y`: the line ends
      `· last commit skipped`, and `git log` lacks the skipped commit.
- [ ] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-abort-reads-aborted-120x40.txt`, from the
      relaunch test.
  - **What right looks like:** the bottom row reads `aborted: rebase main onto fix ✓`; no `REBASING` in the header; the
    cursor on `main`'s tip.
- [ ] MAP.md updated (doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Small: one function, one caller, four tests, one capture. One context (shared with the group's other spec).
