# app-answers-beat-rereads: the staleness pass — the re-push lease comes from the push's own answer, `u` right after `--continue` waits on the named re-read, a relaunch into a live conflict lands on the conflict, `fetch_key` says why it refused, and the assert at the write site

Status: partial — 5 of 8 boxes ticked; 553 blocked: needs graft-ui
Blocked by: **04**.
Build with: Opus — set by the conductor after the scout (`.scratch/m6-conflicts/scout-app-g06.md`); all four bugs and row 505 confirmed real, with lines.
Crates: graft
Needs:
- `app-conflict-resolver` — `open_resolver`, `OpenedFrom::Banner` (piece 3 opens nothing itself, it puts the user one `⏎` away).

## Scout first (Sonnet `Explore`, ≤ 1 page) — `.scratch/m5-history-and-ghost-preview/scout-app.md` first

- `crates/graft/src/update/remote.rs:640` region: what `state.remotes` holds for the tracking tip at the moment the re-push box is built, which `Product` updates it (`RemotesRead`? the push's own `Operated`?), and whether the push entry's `refs_after[tracking]` is in `state` by then. Which of `Operated::Done` for a push carries the new tip.
- `crates/graft/src/update/undo.rs` + `jobs/undo.rs`: on `Operated::Done` for a `Resume(Continue)`, which reads are scheduled (`JournalRead`? `ReadWorkingTree`? `Everything`?) and in what order their `Product`s land; where `undo_key` decides "in progress" (`state.in_progress` from the *previous* working-tree read — KNOWN_GAPS 569).
- `crates/graft/src/update/journal.rs::launch_into_live`: what it sets today (`banner_focus`, `Sticky`) and whether the detail pane is open / on which section at launch.
- `crates/graft/src/update/remote.rs::fetch_key`: the refusal it swallows (KNOWN_GAPS 481) and which `Status` variant the other refusals use.
- KNOWN_GAPS 550's assert site (the write that should assert its precondition) and 553 (`space fold` on the uncommitted row — confirm it is one hint entry to drop).

## Read first

- `docs/design-notes/m6-conflicts.md` — Decision 2 (a), "KNOWN_GAPS rows expiring M6" (557, 569, 505, 481, 550, 553 → here).
- `KNOWN_GAPS.md` rows 557 (**broken**: re-push lease from the stale pre-push tracking tip), 569 (`u` right after `--continue` refused — the re-read window; filed as a question: **verify before fixing**, and if the scout finds the window does not exist, close the row with the line that proves it instead), 505, 481, 550, 553.
- ADR-0037 §3 (`u` after a push — the re-push box this fixes), ADR-0031 (journal), `docs/design/06-undo-journal-design.md` §6.4.
- PROMPT.md §9 (`kill -9` → relaunch offers Continue/Abort).
- `crates/graft/MAP.md` §update::remote (`push_box`, `RemoteState`, `pushed_since`), §update::undo, §update::journal (`launch_into_live`), §jobs (`JournalRead`, `Everything` refresh), §event_loop (the `Product` ordering rule, if written; if not, write it).
- `docs/reviews/m5-09-app-conflict-interim.md` "gaps" table (rows this closes).

## Scope

In, in order (a commit per piece, each with its failing-first test):

1. **Re-push lease from the answer, not the memory** (557): the `Re-push old tip` choice builds `PushParams { source_commit: entry.refs_before[tracking], lease: Lease { expected: entry.refs_after[tracking] } }` from the **push entry's own journal record**, never from `state.remotes`; if the entry lacks `refs_after[tracking]` (older journal), the choice is offered with `expected` read by a `ls-remote`-shaped job **before** the box opens (the box then shows the real tip) — builder picks the cheaper of the two after the scout says what the entry carries, and says which. The box's consequence line names `<old7>` from the same source. Fix the cause: no re-read after the fact.
2. **`u` after `--continue`** (569): `Operated::Done` for `Resume(Continue|Skip|Abort)` sets `state.in_progress = None` **in the same update** (the answer already says the operation ended) and schedules the reads as before; the reads confirm, never contradict. If the reads later say `Some` (a rebase that stopped again is `Operated::Conflicted`, not `Done`, so this is a real disagreement), `Sticky("git says a <kind> is still in progress · Ctrl+R")`, never silently. `undo_key`'s refusal then never fires in the window. If the scout proves the window cannot occur, close 569 with the line and skip the change — say which in NOTES.
3. **Relaunch lands on the conflict** (Decision 2a): `launch_into_live` additionally opens the detail pane on the `CONFLICTS` section (the m5 `Tab`/section helpers) with the cursor on the first conflicted row, keeps `banner_focus = true`, and the `Sticky` reads `<kind> still in progress from a previous run · ⏎ resolve  c continue  a abort` (the `⏎` word only after G4's key is bound — the hint bar's `Banner { conflicted }` row already carries it). No auto-open of the resolver (a relaunch must never start a write). With no conflicted files (a stop on an empty commit, or all resolved before the kill) the words stay `c continue  a abort`.
4. **`fetch_key` refusal in words** (481): whatever it swallowed becomes the same `Status` variant the other refusals use; test asserts the words.
5. **Assert at the write site** (550) and drop `space fold` from the uncommitted row (553, the conductor's ruling): one commit, two tests.
6. `Operated::Conflicted` carries `entry: EntryId` end to end (505) if the scout finds it dropped anywhere between `jobs::operate` and `banner`; else close the row with the line.
7. MAP.md (§update::remote, §update::undo, §update::journal), NOTES: for each row — cause or symptom, and the line that proved it.

Out: anything in the resolver; the M7 rows; `hold_commands` across relaunch (a relaunch loses held commands by design — §6.2; note it stays).

## Signatures

```rust
// graft/src/update/remote.rs
pub fn repush_params(entry: &Entry, tracking: &RefName) -> Option<PushParams>;   // None when the entry lacks the tip → the box is not offered; the choice reads `re-push unavailable: entry has no remote tip`
// graft/src/update/undo.rs
pub fn operation_ended(state: &mut AppState, kind: OperationKind);   // clears in_progress and banner_focus in the same update as Operated::Done for a Resume
// graft/src/update/journal.rs
pub fn launch_into_live(state: &mut AppState, commands: &mut Vec<Command>, live: &[EntryId]) -> bool;   // existing; now also lands the pane on CONFLICTS
```

## Acceptance

`crates/graft/tests/it/` (file names per the scout's layout; each test **fails on `main` before the piece**, asserted in the report by running it once against the pre-piece commit).

- [x] After a push, a second push from elsewhere moves the remote tip; `u` → `Undo after a push` → `Re-push old tip` → the box's `Lease.expected` equals the **journal entry's** `refs_after[tracking]`, not `state.remotes`' current tip; the push is refused by the lease (the remote moved), in words — `repush_lease_comes_from_the_entry` (fixture: `LocalRemote`, m5's push shapes). **Ticked: `graft::it remote_by_key::repush_lease_comes_from_the_entry`** — pumps until `state.remotes`' tip == the teammate's tip, asserts `Lease.expected` == the entry's `refs_after[refs/remotes/origin/<b>]` (itself asserted == our pushed tip), then the refusal words `force-push refused: origin/<b> is not where the box said…` and the bare still at theirs. Red before the fix: lease was their tip. Plus unit `update::remote::tests::a_push_entry_without_its_tip_offers_no_repush`.
- [x] `Operated::Done` for `Resume(Continue)` → `state.in_progress == None` and `undo_key` accepted in the very next update, before any `WorkingTree` product lands — `u_right_after_continue_is_accepted` (or, if the window is disproved, `KNOWN_GAPS` 569 closed with the file:line in NOTES and no test — say which). **Ticked: `update::banner::tests::u_right_after_continue_is_accepted`** (unit, through `update`, no `WorkingTree` product between the answer and `u`; asserts `in_progress == None` and `Command::Undo(id, false)`). Window is real — fixed, not closed. Red before: `in_progress` still `Some(Rebase)`.
- [x] A `WorkingTree` product that says `Some` after the answer said `None` → `Sticky("git says a rebase is still in progress · Ctrl+R")` — `answer_and_read_disagree_says_so`. **Ticked: `update::banner::tests::answer_and_read_disagree_says_so`** — `Status::sticky("git says a rebase is still in progress · Ctrl+R")` exactly; also asserts an agreeing read changes nothing. Red before: status stayed `Done`.
- [ ] `Shape::StoppedRebase` + a `Pending` journal line for it, launched: `banner_focus`, the detail pane open on `CONFLICTS` with the cursor on `both.txt`, `Sticky` reading `rebase still in progress from a previous run · ⏎ resolve  c continue  a abort`; one `⏎` → `Command::OpenConflict` — `relaunch_lands_on_the_conflict`. Same shape with every file resolved before launch → `Sticky` with `c continue  a abort` only, pane on `STAGED` — `relaunch_with_nothing_conflicted`. **Not ticked — one clause is false as written.** `graft::it conflict_interim::relaunch_lands_on_the_conflict` proves: `banner_focus`; pane open on the working tree, cursor's section `Unmerged`; cursor row == `banner::first_conflicted` == **`added-by-them.txt`**, not `both.txt` — git's unmerged order is by path (`graft-fixtures/src/shapes/conflicts.rs:208`, `UA added-by-them.txt` sorts first); `Status::Sticky { reason: "rebase still in progress from a previous run", keys: Some("⏎ resolve  c continue  a abort") }` (reason/keys split, not one string); `resolver.is_none()`; `⏎` → `Command::OpenConflict(_, added-by-them.txt)`. `graft::it conflict_interim::relaunch_with_nothing_conflicted` proves keys `c continue  a abort` and cursor section `Staged`. Conductor: re-word the box to "git's first conflicted path" and tick, or say which file you meant. Both red before (the pane never opened, 120 s pump timeout).
- [ ] `fetch_key` while a fetch is running / during an operation → the refusal in words the scout named — `fetch_refusal_in_words`. **Not ticked — half proven.** `update::remote::tests::fetch_refusal_in_words` proves "while a fetch is running": no command, first job untouched, `Status::Failed("wait for fetch all remotes to finish")` (the variant `pull_key`'s refusals use; words from `preview::open_preview`'s in-flight refusal). Red before: status stayed `Busy`. "During an operation" (git mid-rebase, `in_progress` `Some`) has **no** refusal in `fetch_key` to prove — a fetch mid-rebase is allowed and harmless, and a focused banner swallows `f` anyway. Question for the conductor: is that clause "another write in flight" (then tick) or a new refusal (then a spec change)?
- [ ] The 550 write site asserts its precondition (a debug_assert with a message that names the invariant, plus a test that constructs the violating state and sees the typed refusal, not a panic, in release) — `write_site_refuses_the_impossible_state`; `space fold` gone from the uncommitted row — `hints::tests::uncommitted_row_has_no_fold` (graft-ui test lives in graft-ui: **if the row is drawn by `graft-ui`, this piece is a one-line `graft-ui` change and the conductor sequences it after G4's ui commit** — report `blocked: needs graft-ui` rather than editing it concurrently). **Not ticked — two gaps.** (a) 550: the site is `update::operate::launch_with` (every write passes it). `debug_assert!` message: "invariant: a push sends HEAD's branch, because the journal names a push's branch by head_before". `update::remote::tests::write_site_refuses_the_impossible_state` asserts the check (`remote::push_sends_head`) returns the refusal words for a push of `feature` while HEAD is on `main`, and for detached; `update::remote::tests::the_write_site_asserts_the_push_invariant` (`#[cfg(debug_assertions)]`, `should_panic`) proves the assert fires at the site. **The release path — status set, no job, no panic — is not exercised by any test**: the fast tier builds debug, where the assert fires first. Unverified, not broken. (b) 553: `space fold` is drawn by **graft-ui** — `crates/graft-ui/src/graph_screen/hints.rs:765`, in `TAIL`, appended after `working_tree_row_list()` (:779). **`blocked: needs graft-ui`**; not edited.
- [x] `Operated::Conflicted { entry }` reaches `banner_view`'s inputs unchanged — `conflicted_entry_id_survives`, or 505 closed with the line. **Ticked: `update::operate::tests::conflicted_entry_id_survives`** — `operated` with `Conflicted { entry: EntryId(42,[4;10]) }` → `state.live_entries == [that id]`; the resume path is asserted in `update::banner::tests` (the "stopped again" test, `live_entries == [EntryId(1,[0;10])]`). `banner_view` takes `&AppState`; the id lands there, in `live_entries`. Red before: compile (field absent), and the value only exists because of the new line.
- [x] MAP.md updated; NOTES has the cause/symptom line per row. **Ticked:** `crates/graft/MAP.md` §update::operate (`launch`, `operated`), §update::undo (`operation_ended`, `reread_disagrees`), §update::banner (`resumed`), §update::remote (`fetch_key`, `push_sends_head`, `repush_lease`, `UndoAfterPush`), §update::journal (`launch_into_live`), §update (`AppState::ended_by_answer`, `live_entries`, the "answers beat re-reads" rule), §update::detail (`KeptCursor` re-export), §jobs (`Operated::Conflicted.entry`). NOTES below.

## NOTES (builder, 2026-09-23)

No compaction. Scout used (`scout-app-g06.md`); its facts held with two corrections: `pull_key`'s in-flight guard is **also** silent (`update/remote.rs` `pull_key`, same `return true` as `fetch_key`; its `Status::Failed` is for missing params only), and `push_key`'s too — out of this ticket's scope, not touched; and 553's row is drawn by graft-ui, which the scout did not check.

Per row — cause or symptom, and the line:
- **557 — cause.** `repush_for` took `expected` from `facts.head_upstream.tracking_tip` (was `update/remote.rs:636-640`). Now `repush_lease(push, tracking)` = the push entry's `refs_after[tracking]`, which `graft_ops::apply::read_after` fills right after git pushed (`graft-ops/src/apply.rs:1059-1060`). Option picked: the entry carries it (cheaper than an `ls-remote` job); entries without it → `re-push unavailable: entry has no remote tip`, two choices. **Deviation:** `source_commit` still comes from the undo *target's* `refs_before[branch]`, not the push entry's `refs_before[tracking]` as the Scope line says — the target's is the old tip when a push sent several commits and `u` takes back only the newest; the push entry's would roll the remote back past commits the user is keeping. Signature is `repush_lease(&JournalEntry, &str) -> Option<Oid>`, not `repush_params(..) -> Option<PushParams>` (remote/branch still come from facts). Cost to the user: none; the m5 walk's `Wait::RemotesCaughtUp` workaround is now unnecessary (not removed — the walk is not this ticket's).
- **569 — cause.** The window was real: `banner::resumed`'s `Done` arm never touched `in_progress`; the only writer was `took_fact`'s `WorkingTree(Ok)` arm (`update/mod.rs`, `next.in_progress = read.in_progress`). Now `undo::operation_ended` clears it on the answer (Done and Incomplete); `undo::reread_disagrees` spends `AppState::ended_by_answer` on the next working-tree read. Signature drops the ticket's `kind` param (nothing reads it). **Residual:** `ReadWorkingTree` still carries no `JobId` (`events.rs`), so a read scheduled *before* the resume that lands *after* its answer would be taken as the contradicting read and show the `git says … · Ctrl+R` line spuriously (then `Ctrl+R` clears it). Not observed; the structural fix is an id on `ReadWorkingTree`, a bigger change than this ticket.
- **Decision 2a — cause.** `launch_into_live` (`update/journal.rs`) now takes `(state, commands, next_id)` (ticket's `live: &[EntryId]` / `-> bool` not needed; its two callers thread `commands`), sets `on_working_tree` (else `selection::follow_selection` pulls the pane back onto a commit), opens the changes pane and plants a `KeptCursor` on the first conflicted path / STAGED. Existing `relaunch_into_a_live_conflict` updated: keys now `⏎ resolve  c continue  a abort`.
- **481 — cause.** `fetch_key` returned `true` with no status (`update/remote.rs`, the `operation_in_flight` arm). Now `Status::Failed("wait for <running> to finish")`. The line replaces the `Busy` line until the running write answers (a remote write's next progress event puts `Busy` back).
- **550 — cause (guard added).** Invariant checked at `update::operate::launch_with`; see the box. Release path unverified.
- **553 — not done, blocked: needs graft-ui** (`crates/graft-ui/src/graph_screen/hints.rs:765`).
- **505 — cause.** `jobs::operate::answer` dropped `entry` from `ApplyError::Conflicted`/`ResumeFailed` (was `jobs/operate.rs:310-324`). Now `Operated::Conflicted.entry`, written to `live_entries` by `operate::operated` and `banner::resumed`.
- **Context question (cached working tree trails one stage):** not covered by this fix — that is `working_tree` staleness, the same id-less `ReadWorkingTree` class as the 569 residual. No test.

Fails-before evidence: pieces 1–4 each run red against the pre-piece commit in this session (failures quoted in the boxes). Pieces 5 and 6 were red by compile only (the function / field did not exist); not re-run against a mutated build.

Deviation from process: read six rows of `KNOWN_GAPS.md` by line (550/553/557 etc. were needed to know what 550 and 553 are; the ticket's row numbers are off by one from file lines — 550 is line 549's `entry_branch` row, 553 is line 552's `space fold` row).

## Gates

fmt / clippy / nextest / doc, workspace-wide; `seam_rule`.

## Size

One context; ~350 lines of change, most of it tests. Reads only the scout page and the four named functions.
