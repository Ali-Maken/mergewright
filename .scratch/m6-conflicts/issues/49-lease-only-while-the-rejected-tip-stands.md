# lease-only-while-the-rejected-tip-stands: `P` offers the force-with-lease box only while the tip that was rejected is still the tip

Status: done (2026-09-24, after the conductor approved the harness-readiness fix; see NOTES)
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
All tests below are in `crates/graft/src/update/remote/tests/lease.rs` (module `update::remote::tests::lease`) unless named otherwise.
- [x] `a_rebase_after_a_rejected_push_gets_the_plain_box`: red first (assert failed on `lease == Some(..)`), then green.
- [x] `p_between_a_writes_done_and_its_refs_read_says_so`: red first (the first `P` opened the lease box, `confirm`
  was `Some`), then green.
- [x] `refs_read_follows_ask_and_answer`
- [x] `an_older_refs_answer_after_the_current_one_counts_as_unknown`
- [x] `a_failed_refs_read_gives_the_plain_box_not_the_wait_words`
- [x] `a_rejected_push_with_the_tip_unchanged_still_offers_the_lease` (scenario 10's twin; also asserts both wait
  words: `REFS_PENDING` and `still reading the remotes …`).
- [x] `a_rejection_recorded_without_current_refs_gets_the_plain_box` (`sent: None`, both `Unknown` and `Pending` at
  the rejection).
- [x] `a_commit_after_a_rejected_push_gets_the_plain_box` (the tip moved forward, still diverged: git decides).
- [x] `undo_back_to_the_rejected_tip_offers_the_lease_again`: the tip equals `sent` again and the refs are current, so
  (b) and (c) hold. Correct: the remote still has what that tip lacks.
- [x] `every_push_params_caller_gets_the_plain_box_after_a_move` (rows: `P` via `update`, and
  `apply_context_action(ContextAction::Push)`).
- [x] `the_rejection_line_is_gone_after_the_rebase_lands`
- [x] The existing `after_push` and banner tests are green, unchanged: `cargo nextest run --workspace` 3076/3076,
  which includes every `update::remote::after_push::*`, `update::banner::*` test; `git diff` does not touch
  `after_push.rs`, `banner.rs`, `banner/stop.rs`.
- [x] Captures byte-identical: every TestBackend capture test (`check_shot`, run without `GRAFT_UPDATE_SHOTS`) green
  after the harness change and again after the feature, `git status docs/shots` clean both times (only the new
  capture and the regenerated contact sheet differ now). m4 pty captures (m4-01, -02, -06 at 80x24 and 160x48) via
  `.scratch/m4-remotes/pty-walk-m4.py`: see NOTES (normalised-identical to base; raw drift pre-existing).
  `m5-ui-undo-after-push-80x24`: its capture test green, unchanged.
- [x] Addition (a), conductor 2026-09-24: captures byte-identical after the harness change, before the feature — as
  above, capture tests without `GRAFT_UPDATE_SHOTS`, `git status docs/shots` clean.
- [x] Addition (b): m4 walk scenarios 10 and 11 green: `it::m4_walk::every_scenario::push_rejected_then_lease` and
  `::push_lease_refused_stale` (8 runs each, all green), plus new `::push_rejected_rebased_then_plain` (8 runs green).
- [x] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-plain-box-after-rebase-187x50.txt`, by
  `it::remote_by_key::capture_fix_plain_box_after_rebase` (asserts title/command/action; 7 runs stable). Rebased
  commit id zeroed (`0000000`), said in its header. Line reads `1 commit · …` (one commit ahead). Not judged by eye.
  - Scene: after a rejection, `p`, `r r`, then `P`.
  - **What right looks like:** the box is titled `Push main → origin/main`. Its line starts with the ahead count, for
    example `2 commits`. The command row has no `--force-with-lease`. The action reads `Push`, not `Force push`.
- [x] NOTES: the two greps, the other-readers findings, the settle line for scenarios 10 and 11 (below).
- [x] MAP.md updated (doc, no test): `RejectedPush`, `RemoteState`, `push_params`, `AppState::refs_read`/`RefsRead`.

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
**2026-09-24, builder (opus): stopped at the ticket's own stop case, before any code.** Scout used:
`.scratch/m6-conflicts/scouts/49.md` (in the main checkout, not committed), plus the reads below.

The Touches check for scenarios 10 and 11 says: name the harness line that settles between keys; if it does not
settle, stop and report. **Neither harness settles on the pending states this ticket adds.** Code read, not run:
- A rejected push refreshes `Everything`: `crates/graft/src/jobs/operate.rs:193-197` (`ApplyError::Remote` →
  `Some(Refresh::Everything)`), applied at `update/operate.rs:868-870`. After the rejection a `ReadRefs` and a
  `ReadRemotes` are both in flight.
- **Walk** (`crates/graft/tests/it/m4_walk/run.rs:347-358`, `ready`, used by `Step::WaitFor(Wait::Ready)` before the
  second `P`): waits on `state.remote.reading.is_none()` (the facts side settles) but only on `state.refs.is_some()`
  for refs, which the old refs pass. The two reads are separate pool jobs with no order, so `P` can land while
  `refs_read` is `Pending` → `REFS_PENDING` → `See("the lease box")` fails. Flaky, not deterministic.
- **Capture prelude** (`crates/graft/src/update/prelude.rs:33-90`, `settled`, feeds `m4-02-push-box-lease`'s
  `P → ⏎ P`): checks neither `remote.reading` nor a pending refs read (`let refs = state.refs.is_some();`, :45).
  Harmless today (`facts()` serves the old facts, no refusal). With (b), the second `P` can hit
  `still reading the remotes …` or `REFS_PENDING`, and the capture shows the status line, not the lease box.

**Exact next step (conductor's call; both files are in crate `graft`, this ticket's set):** add
`&& !matches!(state.refs_read, RefsRead::Pending(_))` to `run.rs::ready` and to `prelude.rs::settled`, and
`&& state.remote.reading.is_none()` to `settled`. Both harnesses then wait for exactly the window this ticket opens:
a cause fix, not a sleep. The prelude change touches every capture's timing (it can only wait longer, never feed a
different key), so every capture should regenerate byte-identical; that has to be checked. Then build as specced.

No other stop hit: the `last_rejected_push` grep matches the ticket (`remote.rs:35/262/266`, `push_box.rs:90`,
`in_flight.rs:129/169`, `remote/tests.rs:281/343/483/498`, `view/tests.rs:3014`). Non-test `Command::ReadRefs(`
issues: `mod.rs:103`, `keys.rs:391` only. One more for the spec: `view/tests.rs:3014`'s capture state
(`changes_pane_state`) must get refs with a head for (c) to hold, so check whether that changes `m4-remote-keys.txt`.

**2026-09-24, builder (opus): done.** harness readiness: conductor-approved 2026-09-24, cause.
Scout used: `.scratch/m6-conflicts/scouts/49.md`. No compaction affecting scope (context was summarised once near the
end, after all code and gates; only these NOTES and the commit remained).

Harness settle lines (cause fix, no sleeps/retries — waits on state only):
- `crates/graft/tests/it/m4_walk/run.rs` `ready`: adds `!matches!(state.refs_read, RefsRead::Pending(_))`. New
  `Wait::PreviewPlanned` (preview plan neither `Pending` nor `Idle`) for the new scenario's `r` → `r`.
- `crates/graft/src/update/prelude.rs` `settled`: refs settle only when not `Pending`; `writes_landed` also needs
  `remote.reading.is_none()`.
- **Extension of the approved fix, flagged:** `crates/graft/tests/it/remote_by_key.rs` `ready` got the same
  `refs_read` not-`Pending` wait (same cause, third harness; its lease tests would otherwise race).

Greps (after):
- `Command::ReadRefs(` non-test issues: `update/mod.rs:104` (launch; `state.refs_read.asked(refs)` at `:93`) and
  `update/keys.rs:393` (`next.refs_read.asked(refs)` at `:392`). Other hits are matchers/runner, not issues
  (`undo.rs:795`, `operate.rs:2189` in tests, `banner/stop.rs:93` id match, `jobs/runner.rs:992` executor).
- `.refs = Some` non-test: `update/mod.rs:682` only, `answered(id, true)` at `:683`, `answered(id, false)` at
  `:692`. `radar.rs:560/647/729`, `selection.rs:236`, `view/radar.rs:278` are in `#[cfg(test)]` modules (`:522`,
  `:162`, `:234`).
- `last_rejected_push`: `remote.rs:49` (field), `:276`/`:282` (branch-change clear, compares `params.branch`),
  `push_box.rs:112` (read), `in_flight.rs:131` (clear), `:177` (record with `sent`); tests updated:
  `remote/tests.rs` (`rejected_at_head` helper: refs `Current(1)`, `sent: Some(OURS)`, used by the text table's lease
  case and the unfetched refusal; `failures_are_worded_and_remembered` asserts `sent: None`), `view/tests.rs`
  `capture_m4_remote_keys` (`refs_read = Current(1)`, `sent` = refs head oid(3)). `m4-remote-keys.txt` unchanged.
- `after_push.rs` grep for `push_params|last_rejected_push|refs_read`: empty. `git diff --stat`: 0 lines under
  journal, `after_push`, `banner*`.

Other readers of `state.refs` in the window (all unchanged; findings for the conductor):
`drag.rs:176`, `keys.rs:323`, `context_menu.rs:65`, `radar.rs:122/194/272/398`, `preview.rs:710`,
`geometry.rs:446`, `name_prompt.rs:152`, `banner/stop.rs:148`, `resolver/names.rs:40/111`, `push_box.rs:28`
(`local_tip`) and `:149` (`head_ahead`) — the plain/lease box text can show the pre-write ahead count and tip in the
window, `after_push.rs:245`, `view/entries.rs:139`, `view/strip.rs:34`, `view/strips.rs:341`; `prelude.rs:47`
(changed, above). `facts()`: `p`, `P`, `f` read old facts during a re-read; only (b) guards `P`, only when (a) holds.
`mod.rs:682` still stores an older answer over a newer one; `RefsRead` marks it `Unknown`, graph draws older refs.

pty captures: m4-01/-02/-06 at 80x24 and 160x48 regenerated from base `2c794e05` and from this branch are identical
after normalising the tmp path and commit ids (the script is not byte-reproducible: random tmp path, fresh ids).
**Finding:** the committed files differ raw from both (a `<scratch___>` path, older footer hints, older graph column)
— pre-existing drift, not from this ticket; not regenerated here.

Heavy tier not run. Scenario 13 (`push-rejected-rebased-then-plain`) is fast-tier and green. TESTING.md scenario 13
left for the conductor (text above). Contact sheet rerun: parts 198194 B and 117104 B (≤200000).

Touches: `last_rejected_push` readers/writers (grep, above; compiles against `RejectedPush`); refs issuers/answerer
(greps, `refs_read_follows_ask_and_answer`); `head_landing` (banner files untouched, banner tests green);
`push_params` callers (`every_push_params_caller_gets_the_plain_box_after_a_move`); undo-after-push (grep empty,
file untouched); journal (diff stat); scenarios 10/11 (green, 8 runs); captures (above); `.` menu label and `↑n`
badge (no diff in their files); keymap unchanged; MAP.md updated.
