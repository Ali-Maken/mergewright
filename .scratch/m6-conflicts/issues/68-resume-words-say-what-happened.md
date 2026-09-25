# resume-words-say-what-happened: an abort reads `aborted: … ✓`, a final skip says it skipped

Status: done (2026-09-25)
From: `docs/reviews/m6-usability-final.md` N2 (:112-123).
Build with: **Opus**. Alone this is Sonnet-sized (one function, one state field), but it shares group B's one context with
`69-a-box-owns-the-bottom-row`, which needs Opus (see there).
Scout: `.scratch/m6-conflicts/scouts/resume-words-say-what-happened.md` (at `dbf4fcf2`).
Needs: none. **Order:** group B builds after ticket 67 lands (69 needs 67; this rides in the same group). Either spec
first inside the group.
Crates: graft only (`update/banner.rs` `resume`/`resumed`/`resumed_done`, `update/state.rs`, tests, `tests/it`).
Group: **B** (the bottom row tells the truth), with `69-a-box-owns-the-bottom-row`.

## Read first
- `docs/reviews/m6-usability-final.md` N2 (:112-123).
- This ticket's scout (whole, ≤1 page).
- `crates/graft-ops/MAP.md` :165-167 (`Resume`, `resume` closes **that same entry**; the shell-merge route to
  `AbortMerge`), :170 (`Applied`), :171 (`ApplyError::Git` / `ResumeFailed`).
- `crates/graft-core/MAP.md` :50 (`OperationKind::AbortMerge`), :51 (`EntryStatus`), :56 (`JournalEntry`).
- `crates/graft/MAP.md` :189 (`Status`: the tick is the renderer's, `what` never holds `✓`), :306-309 (`abort_pressed`,
  `continue_pressed`, `resume`, `resumed`), :677 (`done_line`), :746-747 (`run_resume`, `Operated`).

## The cause (rule 7, lines at `dbf4fcf2`, from the scout)
- `resumed_done` writes `Status::done(applied.entry.description)` for every resume (`banner.rs:769`), abort included.
- The `Resume` that was sent is not known at the answer: `AppState::operating` is `Option<JobId>` only (`state.rs:86`);
  its only trace, the `"abort rebase"` string in `remote.running` (`banner.rs:597-601`), is cleared at `:696` before
  `resumed_done` runs.
- The entry's status alone cannot carry the words: `close_resume` also closes a **failed** resume `Aborted
  { reason_for(error) }` (`graft-ops/src/apply/resume.rs:345`). So "`Aborted` means an abort" is false; the words key on
  the `Resume` sent **and** the status.

## Decided here
**1. Keep the sent `Resume` until its answer.** New field beside `operating`, set where `resume` sets `operating`
(`banner.rs:595`), taken by `resumed` on **every** arm where it clears `operating` (`:695`). `resumed`'s stale-answer
guard (`operating == Some(id)`, `:692`) also requires `resuming`'s id to match; one guard, one drop.

**2. The words, one function, one table.** It returns a `Status` (not a string): the tick is the renderer's (MAP :189),
and a row that is not a success must not be a `Done`.

| Resume sent | Entry | Status built | Drawn |
|---|---|---|---|
| `Continue` / `ContinueWith` | `Committed` | `Status::done(d)` | `<d> ✓` (unchanged) |
| `Skip` (finished the operation) | `Committed` | `Status::done(format!("{d}, last commit skipped"))` | `<d>, last commit skipped ✓` |
| `Abort`, entry kind `AbortMerge` (shell-begun) | `Committed` | `Status::done(d)` | `abort merge of <name> ✓` (unchanged; no double word) |
| `Abort`, any other kind (Graft-begun) | `Aborted { .. }` | `Status::done(format!("aborted: {d}"))` | `aborted: <d> ✓` |
| `Continue` / `ContinueWith` / `Skip` | `Aborted { reason }` or `Pending` | `Status::sticky(format!("{d} did not finish: {reason}"))` (`Pending` → reason `still open`) | no `✓` |

- Skip wording moved from the first draft's `<d> ✓ · last commit skipped`: `Done`'s `keys` slot is for key hints, and
  `what` never holds `✓` (MAP :189). The words are the same; the tick lands at the end.
- The last row is not expected on a `Done` answer (see the pin below). It exists so the match is total and never says `✓`
  over a failure; it is not a new path.
- `✓` stays on an abort: the abort he asked for succeeded; the prefix makes the line true.
- `Conflicted` again and `Failed` answers: untouched. Landing unchanged (`Done` → graph focused, `land_on_head`).
- The `Incomplete` arm (`banner.rs:709-712`) words `description` the same way: it takes its `what` from
  `resumed_words`; its own after-state words stay.

**3. Out, filed by the conductor (not fixed here):** the journal browser words a user's abort `failed: aborted`
(`view/journal.rs:190,218` at `dbf4fcf2`). Same H1 fault, different screen; the browser has no `Resume` to read, and
telling an abort's reason from a failure's needs the ops constant. KNOWN_GAPS row, as a finding with those lines.

## Scope
**In:** the `resuming` field; `resumed_words` and its callers (`resumed_done`, the `Incomplete` arm); the pins and
tests below; one capture.
**Out:** `graft-ops`; the journal browser (above); counting several skips; the resolver's notice row (a resume from the
resolver closes it first, `continue_pressed`, so its answer comes through `resumed` too; checked in Touches).

## Signatures
```rust
// graft, update/state.rs — beside `operating`. Set only by `banner::resume`; taken only by `banner::resumed`.
pub(crate) resuming: Option<(JobId, graft_ops::apply::Resume)>,

// graft, update/banner.rs — the one wording of a resume that closed its entry. Nothing else builds these words.
pub(crate) fn resumed_words(
    how: &graft_ops::apply::Resume,
    entry: &graft_core::journal::JournalEntry,
) -> Status;
```

## The failing test first
`update::banner` tests: `aborting_a_rebase_does_not_read_as_done`: a Graft-begun rebase stop, `a y`, the answer `Done`
with the entry `Aborted`: the status is `Done` and draws `aborted: rebase main onto fix ✓`. Red today
(`rebase main onto fix ✓`).

## Touches (each entry, and how the builder checks it)
- **`resumed_done` and the `Incomplete` arm**: the only builders of a resume's closing words. Check:
  `grep -n "entry.description" crates/graft/src/update/banner.rs`: each hit in `resumed*` goes through `resumed_words`.
- **`operating`'s other clearers**: `grep -rn "operating = None\|operating.take()" crates/graft/src`: any that can clear a
  resume's job (a cancel: resume is unstoppable, MAP :308) also clears `resuming`, or NOTES says why it cannot be a
  resume. No stale `resuming` survives into a later write's answer: `a_stale_resume_answer_is_dropped`.
- **Resolver path** (`ReturnTo::Resolver`, `banner.rs:436,443,463`): a `c` from the resolver reaches `resumed`. (The
  resolver has no `a`: that part of this line dropped by the conductor, 2026-09-25; see NOTES.) Check:
  `the_resolver_abort_reads_aborted` (the abort reached from the resolver's screen through the banner: `esc`, `⇥`,
  `a`, `y`; the line reads `aborted: …`).
- **Shell-merge abort** (m6-conflicts/57): `resolver::shell_begun::a_on_the_banner_aborts_a_shell_merge` green;
  `docs/shots/m6/fix-shell-merge-abort-and-undo-120x40.txt` (:83, :124) byte-identical (`git diff --exit-code`);
  TESTING.md:2336 unchanged.
- **Continue captures**: `fix-landing-after-continue-rebase-{120x40,187x50}` byte-identical (continue row unchanged).
- **`undo::land_after_undo`** (not read by the scout): pinned below, not changed.
- **TESTING.md**: lines naming an abort's or final skip's line listed in NOTES for the acceptance ticket.
- **Journal/undo:** entries untouched. **Keymap:** none. **graft-ui:** none.
- **MAP.md:** graft `update::banner` (`resumed_words`), `AppState::resuming`.

## Acceptance (name the verifying test at tick time)
- [x] `aborting_a_rebase_does_not_read_as_done` (`update::banner::tests::resume_words`), red then green — red observed
      (`left: Done { what: "rebase main onto fix" }`), green after `resumed_words`.
- [x] `the_resume_words_say_what_happened`: eight rows (continue, continue-with, final skip, shell-merge `AbortMerge`
      abort, Graft-begun abort, continue over `Aborted`, skip over `Aborted`, continue over `Pending`); each non-`Done`
      row drawn at 120x40 has no `✓`.
- [x] **Pin:** `a_resume_git_fails_answers_failed` (`jobs::operate::tests`): `answer(Err(ApplyError::Git { entry: Aborted,
      .. }))` is `Operated::Failed` (`jobs/operate.rs` `answer`, its `Err(error) =>` arm). It does not reach `Done`.
- [x] `a_stale_resume_answer_is_dropped` (another job's id; and `operating`/`resuming` disagreeing: one guard, one drop).
      Plus `a_resume_that_dies_takes_its_resuming_with_it` (the `write_job_ended` clearer).
- [x] `the_resolver_abort_reads_aborted` (`update::resolver::tests::abort_words`). See NOTES: the resolver has no `a`.
- [x] Real repo: `a_relaunched_rebase_abort_reads_aborted` (`tests/it/conflict_interim/resume_words.rs`): no
      `.git/rebase-merge`, `git status` names no rebase, the footer reads `aborted: rebase feature onto main ✓`, cursor
      on HEAD. (Fixture's description, not the spec's illustrative `main onto fix`.)
- [x] Real repo: `a_final_skip_says_it_skipped` (`tests/it/resolver/resume_words.rs`): Graft-begun `TwoStopRebase` at
      step 2, `Esc`, `⇥`, `s y`: line `<d>, last commit skipped ✓`; `git log` lacks `REBASE_HEAD`'s subject; the
      journal's rebase entry is `Committed` (scout Q3 now confirmed by a test).
- [x] **Pin:** `abort_then_u_words` (`tests/it/conflict_interim/resume_words.rs`): reads `nothing to undo`; see NOTES.
      Not a finding.
- [x] Capture: `docs/shots/m6/fix-abort-reads-aborted-120x40.txt`, from `a_relaunched_rebase_abort_reads_aborted`
      (`check_shot`). Caption in `m6-contact-sheet-captions.tsv` (`contact_sheet_captions_cover_every_m6_capture`).
      Colour **unverified**.
  - **What right looks like (as built):** the graph footer reads `aborted: rebase feature onto main ✓`; no `REBASING`
    in the header; the cursor on HEAD, which after an abort is `feature`'s tip, not `main`'s (see NOTES).
- [x] NOTES: the journal-browser finding, below.
- [x] MAP.md updated (`resumed`, `resumed_words`, `AppState::resuming`; doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Small: one field, one function, two callers, eight tests, one capture. One context shared with 69.

## NOTES (builder, 2026-09-25)
- Scout used: `.scratch/m6-conflicts/scouts/resume-words-say-what-happened.md`, with the amended spec.
- **Cause fixed.** `resumed_done` worded every close `Status::done(description)` because the `Resume` sent was gone by
  the answer. `AppState::resuming` now keeps it; `banner::resumed_words` keys on it and the entry's status.
- **Shell-merge row keyed on status:** `(Committed, Abort)` → `<d>` covers the `AbortMerge` entry (the only abort that
  closes `Committed`) with no second word. An `Aborted` entry on a `Done` answer to an abort is always the user's
  abort: a failed abort is `Err(ApplyError::Git)` → `Failed` (pinned).
- **Cursor after the abort:** on HEAD, which is `feature`'s tip (`Change every file on feature`), not `main`'s: an abort
  puts HEAD back on the branch being rebased. The spec's "main's tip" was wrong for this fixture; the caption says so.
- **Resolver path (spec premise corrected, rule 7):** the resolver has **no `a`**. Keymap §2.6's table names none; with
  the resolver open `a` is claimed and sends no resume (asserted in the test). The only resolver → `resumed` route is
  `c` (`ReturnTo::Resolver` in `banner.rs` `continue_pressed`), whose words are unchanged. Aborting from the resolver is
  `Esc`, `⇥`, `a`, `y` through the banner; the test drives that and reads `aborted: …`.
- **`operating`'s other clearers:** `operate::write_job_ended` (a resume job that panicked, or was cancelled by
  shutdown; `job_failed`/`job_cancelled` both reach it) now also clears `resuming` when the ids match.
  `remote::cancelled` (`remote/in_flight.rs:85`) needs `remote.cancelling == id`, which `Esc` never sets for a resume
  (`Stoppable::No` in `banner::resume`). `undo::undone`/`reverted_to` and `operate::operated`: not resume jobs; a later
  resume overwrites `resuming`, and the guard needs both ids, so a stale one never words a later answer.
- **`abort_then_u_words` (observed):** after the relaunched abort, `u` reads `nothing to undo ✓` (`Status::done`),
  `in_progress` stays `None`, no `.git/rebase-merge`. It does not word the abort as done: not a finding. Question: the
  relaunch fixture's entry has no backup, so this does not show what `u` says after aborting a rebase Graft began in
  this run.
- **Journal-browser finding (for KNOWN_GAPS, not fixed):** a user's abort reads `failed: aborted` in the journal browser.
  `crates/graft/src/view/journal.rs:190` (row: `EntryStatus::Aborted { reason } => format!("failed: {}", …)`) and
  `:218` (detail: `Some(format!("failed: {reason}"))`), at `6e1d42bc`. The browser has no `Resume`; telling an
  abort's reason (`"aborted"`, `graft_ops::apply::resume::close_resume`) from a failure's needs the ops constant.
- **Touches:** `resumed_done` and the `Incomplete` arm both go through `closing` (`grep -n "entry.description"
  banner.rs`: one hit, inside `closing`). Resolver path: above. Shell-merge abort:
  `resolver::shell_begun::a_on_the_banner_aborts_a_shell_merge` green; `fix-shell-merge-abort-and-undo-120x40.txt` and
  `fix-landing-after-continue-rebase-*` unchanged (`git status docs/shots` lists only the new capture). TESTING.md: no
  line names a Graft-begun abort's or a final skip's closing line (`grep -n abort TESTING.md`: only :2336, the shell
  merge, unchanged); nothing for the acceptance ticket to reword. Existing banner tests that answer a resume now set
  `resuming` beside `operating` (8 sites in `banner/tests.rs`); `every_field_spelled_out` gains the field.
  Journal/undo, keymap, graft-ui: none.
- No compaction.
