# lease-only-while-the-rejected-tip-stands: `P` offers the force-with-lease box only while the tip that was rejected is still the tip

Status: ready
From: `docs/reviews/m6-usability.md` M4 (:99-110). A git-write safety path.
Blocked by: none. This keeps DESIGN's trigger ("the lease is offered after a non-fast-forward rejection",
`m4-remotes.md` :14) and stops offering it once the rejection no longer applies. P17 (lazygit's trigger, no rejection
first) is separate and not specced.
Build with: builder-opus. A git-write safety path with a new state (`RefsRead`) every refs read now feeds; the scout's
recommendation (`.scratch/m6-conflicts/scouts/49.md`).
Needs: none. It calls `graft::refs::RefBadges::head` and `graft::update::remote::{push_params, push_key, remote_failed}`,
as merged.
Crates: graft.

## Read first
- `docs/design-notes/m6-usability-fixes.md`: "Decided here" 3 (the rule; follow it exactly), and the Journeys rows
  "Rejected push" and "Force push after his own rebase".
- `.scratch/m6-conflicts/scouts/49.md` (the code facts this spec is amended from).
- `docs/research/push-pull.md` §1.1 (lazygit's trigger, #3468, #4077), §3, §4.
- `docs/design-notes/m4-remotes.md` :13-15, :67-68, :90-99, :125-126, :142-143, :160-163.
- `TESTING.md` :1502-1516 (scenarios 10 and 11; both must behave as before).
- `crates/graft/MAP.md` §`update::remote` (`RemoteState`, `push_params`, `push_box`, `remote_failed`, `UndoAfterPush`),
  `RefBadges::head` (line 78), and §`update::state` (`AppState::refs`, `head_landing`).
- Source, re-read and confirmed by the planner 2026-09-24 (all under `crates/graft/src/update/`):
  - `operate.rs:755-758`: `operated` clears `operating` **before** its refresh (`:779`, `:790`, `:832`), so
    `operation_in_flight` is false the moment `Done` lands.
  - `operate.rs:971-997`: `refresh_scope(Everything)` runs `keys::refresh_key`, which pushes `Command::ReadRefs(next_id())`
    at `keys.rs:391`. The id is not stored.
  - `mod.rs:92-103`: the launch's `ReadRefs(refs)`. The only other production `ReadRefs` issue.
  - `mod.rs:680-684`: every `Product::Refs(Ok)` is stored, whatever its id. `:686-694`: a failure keeps the old refs.
  - `remote.rs:219-230`: `request` sets `remote.reading` but leaves `remotes` `Ready` during a re-read (`:226`), so
    `facts()` (`:277-285`) serves the old facts then.
  - `remote.rs:249-274`: the branch-change clear at `:260-267`, run when the remote facts land.
  - `remote/push_box.rs:15-32` (`push_key`), `:44-108` (`push_params`, the lease at `:88-106`).
  - `operate.rs:850-872`: a failed write sets its status line (where the rejection is recorded) **before** its refresh.

## What Step 0 found (closed by the planner; not the builder's to redo)
`P` can run on the refs from before a write, and nothing tracked the pending read:
- `Done` → `operating = None` (`operate.rs:758`) → `ReadRefs` pushed with an unstored id (`keys.rs:391`) → any refs
  answer is stored (`mod.rs:681`). Between the second and third, `P` reads the old `HEAD`.
- The same window opens after `Ctrl+R` (`keys.rs:262`), an undo (`undo.rs:576`), a `--continue`/`--abort`/`--skip`
  (`banner.rs:666-747`) and a cancel (`remote/in_flight.rs:87`). All refresh through `refresh_key`.
- **Decision: a pending-refs state, `AppState::refs_read`.** `P` answers in words while a read is pending, and only when
  the answer would be the lease. Everything below follows from it.

## The rule (from the note; do not reinterpret)
`P` builds the lease variant only when **all** of these hold:
- **(a)** `last_rejected_push` matches this push's branch, remote and remote branch (`push_box.rs:88-96`, unchanged).
- **(b)** The refs are current (`RefsRead::Current`) and the remote facts are not being re-read
  (`remote.reading.is_none()`).
- **(c)** `HEAD`'s tip in those refs equals `RejectedPush::sent`.

When (a) holds:
- **(b) fails because a read is in flight** → no box. `Err(REFS_PENDING)` or, for the facts, the existing
  `still reading the remotes — try again in a moment` (`remote.rs:282`). `push_key` shows it through its existing
  `Err` path (`push_box.rs:29`).
- **(b) fails because the refs are `Unknown`** (the read failed, or an older answer overwrote a newer one) → the plain
  box.
- **(c) fails, or `sent` is `None`, or `state.refs` is `None`** → the plain box.

When (a) does not hold: the plain box, exactly as today. Pending reads do not change it.

The existing clears at `in_flight.rs:129` and `remote.rs:260-267` stay. The refusal
`f to fetch first — nothing to lease against` (`push_box.rs:103`) is unchanged, and only reached when (a), (b) and (c)
hold.

**Where it lands him:** `P` in the window → the status line says `still reading the refs — try again in a moment`,
focus unchanged, no box. The read is local and lands in milliseconds; the next `P` gets the right box. No journey count
changes.

## Scope
**In:**
- `RefsRead` and `AppState::refs_read`, fed at the two `ReadRefs` issues and the one answer site.
- `RejectedPush` holds the params plus `sent`, recorded only from current refs.
- `push_params` applies (b) and (c).

**Out:**
- The rejection line's wording (review m6, and `P force with lease` in `graft-ops`).
- The undo-after-push re-push: its params come from `UndoAfterPush.repush` (`confirm.rs:134-141` →
  `undo::after_push_chosen` → `after_push.rs:244-250`), never from `push_params` or `last_rejected_push`.
- P17. Any ancestry walk.
- Every other reader of `state.refs` or `facts()` in the window (Touches lists them). They stay as they are.
- Dropping stale refs answers at `mod.rs:681`. It would strand `head_landing`'s id wait (`state.rs:263-273`).

## The failing tests first
`update/remote/tests.rs`:
1. `a_rebase_after_a_rejected_push_gets_the_plain_box`.
   - Setup: refs `Current` with head `A`; a rejection through `remote_failed(.., NonFastForward, Push(params))`; then a
     refs answer with head `B` for the read asked after it (`Current` again).
   - Assert: `push_params(&state).unwrap().lease == None`, and `push_box(..)`'s title starts `Push `, not `Force-push`.
   - Red on today's code (lease `Some`).
2. `p_between_a_writes_done_and_its_refs_read_says_so` (replaces Step 0's pin).
   - Setup: as 1 up to the rejection; then an `Operated::Done` whose `applied.refresh` is `Everything` (as the M5
     rebase tests build it). Its refs answer has **not** arrived.
   - Press `P` through `update`. Assert: `confirm.is_none()`, the status is `REFS_PENDING`, no `Command::Operate`.
   - Deliver the refs answer with the id from the `Command::ReadRefs` that `Done` emitted, head `B`. Press `P`: the
     plain box.
   - Red on today's code (the lease box opens on the first `P`).

## Signatures
```rust
// graft/src/update/state.rs
/// Whether `AppState::refs` is the answer to the last refs read asked for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RefsRead {
    /// No read asked, the read asked failed, or an older answer overwrote a newer one.
    #[default]
    Unknown,
    /// This read was asked and has not answered.
    Pending(JobId),
    /// `refs` holds this read's answer, and no read has been asked since.
    Current(JobId),
}
impl RefsRead {
    /// A `Command::ReadRefs(id)` was issued: `→ Pending(id)`.
    pub fn asked(&mut self, id: JobId);
    /// A refs answer landed. `Ok`: `Pending(id)` → `Current(id)`; `Pending(other)` stays;
    /// `Current(other)` → `Unknown` (it overwrote a newer answer, `mod.rs:681`); `Unknown` stays.
    /// `Err`: `Pending(id)` → `Unknown`; anything else stays.
    pub fn answered(&mut self, id: JobId, ok: bool);
}
pub struct AppState { pub refs_read: RefsRead, /* rest unchanged */ }
//   callers: mod.rs:103 (launch) and keys.rs:391 call `asked` beside the command;
//            mod.rs:680-694 calls `answered` in both arms, beside the existing store.

// graft/src/update/remote.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RejectedPush {
    pub params: PushParams,
    /// `HEAD`'s tip when git refused the push, read from current refs. `None` when the refs were not current.
    pub sent: Option<Oid>,
}
pub struct RemoteState { pub last_rejected_push: Option<RejectedPush>, /* rest unchanged */ }
// update/remote/in_flight.rs: remote_failed's signature unchanged. Its NonFastForward arm (:168-170) records
//   RejectedPush { params: params.clone(),
//                  sent: matches!(state.refs_read, RefsRead::Current(_))
//                        .then(|| state.refs.as_deref().and_then(RefBadges::head)).flatten() }

// graft/src/update/remote/push_box.rs: signatures unchanged
pub(crate) const REFS_PENDING: &str = "still reading the refs \u{2014} try again in a moment";
pub fn push_params(state: &AppState) -> Result<PushParams, String>;   // applies (a), (b), (c)
```

## Touches (each entry, and how the builder checks it)
- **Readers and writers of `last_rejected_push`:** `in_flight.rs:129`, `:169`; `remote.rs:262-266`; `push_box.rs:90`;
  tests `remote/tests.rs:281-282`, `:343-345`, `:484`, `:499`; `view/tests.rs:3014-3015`. No other hit (planner grep).
  - Check: `grep -rn last_rejected_push crates` before and after. Every hit compiles against `RejectedPush`.
  - The tests that expect the lease box (`:281`, `view/tests.rs:3014`) set `refs_read = RefsRead::Current(..)` and
    `sent` to their refs head. NOTES lists each.
- **Issuers and answerers of the refs read:** `mod.rs:103`, `keys.rs:391`, `mod.rs:680-694`. Production `state.refs`
  writers are only `mod.rs:681`: `radar.rs:560/647/729`, `selection.rs:236` and `view/radar.rs:278` are inside
  `#[cfg(test)]` modules (`radar.rs:522`, `selection.rs:162`, `view/radar.rs:234`; planner-checked).
  - Check: `grep -rn "Command::ReadRefs(" crates/graft/src` and `grep -rn "\.refs = Some" crates/graft/src`; every
    non-test issue has `asked` beside it, the one non-test store has `answered`. NOTES pastes both greps.
  - Check: `refs_read_follows_ask_and_answer`, a table test over `RefsRead::{asked, answered}`, every transition above.
  - Check: `an_older_refs_answer_after_the_current_one_counts_as_unknown` (then `P` → the plain box).
  - Check: `a_failed_refs_read_gives_the_plain_box_not_the_wait_words`.
- **`head_landing`** (`state.rs:263-273`, `banner::refs_answered`): its own id, unchanged.
  - Check: `git diff` leaves `banner.rs` and `banner/stop.rs` alone; the banner tests are green.
- **Callers of `push_params`:** `push_key` (`push_box.rs:19`) and the `.` menu's `Push (P)`
  (`context_menu.rs:257-258` → `push_key`). No other non-test caller.
  - Check: `every_push_params_caller_gets_the_plain_box_after_a_move`, one row per caller.
- **Other readers of refs in the window (unchanged; each goes in NOTES as a finding for the conductor):**
  - the push box's own text: `head_ahead` (`push_box.rs:21`, `:113`) and `local_tip` (`:22`), and
    `repush_chosen` (`after_push.rs:245-246`). In the window the box can show the pre-write ahead count and tip;
  - `name_prompt.rs:152`, `context_menu.rs:65`, `drag.rs:176`, `geometry.rs:446`, `radar.rs:122`, `:194`, `:272`,
    `:398`, `preview.rs:710`, `keys.rs:323`, `prelude.rs:45`;
  - `facts()` (`remote.rs:277-285`): `p`, `P` and `f` read the old remote facts during a re-read. For `P`, only (b)
    guards it, and only when (a) holds;
  - `mod.rs:681` stores an older answer over a newer one. `RefsRead` marks it `Unknown`; the graph draws the older refs.
  - Check: `grep -rn "state\.refs\b\|next\.refs\b" crates/graft/src --include='*.rs'` minus tests, pasted in NOTES
    with each hit marked "unchanged".
- **The undo-after-push re-push** (`after_push.rs`, `ConfirmThen::UndoAfterPush`): unaffected (see Out).
  - Check: `grep -n "push_params\|last_rejected_push\|refs_read" crates/graft/src/update/remote/after_push.rs` is empty
    after the change; the existing `after_push` tests are green; `git diff` does not touch `after_push.rs`.
- **Journal and undo.** A push is journaled and transparent to undo (`m4-remotes.md` :90-99). No journal change.
  - Check: `git diff --stat` touches nothing under `journal`; name the stat line in NOTES.
- **Scenario 10 (`push-rejected-then-lease`)** and **scenario 11 (`push-lease-refused-stale`)**, `TESTING.md` :1502-1516.
  Nothing moves between the rejection and the second `P`, so both still get the lease box, once the rejection's own
  refresh (if `operation_failed` refreshes, `operate.rs:868-870`) has landed.
  - Check: find their tests (`grep -rn "rejected_then_lease\|lease_refused_stale" crates`). Name the harness line that
    settles between keys (every job answered before the next key). If it does not settle, **stop and report**: the
    walk would press `P` inside the window.
  - Fast-tier: run it. Heavy-tier: don't; say so in NOTES, and add the fast-tier twin
    `a_rejected_push_with_the_tip_unchanged_still_offers_the_lease`.
- **The new scenario** `push-rejected-rebased-then-plain`: rejected, then `p` (the diverged line), then `r r`, then
  `P` → the plain box, `⏎` → the bare's `main` is the local tip, and the command has no `--force`.
  - Written in the same harness as scenario 10. Run it if it is fast-tier; otherwise NOTES says so and the unit tests
    stand.
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
- **Keymap.** `P` §2.3 :196 is unchanged (no key meaning changes).
- **TESTING.md** (the conductor edits it): add scenario 13, `push-rejected-rebased-then-plain`, with the exact text
  below. Scenarios 10 and 11 are unchanged.
- **MAP.md:** graft §`update::remote`: the `RemoteState` line becomes "cleared on success or a branch change; offered
  only while the refs are current and `HEAD` is still the tip that was refused (`RejectedPush::sent`)". Add a
  `RejectedPush` line. §`update::state`: add `RefsRead` and `AppState::refs_read` (who asks, who answers).

## Acceptance (name the verifying test at tick time)
- [ ] `a_rebase_after_a_rejected_push_gets_the_plain_box`: red first, then green.
- [ ] `p_between_a_writes_done_and_its_refs_read_says_so`: red first, then green.
- [ ] `refs_read_follows_ask_and_answer`
- [ ] `an_older_refs_answer_after_the_current_one_counts_as_unknown`
- [ ] `a_failed_refs_read_gives_the_plain_box_not_the_wait_words`
- [ ] `a_rejected_push_with_the_tip_unchanged_still_offers_the_lease` (scenario 10's twin).
- [ ] `a_rejection_recorded_without_current_refs_gets_the_plain_box` (`sent: None`, both `Unknown` and `Pending` at
  the rejection).
- [ ] `a_commit_after_a_rejected_push_gets_the_plain_box` (the tip moved forward, still diverged: git decides).
- [ ] `undo_back_to_the_rejected_tip_offers_the_lease_again`: the tip equals `sent` again and the refs are current, so
  (b) and (c) hold. Correct: the remote still has what that tip lacks.
- [ ] `every_push_params_caller_gets_the_plain_box_after_a_move`
- [ ] `the_rejection_line_is_gone_after_the_rebase_lands`
- [ ] The existing `after_push` and banner tests are green, unchanged.
- [ ] Captures byte-identical. NOTES gives the list and `git status docs/shots` clean.
- [ ] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-plain-box-after-rebase-187x50.txt`.
  - Scene: after a rejection, `p`, `r r`, then `P`.
  - **What right looks like:** the box is titled `Push main → origin/main`. Its line starts with the ahead count, for
    example `2 commits`. The command row has no `--force-with-lease`. The action reads `Push`, not `Force push`.
- [ ] NOTES: the two greps, the other-readers findings, the settle line for scenarios 10 and 11.
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
Medium. One context.

## NOTES
