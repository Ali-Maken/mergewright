# acceptance-history: one command that walks "you see the result before you confirm", what right looks like, the captures, and M5's contact sheet

Status: ready
Blocked by: 01–10
Build with: **Fable** — the milestone's acceptance ticket (CLAUDE.md standing rule); last in order, before the review. Does not count against the Fable build expectation.
Crates: graft (`examples/`, `tests/it/`) plus `TESTING.md`, `README.md`, `CHANGELOG.md`, `docs/shots/`, `.scratch/m5-…/pty-walk-m5.py`. **No library code in any crate**; a needed library change is `blocked: needs <crate>` with the signature named.
Needs: every other M5 group — `secret-field`, `undo-cancel-rollback`, `refresh-race`, `git-history-reads`, `ops-history-operations`, `ui-ghost-banner-conflict`, `hint-bar-redesign`, `app-preview-and-keys`, `app-conflict-interim`, `app-drag-drop-and-stash-branch`.

## Read first

- `docs/design-notes/m5-history-and-ghost-preview.md` — whole (two pages): the Promise line is the question every scenario answers.
- `docs/adr/0033-times-i-left.md` (whole); ADR-0035 §4–5; ADR-0036 §1 (runs alone, after the merge); **`docs/adr/0037-m4-checkpoint-decisions.md` §2** (one contact sheet per milestone).
- PROMPT.md §1.5 (Ghost Preview), §3 "Feedback", §8 M5 row and the no-resolver paragraph (line 599), §9.
- `docs/reviews/m4-07-acceptance.md` — M1 (prove every clause, not one), M3/M4 (the walk split by seam: `tests/it/m4_walk/{mod,fixture,truth,steps,run,vocabulary,<scenarios by promise>,every_scenario}.rs` and `tests/it/coverage.rs` — **copy that shape**, never one file), M5 (never claim in TESTING.md what the walk does not assert), Minor 2 (a capture the user cannot judge is not a capture).
- `.scratch/m4-remotes/issues/07-acceptance-remotes.md` — Scope pieces 1–6 and both NOTES sections (pyte pitfalls, `--capture`, `--matrix`, the split); `TESTING.md` "Remotes" (grep the heading) — the structure to mirror; `crates/graft/MAP.md` `examples/` lines and §update `Harness`/`prelude`.
- `scripts/contact-sheet.sh` header (usage, the captions TSV format) and `docs/shots/m4-contact-sheet-captions.tsv` (the caption style: what right looks like, in the user's terms).
- `crates/graft-fixtures/MAP.md` (`Shape`, `git::run`, `remote::LocalRemote`); `crates/graft-ops/MAP.md` §preview, §apply, §undo; `crates/graft/MAP.md` §update::preview, §update::banner, §update::drag, §update::name_prompt (after the app groups).
- Every `docs/shots/m5-ui-*.txt` the app groups left (they become part of the sheet; regenerate rather than duplicate).
- `KNOWN_GAPS.md` rows with expiry M5 (grep `M5`) — read-only; the rows go through the conductor.

## Scope

In:

1. **The walk** — `cargo run -p graft --example m5_walk -- [--capture <name>] [--only <name>] [--list] [--stems] [--matrix]`, `examples/m5_walk.rs` including `tests/it/m5_walk/` by `#[path]` (the M4 split: `mod.rs` list + `fixture.rs`, `truth.rs`, `steps.rs`, `run.rs`, `vocabulary.rs`, scenarios by promise — `preview.rs`, `merge_rebase.rs`, `pick_revert_reset.rs`, `refs_and_names.rs`, `conflicts.rs`, `time_machine.rs`, `remotes.rs` — and `every_scenario.rs`; largest file under 400 lines). Each scenario: a corpus shape copied to scratch (`copy_into_temp`) plus scratch commits made with `graft_fixtures::git::run` (a conflict = two branches editing one line; a dirty tree = an edited tracked file; a foxtrot = the shape the radar tests use, else built by hand and named in NOTES), the real app driven headless through `Harness::prelude` over the real pool, and **truth read back by running git** (`for-each-ref`, `rev-parse HEAD`, `symbolic-ref HEAD`, `write-tree` of the index, file bytes, `.git/index` bytes, `rebase-merge/`/`MERGE_HEAD` presence, `stash list`, the journal file) — never from `AppState`. **The one thing read from `AppState` is the claim**: the `PreviewPlan` behind the box (`PreviewState.plan`), so the walk can assert *the ghost equals what git did*: after `Enter`, every `RefMove` with a `Real` target equals `for-each-ref`; every `Ghost` target's ref now points at a commit whose parents are the ghost's parents' real counterparts and whose subject equals the ghost's subject (rebase: `git patch-id` of each replayed commit equals its ghost's `replays`); `HeadMove` equals `symbolic-ref`/`rev-parse HEAD`; `WorktreeEffect::Updated { paths }` ⊆ `git diff --name-only` between before and after; `Lost { paths }` are gone and back after `u`. Prints `ok <name>` / `FAIL <name>: <≤ 20 lines>`, exit 1 on any failure. Scenarios, in order (a name in `code` is the scenario's; every one ends in `git fsck --no-dangling` clean):
   - **preview** — `preview-opens-with-ghosts-and-plan-commands` (`r` on a branch row: box title `Preview: rebase …`, `commands == plan.commands`, ghost rows present; nothing ran: refs unchanged) · `esc-cancels-nothing-runs` · `cancel-is-the-default-choice` (`Enter` at once = nothing ran) · `stale-preview-refused` (`Shell` step moves `main` between `Ready` and `Enter`; status `main moved since the preview · r again`; refs unchanged) · `preview-too-small-never-confirms` (38×10: the one-liner; `Enter` does nothing) · `preview-refused-during-an-operation`
   - **merge** — `merge-ff-preview-equals-git` (no ghost, `RefMove → Real(source_tip)`) · `merge-no-ff-ghost-equals-git` (one ghost merge; after: `main`'s commit has the two parents; `u` → `main` back, tree back; `U` → forward) · `merge-already-up-to-date-offers-close` · `merge-forecast-names-the-conflicting-path` (box line `may conflict (forecast): <path>`; `Esc`)
   - **rebase** — `rebase-two-commits-ghost-equals-git` (patch-ids; `u` restores the branch and the tree) · `rebase-dirty-tree-refused-offers-stash` (`rebase needs a clean tree · S stash 1 changed files`; bytes unchanged) · `rebase-via-drop-menu` (mouse `Down` on the badge, `Drag`, `Up` on `main`'s row, menu, `Enter` on `Rebase …` → the same preview → confirm → ghost equals git)
   - **pick / revert / reset / checkout** — `cherry-pick-via-menu-ghost-equals-git` · `revert-via-menu-ghost-equals-git` · `revert-merge-uses-mainline` · `reset-hard-says-loses-and-u-brings-it-back` (box `working tree: LOSES <file> (backup saved)`; after: file gone; `u`: bytes back, `main` back) · `reset-mixed-keeps-the-tree` · `move-a-branch-not-checked-out` (`.` menu on a non-current branch: `update-ref`, HEAD untouched) · `checkout-branch-via-menu` (`symbolic-ref HEAD`; `u` → back) · `checkout-detached-via-menu`
   - **refs and names** — `b-creates-and-checks-out` (name prompt; `u` deletes; `U` recreates) · `b-refuses-bad-and-existing-names` (refusal lines under the field; nothing written) · `t-creates-a-tag` · `delete-branch-then-u-restores` · `delete-tag-then-u-restores` · `stash-b-branches-from-stash` (`S`, `b`, name, preview, confirm: branch exists at the stash's base, stash gone, `u` → `stash list` has it again)
   - **fix** — `fix-revert-merge-on-a-foxtrot` (`.` on the marked row → `Fix: revert the merge (-m 2)` → confirm → `main`'s tree equals the previous tip's tree) · `fix-reset-to-before-the-merge` · `fix-rebase-instead` (linear history, patch-ids preserved)
   - **conflicts (the honest interim)** — `conflict-focuses-banner-and-lists-files` (header `⚠ MERGING — conflict …`, `CONFLICTS (1)` first, `MERGE_HEAD` present, journal `Pending`, hint bar `c continue  s skip  a abort  . more` at 80) · `banner-a-aborts-and-restores` (`[ Cancel ] [ Abort ]`, tree byte-identical, `MERGE_HEAD` gone, entry `Aborted("aborted")`) · `continue-refuses-until-resolved-then-one-entry-undoes` (`c` refused in words; `Shell` edits the file; `Space` refused while markers, then staged; `c` → `Committed`; **one** entry; `u` → pre-merge refs and tree) · `rebase-stops-twice-same-entry` · `mergetool-absent-refuses-in-words` · `e-opens-editor-at-first-marker` (the `OpenExternalEditor` command's line == `grep -n '^<<<<<<< '` first hit) · `conflict-survives-relaunch` (drop the harness mid-conflict, reopen: banner focused, sticky `rebase still in progress from a previous run · c continue  a abort`, entry still `Pending`; `a`) · `u-refused-during-an-operation` · `journal-row-says-in-progress`
   - **time machine** — `time-machine-three-steps` (three writes; `J`, `Enter` on the oldest: box lines name each ref `a → b`, `index and working tree restored from backup`, `3 steps`; confirm: `state_at(target)` equals git now, three Undo entries, `U` ×3 returns) · `time-machine-esc-returns-to-journal`
   - **remotes** — `pull-diverged-offers-m-and-r` (`LocalRemote`; sticky `main and origin/main have diverged · m merge it  r rebase onto it`; `r` → preview onto the tracking ref → confirm → ghost equals git, bare untouched) · `u-after-push-asks-once` (box with three choices; `Undo locally` → local back, bare unchanged; second `u` on the same entry does not ask) · `u-after-push-repush-old-tip` (`Re-push old tip` → lease box → `Enter` → bare's `main` == the push entry's `refs_before[tracking]`, local unchanged) · `u-after-push-cancel-asks-again`
   - **regressions from pass 1** — `esc-cancels-undo-and-rolls-back` (a 3-step undo stopped with `Esc` through the real `CancelJob` path: state equals pre-undo; entry `Aborted("cancelled")`) · `ctrl-r-twice-leaves-one-history-job` (the M4 workaround removed; `ReadRefs` completes) · `password-field-is-secret` (`HttpRemote`: `Debug` of `AppState` with the prompt open holds no password bytes; the capture shows `•` only)
   - `hints-at-80-show-three-and-more` (footer for branch row / plain row / banner / changes pane, exact strings from the note's table)
2. **`TESTING.md` "History"** section: the command; one numbered entry per scenario saying **concretely what right looks like** — exact box lines, status strings, `for-each-ref` relations, byte equalities (`after r ⏎ ⏎ on [main] with HEAD on feature/auth: git rev-parse feature/auth == the ghost's real counterpart, whose parent == main's tip and whose patch-id == 7f6e5d4's; the journal's last entry is kind rebase, refs_before {refs/heads/feature/auth: 7f6e…}` — never "looks correct"; quote only what a `See`/`expect` asserts, per m4-07 M5); "What the walk proves and what it does not" (headless pty grids, no colour, no real `git mergetool` hand-over, forecasts are per-commit not chained, no pixel-tier ghosts, no interactive rebase, no 120×40 except the banner, no real terminal); per-breakpoint rows for the preview box (rebase, merge, reset hard), the banner + CONFLICTS pane (80×24 / 120×40 / 160×48), the name prompt, the drop menu, the `u`-after-push box, the time-machine box, the hint bars; per-terminal rows (Ghostty, Zed, VS Code) `not run`, each with what to look for (`ghost rows are dashed and dimmer than real rows`, `the box sits above the footer, never over it`); a "Times I left" reminder: every reach for the shell during a merge/rebase/reset in daily use goes in that table.
3. **Captures** under `docs/shots/` via `.scratch/m5-history-and-ghost-preview/pty-walk-m5.py` (from `pty-walk-m4.py`; raw streams under `target/`; a mouse-driven capture is taken through the harness prelude's mouse events, not the pty script): `m5-<nn>-<what>-{80x24,160x48}.txt` for: preview rebase (ghost rows + box), preview merge no-ff, preview reset hard (`LOSES` line), preview time machine, preview too small (a 38×10 pty: the one-liner), banner focused + CONFLICTS pane (**also 120x40**), name prompt with a refusal, drop menu with a dim item, `u`-after-push box, stale-preview sticky, diverged-pull sticky, hint bars (branch row, banner, changes pane at 80 only if `hint-bar-redesign` did not already capture them — reuse, do not duplicate). Every file ≤ 8 KB, hard cap 200 KB; the script asserts no password bytes in any capture and refuses to write one that shows the graph with no box in a preview scenario. Captures go straight to files; the report names paths only.
4. **The contact sheet (ADR-0037 §2)** — write `docs/shots/m5-contact-sheet-captions.tsv` (`filename<TAB>caption`, one line per `m5-*.txt` capture **including the app groups' `m5-ui-*` files**, caption = the TESTING.md line for that capture, in the user's terms: `two (ghost) rows above [feature/auth], sha and when read —, box title Preview: rebase feature/auth onto main, [ Cancel ] highlighted`), then run `scripts/contact-sheet.sh m5` → `docs/shots/m5-contact-sheet.txt`. A test (`contact_sheet_captions_cover_every_m5_capture`) asserts every `docs/shots/m5-*.txt` except the sheet has a caption line and no caption names a missing file. The sheet is exempt from the 200 KB cap but stays trimmed (the script assembles; it never invents).
5. **`--matrix`**: `tests/it/coverage.rs` gains `M5_STEMS` and a fifth table; `docs/shots/35-coverage-matrix.txt` regenerated (the walks' tables in stem order, m4-07 Minor 7).
6. **`README.md` / `CHANGELOG.md`** M5 sections: what works (the keys, the preview, the honest interim — *"a conflict shows you the files and hands you to your mergetool or editor; Graft does not resolve conflicts until M6"*), what is deferred, that every history key shows its result first. **KNOWN_GAPS rows are the conductor's** (m4-07 C1): NOTES lists, for the conductor to file, every row the builders' NOTES named (no resolver → M6 re-worded; Esc = abort → M7; pixel-tier ghosts → M8; annotated tags → M7; chained forecasts → M7; `last_rejected_push` → M7; anything the walk found), marked unverified-by-design vs broken, and the rows M5 closed (204, 217, 218, 229, 455, 469, 471–474, 483–485 — each only if its group's NOTES said closable). README/CHANGELOG say "see KNOWN_GAPS.md" only for rows the conductor has confirmed filed (hash in NOTES).
7. **Context guard**: one line per scenario; a failing scenario ≤ 20 lines; captures and the sheet are never read back into the context (sizes and `head -3` only); gate output quiet, full output only on the failing part.

Out: fixing what the walk finds (a failing scenario is a finding for the conductor, filed as a ticket); the heavy tier and the release perf run (the reviewer's — commands recorded in TESTING.md, figures `not run`); running in a real terminal (the user does); any library change.

## Signatures

```rust
// tests/it/m5_walk/mod.rs (+ the split files; included by #[path] from examples/m5_walk.rs)
pub fn scenarios() -> Vec<Scenario>;
pub struct Scenario { pub name: &'static str, pub shape: Shape, pub remote: Option<RemoteKind>, pub size: (u16, u16) /* default 80×24 */,
    pub setup: fn(&Fixture) -> Result<(), String>, pub steps: Vec<Step>, pub expect: fn(&Fixture, &Marks) -> Result<(), String>, pub capture: Option<&'static str> }
pub enum Step { Keys(Vec<KeyPress>), Mouse(Vec<MouseEvent>), Shell(&'static [&'static str]), Type(&'static str), TypeSecret(&'static str), Wait(Wait), Mark(&'static str), See(fn(&AppState) -> Result<(), String>), Relaunch }
pub enum Wait { Ready, PreviewReady, BannerFocused, Box, Prompt, JournalHas(OperationKind), CredentialPrompt }
pub struct Truth { pub refs: BTreeMap<String, String>, pub head: String, pub head_symbolic: Option<String>, pub index_tree: String, pub files: BTreeMap<PathBuf, Vec<u8>>, pub index_bytes: Vec<u8>,
    pub in_progress: Option<String>, pub stashes: Vec<String>, pub journal: Vec<JournalEntry>, pub bare: Option<BTreeMap<String, String>>, pub statuses: Vec<String> }
pub struct Claim { pub plan: Arc<PreviewPlan> }   // the preview's own prediction, taken at `Wait::PreviewReady`
pub fn ghost_equals_git(claim: &Claim, before: &Truth, after: &Truth, fixture: &Fixture) -> Result<(), String>;   // the promise's assertion, one function, used by every "ghost-equals-git" scenario
pub fn run_scenario(s: &Scenario, capture: Option<&Path>) -> Result<(), String>;
pub fn diff_truth(before: &Truth, after: &Truth) -> Vec<String>;   // ≤ 20 lines
// tests/it/coverage.rs
pub const M5_STEMS: &[&str];
pub fn history_matrix() -> Table;
```

## Acceptance (every tick names its test or command at tick time)

- [ ] `cargo run -p graft --example m5_walk` prints `ok <name>` for every scenario (≥ 45) and exits 0; `--only <name>` runs one; a deliberately broken expectation prints `FAIL <name>: …` and exits 1 (reverted; the run named in NOTES with its output path under `target/`) — `m5_walk::every_scenario::every_scenario_has_a_test`
- [ ] One nextest test per scenario, slowest < 4 s, no `sleep`/poll loop; the fast tier's wall time recorded in TESTING.md's top table — `cargo nextest run -p graft` (`m5_walk::every_scenario::*`)
- [ ] Every "ghost-equals-git" scenario goes through `ghost_equals_git` and that function checks refs, ghost parents, subjects, patch-ids, HEAD and worktree paths — `ghost_equals_git_is_the_only_claim_check` (a test that greps the scenario files for `PreviewState`/`plan` reads outside `Claim`)
- [ ] `TESTING.md` "History": one entry per scenario quoting only what the walk asserts; the proves/does-not list; breakpoint and terminal tables — by reading, counts in NOTES
- [ ] Captures: the list in piece 3 at both sizes (banner also 120x40, too-small at 38x10), each ≤ 8 KB, none showing a password; `35-coverage-matrix.txt` has five tables — `m5_walk::every_scenario::captures_are_named_keyed_and_listed_in_the_matrix`, `it::coverage::tests::*`
- [ ] `docs/shots/m5-contact-sheet-captions.tsv` covers every `m5-*` capture and `scripts/contact-sheet.sh m5` wrote `docs/shots/m5-contact-sheet.txt` with no `NOT RECORDED` line — `contact_sheet_captions_cover_every_m5_capture` + `grep -c 'NOT RECORDED' docs/shots/m5-contact-sheet.txt` = 0 in NOTES
- [ ] README and CHANGELOG M5 sections present and honest about the interim (the sentence "does not resolve conflicts" is in both) — by reading; KNOWN_GAPS rows listed in NOTES for the conductor, **this tick stays unticked until the conductor's filing hash is written here**
- [ ] Every tick above names its test or command — inspection

## Gates

`cargo fmt --check` · `cargo clippy --all-targets --message-format=short -- -D warnings` · the same with `--features heavy-tests` · `cargo nextest run` · `cargo test --doc` — workspace-wide, quiet, in this group's own `CARGO_TARGET_DIR`. The release perf run and the heavy tier are the reviewer's (ADR-0021).

## Size

One context; the M4 walk cost one Fable run and this one has more scenarios with lighter setup each — copy the M4 split, add the claim check. Scout `tests/it/m4_walk/` helper names first (Haiku, ≤ 1 page); say so. Stop seam: after the preview + merge + rebase scenarios land with `ghost_equals_git` (pieces 1 partial, 2 partial), the rest in a handoff note; captures and the sheet last.

---

# CARRIED WORK — added by the conductor, 2026-09-21

These accumulated across tickets 13–29 and are tagged to this group in
`KNOWN_GAPS.md`. **Builders never read `KNOWN_GAPS.md`**, so they are copied
here in full. Each closes when its row can be moved to `docs/gaps/closed.md`
with evidence; report which you closed and which you did not, and why.

## Captures that are stale, and must be regenerated

1. **17 captures under `docs/shots/`** carry the old 7-glyph mask and a
   one-cell shift. Ticket 23 regenerated them mid-sweep and then deliberately
   put them back (`git show HEAD:<p> > <p>`, never `git checkout`) rather than
   fight a concurrent rebuild. They are yours now.
2. **Six `docs/shots/m5-ui-preview-*.txt` still say `(forecast)`.** The string
   is gone from `update/preview.rs`; the captures predate the change.
3. **Five `docs/shots/m2-*.txt`** are stale from hint-bar drift — and the bar
   has changed twice more since (tickets 27, 28), so re-read them before
   assuming the old diagnosis still describes them.
4. **`m4-03` and `m4-04` pty captures** still show the old 7-glyph mask.
   Regenerate with `m4_walk --capture`, then `scripts/contact-sheet.sh m4`.
5. **The confirm section of `docs/shots/m4-credential-prompt-and-hints.txt`**
   predates group 04's frame fix.

## Two that are decisions, not regenerations

6. **`docs/shots/m4-credential-prompt-in-app.txt` and the M4 contact sheet
   print the fixture's literal password in their caption** — "with `hunter2`
   typed". It is a test literal, never a typed secret reaching a row, so
   ADR-0038 §2 is intact. **But this repo may be published.** Writer:
   `crates/graft/src/view/tests.rs::capture_m4_credential_prompt_in_app`.
   Remove the literal; do not weaken the mask.
7. **`docs/shots/m5-ui-confirm.txt` draws its box on a blank screen** and is
   superseded by the four `m5-ui-*-over-graph-*` files from ticket 26. Its
   writer is `graft-ui/src/confirm.rs::capture_for_docs_shots`. **Decide:
   delete it, or give it real content the way ticket 26 did.** Say which and
   why.

## Two stale prose items — the class this milestone kept failing

8. **`crates/graft/src/update/detail/keys.rs:137-140`** still explains `M`/`e`
   by "the diff of an unmerged file is the merge's own combined diff". That is
   no longer the diff shown.
9. **`crates/graft/src/jobs/askpass.rs`'s module doc** still describes the
   pre-M5 plain-`TextField` password leak as current, and says the fix "means a
   secret-backed field in graft-ui" — that field shipped in group 01.

## One missing test

10. **The banner and the CONFLICTS section have no width-bounds test.** Group
    09 captured at 80x24, 120x40 and 160x48 but added no bounds *test*. The
    sweep to extend is `crates/graft-ui/tests/pane_boundaries.rs` (widths
    20..=240), beside ticket 28's border rules.

## Three review Minors

11. **Minors 5, 6 and 7 of `docs/reviews/m5-14-to-22-eye-check-round.md`** —
    capture/caption fixes, explicitly deferred to the contact sheet.

12. **`docs/shots/m3-changes-pane-80x24.txt` — the 80x24 block no longer shows
    what its caption promises.** Review `m5-23-to-29.md` Minor 6. Ticket 29's
    reserved bar row cost the block `M routes.rs`, so `:26-29` reads
    `UNSTAGED (2)` → `▾ src/` → `▾ api/` → `fn routes()` — a section with a
    folder and no file under it, and no `UNTRACKED` row at all — while the
    caption still says "Three section rows … each with its files indented
    under it" and only warns that `notes.md` "may sit below the fold".
    Writer: `crates/graft/src/view/tests.rs::capture_m3_changes_pane`.
    **Read the regenerated file and make the caption describe it**, naming
    which rows are below the fold at that width. Do not widen the capture to
    make the old caption true.

## Before you start: this carried list is itself partly stale

Added by the conductor after tickets 30 and 31 merged, **which regenerated
about 21 captures between them** (30: the two `m5-ui-confirm-over-graph-*`
and the hints ladder; 31: eighteen, plus two new `m5-hints-detail-*`).

**So do not trust items 1–5 above as a work list.** Several of those files
have already been regenerated by a later ticket, and at least one diagnosis in
them predates two rounds of hint-bar change. **Re-derive the stale set from
the repo as it is now** — that is the job — and report what you actually
found against what this list claims. If an item is already fixed, say so and
move on; if the list missed a file, that is worth more than confirming the
ones it named.

This is the same fault the whole milestone has been about: **a list and the
thing it describes, with nothing checking one against the other.** Do not
become its seventh instance by working from the list instead of the repo.

## One more for the user's eye, not a defect

**At 160 columns on a non-current branch row, `q quit` now degrades to a bare
`q`.** Caused by `b branch` and `t tag` going live in ticket 10 — the ladder
behaving as designed under two more Live hints, not a regression. **160 is a
width the user reads**, so it belongs in the contact sheet under its own "what
right looks like" line, called out rather than buried, and **he decides**
whether the tail is now too long. Do not tick it either way.

## NOT yours — for the record, so you do not chase it

- **The real `git mergetool` hand-over has never run in a real terminal.** Unit
  tests prove the terminal is restored around a tool that *fails*. Only the
  user can close this, at a real terminal on a real conflict; it goes in
  `TESTING.md`'s "Times I left" when he does. Do not fake it and do not tick
  it.
