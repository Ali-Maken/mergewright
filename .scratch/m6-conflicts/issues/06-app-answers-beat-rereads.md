# app-answers-beat-rereads: the staleness pass — the re-push lease comes from the push's own answer, `u` right after `--continue` waits on the named re-read, a relaunch into a live conflict lands on the conflict, `fetch_key` says why it refused, and the assert at the write site

Status: ready
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

- [ ] After a push, a second push from elsewhere moves the remote tip; `u` → `Undo after a push` → `Re-push old tip` → the box's `Lease.expected` equals the **journal entry's** `refs_after[tracking]`, not `state.remotes`' current tip; the push is refused by the lease (the remote moved), in words — `repush_lease_comes_from_the_entry` (fixture: `LocalRemote`, m5's push shapes).
- [ ] `Operated::Done` for `Resume(Continue)` → `state.in_progress == None` and `undo_key` accepted in the very next update, before any `WorkingTree` product lands — `u_right_after_continue_is_accepted` (or, if the window is disproved, `KNOWN_GAPS` 569 closed with the file:line in NOTES and no test — say which).
- [ ] A `WorkingTree` product that says `Some` after the answer said `None` → `Sticky("git says a rebase is still in progress · Ctrl+R")` — `answer_and_read_disagree_says_so`.
- [ ] `Shape::StoppedRebase` + a `Pending` journal line for it, launched: `banner_focus`, the detail pane open on `CONFLICTS` with the cursor on `both.txt`, `Sticky` reading `rebase still in progress from a previous run · ⏎ resolve  c continue  a abort`; one `⏎` → `Command::OpenConflict` — `relaunch_lands_on_the_conflict`. Same shape with every file resolved before launch → `Sticky` with `c continue  a abort` only, pane on `STAGED` — `relaunch_with_nothing_conflicted`.
- [ ] `fetch_key` while a fetch is running / during an operation → the refusal in words the scout named — `fetch_refusal_in_words`.
- [ ] The 550 write site asserts its precondition (a debug_assert with a message that names the invariant, plus a test that constructs the violating state and sees the typed refusal, not a panic, in release) — `write_site_refuses_the_impossible_state`; `space fold` gone from the uncommitted row — `hints::tests::uncommitted_row_has_no_fold` (graft-ui test lives in graft-ui: **if the row is drawn by `graft-ui`, this piece is a one-line `graft-ui` change and the conductor sequences it after G4's ui commit** — report `blocked: needs graft-ui` rather than editing it concurrently).
- [ ] `Operated::Conflicted { entry }` reaches `banner_view`'s inputs unchanged — `conflicted_entry_id_survives`, or 505 closed with the line.
- [ ] MAP.md updated; NOTES has the cause/symptom line per row.

## Gates

fmt / clippy / nextest / doc, workspace-wide; `seam_rule`.

## Size

One context; ~350 lines of change, most of it tests. Reads only the scout page and the four named functions.
