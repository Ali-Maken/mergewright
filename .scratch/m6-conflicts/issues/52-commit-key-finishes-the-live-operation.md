# commit-key-finishes-the-live-operation: `c` in the graph or the changes pane, mid-operation, does what the banner's `c` does, where the banner's `c` acts

Status: ready (unblocked 2026-09-24: P14 approved, picks item 12; the keymap is edited, §2.4 and §2.6a. Reconciled
with scout 52 on 2026-09-25)
From: `docs/reviews/m6-usability.md` M3 (:96-97), the rest of the direction.
Build with: builder-opus. Why: brownfield work across two crates, plus a public graft-ui type change that every hint
test enumerates. Not Fable: the key route is one guarded call, and the bar-vs-key agreement test makes drift visible.
Scout: `.scratch/m6-conflicts/scouts/52.md` (main @ 5d0947e7). Read it before any source.
Needs:
- `commit-key-never-opens-a-refusing-editor` merged: `live_operation`, `operation_refusal`, and the check in `open`.
- `finish-summary-is-a-preview` and `last-file-opens-the-summary` merged: the summary's `y`. They share
  `update/finish` and `update/resolver` with this ticket.
- **Ticket 60 merged. 52 rebases on it.** 60 adds a `continue_pressed` caller (`finish_if_owed`) and edits its
  Resolver branch (`banner.rs:456`). 52 **does not edit `banner.rs`**. It adds one caller from `update/keys.rs`.
- `graft::update::banner::continue_pressed` and `graft::update::finish::open_finish`, as they are after 60 merges.
Crates: graft (`update/keys.rs`, `update/commit_editor.rs`, `view/rows.rs`, tests) and graft-ui (a **public type
change**: `HintContext::ChangesPane` gains a field, `HintContext::OperationLive` gains one, plus a new enum; also
`hints/lists.rs`, `catalogue.rs`, `graph_screen/tests.rs`).
- **This is wider than the old spec said.** The old spec said graft-ui "hints only". The behaviour in scope is the
  same: the old spec already asked for per-kind labels. But no hint context carries the operation kind (scout 52,
  item 4), so the labels need the type change and the `view/rows.rs` construction.
Lane: wave 2, third, after 60.

## Read first
- This spec's **What `c` does, per kind** table. It replaces the old one-line guard.
- `docs/design-notes/m6-usability-fixes.md`:
  - P14 (:122-129);
  - the Journeys row "Finish a merge from the pane" (:61);
  - "Where each event lands him" (:66-71);
  - Decided here, item 2 (:84-91): the pane words.
- `docs/design/02-keymap.md`:
  - §2.4 (`c` / `a` row, :244);
  - §2.6 `c`;
  - §2.6a (the `c` row, :324);
  - §2.6b.
- `crates/graft/MAP.md`:
  - `commit_editor`: `live_operation` :242, `operation_refusal` :245, `open` :246;
  - `update::banner`: `continue_pressed` :304;
  - `update::finish`: `ReturnTo` :357, `open_finish` :362.
- `crates/graft-ui/MAP.md`:
  - `hint_context` :127;
  - `OperationLive` :132;
  - `Resolver { .. finishable }` (:22). This is the precedent for `c finish` / `c continue`.

## What `c` does, per kind (graph or changes pane, `live_operation(state) == Some(live)`)
The pane's `c` calls the banner's `c` handler only where that handler **acts**. Otherwise it keeps M3a's words.

| Live operation | Pane `c` calls | What he sees (rule 3: every row is visible) | Bar label |
|---|---|---|---|
| Merge, begun in Graft | `banner::continue_pressed(.., ReturnTo::Banner, ..)` | The finish summary. Files still unmerged → the banner's refusal words (sticky). Status read out → `CHECKING`. | `c finish` |
| Merge, begun in the shell | same | same | `c finish` |
| Rebase, cherry-pick or revert, begun in Graft | same | Whatever the banner's `c` shows from the same state (continue, or the same refusal words). | `c continue` |
| Rebase, cherry-pick or revert, begun in the shell | `commit_editor::open` (no change) | M3a's words: the whole sentence as a sticky reason. **No command is sent.** | no `c` |
| Bisect (either) | `commit_editor::open` (no change) | M3a's bisect words, sticky. No command. | no `c` |

Why the guard is narrower than `live_operation(state).is_some()` (scout 52, item 1):
- For a bisect, `continue_pressed` → `resume` returns false **and says nothing** (`banner.rs:574`).
- For a shell-begun non-merge, it emits `Command::Resume`, and ops refuses later.
- The plain guard would drop M3a's pane words for both.

## Scope
**In:**
- A new predicate in `update/commit_editor.rs`, next to `live_operation`, and **the only place the table's
  rows are decided**: `pane_c_acts`. Two callers read it:
  - the key route in `update/keys.rs` (`graph_key` / `is_commit_key`, `keys.rs:260-286`), before `commit_editor::open`;
  - the hint construction in `view/rows.rs` (:376-392 for `OperationLive`, :554 for `ChangesPane`).
- The graph and changes-pane bars label `c` per the table. The label goes where `c commit` sits now.
- **`a` is unchanged.** It refuses in words (M3a) and never aborts. Its bar label is not this ticket's.

**Out:**
- The banner's own keys and the banner's `c`. `banner.rs` is not edited. **The banner's silent `c` on a bisect**
  (`banner.rs:574`) is a separate defect. The conductor files it (text below).
- The summary's confirm key (`finish-summary-is-a-preview`).
- A `ReturnTo::Pane` variant. `Esc` from the summary goes to the banner, the operation's home.

## Find and pin first (before any change; stop if the finding differs)
1. **Focus after `Esc` from a summary the pane opened.** Scout 52 could not confirm it. `finish::keys::escape`
   (`finish/keys.rs:289-298`) sets `banner_focus = true` and does not touch `detail.focus`.
   - Write `esc_from_a_summary_opened_in_the_pane_lands_on_the_banner` against **today's** `ReturnTo::Banner` path.
     Open the summary from the banner, with the changes pane focused underneath, then press `Esc`.
   - Pin what the frame and the next key actually do:
     - (a) the summary is closed and `finish_draft` is kept;
     - (b) the header draws the banner focused (`BannerView.focused`);
     - (c) the footer context is `Banner`;
     - (d) the next key, for example `⇥`, is handled by `banner_key` and not by the pane;
     - (e) the value `detail.focus` holds. Record it in NOTES.
   - **Spec expects (a)–(d).** If any differs, or the banner is not drawn at all: **stop and report
     `blocked: Esc lands <where>`**. Don't add a `ReturnTo` variant.
2. **Graph-row frames during a live operation are wrapped.** List every path in `view/rows.rs` that builds
   `WorkingTreeRow`, `BranchRow`, `PlainRow` or `Marked` while `live_operation` is `Some` and not wrapped in
   `OperationLive` (for example, when `one_tab_to_banner` is false).
   - None: pin that with a test.
   - Any: **stop and report them.** The label would not reach that frame, and that needs a spec decision.
3. **Fixture.** `live_operation` reads InGraft only when `live_entries` is non-empty (`commit_editor.rs:58-66`).
   Every InGraft fixture sets it. Check this with an assertion inside each fixture, not by trusting it.

## The failing test first
`crates/graft/tests/it/resolver/shell_begun.rs`: `c_in_the_changes_pane_opens_the_summary`.
- Setup: 20's J3 state (a shell merge, resolved), the changes pane focused. Press `c`.
- Assert: `finish.is_some()`, `return_to == ReturnTo::Banner`, `commit_editor.is_none()`.
- Red after M3a: `c` refuses in words.
- Siblings:
  - `c_in_the_graph_opens_the_summary_on_a_graft_begun_merge`
  - `c_in_the_graph_continues_a_graft_begun_rebase`: emits exactly the commands and status the banner's `c` emits
    from the same state. `live_entries` is set.
  - `c_in_the_pane_on_a_shell_begun_rebase_refuses_in_words_and_sends_nothing`: M3a's words, `commands` empty.
  - `c_in_the_pane_mid_bisect_refuses_in_words_and_sends_nothing`: status is the bisect words (non-empty), and
    `commands` is empty.

## Signatures
```rust
// graft, update/commit_editor.rs: new. The only place the table's rows are decided.
/// True where the banner's `c` acts on this operation: any merge; a rebase, cherry-pick or revert begun in Graft.
/// False for a bisect and for a shell-begun non-merge (the banner's `c` is silent or refused later there).
pub(crate) fn pane_c_acts(live: (InProgress, Begun)) -> bool;

// graft, update/keys.rs: the `c` route, before commit_editor::open (`keys.rs:281-286`):
//   if !amend {
//       if let Some(live) = live_operation(state) {
//           if pane_c_acts(live) {
//               banner::continue_pressed(state, commands, ReturnTo::Banner, next_id);
//               return true;
//           }
//       }
//   }
//   // otherwise, unchanged: commit_editor::open (M3a's words for any live operation)
// `continue_pressed`: `(&mut AppState, &mut Vec<Command>, ReturnTo, next_id)` (graft MAP :304). `a` keeps M3a's refusal.

// graft-ui, hints: new enum and two changed variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiveC {
    Finish,   // `c finish`
    Continue, // `c continue`
    Refuses,  // no `c` hint
}
// HintContext::ChangesPane { unmerged, live: Option<LiveC> }  // None: no operation live → `c commit`, as today
// HintContext::OperationLive(Box<HintContext>, LiveC)         // was OperationLive(Box<HintContext>)

// graft, view/rows.rs: the one mapping.
//   live_operation(state).map(|live| match (live.0, pane_c_acts(live)) {
//       (_, false) => LiveC::Refuses,
//       (InProgress::Merge, true) => LiveC::Finish,
//       (_, true) => LiveC::Continue,
//   })
```

## Touches (each entry, and how the builder checks it)
- **The banner's `c`:** unchanged, and called from three places after 60. The pane's call is new.
  - Check: `c_from_the_pane_and_from_the_banner_emit_the_same`, a table over:
    - merge InGraft;
    - merge InShell;
    - rebase InGraft;
    - merge InGraft with unmerged files remaining.
  - For each row, the same `commands`, the same `status`, and the same `finish` state.
- **Bar and key agree:** `the_bar_offers_c_exactly_where_c_acts`. Over all five table rows × {changes pane, graph
  row}:
  - the bar has a `c` hint ⇔ pressing `c` sends a command or opens the summary;
  - `c commit` never appears while an operation is live.
- **Journal and undo.** A merge finished from the pane makes exactly one `FinishMerge` entry, and `u` lands on the
  banner as ticket 41 says.
  - Check: `finishing_from_the_pane_journals_once_and_u_lands_on_the_banner`.
- **The summary's `Esc`:** find-and-pin 1.
- **M3a's pane words** now answer `c` only for the table's last two rows, and `a` everywhere. (The old reference,
  `the_pane_form_drops_only_esc`, does not exist; scout 52, item 3.)
  - Check: the two `…refuses_in_words_and_sends_nothing` tests and `a_mid_operation_still_refuses_in_words`.
- **`update/keys/bound.rs:47-48`** (the `c commit` / `a amend` table) and its tests (`update/tests.rs:3576`, `:4847`).
  - Check: `every_bound_action_is_hinted_live_or_is_named_here` is green. If `c` needs its live forms named there,
    name them. Don't delete the rows.
- **`every_context`** (`graft-ui catalogue.rs:34`). It must yield `ChangesPane` with each `live` value and
  `OperationLive` with each `LiveC`, via `operation_live_form`.
  - Check: `every_key_the_bar_draws_is_answered` and `every_context` coverage are green over the new forms.
- **Per-context test counts** (`graft-ui graph_screen/tests.rs:1982-1983`). Update each count to the new number, and
  say in NOTES why each changed.
- **Captures.** Every capture of the graph or changes pane during a live operation changes its bar. Scout 52's grep
  matched 119 files, too broad to list by hand.
  - Check: run the capture tests. `git diff --stat docs/shots` names what changed, and NOTES lists each file with the
    test that regenerated it.
  - If a changed bar has no test that regenerates it, report it. Don't hand-edit.
- **The walk.** `editor-to-finish` becomes `c y` = 2 (the summary's `y` is built by then).
  - Check: `journey_counts_match_the_note` (`tests/it/m5_walk/conflict_journeys.rs:1237`) is green with the row below.
  - Update `.scratch/m6-conflicts/pty-walk-m6.py`.
- **Other screens that draw the same state.**
  - The banner's `every file done ✓ · c finishes the merge`: unchanged.
  - The resolver's `finishable` (`c finish` / `c continue`): unchanged. Check it is still green.
- **Keymap:** already edited (§2.4 :244, §2.6a :324). The builder reads them and does not edit them.
- **TESTING.md** (the acceptance ticket edits it): the `editor-to-finish` row → keys `c y`, count 2.
- **MAP.md:**
  - graft: `commit_editor` (`pane_c_acts`), `update::keys` (the `c` route), `view::rows` (`hint_context`);
  - graft-ui: `HintContext::ChangesPane`, `OperationLive`, `LiveC`, `every_context`.

## Acceptance (name the verifying test at tick time)
- [ ] Find-and-pin 1: `esc_from_a_summary_opened_in_the_pane_lands_on_the_banner`, with (a)–(d) holding and (e)
      recorded in NOTES.
- [ ] Find-and-pin 2: the test that pins "no unwrapped graph-row context while live" (name it at tick time).
- [ ] `c_in_the_changes_pane_opens_the_summary`: red first, then green.
- [ ] `c_in_the_graph_opens_the_summary_on_a_graft_begun_merge`
- [ ] `c_in_the_graph_continues_a_graft_begun_rebase`
- [ ] `c_in_the_pane_on_a_shell_begun_rebase_refuses_in_words_and_sends_nothing`
- [ ] `c_in_the_pane_mid_bisect_refuses_in_words_and_sends_nothing`
- [ ] `c_from_the_pane_and_from_the_banner_emit_the_same`
- [ ] `the_bar_offers_c_exactly_where_c_acts`
- [ ] `finishing_from_the_pane_journals_once_and_u_lands_on_the_banner`
- [ ] `a_mid_operation_still_refuses_in_words`
- [ ] `every_key_the_bar_draws_is_answered`, `every_bound_action_is_hinted_live_or_is_named_here`, `every_context`
      coverage: all green over the new forms.
- [ ] `journey_counts_match_the_note`, with `editor-to-finish` = 2.
- [ ] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-c-in-pane-finishes-187x50.txt`.
  - Scene: two frames. Frame 1: the changes pane over a resolved shell merge, its bar showing `c finish`. Frame 2:
    after `c`, the summary.
  - **What right looks like:**
    - frame 1's bar contains `c finish` and not `c commit`;
    - frame 2's top border is `╭ Preview: finish merging feature into main`;
    - frame 2's bottom border starts `╰ y commit`.
- [ ] MAP.md updated (doc, no test).

## For the conductor (exact text, in this spec's merge commit)
In the `journey-counts` fence, replace the `editor-to-finish` row with:
```
editor-to-finish              2                     # c y from the changes pane (P14 approved 2026-09-24; usability M3) · was 5 (c ⇥ ⇥ c ⏎)
```

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium: a public graft-ui type change and its catalogue and counts, plus about 12 tests. One context. If the
capture regeneration or find-and-pin 2 grows it past one context, hand off at the seam: graft-ui committed first.

## NOTES
