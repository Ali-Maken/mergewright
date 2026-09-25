# start-over-a-whole-file-choice: `d` walks back a keep-or-delete choice

Status: done
From: user picks 2026-09-24 item 9 (batch 7, "yes"); `docs/design-notes/m6-conflicts.md` :830 (the proposal);
prototype scene 8 (:1290-1300).
Amended 2026-09-24 (spec-only pass) after scout `.scratch/m6-conflicts/scouts/57-58.md`; the planner re-read
`graft-ops/src/apply.rs:192-230`, `operation/conflict.rs:238-290`, `apply/restart.rs:120-201`,
`graft/src/update/resolver/restart.rs` (whole file) and `graft-ui/src/graph_screen/hints/lists.rs:319-351`, and
confirmed `docs/design/02-keymap.md` §2.6's `d` row (edited 2026-09-24) says what this spec builds.
Build with: builder-opus — a git write inside a live operation, with two callers of one gate; the rule is set here,
so no design is left to the builder.
Needs: `undo-and-abort-a-shell-begun-merge` merged (same crates, lane O, second).
Crates: graft-ops, graft.
Lane: wave 1, lane O, second.

## Read first
- `docs/design/02-keymap.md` §2.6, the `d` row.
- `docs/design-notes/m6-conflicts.md` :820-835 (the proposal) and prototype scene 8 (:1290-1300);
  `.scratch/m6-conflicts/spec-pass-edits-2026-09-24.md` E6, J17.
- `.scratch/m6-conflicts/scouts/57-58.md` §58 (the code as it stands, with lines).
- `crates/graft-ops/MAP.md` §apply `StageSource` … `restart_gate` (:172-174), `Refusal::NotMarkerShape` (:48).
- `crates/graft/MAP.md` §update::resolver `DoneOpen` / `start_over_refusal` (:330), `restart_landed` (:333),
  `WholeFileShape` (:318); §update::confirm `ConfirmThen::RestartConflict` (:257).

## Decisions (made in this pass; build them, do not re-decide)
1. **The seam: the kind travels in the params, and ops checks it against the stages it recovers.**
   `apply` builds `op.plan(&facts)` (`apply.rs:209`) from `RestartParams` only, after `prepare_restart` (`:202`) has
   already recovered the stages, and the folded path (`fold::folded_step`, `:203`) runs before `:209`. Handing the kind
   from prep to the plan would mean two plan builders, and `Operation::plan` would lie for a whole-file start-over.
   Instead:
   - `RestartParams` gains `kind: RestartKind`, and the app sends the kind the gate answered (`StartOver::Can { kind }`).
   - `Operation::plan` builds from the kind: `Markers` → `checkout --merge -- <p>`, today's plan; `WholeFile` →
     `checkout --ours|--theirs -- <p>`, the side that survived: stage 2 for `UD`/`AU`, stage 3 for `DU`/`UA`. Every path
     reads this one plan. The builder names in NOTES the line where `folded_step` takes it.
   - `prepare_restart` compares the kind it recovers (from `restart_source`, the one rule) with `params.kind`. If they
     differ, it refuses `Refusal::RestartChanged { path }` before anything is written. The words:
     `<path> changed since it was opened · open it again`. This is the stale-gate backstop that `restart_gate`'s doc
     comment promises, in the style of `Moved` and `StashListChanged`.
2. **Invalid shapes can't be written.** `RestartKind::WholeFile` holds a `OneSided`, which has exactly the four shapes.
   `DD` (`BothDeleted`) stays `NotMarkerShape`. A still-unmerged one-sided file (source `Index`, nothing chosen yet)
   stays refused by `check_restart`, unchanged.
3. **Binary: unchanged from today, whatever today answers.** The planner found no binary check in `restart_source`
   (`restart.rs:120-151`), so it does not assert a refusal. `gate_and_apply_agree_on_every_shape` records today's
   answer for a binary `UU`, and NOTES says what that answer is.
4. **The box's command line comes from the kind:** `git checkout <kind.checkout_flag()> -- <p>`, never the
   hardcoded `--merge` (`resolver/restart.rs:65`). There is one source for the flag, and the plan uses it too.
5. **The whole-file card (unmerged, nothing chosen) still shows no `d`.** `resolver_card_list` (`lists.rs:328`) is
   right as it stands: there is nothing to start over. `d` shows on the **done** file, through `resolver_done_list(restart)`
   (`lists.rs:343`), which already reads the gate. This ticket changes no `graft-ui` file. Lane U's
   `hint-words-keep-the-way-out` marks `esc` on the card and `d`/`esc` on the done list.

## Scope
**In: graft-ops**
- The gate (`restart_source`, shared by `restart_gate` and `prepare_restart`): for a **done** file whose recovered
  stage set (`StageSource::Backup`) is `UD`, `DU`, `AU` or `UA`, the answer is
  `Ok(RestartPlan { source, kind: WholeFile { shape } })`. This applies in an operation begun in Graft (the live entry's
  current-step chain) and with none live (the journal's in-force entries, such as a stash pop). Markers as today, now
  `kind: Markers`. `Index` source → `kind: Markers` (`check_restart` has already refused a one-sided shape).
- The write: `run_restart_prep` puts the stages back into the index (`restore_stages`), as for markers, and then the
  plan's `checkout --ours|--theirs -- <path>` writes the surviving side's blob to the worktree. Both command lines go
  in the Command Log (prep first, as today). There is no `checkout --merge` for a whole-file start-over. It is
  journaled exactly as a marker start-over: folded into the live entry, behind a chained `Step` backup.
- A shell-begun merge is unchanged: `live_entry` is `None`, the chain is empty, so the answer is `NoStagesKept`
  (`restart.rs:128`).

**In: graft**
- `jobs::conflict::start_over` keeps the kind: `StartOver::Can { kind }`.
- `restart_box` words it by kind. **Markers**: exactly as today. **WholeFile**: title `Start <p> over?`; line 1
  `the choice comes back: keep or delete`; line 2 `edits since the choice are lost · not undoable while the <kind> is
  open` (no operation → `edits since the choice are lost`); command `git checkout --ours|--theirs -- <p>`; cursor on
  `Cancel`. `ConfirmThen::RestartConflict { path, kind }` → `Operation::RestartConflict(RestartParams { path, kind })`.
- `restart_landed`: unchanged in code. The file is unmerged again with its one-sided shape, so `reopen` opens it as the
  whole-file card (`deleted by them · o keeps the file  t deletes it` for `UD`), the list row reads `keep or delete`,
  and `finish::forget` drops its choice (`restart.rs:123`, already there).
- The done file's bar and identity row offer `d start over` through `start_over_refusal`, the one predicate. It needs
  no change beyond `StartOver`'s new shape.

**Out:**
- Shell-begun merges (KNOWN_GAPS :613, a separate row).
- `DD`; binary (decision 3).
- The keymap text (done in the spec pass; confirmed).
- `graft-ui` (decision 5).

## The failing test first
graft-ops `tests/it/restart.rs` (or the file holding today's restart tests): `start_over_brings_back_a_modify_delete_choice`.
- Setup: a merge begun through `apply(Merge)` stops with `gone.txt` `UD`; `ResolveWholeFile` keeps yours (the file is
  staged, not unmerged). `restart_gate` → `RestartPlan { kind: WholeFile { shape: DeletedByThem }, .. }`;
  `apply(RestartConflict { path: gone.txt, kind })`.
- Assert: `git ls-files -u -- gone.txt` lists stages 1 and 2; the worktree file equals stage 2's blob; the merge is
  still live; the Command Log's last line is `checkout --ours -- gone.txt`, with `--literal-pathspecs` as every plan
  writes it.
- Red today: `NotMarkerShape` (`restart.rs:142`).

## Signatures
```rust
// graft-ops/src/operation.rs (or the child holding RestartParams)
pub struct RestartParams { pub path: PathBuf, pub kind: RestartKind }     // `kind` added
pub enum RestartKind { Markers, WholeFile { shape: OneSided } }
pub enum OneSided { DeletedByThem, DeletedByUs, AddedByUs, AddedByThem }
impl OneSided {
    #[must_use] pub fn from_how(how: UnmergedHow) -> Option<Self>;          // None for UU, AA, DD
    #[must_use] pub fn how(self) -> UnmergedHow;
}
impl RestartKind {
    /// `--merge`, `--ours` (stage 2 survives: UD, AU) or `--theirs` (stage 3: DU, UA). The plan and the app's box both read it.
    #[must_use] pub fn checkout_flag(&self) -> &'static str;
}
pub enum Refusal { /* … */ RestartChanged { path: PathBuf } }             // the gate went stale; nothing written
// graft-ops/src/apply/restart.rs
pub struct RestartPlan { pub source: StageSource, pub kind: RestartKind }
pub fn restart_gate(repo: &RepoHandle, facts: &RepoFacts, path: &Path)
    -> Result<Result<RestartPlan, Refusal>, RestartReadError>;             // was Result<Result<StageSource, Refusal>, …>
pub(crate) fn restart_source(repo: &RepoHandle, entries: &[JournalEntry], facts: &RepoFacts, path: &Path, stop: &dyn Stop)
    -> Result<Result<RestartPlan, Refusal>, FactsError>;                   // the one rule; prepare_restart compares its kind with params.kind
// graft-ops/src/operation/conflict.rs
pub(super) fn plan_restart(params: &RestartParams) -> Vec<Invocation>;     // signature unchanged; flag from params.kind

// graft/src/jobs/conflict.rs
pub enum StartOver { Can { kind: RestartKind }, Cannot { why: String } }
// graft/src/update/confirm.rs
pub enum ConfirmThen { /* … */ RestartConflict { path: PathBuf, kind: RestartKind } }
```

## Touches (each entry, and how the builder checks it)
- **Every caller of `restart_gate` / `restart_source` / `RestartParams`**: the app's done-file read, the whole-file
  card's gate, `prepare_restart`, and every test building `RestartParams`.
  - Check: `grep -rn "restart_gate\|restart_source\|RestartParams" crates`; each adapts, and none is left defaulting
    to `Markers` without a reason. Clippy workspace-wide.
- **Every `StartOver::Can` fixture**: `update/tests.rs:4640,5140`, `view/resolver/tests.rs:266,311`,
  `resolver/answers.rs:79` (from the scout).
  - Check: `grep -rn "StartOver::Can" crates`; each gets `kind: RestartKind::Markers`, unless it draws a whole-file
    done file.
- **`prepare_restart` and the gate agree** (one rule, two callers — the m6-conflicts/28 invariant).
  - Check: `gate_and_apply_agree_on_every_shape`. It is a table over `UU` with markers, `UD`, `DU`, `AU`, `UA`, `DD`,
    binary `UU`, and a still-unmerged one-sided file; for each, the gate's yes/no and kind equal what
    `apply(RestartConflict)` does.
- **A stale gate.** Check: `start_over_refuses_when_the_shape_changed_since_the_gate` (kind `Markers` sent for a
  `UD` stage set → `RestartChanged`, nothing written, no journal line).
- **Each whole-file shape restores the right side.** Check: `start_over_restores_the_surviving_side`, four rows,
  including one after a **delete** choice (the file is gone from disk, and the start-over writes it back).
- **The marker start-over is unchanged.** Check: the existing restart tests stay green unedited apart from the added
  `kind: Markers`; the marker box's text is byte-identical (`restart_box_words_for_markers_unchanged`).
- **Journal and undo**: folded into the live entry like a marker start-over. `u` after the operation ends restores
  the state from before the restart, as it does for markers. Check: `undo_after_a_whole_file_start_over_matches_markers`.
- **Ticket 57's undo gate**: this ticket's steps inside a Graft-begun merge fold into the live entry, so they are not
  entries of their own and never meet the gate. Check: the test above runs its `u` after the merge is finished.
- **The app**: the box's words and command line, the landing card, the list row, and `finish::forget`.
  - Check: `d_on_a_done_modify_delete_file_reopens_its_card` (graft `it/resolver/`); `done_whole_file_offers_start_over`
    (the done card's keys show `d start over`); `whole_file_box_names_the_surviving_side`
    (`git checkout --ours -- gone.txt`).
- **A shell-begun merge is unchanged.** Check: `start_over_of_a_whole_file_is_still_refused_in_a_shell_merge`, with
  today's `NoStagesKept` words.
- **Hints**: `restart: true` on a done whole-file file. The card list is untouched (decision 5). Check:
  `every_key_the_bar_draws_is_answered`.
- **Captures**: `fix-done-*-200`, and any other capture of a done whole-file file, gain `d start over`. Regenerate
  them and list them in NOTES. **Lane U's 55 and 56 also rewrite `fix-done-*` and other resolver captures.** Whichever
  lane merges second regenerates the shared captures on top of the first, and never merges capture text by hand
  (conductor).
- **The walk**: scenario `walk-back-a-whole-file-choice`. It starts on `notes.txt` after `gone.txt` was kept; the keys
  are `k d → ⏎` (4). Check: `journey_counts_match_the_note` with the row below.
- **Docs**: `crates/graft-ops/MAP.md` (`restart_gate`, `RestartPlan`, `RestartKind`, `OneSided`, `RestartChanged`,
  `plan_restart`'s flag); `crates/graft/MAP.md` §update::resolver (`DoneOpen`, the box's words by kind),
  §update::confirm (`RestartConflict { path, kind }`).

## Acceptance (name the verifying test at tick time)
- [x] `start_over_brings_back_a_modify_delete_choice` (graft-ops `tests/it/restart_whole_file.rs`): red first
      (`the gate refused: gone.txt: deleted by them — no markers to take from`, i.e. `NotMarkerShape`), then green.
- [x] `start_over_restores_the_surviving_side` (graft-ops `restart_whole_file.rs`): UD keep, DU delete, AU delete, UA
      keep — stages byte-identical to the merge's, the surviving stage's blob on disk (the delete rows write the file back).
- [x] `gate_and_apply_agree_on_every_shape` (graft-ops `restart_whole_file.rs`; binary answer in NOTES)
- [x] `start_over_refuses_when_the_shape_changed_since_the_gate` (graft-ops `restart_whole_file.rs`; both ways round,
      journal bytes / refs / index / status / file bytes unchanged)
- [x] `undo_after_a_whole_file_start_over_matches_markers` (graft-ops `restart_whole_file.rs`)
- [x] `restart_box_words_for_markers_unchanged`, `whole_file_box_names_the_surviving_side` (graft
      `update::resolver::tests::file_list::start_over`; plus `whole_file_box_with_no_operation_open`)
- [x] `d_on_a_done_modify_delete_file_reopens_its_card` (graft `it resolver::file_list::start_over`),
      `done_whole_file_offers_start_over` (graft `update::resolver::tests::file_list::start_over`)
- [x] `start_over_of_a_whole_file_is_still_refused_in_a_shell_merge` (graft-ops `restart_whole_file.rs`)
- [x] `journey_counts_match_the_note` with `walk-back-a-whole-file-choice` = 4 (scenario in
      `m5_walk/conflict_journeys.rs`, run by `m5_walk::every_scenario::journey_walk_back_a_whole_file_choice`).
- [x] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-start-over-whole-file-120x40.txt`, three
      frames: `gone.txt` done (bar with `d start over`); the box; the card after `→ ⏎`. Asserted by
      `d_on_a_done_modify_delete_file_reopens_its_card`; judged by eye: the user's.
  - **What right looks like:** frame 1's bottom row contains `d start over`. Frame 2 contains
    `the choice comes back: keep or delete` and `git checkout --ours -- gone.txt`, and does **not** contain `--merge`,
    with `[ Cancel ]` under the cursor. Frame 3's card reads `deleted by them · o keeps the file  t deletes it`, and the
    list row reads `keep or delete`.
- [x] MAP.md updated (doc, no test): graft-ops `RestartParams`, `RestartKind`/`OneSided`, `Refusal::RestartChanged`,
      `prepare_restart`, `restart_gate`/`RestartPlan`; graft `ConfirmThen::RestartConflict`, `start_over_refusal`/box
      words, `jobs::conflict` `StartOver::Can { kind }`.

## For the conductor (exact text, in this spec's merge commit)
Add to the `journey-counts` fence, after `walk-back-a-done-file`:
```
walk-back-a-whole-file-choice 4                     # batch 7 (approved 2026-09-24): k d → ⏎ · ticket 50 → 3 (k d y) · Tower (?)
```
**Done by the builder** in this branch (`docs/design-notes/m6-conflicts.md`, the line above verbatim):
`journey_counts_match_the_note` asserts walked == noted, so the scenario could not land green without it.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium. One context.

## NOTES
Status: done. No compaction. Scout: `.scratch/m6-conflicts/scouts/57-58.md` §58 (used; its contradictions 5 and 6
are what decisions 1 and 4 answer).

- **Where the plan is built.** `folded_step` takes the one plan at `crates/graft-ops/src/apply/fold.rs:129`
  (`let plan = op.plan(facts);` in `inside_live_operation`); the entry-of-its-own path at `apply.rs:209`. Both read
  `plan_restart`, whose flag is `params.kind.checkout_flag()`.
- **Binary (decision 3), today's answer:** a binary `UU` (`pic.bin`, NULs on every side) chosen whole
  (`ResolveWholeFile` ours) in a Graft-begun merge: the gate answers `Ok(RestartPlan { source: Backup, kind: Markers })`
  and the act runs `checkout --merge`, which puts all three stages back (`ls-files -u` lists 1, 2, 3). What git leaves
  on disk for the binary file is not asserted.
- **What changed in existing tests, and why** (the ticket said "unedited apart from `kind: Markers`"; these asserted
  the refusal this ticket lifts):
  - graft-ops `restart_gate::a_whole_file_done_in_an_app_merge_is_not_marker_shape` → renamed
    `…_is_whole_file`, asserts `WholeFile { DeletedByThem }`.
  - graft-ops `restart_gate::the_gate_and_the_restart_agree` sends the gate's kind (as the app does) and checks
    markers only for `Markers`.
  - graft `it resolver::file_list::start_over::a_whole_file_done_file_offers_no_start_over` → replaced by
    `d_on_a_done_modify_delete_file_reopens_its_card`.
  - graft `capture_cannot` (`fix-done-start-over-cannot-120x40/200x50`): Graft's own merge no longer has a "cannot"
    whole-file done file, so the capture was retaken on the shell-begun `gone_and_notes_merge` (`NoStagesKept`,
    `can't start over · gone.txt: the conflict's sides were not kept`). Its two captions updated to match.
- **Undo, precisely:** after the merge is finished, `u` takes the whole merge back — to `main`'s tip and bytes — for a
  whole-file start-over exactly as for a marker one (same entry count at the finish, same status and bytes after `u`).
  "Restores the state from before the restart" in the spec is that: the restart folds, so it has no `u` of its own.
- **Default `Markers` in the app, with a reason:** `start_over_kind` (`update/resolver/restart.rs`) answers `Markers`
  for a file open with its blocks (no gate answer carried): ops' `Index` arm is always `Markers` there, and a stale
  answer is ops' `RestartChanged` to refuse.
- **Verified by test, not by eye:** the box, card and list words (frames asserted). Colour **unverified**.
- **Not verified:** `every_key_the_bar_draws_is_answered` passes but its fixtures do not include a done whole-file
  file; the bar ⇔ box agreement for one is covered by `done_whole_file_offers_start_over` instead.

**Touches:**
- Box text: marker box byte-identical (`restart_box_words_for_markers_unchanged`, `d_starts_over_behind_a_box` unedited
  and green); whole-file box new (`whole_file_box_names_the_surviving_side`, `…_with_no_operation_open`).
- Journal/undo: folded into the live entry, no line of its own (`undo_after_a_whole_file_start_over_matches_markers`,
  `d_on_a_done_modify_delete_file_reopens_its_card` entry count); refusal writes nothing
  (`start_over_refuses_when_the_shape_changed_since_the_gate`); ticket 57's undo gate not met (the `u` runs after the
  finish).
- Every `d` path: `grep -rn "restart_gate\|restart_source\|RestartParams\|StartOver::Can" crates` — every
  `RestartParams` builder sends `Markers` (existing marker tests) or the gate's kind; the app's done file and whole-file
  card read `StartOver::Can { kind }`; `ConfirmThen::RestartConflict { path, kind }` → `RestartParams { path, kind }`;
  shell-begun merge unchanged (`start_over_of_a_whole_file_is_still_refused_in_a_shell_merge`,
  `a_shell_merge_done_file_answers_d_in_words` green). `graft-ui`: no file changed (decision 5).
- Captures: new `fix-start-over-whole-file-120x40.txt`; retaken `fix-done-start-over-cannot-120x40.txt`,
  `fix-done-start-over-cannot-200x50.txt`; `fix-done-start-over-can-120x40.txt` unchanged (a marker file). No other
  capture drifted (full `nextest` green, `check_shot` fails on drift). Contact sheet rerun
  (`scripts/contact-sheet.sh --captions-order m6 docs/shots/m6`): part 1 198 421 B, part 2 133 662 B; the split point
  unchanged. Captions tsv: two rows reworded, one row added.
- Docs: `crates/graft-ops/MAP.md`, `crates/graft/MAP.md`, `TESTING.md` (journey row), the design note's
  `journey-counts` fence.
