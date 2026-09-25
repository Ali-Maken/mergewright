# commit-key-finishes-the-live-operation: `c` in the graph or the changes pane, mid-operation, does what the banner's `c` does, where the banner's `c` acts

Status: ready (unblocked 2026-09-24: P14 approved, picks item 12; the keymap is edited, §2.4 and §2.6a. Reconciled
with scout 52 on 2026-09-25. **Respecified 2026-09-25 after find-and-pin 2 stopped the first build: decision B,
below.**)
From: `docs/reviews/m6-usability.md` M3 (:96-97), the rest of the direction.
Build with: builder-opus. Why: brownfield work across two crates, plus a public graft-ui type change that every hint
test enumerates. Not Fable: the key route is one guarded call, the wrap is one condition, and the bar-vs-key
agreement test makes drift visible.
Scout: `.scratch/m6-conflicts/scouts/52.md` (main @ 5d0947e7). Read it before any source. The first build's NOTES
(commit 61dd6ba9) list the two unwrapped frames; read them too.
Needs:
- `commit-key-never-opens-a-refusing-editor` merged: `live_operation`, `operation_refusal`, and the check in `open`.
- `finish-summary-is-a-preview` and `last-file-opens-the-summary` merged: the summary's `y`. They share
  `update/finish` and `update/resolver` with this ticket.
- **Ticket 60 merged. 52 rebases on it.** 60 adds a `continue_pressed` caller (`finish_if_owed`) and edits its
  Resolver branch (`banner.rs:456`). 52 **does not edit `banner.rs`**. It adds one caller from `update/keys.rs`.
- `graft::update::banner::continue_pressed` and `graft::update::finish::open_finish`, as they are after 60 merges.
Crates: graft (`update/keys.rs`, `update/commit_editor.rs`, `view/rows.rs`, tests) and graft-ui (a **public type
change**: `HintContext::ChangesPane` gains a field, `HintContext::OperationLive` becomes a struct variant with two new
fields, plus a new enum; also `hints.rs` (:370-375), `hints/lists.rs`, `catalogue.rs`, `graph_screen/tests.rs`).
- **This is wider than the old spec said.** The old spec said graft-ui "hints only". The behaviour in scope is the
  same: the old spec already asked for per-kind labels. But no hint context carries the operation kind (scout 52,
  item 4), so the labels need the type change and the `view/rows.rs` construction.
Lane: wave 2, third, after 60.

## Read first
- This spec's **What `c` does, per kind** table. It replaces the old one-line guard.
- This spec's **Decision after find-and-pin 2**.
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
"Graph" means **every** graph-row frame while the operation is live, including the two frames below.

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

## Decision after find-and-pin 2 (2026-09-25): B, with `⇥ banner` as a field
The first build found two live frames that fall to `view/rows.rs:394` and get an unwrapped `PlainRow`, so the bar
kept `c commit`:
1. **Bisect.** `banner_view` is None (`banner.rs:180`, `:63`).
2. **Detail pane open, graph focused.** `one_tab_to_banner` is false (`banner/stop.rs:311`). `c` reaches `graph_key`
   (`keys.rs:281`).

**Decision:** `OperationLive` wraps the graph-row context **whenever `live_operation(state)` is Some**. The `⇥ banner`
hint becomes a field, `tab_to_banner`, set from `one_tab_to_banner`. `OperationLive` now means "an operation is
live", which is what its name says.

Why not A:
- A with `live` on each row context puts the field on four variants (`WorkingTreeRow`, `BranchRow`, `PlainRow`,
  `Marked`). A fifth row context added later can forget it, and the bar lies again. The same bug, one variant on.
- A with a second wrapper gives two nested wrappers whose order matters, and doubles the catalogue forms.
- B has one condition at one site. "Wrapped ⇔ live" is structural, so these two frames can't recur.
- B's cost is small: this ticket already changes `OperationLive` (it gains the `c` label), so every
  `operation_live_form` test is already touched.

**The two frames, after this ticket:**

| Frame | Bar | `c` does |
|---|---|---|
| 1. Bisect, graph row | the inner row's hints, **no `c` hint**, **no `⇥ banner`** (`LiveC::Refuses`, `tab_to_banner: false`) | `commit_editor::open`: M3a's bisect words, sticky. `commands` empty. |
| 2. Detail pane open, graph focused | the inner row's hints with the table's `c` label (`c finish` / `c continue` / none), **no `⇥ banner`** | the table's row: `continue_pressed(.., ReturnTo::Banner, ..)` where `pane_c_acts`; M3a's words otherwise. `Esc` from a summary opened here lands on the banner (find-and-pin 1b). |

In both frames, `⇥` keeps whatever the inner row context hints for it today. This ticket does not relabel it.

**What else `OperationLive` does.** If `operation_live_form` changes any hint beyond `⇥ banner` and `c` (for example,
it hides a key), that change now reaches frames 1 and 2 too. The builder lists each such change in NOTES.
`the_bar_offers_c_exactly_where_c_acts` checks `c`, and `every_key_the_bar_draws_is_answered` checks the rest. If any
hidden or relabelled key **acts** in frame 1 or 2, stop and report `blocked: OperationLive hides <key>, which acts
in <frame>`.

## Scope
**In:**
- A new predicate in `update/commit_editor.rs`, next to `live_operation`, and **the only place the table's
  rows are decided**: `pane_c_acts`. Two callers read it:
  - the key route in `update/keys.rs` (`graph_key` / `is_commit_key`, `keys.rs:260-286`), before `commit_editor::open`;
  - the hint construction in `view/rows.rs` (:376-394 for `OperationLive`, :554 for `ChangesPane`).
- The wrap condition in `view/rows.rs`: `OperationLive` iff `live_operation(state).is_some()`. The fall-through at
  `:394` no longer builds a bare row context while an operation is live.
- `⇥ banner` in `hints.rs` (:370-375) is drawn only when `tab_to_banner`.
- The graph and changes-pane bars label `c` per the table. The label goes where `c commit` sits now.
- **`a` is unchanged.** It refuses in words (M3a) and never aborts. Its bar label is not this ticket's.

**Out:**
- The banner's own keys and the banner's `c`. `banner.rs` and `banner/stop.rs` are not edited. **The banner's silent
  `c` on a bisect** (`banner.rs:574`) is a separate defect. The conductor files it.
- Whether a bisect should have a banner at all (`banner_view` None). Not this ticket's.
- The summary's confirm key (`finish-summary-is-a-preview`).
- A `ReturnTo::Pane` variant. `Esc` from the summary goes to the banner, the operation's home.

## Find and pin first (before any change; stop if the finding differs)
1. **Focus after `Esc` from a summary the pane opened.** Scout 52 could not confirm it. `finish::keys::escape`
   (`finish/keys.rs:289-298`) sets `banner_focus = true` and does not touch `detail.focus`.
   - (1a) Write `esc_from_a_summary_opened_in_the_pane_lands_on_the_banner` against **today's** `ReturnTo::Banner`
     path. Open the summary from the banner, with the changes pane focused underneath, then press `Esc`.
   - (1b) The same, with the detail pane open and the graph focused (frame 2):
     `esc_from_a_summary_opened_in_the_graph_with_details_open_lands_on_the_banner`.
   - Pin what the frame and the next key actually do, in each:
     - (a) the summary is closed and `finish_draft` is kept;
     - (b) the header draws the banner focused (`BannerView.focused`);
     - (c) the footer context is `Banner`;
     - (d) the next key, for example `⇥`, is handled by `banner_key` and not by the pane;
     - (e) the value `detail.focus` holds. Record it in NOTES.
   - **Spec expects (a)–(d).** If any differs, or the banner is not drawn at all: **stop and report
     `blocked: Esc lands <where>`**. Don't add a `ReturnTo` variant.
2. **Done by the first build** (commit 61dd6ba9 NOTES): two unwrapped frames, decided above. After the change, pin it
   with `no_graph_row_context_is_unwrapped_while_live`, over:
   - bisect;
   - detail pane open, graph focused;
   - the plain one-`⇥` frame (wrapped, `tab_to_banner: true`);
   - no operation live (never wrapped).
   If the builder finds a **third** path to a bare row context while live, it wraps it by the same condition and
   lists it in NOTES. There is no stop: the rule now covers it.
3. **Fixture.** `live_operation` reads InGraft only when `live_entries` is non-empty (`commit_editor.rs:58-66`).
   Every InGraft fixture sets it. Check this with an assertion inside each fixture, not by trusting it. The bisect
   fixture asserts `live_operation(state)` is `Some((InProgress::Bisect, _))` and `banner_view` is None.

## The failing test first
`crates/graft/tests/it/resolver/shell_begun.rs`: `c_in_the_changes_pane_opens_the_summary`.
- Setup: 20's J3 state (a shell merge, resolved), the changes pane focused. Press `c`.
- Assert: `finish.is_some()`, `return_to == ReturnTo::Banner`, `commit_editor.is_none()`.
- Red after M3a: `c` refuses in words.
- Siblings:
  - `c_in_the_graph_opens_the_summary_on_a_graft_begun_merge`
  - `c_in_the_graph_with_details_open_opens_the_summary` (frame 2, a merge).
  - `c_in_the_graph_continues_a_graft_begun_rebase`: emits exactly the commands and status the banner's `c` emits
    from the same state. `live_entries` is set.
  - `c_in_the_pane_on_a_shell_begun_rebase_refuses_in_words_and_sends_nothing`: M3a's words, `commands` empty.
  - `c_in_the_pane_mid_bisect_refuses_in_words_and_sends_nothing`: status is the bisect words (non-empty), and
    `commands` is empty.
  - `c_in_the_graph_mid_bisect_refuses_in_words_and_sends_nothing` (frame 1): the same assertions, graph row focused.

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
// HintContext::OperationLive {
//     row: Box<HintContext>,
//     c: LiveC,
//     tab_to_banner: bool, // `⇥ banner` drawn iff true
// }                                                        // was OperationLive(Box<HintContext>)

// graft, view/rows.rs: the one mapping and the one wrap condition.
//   let live_c = live_operation(state).map(|live| match (live.0, pane_c_acts(live)) {
//       (_, false) => LiveC::Refuses,
//       (InProgress::Merge, true) => LiveC::Finish,
//       (_, true) => LiveC::Continue,
//   });
//   match live_c {
//       Some(c) => HintContext::OperationLive { row: Box::new(row), c, tab_to_banner: one_tab_to_banner },
//       None => row,
//   }
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
  row one `⇥` from the banner, graph row with details open}, plus bisect with no banner:
  - the bar has a `c` hint ⇔ pressing `c` sends a command or opens the summary;
  - `c commit` never appears while an operation is live.
- **`⇥ banner`:** drawn only where one `⇥` reaches the banner.
  - Check: `tab_banner_is_hinted_only_where_one_tab_reaches_it`: frame 2 and bisect have no `⇥ banner`; the plain
    live frame has it, and pressing `⇥` there focuses the banner.
- **Other hints `OperationLive` changes:** now reach frames 1 and 2 (see the decision). NOTES lists each.
  - Check: `every_key_the_bar_draws_is_answered` over the new forms.
- **Bisect:** its graph bar loses `c commit`. `banner.rs` and the bisect screen are not edited.
  - Check: `c_in_the_graph_mid_bisect_refuses_in_words_and_sends_nothing`, and the bisect captures (below).
- **Journal and undo.** A merge finished from the pane makes exactly one `FinishMerge` entry, and `u` lands on the
  banner as ticket 41 says.
  - Check: `finishing_from_the_pane_journals_once_and_u_lands_on_the_banner`.
- **The summary's `Esc`:** find-and-pin 1a and 1b.
- **M3a's pane words** now answer `c` only for the table's last two rows, and `a` everywhere. (The old reference,
  `the_pane_form_drops_only_esc`, does not exist; scout 52, item 3.)
  - Check: the three `…refuses_in_words_and_sends_nothing` tests and `a_mid_operation_still_refuses_in_words`.
- **`update/keys/bound.rs:47-48`** (the `c commit` / `a amend` table) and its tests (`update/tests.rs:3576`, `:4847`).
  - Check: `every_bound_action_is_hinted_live_or_is_named_here` is green. If `c` needs its live forms named there,
    name them. Don't delete the rows.
- **`every_context`** (`graft-ui catalogue.rs:34`). It must yield `ChangesPane` with each `live` value and
  `OperationLive` with each `LiveC` × `tab_to_banner` ∈ {true, false}, via `operation_live_form`.
  - Check: `every_key_the_bar_draws_is_answered` and `every_context` coverage are green over the new forms.
- **Every `operation_live_form` test** and `hints.rs:370-375`: rewritten for the struct variant. None deleted.
  - Check: NOTES lists each changed test and what its assertion now says.
- **Per-context test counts** (`graft-ui graph_screen/tests.rs:1982-1983`). Update each count to the new number, and
  say in NOTES why each changed.
- **Captures.** Every capture of the graph or changes pane during a live operation changes its bar, **now including
  bisect frames and frames with the detail pane open**. Scout 52's grep matched 119 files, too broad to list by hand.
  - Check: run the capture tests. `git diff --stat docs/shots` names what changed, and NOTES lists each file with the
    test that regenerated it.
  - If a changed bar has no test that regenerates it, report it. Don't hand-edit.
- **The walk.** `editor-to-finish` becomes `c y` = 2 (the summary's `y` is built by then).
  - Check: `journey_counts_match_the_note` (`tests/it/m5_walk/conflict_journeys.rs:1237`) is green with the row below.
  - Update `.scratch/m6-conflicts/pty-walk-m6.py`.
- **Other screens that draw the same state.**
  - The banner's `every file done ✓ · c finishes the merge`: unchanged.
  - The resolver's `finishable` (`c finish` / `c continue`): unchanged. Check it is still green.
- **Keymap:** already edited (§2.4 :244, §2.6a :324). The builder reads them and does not edit them. `⇥ banner`
  being conditional is a hint change, not a key change: `⇥` still does what it does.
- **TESTING.md** (the acceptance ticket edits it): the `editor-to-finish` row → keys `c y`, count 2.
- **MAP.md:**
  - graft: `commit_editor` (`pane_c_acts`), `update::keys` (the `c` route), `view::rows` (`hint_context`, the wrap
    condition);
  - graft-ui: `HintContext::ChangesPane`, `OperationLive` (new meaning: "an operation is live"; `tab_to_banner`),
    `LiveC`, `every_context`.

## Acceptance (name the verifying test at tick time)
- [ ] Find-and-pin 1a: `esc_from_a_summary_opened_in_the_pane_lands_on_the_banner`, with (a)–(d) holding and (e)
      recorded in NOTES.
- [ ] Find-and-pin 1b: `esc_from_a_summary_opened_in_the_graph_with_details_open_lands_on_the_banner`, the same.
- [ ] Find-and-pin 2: `no_graph_row_context_is_unwrapped_while_live`, over its four frames.
- [ ] `c_in_the_changes_pane_opens_the_summary`: red first, then green.
- [ ] `c_in_the_graph_opens_the_summary_on_a_graft_begun_merge`
- [ ] `c_in_the_graph_with_details_open_opens_the_summary`
- [ ] `c_in_the_graph_continues_a_graft_begun_rebase`
- [ ] `c_in_the_pane_on_a_shell_begun_rebase_refuses_in_words_and_sends_nothing`
- [ ] `c_in_the_pane_mid_bisect_refuses_in_words_and_sends_nothing`
- [ ] `c_in_the_graph_mid_bisect_refuses_in_words_and_sends_nothing`
- [ ] `c_from_the_pane_and_from_the_banner_emit_the_same`
- [ ] `the_bar_offers_c_exactly_where_c_acts`
- [ ] `tab_banner_is_hinted_only_where_one_tab_reaches_it`
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
- [ ] Capture (rule 6; colour **unverified**): `docs/shots/m6/fix-c-live-frames-187x50.txt`.
  - Scene: two frames. Frame 1: mid-bisect, a graph row focused. Frame 2: a resolved merge, detail pane open, graph
    focused.
  - **What right looks like:**
    - frame 1's bar contains neither `c commit` nor `c finish` nor `c continue`, and no `⇥ banner`;
    - frame 2's bar contains `c finish`, not `c commit`, and no `⇥ banner`.
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
Medium to large: a public graft-ui type change and its catalogue and counts, plus about 16 tests and two captures.
One context. If capture regeneration grows it past one context, hand off at the seam: graft-ui committed first.

## NOTES

**Builder, 2026-09-25 (blocked at find-and-pin 2; scout 52 used). No compaction.**
- Find 1 holds. `esc_from_a_summary_opened_in_the_pane_lands_on_the_banner` (update/finish/tests.rs): (a) summary
  closed, `finish_draft` kept; (b) `banner_view().focused`; (c) `footer_context` == `banner_context`; (d) next `⇥` is
  `banner_key`'s (`banner_focus` false, `detail.focus` → `Graph`, which the pane's `⇥` from Files would not give);
  **(e) `detail.focus` stays `Files`** (`finish/keys.rs:289-298` never touches it).
- **Find 2: two live paths build unwrapped row contexts.** Observed with a throwaway probe calling `footer_context`
  (not committed); lines:
  1. **Bisect.** `live_operation` = `Some((Bisect, InShell))` but `banner_view` is `None` (`banner.rs:180` `live()?`;
     `kind_word(Bisect)` = None, `banner.rs:63`), so `footer_context` falls to `_ => hint_context` (`view/rows.rs:394`)
     → `PlainRow` (probe output). Row bar keeps `c commit` for a key that refuses. Table row "Bisect: no `c`" can't
     reach this frame.
  2. **Detail pane open, graph focused** (`DetailFocus::Graph`, merge live, banner unfocused). `one_tab_to_banner`
     is false (`banner/stop.rs:311`: needs pane closed or focus Diff) → `view/rows.rs:394` → `PlainRow` (probe
     output). Pressing `c` here reaches `graph_key` (`keys.rs:281`), so after the build `c` would finish while the bar
     says `c commit`.
  - Pane closed, merge live: `OperationLive(PlainRow)` as expected.
- Spec decision needed: where `LiveC` lives so it reaches these frames. Option A: carry it on the row contexts
  themselves (or a wrapper independent of `⇥ banner`), so `OperationLive` keeps meaning "one `⇥` to the banner".
  Option B: wrap in `OperationLive` whenever `live_operation` is Some and let the `⇥ banner` hint be conditional —
  changes what `OperationLive` means (`hints.rs:370-375`, every `operation_live_form` test).
- Remaining: everything after find 2 (all other ticks).

