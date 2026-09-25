# 78 — review fix for 68, 69: no failure goes unseen under a box; one write at a time; the prompt names its push

Status: draft
From: `docs/reviews/m6-final-fixes-68-69.md` Major 1, 2, 3; Minor 1, 2, 3, 5, 6, 7 (Minor 4 and parts of 5, 7 go to
KNOWN_GAPS, below). Each names its lines.
Blocked by: **77 merged** (it builds in graft-ops + graft now; both touch `update/undo.rs` launch sites and
`docs/shots/m6/fix-box-owns-the-bar-*` frame 3).
Crates: **graft only**, plus `TESTING.md`, `docs/shots/`. **graft-ui: none.** The credential footer is graft's
`SanitizedText` (`CredentialView::footer`), and its width comes from `graft_ui::credential_prompt::credential_area`
(pub). If the fit needs a graft-ui change: `blocked: needs graft-ui`.
Build with: **builder-opus** (conductor, after scout 78). Brownfield across `update/` and `view/` (ADR-0035). Two
structural parts: one seam in `update::update`, and one guard at the one place `operating` is set.
Scout: `.scratch/m6-conflicts/scouts/78.md` (done). Findings to fold in: the two resolver callers already refuse (`resolved.rs:45`, `whole_file.rs:80`); the editor guard `commit_editor.rs:513` covers commit; `wait for … to finish` is built twice (`remote.rs:342`, `preview.rs:284`), make it one; unguarded: `name_prompt.rs:278,286`, `confirm.rs:206`, `preview.rs:706/717`; pin whether confirm/preview can open while a write runs. Overlaps 77 at `undo.rs:256,395`, `confirm.rs:212-225`: rebase on 77 first. Questions below.

## Read first
- The review, whole (≤ 1 page of findings).
- Ticket 69 "Decided here" 1-6 and NOTES; ticket 68 "Decided here" 2.
- `crates/graft/MAP.md`: `Status` (:190), `text_field_open`/`modal_owns_keys` (:188-189), `operation_in_flight`/`launch`
  (:226-227), `update::credential` (:269-278), `preview_refusal` (:398), `fetch_key` (:423), `update::name_prompt`
  (:460-467), `credential_view` (:667), `graph_view` (:680), `modal_owns_bottom_row`/`modal_problem`/
  `bottom_row_problem`/`graph_problem` (:687-690).
- `docs/research/ux-principles.md` H1, §3.2.

## Scout questions (≤ 1 page, lines on main after 77)
1. **Every `operate::launch`/`launch_unstoppable` caller** and every other setter of `operating`. From a grep at
   `1c14648f` (conductor's list; not verified by reading):
   - no in-flight check in the same function: `name_prompt.rs:278,286` (`create`, the review's hole);
     `confirm.rs:206,236` (`act`; only `RestartConflict` guarded, `:232`); `commit_editor.rs:784` (`commit`; is
     `editor_key`'s `:513` its guard?); `resolver/resolved.rs:49` (`mark_resolved`), `resolver/whole_file.rs:84`
     (`whole_file_take`; the resolver refuses to *open* over a write, `resolver/open.rs:66`, but can one start while
     it is open?); `preview.rs:706,717` (the preview's `y`; `preview_refusal` runs at open only).
   - guarded earlier in the function: `operate.rs:239,472,617`; `remote.rs:336,368`; `stash.rs:428,451`;
     `undo.rs:256,395`; `banner.rs:595`.
   For each: reachable with a write running (how), and whether the key's modal stays open after it.
2. Every reader of `AppState::status` that draws it (graph footer, `draw_status_or_hints` bars, stash overlay notice,
   status screen `problem`, commit editor, resolver notice row).
3. The one builder of `wait for <running> to finish` (`fetch_key`, `preview_refusal`): one function or two copies?
4. Where the running write's description (`push main → origin/main`) lives when the credential prompt opens, and who
   builds `CredentialView::footer` today.
5. What `Esc` does today with a `Failed` status standing and a write running.

## Decided here
1. **Major 1: both options.** Rule, in his terms: *a message that arrives while a box covers the bottom row waits for
   the box to close; a failure is shown when it closes, even if the box's own act started something new.*
   - `acknowledge` skips while a modal owns the bottom row. A `Done` that arrives under the box stays; the first key
     after the box closes acknowledges it, as today.
   - A failure (`Failed`/`Sticky`) that arrives under a modal is copied to `AppState::unseen`. One seam catches it: the
     end of `update::update`, comparing `before` and `next`. It is kept only while the modal stands both before and
     after the event. When the modal closes, `unseen` is dropped if `status` still holds that failure (the status
     draws itself). Otherwise it is kept.
   - **The modal's act writes `Busy` over it** (the review's `topic ⏎`): the row draws `unseen` in place of the
     status (graph footer and every `draw_status_or_hints` bar, through `shown_status`). It has its keys, or
     `esc dismiss`. `Esc` only dismisses it: it does not cancel the new write. That is what the row says; a second
     `Esc` cancels. Any other key dismisses it and acts, and does not also acknowledge the status under it. The status
     under it (the create's `Busy`/`Done`) is drawn next.
   - One slot: two failures under one box keep the later one, as the bare status line does today (KNOWN_GAPS row).
   - A `Done` that the act replaces is superseded, not held: no work is at risk, and the graph shows the result.
   - Moved: `modal_owns_bottom_row` becomes an `AppState` method, because `update` must not call `view`. The view
     callers switch to it.
2. **Major 2: the prompt names its push, in the box's bottom border.** `esc cancel the push` becomes
   `esc cancel push main → origin/main`: the running write's own description, the words its `Busy` line draws.
   - Not a preference question. It puts back the information F9 put on the screen on purpose, in the place that
     already names what `Esc` cancels. The title keeps the host (M4's decision).
   - If it does not fit the box (`credential_area(frame).width - 4`), today's `esc cancel the <verb>`.
   - Caption row 122 is reworded: every frame's bottom row is `⏎ send  esc cancel`, and the push is named in the
     border. F9 is superseded by 69 Decided 1, with 78 Decided 2 as where the information went.
3. **Major 3: one write at a time is enforced where `operating` is set, not at each key.**
   - `claim_write` is the only setter of `operating`. It refuses without writing anything while a write runs.
   - `launch`/`launch_unstoppable` return its `Result`, `#[must_use]`, so the compiler lists every caller.
   - Callers whose modal closes on the act word the refusal as `f` does (`Status::Failed(InFlight::words())`).
   - The name prompt refuses **every kind** into its own refusal, keeps the typed name, leaves `status` and
     `operating` alone, and plans nothing.
   - Any other caller whose modal stays open after the refused key: **stop**, `blocked: needs a decision`, naming it.
     Do not add a refusal field on the spot (69 Decided 3).
   - Existing early guards stay (they run before side effects). Their now-unreachable `Err` arm words the same
     refusal, commented.
4. **Minor 2: the refusal goes with its cause.** `NamePrompt::refusal` stops holding words. `NotNow` is recomputed by
   the view each frame (`not_now`), so the line goes when the fetch ends. `⏎` again then creates: **1 key, no
   retyping**.
5. **Minor 1:** a finishing skip reads `<d>, skipped a commit ✓`. That is true whichever commit it was. Counting skips
   stays out (68).
6. **Minor 3:** the commit-graph offer row is blanked under a modal (`modal_blanked`). Its `B`/`X` are swallowed by
   every modal (UX §3.2).
7. **Minor 5 (cheap half):** the four M4 pty captures and their m4 caption rows are marked in their header line as
   predating 69/78, as 76 did for m2. Retaking them needs the pty walk: that goes to KNOWN_GAPS.
8. **Minor 6:** all three test gaps filled (Acceptance). **Minor 7:** the `jobs/operate.rs` inline test module moves to
   `jobs/operate/tests.rs`. The size rows are the conductor's.

## Signatures
```rust
// graft, update/state.rs
/// A failure that arrived while a modal owned the bottom row and was replaced before the row drew it
/// (m6-conflicts/78). `None` whenever `status` itself still holds it.
pub(crate) unseen: Option<Status>,
impl AppState {
    /// Credential prompt, name prompt, confirm box, Ghost Preview (moved from `view::status`, unchanged).
    #[must_use] pub(crate) fn modal_owns_bottom_row(&self) -> bool;
    /// What the status rungs draw once no modal owns the row: `unseen` while it stands, else `status`.
    #[must_use] pub(crate) fn shown_status(&self) -> &Status;
}
impl Status { #[must_use] pub(crate) fn is_failure(&self) -> bool; } // Failed | Sticky

// graft, update/mod.rs — last thing `update` does, every event.
fn keep_unseen_failure(before: &AppState, next: &mut AppState);
// graft, update/keys.rs — before `acknowledge` in `key_event`. `true` = consumed (`Esc`).
pub(super) fn dismiss_unseen(next: &mut AppState, key: &KeyPress) -> bool;

// graft, update/operate.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InFlight { pub(crate) running: String }
impl InFlight { /// `wait for <running> to finish`: the one wording (fetch_key, preview_refusal call it).
    #[must_use] pub(crate) fn words(&self) -> String; }
/// The only setter of `AppState::operating`. Writes nothing on `Err`.
pub(super) fn claim_write(state: &mut AppState, id: JobId) -> Result<(), InFlight>;
#[must_use] pub(super) fn launch(next: &mut AppState, commands: &mut Vec<Command>, op: Operation,
    next_id: &mut impl FnMut() -> JobId) -> Result<(), InFlight>;
#[must_use] pub(super) fn launch_unstoppable(next: &mut AppState, commands: &mut Vec<Command>, op: Operation,
    why: &'static str, opened_by: Option<&'static str>, next_id: &mut impl FnMut() -> JobId) -> Result<(), InFlight>;

// graft, update/name_prompt.rs — `NamePrompt::refusal: Option<PromptRefusal>` (was `Option<String>`)
pub(crate) enum PromptRefusal { EmptyName, NotNow }
/// Why `⏎` would be refused now: a write running (every kind), or git mid-operation (stash branch only,
/// `preview_refusal`). `None` when it may create.
#[must_use] pub(crate) fn not_now(state: &AppState, kind: &NameKind) -> Option<String>;
```

## Acceptance (name the verifying test at tick time; red observed first where marked)
- [ ] **M1** `a_failure_under_a_box_is_shown_when_it_closes` (`view::tests::bottom_row`, 120x40 and 80x24, red):
      `f` running, `b`, the fetch answers `Failed`, then another job answers `Done`, `esc`: the row reads the fetch
      failure; `esc` again: the `Done` line.
- [ ] **M1** `a_boxs_act_does_not_hide_a_failure` (red; the review's steps): `f`, `b`, fetch `Failed`, `topic ⏎`: prompt
      closed, row = the fetch failure with `esc dismiss`, `status` = the create's `Busy`. `Esc`: the row is the create's
      line and the create is **not** cancelled (`operating` unchanged, no `CancelJob`).
- [ ] **M1** `a_done_under_a_box_is_not_acknowledged_unseen` (red): `fetched` arrives under the prompt, three keys
      typed: still `Done`. After `esc`: drawn. The next key: `Ready`.
- [ ] **M1** `a_failure_that_stood_before_the_box_is_not_held`: the rejection sticky, `P`, lease `y`: `unseen` is `None`
      (69 Decided 2 unchanged).
- [ ] **M2** `the_credential_prompt_names_its_push` (both sizes): border ends `esc cancel push main → origin/main`. With a
      60-char branch: `esc cancel the push`, and nothing is cut.
- [ ] **M3** `a_write_in_flight_refuses_inside_the_prompt`, one row per kind (branch, branch + checkout, tag, stash
      branch), each with a real `f` launch (red for the first three): prompt open, name kept, row
      `wait for fetch … to finish`, `status` and `operating` unchanged, no `Operate` command.
- [ ] **M3** `one_write_at_a_time`: `claim_write` with `operating` set is `Err` and leaves the state `==` before. One row
      per scout-listed caller reachable with a write running: refused in words, nothing launched.
- [ ] **M3** `in_flight_words_are_one_wording`: `fetch_key`, `preview_refusal` and `InFlight::words` give one string.
- [ ] **Minor 2** `the_prompts_refusal_goes_with_its_cause` (red): the above, then the fetch answers: the row is the
      prompt's keys. `⏎`: creates. **Keys from the fetch's end to created: 1.**
- [ ] **Minor 6c** `git_mid_operation_refuses_inside_the_stash_prompt` (the `live_refusal_words` arm).
- [ ] **Minor 1** `a_first_stop_skip_that_finishes_says_skipped` (real repo: three commits, stop on the first, `s y`):
      `<d>, skipped a commit ✓`. `a_final_skip_says_it_skipped` reworded.
- [ ] **Minor 3** `the_commit_graph_offer_hides_under_a_box` (both sizes, lease box and credential prompt).
- [ ] **Minor 6a** `a_key_that_keeps_a_modal_open_leaves_the_status_alone`:
      - drops its `Done`→`Ready` allowance (Decided 1);
      - adds 67's refused preview and 80x24;
      - the bound becomes `checked == <the exact count>`.
- [ ] **Minor 6b** `the_search_rung_is_blank_under_the_preview` (search active, preview open).
- [ ] Capture `docs/shots/m6/fix-status-under-a-box-{120x40,80x24}.txt`, bottom rows asserted:
      1. the prompt over a failed fetch: prompt keys;
      2. after `topic ⏎`: the failure;
      3. after `esc`: the create's line;
      4. `⏎` refused in the prompt: `wait for …`;
      5. the fetch done: the prompt's keys;
      6. `esc`: `fetched ✓`.
      Colour unverified. Judged by the user (rule 6).
- [ ] Captures regenerated through their tests: `m4-credential-prompt-in-app` (the header reworded),
      `m5-ui-credential-over-graph-{80x24,160x48}`, `m6/fix-box-owns-the-bar-80x24` (frame 3, the offer row).
      Then `git status docs/shots`; list every moved file.
- [ ] m6 caption row 122 reworded (Decided 2) and a row added for the new capture
      (`contact_sheet_captions_cover_every_m6_capture`). The m4 captions rows 5 and 18 and the four M4 pty headers are
      marked (Decided 7). Sheets rebuilt (`scripts/contact-sheet.sh`), each part ≤ 200 000 B.
- [ ] TESTING.md: a row for the new capture, saying what each frame's bottom row reads. Row 1587 says the border now
      names the push (pty capture predates it).

## Touches (check each; the report's **Touches:** line says how)
- **Every launch caller** (scout Q1): the compiler (`#[must_use]`), and a row in `one_write_at_a_time`.
- **Every status reader** (scout Q2): each drawn rung reads `shown_status`, or NOTES says why not.
- **69's tests**: `a_modal_owns_the_bottom_row`, `a_busy_of_another_job_survives_a_box`, `esc_from_the_box_brings_the_
  rejection_back` green, unedited except 6a.
- **67's refused preview**: its tests green.
- **Hint bar**: `every_key_the_bar_draws_is_answered` green.
- **Journal/undo**: no entry or undo change. 77's tests green.
- **Keymap** (`docs/design/02-keymap.md`): no binding change. If §2.1's `Esc` row does not say that a drawn failure is
  dismissed before a running write is cancelled, NOTES gives the line and the conductor adds the sentence.
- **State docs**: MAP `AppState::operating` ("refused in words by `claim_write`", replacing "no-op"), `unseen`,
  `shown_status`, `modal_owns_bottom_row` (moved), `PromptRefusal`, `not_now`, `InFlight`, `launch`, `Status::is_failure`,
  `credential_view`'s footer, `resumed_words`' skip row.
- **KNOWN_GAPS**: the conductor's (rows below). Builder does not edit it.
- **Line counts** of every file this grows, in NOTES.

## KNOWN_GAPS rows (conductor files)
- n3 extended (Minor 4): the credential prompt's keys are listed twice and disagree. The border reads
  `⏎ send  ⌃U clear  esc cancel …`; the bar reads `⏎ send  esc cancel` (`m4-credential-prompt-in-app.txt:50,52`).
- Minor 5: the M4 pty captures `m4-03-credential-masked-*`, `m4-04-credential-username-*` and the m4 sheet
  (`:247,279,335,367`) predate 69/78; retake them with the pty walk at the next acceptance run.
- Minor 7: size rows M6/31 and M6/11+12 are stale. Current counts: `banner.rs` 865, `rows.rs` 759, `preview.rs` 1292,
  `state.rs` 911, `undo.rs` 1511, `view/tests/bottom_row.rs` 413, plus 78's NOTES.
- Decided 1 limit: two failures under one box: only the later is held.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`,
workspace-wide.

## Size
Medium-large, one context. If short, hand off Decided 3's caller sweep (`claim_write` + `one_write_at_a_time`) as the
second half; the name-prompt refusal stays with the first.
