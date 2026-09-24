# lease-only-while-the-rejected-tip-stands: `P` offers the force-with-lease box only while the tip that was rejected is still the tip

Status: ready
From: `docs/reviews/m6-usability.md` M4 (:99-110). A git-write safety path.
Blocked by: none. This keeps DESIGN's trigger ("the lease is offered after a non-fast-forward rejection",
`m4-remotes.md` :14) and stops offering it once the rejection no longer applies. P17 (lazygit's trigger, no rejection
first) is separate and not specced.
Build with: *(blank: brownfield; the conductor sets it after the scout, ADR-0035)*
Scout brief: how and when `AppState::refs` is re-read after a write lands (is there an in-flight marker, and can `P` run
between a write's `Done` and the refs answer?), plus every reader and writer of `last_rejected_push`, in tests too.
Needs: none. It calls `graft::refs::RefBadges::head` and `graft::update::remote::{push_params, push_key, remote_failed}`,
as merged.
Crates: graft.

## Read first
- `docs/design-notes/m6-usability-fixes.md`: "Decided here" 3 (the rule; follow it exactly), and the Journeys rows
  "Rejected push" and "Force push after his own rebase".
- `docs/research/push-pull.md` §1.1 (lazygit's trigger, #3468, #4077), §3, §4.
- `docs/design-notes/m4-remotes.md` :13-15, :67-68, :90-99, :125-126, :142-143, :160-163.
- `TESTING.md` :1502-1516 (scenarios 10 and 11; both must behave as before).
- `crates/graft/MAP.md` §`update::remote` (`RemoteState`, `push_params`, `push_box`, `remote_failed`, `UndoAfterPush`),
  and `RefBadges::head` (line 78).
- Source, re-read and confirmed by the planner 2026-09-24:
  - `update/remote/in_flight.rs:121-139` (`remote_done` clears the flag at :129);
  - `:150-171` (`remote_failed` sets it at :168-170);
  - `update/remote.rs:249-274` (a branch change clears it at :260-267);
  - `update/remote/push_box.rs:44-108` (`push_params`: the lease at :88-106).
  - `grep last_rejected_push crates` finds no other writer.

## The rule (from the note; do not reinterpret)
`P` builds the lease variant only when **both** of these hold:
- **(a)** `last_rejected_push` matches this push's branch, remote and remote branch (`push_box.rs:88-96`, unchanged).
- **(b)** `HEAD`'s tip as the refs report it now equals the tip recorded when that rejection was recorded.

Otherwise it builds the plain box.

**Unknown counts as "not the same"**, and gets the plain box. Unknown means any of:
- no tip was recorded (`state.refs` was `None` at the rejection);
- `state.refs` is `None` now;
- a refs read requested after the last write has not landed (Step 0 decides whether this can happen).

The existing clears at `in_flight.rs:129` and `remote.rs:260-267` stay. The existing refusal
`f to fetch first — nothing to lease against` (`push_box.rs:103`) is unchanged, and only reached when (a) and (b) hold.

## Scope
**In:**
- **Record the tip at the rejection:** `last_rejected_push: Option<RejectedPush>` holds the params plus the tip.
- **`push_params` applies (b).**

**Out:**
- The rejection line's wording (review m6, and `P force with lease` in `graft-ops`).
- The undo-after-push re-push: its lease comes from the journal (`repush_lease`), never from `last_rejected_push`.
- P17.
- Any ancestry walk.

## Step 0: the staleness question, by test, before the fix
`update/remote/tests.rs`: `p_between_a_writes_done_and_its_refs_read`.
- Setup: a rebase `Done` arrives (use whatever `Operated` event the M5 tests use), and the refs answer has **not**
  arrived yet. Press `P`.
- Record, in NOTES with lines: whether `P` runs at all (`operation_in_flight`?), and what `state.refs` holds.
- If `P` can run on the pre-write refs **and** the app already tracks a pending refs read: (b) treats pending as
  unknown, and this test is kept as a pin.
- If `P` can run on the pre-write refs **and nothing tracks the pending read:** **stop.** Report
  `blocked: P can read pre-write refs and no pending-refs marker exists — <file:line>`. Adding one is a state decision
  for the conductor.
- If `P` cannot run in that window: NOTES says why, with the line, and the test pins it.

## The failing test first
`update/remote/tests.rs`: `a_rebase_after_a_rejected_push_gets_the_plain_box`.
- Setup:
  - a rejection recorded through `remote_failed(.., NonFastForward, Push(params))` while the refs head is `A`;
  - then refs land with head `B` (the rebase).
- Assert: `push_params(&state).unwrap().lease == None`, and `push_box(..)`'s title starts `Push `, not `Force-push`.
- Red on today's code (lease `Some`).

## Signatures
```rust
// graft/src/update/remote.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RejectedPush {
    pub params: PushParams,
    /// `HEAD`'s tip when git refused the push: what that push sent. `None` when the refs were not read then.
    pub sent: Option<Oid>,
}
pub struct RemoteState { pub last_rejected_push: Option<RejectedPush>, /* rest unchanged */ }
// update/remote/in_flight.rs: remote_failed signature unchanged; its NonFastForward arm records
//   RejectedPush { params: params.clone(), sent: state.refs.as_deref().and_then(RefBadges::head) }
// update/remote/push_box.rs: signatures unchanged
pub fn push_params(state: &AppState) -> Result<PushParams, String>;   // applies (a) && (b)
```

## Touches (each entry, and how the builder checks it)
- **Readers and writers of `last_rejected_push`:** `in_flight.rs:129` and `:169`, `remote.rs:262-266`,
  `push_box.rs:90`, and the tests at `remote/tests.rs:281`, `:343`, `:484`, `:499`, and `view/tests.rs:3014`.
  - Check: `grep -rn last_rejected_push crates` before and after. Every hit compiles against `RejectedPush`.
  - The two view and remote tests that expect the lease box set `sent` to their state's refs head. NOTES lists them.
- **Callers of `push_params`:** `push_key`, the `.` menu's `Push (P)` (`ContextAction::Push`), and anything else
  `grep -rn "push_params(" crates/graft/src` finds.
  - Check: one test per caller, showing it gets the plain box after the tip moved.
- **The undo-after-push re-push** (`after_push.rs`, `ConfirmThen::UndoAfterPush`): unaffected.
  - Check: the existing `after_push` tests are green, and `git diff` does not touch `after_push.rs`.
- **Scenario 10 (`push-rejected-then-lease`)** and **scenario 11 (`push-lease-refused-stale`)**, `TESTING.md` :1502-1516.
  Nothing moves between the rejection and the second `P`, so both still get the lease box.
  - Check: find their tests (`grep -rn "rejected_then_lease\|lease_refused_stale" crates`).
  - A fast-tier test: run it.
  - A heavy-tier test: don't run it. Say so in NOTES, and add the fast-tier twin
    `a_rejected_push_with_the_tip_unchanged_still_offers_the_lease`.
- **The new scenario** `push-rejected-rebased-then-plain`: rejected, then `p` (the diverged line), then `r r`, then
  `P` → the plain box, `⏎` → the bare's `main` is the local tip, and the command has no `--force`.
  - Written in the same harness as scenario 10. Run it if it is fast-tier; otherwise NOTES says so and the unit test
    stands.
- **The rejection line after the rebase.** The sticky `… · P force with lease` must not still show once `r r` has
  landed, or it would promise a box `P` no longer opens.
  - Check: `the_rejection_line_is_gone_after_the_rebase_lands`. If it persists, **stop and report**: that is the status
    lifecycle, not this ticket.
- **Captures.** `m4-02-push-box-lease-80x24` and `-160x48`, `m4-01-push-box-plain-*`, `m4-06-rejected-sticky-*`, and
  `m5-ui-undo-after-push-80x24`.
  - Check: regenerate them through their tests. All must come out byte-identical, since no box's text changed. Any diff
    is a stop.
- **Other screens that draw the same state.** The graph badge `↑n` and the `.` menu's `Push (P)` label: unchanged.
  - Check: grep for the label; no diff.
- **Journal and undo.** A push is journaled and transparent to undo (`m4-remotes.md` :90-99). No journal change.
  - Check: `git diff` touches nothing under `journal`.
- **Keymap.** `P` §2.3 :196 is unchanged (no key meaning changes).
- **TESTING.md** (the conductor edits it): add scenario 13, `push-rejected-rebased-then-plain`, with the exact text
  below. Scenarios 10 and 11 are unchanged.
- **MAP.md:** graft §`update::remote`. The `RemoteState` line becomes "cleared on success or a branch change; offered
  only while `HEAD` is still the tip that was refused (`RejectedPush::sent`)". Add a `RejectedPush` line.

## Acceptance (name the verifying test at tick time)
- [ ] Step 0 done; the outcome and lines are in NOTES; `p_between_a_writes_done_and_its_refs_read` is kept as a pin (or
  the stop was reported).
- [ ] `a_rebase_after_a_rejected_push_gets_the_plain_box`: red first, then green.
- [ ] `a_rejected_push_with_the_tip_unchanged_still_offers_the_lease` (scenario 10's twin).
- [ ] `a_rejection_recorded_without_refs_gets_the_plain_box` (`sent: None`).
- [ ] `a_commit_after_a_rejected_push_gets_the_plain_box` (the tip moved forward, still diverged: git decides).
- [ ] `undo_back_to_the_rejected_tip_offers_the_lease_again`: the tip equals `sent` again, so (b) holds. That is
  correct: the remote still has what that tip lacks.
- [ ] `every_push_params_caller_gets_the_plain_box_after_a_move`
- [ ] `the_rejection_line_is_gone_after_the_rebase_lands`
- [ ] The existing `after_push` tests are green, unchanged.
- [ ] Captures byte-identical. NOTES gives the list and `git status docs/shots` clean.
- [ ] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-plain-box-after-rebase-187x50.txt`.
  - Scene: after a rejection, `p`, `r r`, then `P`.
  - **What right looks like:** the box is titled `Push main → origin/main`. Its line starts with the ahead count, for
    example `2 commits`. The command row has no `--force-with-lease`. The action reads `Push`, not `Force push`.
- [ ] MAP.md updated (doc, no test).

## For the conductor (exact text)
Add `TESTING.md` scenario 13, after :1516:
`13. **push-rejected-rebased-then-plain** — as 11 (the teammate pushed, ours locally); \`P\` \`→\` \`⏎\`: sticky \`push
rejected …\`; \`p\`: \`main and origin/main have diverged · m merge it  r rebase onto it\`; \`r\` \`r\`: rebased; \`P\`:
a box titled \`Push main → origin/main\` (not \`Force-push\`), its command without \`--force-with-lease\`; \`⏎\`: the
bare's \`main\` is the local tip, the teammate's commit its ancestor.`

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Small to medium. One context.

## NOTES
