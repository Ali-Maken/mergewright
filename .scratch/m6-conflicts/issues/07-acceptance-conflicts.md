# acceptance-conflicts: the walk proves a real conflict is resolvable without leaving the app — merge, rebase-stops-twice, whole-file shapes, stash pop, base, start over, `$EDITOR` round trip, and `kill -9` mid-rebase then relaunch — with the TESTING.md checklist, the captures, the contact sheet, and the "Times I left" line

Status: partial — 4 of 6 boxes. The two open boxes wait on product fixes outside this ticket's crate set (F1, F2, F4, F6) and two spec questions (F3, F5); see Findings and Handoff below.
Blocked by: **01**, **02**, **03**, **04**, **05**, **06**, **08**.
Build with: Fable — the acceptance ticket (does not count against the Fable build budget): it decides what each scenario must prove from git's own state, and writes the "what right looks like" lines the user judges by.
Crates: graft (tests only: `crates/graft/tests/it/`), docs (`TESTING.md`, `docs/shots/`, `.scratch/m6-conflicts/pty-walk-m6.py`)
**Added by the conductor after group 08 was filed (2026-09-22):** one scenario,
`undo-a-resolution-in-a-stash-pop-conflict` — m5's stash-pop shape; `⏎` from the
pane; `o`, `⏎` → `all conflicts resolved`; `u` → the pane's `CONFLICTS` row is
back, `ls-files -u` lists the path, the file bytes equal the bytes before `⏎`;
`u` → the pop is undone (clean tree, stash still listed); `U` → the conflict is
back, markers on disk and `ls-files -u` non-empty. Truth from git, never from
`AppState`. **What right looks like:** `git ls-files -u` prints three lines for
`p` after the first `u`, none after the second, three again after `U`.
Also: `start-over-brings-the-markers-back` (already below) depends on group 08
for its "after `⏎`" step; its *what right looks like* line gains — `git ls-files
-u` lists the path with the **same stage oids** as before the take.

Needs: every other M6 ticket — `git-conflict-model`, `ops-conflict-operations`, `ui-conflict-resolver`, `app-conflict-resolver`, `app-resolver-whole-files-and-base`, `app-answers-beat-rereads`.

## Read first

- `docs/design-notes/m6-conflicts.md` — Promise (the four end states and the "Times I left" expectation), Decision 2 (b) (how the harness fakes `kill -9`), New UI (the mockups the captures are compared with).
- CLAUDE.md "The seven that pay" 1, 5, 6, 7; "Captures stay small"; ADR-0037 (contact sheet); ADR-0033 ("Times I left" is the only measurement).
- `crates/graft/tests/it/m5_walk/` **MAP** — `crates/graft/MAP.md` §tests::m5_walk (`Step`, `Wait`, `fixture.rs`, `truth.rs`, `every_scenario.rs`, the `--only/--list/--stems/--matrix/--capture` CLI, `no_secret_leaked`); `crates/graft/tests/it/coverage.rs::write_matrix` and `M5_STEMS`. **Reuse, never rebuild**: M6 scenarios are new entries in the same table with the same `Step`s; new `Step`s only where named below.
- TESTING.md "History walk" (lines ~1606–1643; scenarios 24–32 at ~1840–1886 — the shape to copy), "Times I left", "Hint bar".
- `.scratch/m5-history-and-ghost-preview/pty-walk-m5.py` (the pty capture script; copy to `pty-walk-m6.py`, do not extend the m5 one), `scripts/contact-sheet.sh`.
- `crates/graft-fixtures/MAP.md` §Shape (`StoppedRebase`, `StoppedMerge`, the m5 `conflicting`/`twice_conflicting` shapes and the stash-pop conflict shape), §capture (`GRAFT_UPDATE_SHOTS=1`, redaction).
- `docs/field-notes.md` (the eye-check findings the captures must not repeat: caption per width, never captured mid-load, `ghost` never drawn).

## Scope

In:

1. **Harness additions** (`m5_walk/` keeps its name; the table is one table): `Step::Kill` — the harness drops the app without shutdown while git's child is running (byte-identical on disk to SIGKILL; the m5 `Relaunch` step already reopens), asserting in `truth.rs` that `rebase-merge/` or `MERGE_HEAD` exists and the journal's last line is `Pending`; `Wait::ConflictOpened` (resolver overlay present with `file: Some`); `Wait::Resolved(path)` (`ls-files -u` no longer lists it). `truth.rs` gains `resolved_bytes(path)`: the working-tree bytes, compared with `git checkout --ours/--theirs` bytes computed on a scratch clone. `no_secret_leaked` unchanged.
2. **Scenarios** (numbered after m5's 49, `M6_STEMS` in `coverage.rs`), each ending in a truth check from git, never from `AppState`:
   - `merge-resolved-with-the-keys`: `StoppedMerge`, `⏎` on `both.txt`, `o`, `⏎` → resolved; `three.txt`: `o`, `n`, `t`, `n`, `b`, `⏎`; the bytes equal ours / theirs / ours+theirs per block; `c` → merge commit exists, `ls-files -u` empty, journal one entry, `u` reverts the merge.
   - `rebase-stops-twice-resolved-twice`: m5's `twice_conflicting`; resolve, `c`, resolve again, `c`; one journal entry; `u` restores the pre-rebase tip.
   - `every-whole-file-shape`: `StoppedMerge`; each of the seven `UnmergedHow` files resolved through the card's `o` or `t`; the tree afterwards matches a scratch clone resolved with the raw git commands from G2's table.
   - `crlf-and-no-final-newline-survive`: `crlf.txt` `t`, `nofinal.txt` `o` → bytes equal the git-computed sides (line endings intact).
   - `zdiff3-and-base-on-demand`: `zdiff3.txt` shows base without `B`; `three.txt` `B` shows three bases; `both-added.txt` `B` says `no base (added on both sides)`.
   - `start-over-brings-the-markers-back`: `o`, `⏎` (staged), reopen, `d`, `Start over` → `ls-files -u` lists it, `open() == 1`.
   - `editor-edits-are-never-clobbered`: `e` with `$EDITOR` set to a script that rewrites the file; back in the resolver `o` → `changed on disk · re-read` (if the fingerprint was stale) or the take applies to the re-read file; the edit's bytes survive.
   - `mergetool-hand-over-and-back`: `M` with `merge.tool` set to a script that resolves and exits 0 → the resolver closes with `resolved outside`; exit 1 → `mergetool exited 1 for <path>` and the resolver re-reads (closes KNOWN_GAPS 479's "headless mergetool path untested").
   - `stash-pop-conflict-resolves-without-a-banner`: m5's stash-pop shape; `⏎` from the pane; `o`, `⏎` → `all conflicts resolved`, no banner, journal row as G5 says.
   - `kill-9-mid-rebase-then-relaunch`: `twice_conflicting`, first stop, `o` on one block (file half-taken on disk), `Step::Kill`, `Relaunch` → banner focused, pane on `CONFLICTS`, `Sticky` `rebase still in progress from a previous run · ⏎ resolve  c continue  a abort`, `⏎` → the resolver shows the half-taken file (`1 of 2 open`), finish, `c`, `c` → done; one journal entry; **a second variant** `Relaunch` then `a` → tree back to before the rebase.
   - `u-right-after-continue-is-accepted` (G6 piece 2; or the scenario is dropped with G6's proof line).
   - `repush-lease-is-the-entrys-tip` (G6 piece 1): the m5 push shape, remote moved, `u` → `Re-push old tip` → refused by the lease in words.
   - `resolver-is-modal-and-esc-returns`: `u`/`Tab`/`/` swallowed; `Esc` → banner focus; from the pane → pane.
   - `tiny-frame-says-so`: 50×8 → the one line; `Esc` closes.
3. **TESTING.md "Conflicts walk"** section: the command (`cargo nextest run -p graft --test it m6 -- --list` style, per the m5 section), one entry per scenario with **what right looks like** stated concretely (`the result column reads "if is_expired() {" on line 40 and "git show :3:both.txt" gives the same bytes`, never "looks correct"), and the **"Times I left"** table gains its first rows: one per scenario where a reach for the shell would have happened in M5 (`mergetool`, `$EDITOR`, `git checkout --ours`, `git rm`) with the M6 outcome (`0 — resolved in the resolver`), and the residual reaches named as rows with expiry M7 (submodule, rename/rename, `conflict-marker-size`).
4. **Captures** under `docs/shots/m6/` via `pty-walk-m6.py` against the real binary: 120×40 and 80×24 of the resolver on `three.txt` (one open, one took theirs, one open with base shown), the whole-file card, the `2 conflicts still open` refusal, the relaunch `Sticky` (this one taken after `kill -SIGKILL` of the real process, the script asserts the pid died), the `all conflicts resolved · c continue` end state. Each capture's caption is the TESTING.md line; captured after `Wait::Ready`, never mid-load (field-notes). Each ≤ 200 KB; `GRAFT_UPDATE_SHOTS=1` regenerates. Rule 5: colour unverified, said in the sheet's header.
5. **Contact sheet** `docs/shots/m6/contact-sheet.txt` via `scripts/contact-sheet.sh m6-conflicts`, in ticket order.
6. **Context guard**: every capture and truth dump goes to a file; the builder reads summary lines and reports paths only; `--matrix` writes `docs/perf/`-style coverage to `.scratch/m6-conflicts/matrix.txt`.

Out: colour; the heavy tier (fuzz, release perf — the M6 review's); any product change (a failing scenario is filed in the ticket as a finding with its file:line, rule 7, not fixed here).

## Signatures

```rust
// crates/graft/tests/it/m5_walk/step.rs (additions)
pub enum Step { /* existing */ Kill }
pub enum Wait { /* existing */ ConflictOpened, Resolved(&'static str) }
// crates/graft/tests/it/m5_walk/truth.rs (additions)
pub fn resolved_bytes(repo: &Path, path: &str) -> Vec<u8>;
pub fn side_bytes(repo: &Path, path: &str, side: Side) -> Vec<u8>;   // git checkout --ours|--theirs on a scratch clone
pub fn last_journal_line_is_pending(repo: &Path) -> bool;
// crates/graft/tests/it/coverage.rs
pub const M6_STEMS: &[&str];
```

## Acceptance

- [ ] Every scenario above is in the walk table and passes with `cargo nextest run` (fast tier, under 60 s total for M6's rows on a laptop — measured, number in the report) — `every_scenario` covers them; `--list` prints them; `--matrix` shows every M6 stem hit. **Not met.** Held: all 19 are in the table (`m5_walk::every_scenario::every_scenario_has_a_test`, passing); `--list` prints them (the last 19 lines, run 2026-09-23); 19 rows in **5.3–5.4 s**, two runs (`docs/perf/m6-07.txt`). Not held: 4 rows red on purpose (F1–F5 below); `--matrix` hits 2 of 9 M6 stems (`.scratch/m6-conflicts/matrix.txt`), the rest blocked on F6.
- [x] `kill-9-mid-rebase-then-relaunch` proves from git: `rebase-merge/` present after `Kill`, the half-taken file's bytes on disk before relaunch contain one `<<<<<<<`, after the walk `ls-files -u` empty and `rev-parse HEAD` is the rebased tip — `truth.rs` assertions named in the scenario. — `m5_walk::every_scenario::kill_9_mid_rebase_then_relaunch` (passing): `Step::Kill` → `run.rs::killed` asserts `truth::operation_on_disk` and `truth::last_journal_line_is_pending`; `half_taken_on_disk` counts one `<<<<<<< ` in the `half taken` mark (taken before the kill) and `in_progress == "rebase"` after the relaunch; `expect` checks `ls-files -u` empty, `rev-parse HEAD` == rebased `feature`, `feature~2` == `main`, one `Committed` `Rebase` entry. The abort variant: `kill_9_mid_rebase_then_abort` (passing).
- [x] TESTING.md "Conflicts walk" has one entry per scenario with a concrete "what right looks like" line, and "Times I left" has its first rows with the M6 count per reach — reviewed by reading the section (the reviewer counts entries vs scenarios). — verified by reading: 19 numbered entries under §Conflicts walk "The scenarios", one per M6 name in `every_scenario` (in the same order); "Times I left" has 4 rows at `0` (mergetool, `$EDITOR`, `checkout --ours/--theirs`, `git rm`), 1 still-the-shell row (F5) and 3 residual rows expiring M7.
- [ ] `docs/shots/m6/` has the six captures at both widths, each ≤ 200 KB, redacted (`no_secret_leaked` + the m5 redaction of home/scratch/port), captioned; the contact sheet concatenates them in ticket order; the relaunch capture's script log shows the pid killed with SIGKILL — paths in the report. **Not met — blocked on F6.** Held: the whole-file card and the binary card at both widths (`docs/shots/m6/m6-02-*`, `m6-04-*`, ≤ 435 B each, through `graft-fixtures capture`, asserted green on a second run); captioned (`every_scenario::contact_sheet_captions_cover_every_m6_capture`, passing); sheet `docs/shots/m6/m6-contact-sheet.txt`. Not held: the resolver on `three.txt`, the refusal, the relaunch Sticky, `all conflicts resolved` — every capture pressing a key after the resolver opens races its opening read (F6), so none was committed. The SIGKILL path of `pty-walk-m6.py` is written but has not yet produced a capture (its pre-kill `o` is raced the same way).
- [x] The builder's report lists what it observed and what it could not (rule 1): mergetool with a real tool is **unverified** unless a script stood in; colour unverified (rule 5); the sheet is for the user's eye (rule 6). — this ticket's "Observed / not observed" below, and the hand-back report.
- [x] Any scenario that fails is a finding in this ticket with file:line (rule 7), and the walk row stays red — no product edit, no skipped test. — the 4 red rows in `docs/perf/m6-07.txt` (`merge_resolved_with_the_keys`, `start_over_brings_the_markers_back`, `undo_a_resolution_in_a_stash_pop_conflict`, `resolver_is_modal_and_esc_returns`) are F1–F5 below, each with its line; `grep '#\[ignore' crates/graft/tests/it/m5_walk/` finds none; `git diff --stat` touches no `src/`.

## Findings (rule 7)

- **F1 — defect.** `/` opens the search prompt behind the open resolver. `crates/graft/src/update/mod.rs:655-656` runs `search::search_key` before `keys::key_event`, whose modal chain is where `resolver_key` sits (`crates/graft/src/update/keys.rs:95-98`); `search.rs:16-21` opens the prompt. Inferred, not observed: with a search active, `n`/`N` would be taken by search too (`search.rs:22-24`). Row: `resolver-is-modal-and-esc-returns` (red).
- **F2 — defect.** Redo (`U`) of `undo: pop stash@{0}`, for a pop that stopped on a conflict (the stash kept), runs `git stash drop`: `crates/graft-ops/src/undo.rs:320-323`, the `(StashPop…, true) => StashDrop{pinned}` arm, does not look at the aborted pop's `refs_after`, which still has `refs/stash`. The no-op guard in `restore/steps.rs:373-387` (`stash_drop`) only covers the undo direction. Observed: stash list empty after `U`; the Redo entry's `refs_after` has `refs/stash` None; its commands include `git stash drop 'stash@{0}'`. The stash commit stays pinned (not lost). Row: `undo-a-resolution-in-a-stash-pop-conflict` (red).
- **F3 — question (spec).** Start over after `⏎` in a merge cannot be reached: `u` on the stage is refused during a merge (`crates/graft/src/update/undo.rs:102-108`, `in_progress_refusal`: `finish or abort the merge first`), and a staged path has no `CONFLICTS` row to reopen. Start over **before** `⏎` passes with the stop's stage oids. Is the ticket's "after `⏎`" a path the design wants (a key on a staged row?) or should the scenario drop it? Row: `start-over-brings-the-markers-back` (red).
- **F4 — defect.** `crates/graft/src/update/resolver/answers.rs:235-240` offers `all conflicts resolved · c continue` when the resolver was opened from the changes pane, but gives the banner focus only when it was opened from the banner (`:236-237`), so `c` pressed next (on the pane) opens the commit editor. The same file's comment at `:243-244` says a line must not offer a key that does nothing. Row: `merge-resolved-with-the-keys` (red).
- **F5 — question (spec).** The ticket's `c` → merge commit is on `StoppedMerge`, a merge **begun in the shell**; `c` from the banner refuses it by design: `crates/graft-ops/src/operation.rs:574-575` (`nothing Graft started is in progress; an operation begun in the shell is finished there`). A `Times I left` row records the reach. The Graft-begun `c` → commit → `u` path is covered by `u-right-after-continue-is-accepted` and both rebase scenarios. Should the merge scenario start its merge in the app instead? Row: `merge-resolved-with-the-keys` (red, after F4).
- **F6 — defect (harness, product crate).** The harness prelude feeds the next key while the resolver's opening read is in flight: `crates/graft/src/update/prelude.rs:31-65` (`settled`) never looks at `state.resolver` (`job`, `base_job`). The opening read sets `job` (`crates/graft/src/update/resolver.rs:149`), and a take refuses while `job` is set (`resolver/take.rs:50-61`; `restart.rs:29` and `base.rs:37` alike). Observed in the pty: 6 of 9 captures show `still writing the last take` and no take. Fix (one clause in `settled`): `state.resolver.as_ref().is_none_or(|open| open.job.is_none() && open.base_job.is_none())`. Blocks box 4 and the `--matrix` clause of box 1. Raw streams: `<target>/pty-walk-m6/` (not committed).
- **F7 — textual (wording).** The refusal while the **opening read** is in flight says `still writing the last take` — no take was pressed (same lines as F6; seen on the pty frames of `m6-05`, `m6-09`).
- **Questions for the user's eye:** the relaunch after a kill reads `1 of 1 conflicts open`, not the ticket's `1 of 2` (the file is re-read from disk, where one block was already taken); a conflicted in-app stash pop's Sticky is git's own `pop stash@{0}: \`git stash pop --index 'stash@{0}'\` failed (1): Index was not unstashed.`; a whole-file card's title says `0 of 0 conflicts open` (`0/0 open` at 80 columns); `no more conflicts · ⏎ marks <file> resolved` (the user decides that wording).

## Fix F2

- **Cause, not symptom.** The redo's stash step came from the root operation's kind alone: pop/drop/branch-from-stash → `StashDrop{pinned}`, as if the root had always dropped its stash. A pop git refused on a conflict never did (git drops only after a clean apply), so its undo's `StashStore` found the commit listed and did nothing — and the redo then dropped a stash no operation ever removed. Now (`crates/graft-ops/src/undo.rs`, `stash_step`, the `(StashPop | StashDrop | BranchFromStash, true)` arm) the redo drops only what its undo stored back: the **`Undo` entry's own** `refs/stash` before/after (`stash_moved_to`, the check a push's undo already used) must show it moved to the pinned commit. Not the root pop's `refs_after`, as the finding suggested: popping `stash@{1}` leaves `refs/stash` unmoved whether git dropped the stash or not, so that reading would only fix `stash@{0}`.
- **Proving tests (seen red first, then green):** `crates/graft-ops/tests/it/stash_pop_redo.rs::redo_of_a_conflicted_pop_keeps_the_stash_git_kept` (`stash@{0}`; red: list `[]` after `U`) and `::redo_of_a_conflicted_pop_below_the_top_keeps_both_stashes` (`stash@{1}`; red: one of two left) — pop → `u` → `U` in a real repo, the stash list read with `git stash list`; `crates/graft-ops/src/undo/tests.rs::redo_drops_only_the_stash_its_undo_stored` (plan: no `StashDrop`, no pins, no `refs/stash` touched; red: `StashDrop{0x70}` present). The other side, a guard (green before and after): `stash_pop_redo.rs::redo_of_a_clean_pop_below_the_top_drops_it_again` — a clean pop of `stash@{1}` is stored back by `u` and dropped again by `U`, though the pop never moved `refs/stash`; plus the unchanged unit test `redo_of_an_undone_stash_pop_drops_the_stash_after_the_generic_steps`.
- **Walk row:** `m5_walk::every_scenario::undo_a_resolution_in_a_stash_pop_conflict` green with no test change (`cargo nextest run --workspace undo_a_resolution_in_a_stash_pop`, 2026-09-23).
- **What `U` now does to the user:** after `u` of a conflicted pop, `U` puts the conflict back (markers, unmerged index) and leaves the stash on the list, where git left it. A redo of a pop that did drop its stash drops it again, as before.
- **Cost:** a redo whose undo never recorded `refs/stash` after (an undo that failed and whose after-state could not be read) is now refused — `Incomplete { field: "refs_after[refs/stash]" }` — instead of guessing. Not observed in a real run; covered by the unit test above.
- **Edge, not changed (question):** a commit listed twice in the stash (possible only through `git stash store`): a clean pop of the top copy leaves the other listed, the undo's store is a no-op, and the redo drops nothing — the list ends one longer than after the pop. Nothing is lost.
- **Size (PROMPT.md 6.2, pre-existing):** `crates/graft-ops/src/undo.rs` was 579 lines and is 603 now; `undo/tests.rs` 575. The new integration tests went to their own file rather than grow `unmerged_index.rs` (424). Splitting `undo.rs` is outside this fix.
- Review: `code-review` skill, two read-only `reviewer` agents (standards, spec). Spec: approach accepted, 0 Critical/Major. Acted on: tests moved to their own file, a named struct for the walk's lists, a comment on why a `refs/stash` moved elsewhere drops nothing, the `stash@{1}` clean-pop guard test. Not acted on: renaming `stash_moved_to` to a verb; the repeated `unwrap_or_else(… explain())` (the file's existing pattern).
- Gates (workspace-wide, 2026-09-23): `cargo fmt --check` clean; `cargo clippy --all-targets -- -D warnings` clean; `cargo nextest run` 2652/2655 — the 3 red are the F1, F3 and F4/F5 rows, untouched; `cargo test --doc` 3/3.
- Scout: none — the finding carried the lines; the change is one match arm and its helper in one file.

## Observed / not observed

- **Observed (fast tier, headless, real git):** the 15 green rows; the 4 red rows fail on their finding's check and nothing earlier (each red scenario checks every other clause first).
- **Observed (pty, real binary):** the two card captures at both widths; the prelude race (F6) on six others.
- **Not observed:** the terminal hand-over itself (`Step::HandOver` runs the loop's own command builders headless; suspend/resume of a real terminal is not exercised); **mergetool with a real tool — unverified, a script stood in**; a real `SIGKILL` → relaunch capture (the script path exists; blocked on F6); **colour — unverified** (rule 5; the sheet says so); how anything looks — the sheet is for the user's eye (rule 6).
- **Release + heavy tier:** `cargo test --release --features heavy-tests --workspace --no-fail-fast` — 2681 passed, 4 failed (the same four rows), 1 ignored, 180 s (`docs/perf/m6-07.txt`, from `.scratch/m6-conflicts/perf-m6-07.sh`). Frame budgets held under `--release`.
- **Gates (workspace-wide, 2026-09-23):** `cargo fmt --check` clean; `cargo clippy --all-targets -- -D warnings` clean; `cargo nextest run` 2647/2651 (the 4 red rows above); `cargo test --doc` 3/3. Clippy's `too_many_lines` is allowed, with a reason, on seven scenario functions (declarative step tables, the `tests/it/gallery.rs` precedent).

## Deviations from the spec

- `Wait::ResolvedPath(path)`, not `Resolved(path)` — `Wait::Resolved` is M5's (every path). Truth fns return `Result` (`side_bytes`, `resolved_bytes`) rather than bare `Vec<u8>`.
- Extra `Step`s: `Resolve(path)`, `HandOver(label, key)`, `FocusBanner`; extra `Wait`s: `ConflictedPath(path)`, `ResolverClosed`.
- Extra scenarios: `whole-file-card-glimpses-a-large-file`, `binary-card-shows-its-size`, `n-past-the-last-block-says-no-more`, `kill-9-mid-rebase-then-abort` (the spec's second variant as its own row). Extra capture `m6-09-start-over-box`; `m6-03` card on a large file, `m6-04` binary, `m6-08` `no more conflicts`.
- `undo-a-resolution-in-a-stash-pop-conflict` pops **in the app** (`S`, `p`): a pop run in the shell has no journal entry for the second `u`.
- `HandOver`'s editor lookup strips `GIT_EDITOR`/`VISUAL`/`EDITOR` from `git var GIT_EDITOR`: this machine exports `GIT_EDITOR=true`, which git ranks above the fixture's `core.editor`.
- Rebase scenarios take `b`/`t`, not `o`, where `o` on every block would make a replayed commit empty (git drops it).
- Contact sheet is `docs/shots/m6/m6-contact-sheet.txt` via `scripts/contact-sheet.sh m6 docs/shots/m6` (the script gained a shots-dir argument, the M6 promise, and a tail for earlier milestones' changed captures — the four `m5-hints-*` M6 changed, captioned "the uncommitted row has no `space fold`").
- Two scenario test bugs fixed on the way (cause, not symptom): `zdiff3-and-base-on-demand` counted only `block.base`, while `B` attaches `block.reproduced` (`graft-git/src/conflict.rs:1143` `attach_base`); `whole-file-card-glimpses-a-large-file` looked for the note row without `G` — it is the card's last row, under 500 lines (`graft-ui/src/conflict_resolver.rs:311-312`).

## Handoff

- Next step: fix F6 (one clause, `crates/graft/src/update/prelude.rs::settled`, plus a unit test beside `the_prelude_holds_a_key_while_a_preview_plan_is_pending`), then run `GRAFT_UPDATE_SHOTS=1 GRAFT_TARGET=<target> python3 .scratch/m6-conflicts/pty-walk-m6.py 120x40` and `80x24`, re-run without the variable (must be green), add the new files' captions to `docs/shots/m6/m6-contact-sheet-captions.tsv` (TESTING.md §Conflicts walk has the lines), `scripts/contact-sheet.sh m6 docs/shots/m6`, `m5_walk --matrix`. The `m6-06` run prints the SIGKILL line the box asks for.
- F1, F2, F4 are product fixes; F3 and F5 need a ruling. Each red row turns green on its fix with no test change.
- Gotchas: the `on_screen` text for `m6-01` (`took theirs`) and `m6-06`'s pre-kill wait (`took ours`) match `resolver/answers.rs:172` (`took {side} in …`) but have not been seen on a pty frame yet.

## Notes

- Scout used: `scout-m6-07.md` in the builder's scratchpad (not committed).
- **Context was compacted twice** during this ticket. What remains is the Handoff above.

## Gates

fmt / clippy / nextest / doc, workspace-wide (tests-only change, still the full set).

## Size

One context; scenarios ~600 lines in the existing table shape, TESTING.md ~150 lines, the pty script copied and trimmed. Captures are read as paths only.
