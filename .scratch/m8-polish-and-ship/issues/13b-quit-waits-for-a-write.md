# `q` while a write that can't stop runs: quit waits for it; `q quit` on the banner's bar

Status: ready
Build with: _(blank — scout `scouts/13b-quit.md` running; conductor sets it after)_
Blocked by: 13
Needs: ticket 13's review fix (Major 1: `banner_key` sends `q` to `quit::asked`), merged. No other ticket's signature.
Brownfield (ADR-0035): **scout first**. The conductor sets `Build with` after it.

**The user has not seen this.** Box words, the waiting status line, Graft quitting by itself when the write ends,
and `q quit` on the banner's bar are all new on screen. Keymap rows are marked Proposed. The captures below are what
he judges.

## Read first
- `docs/design/02-keymap.md`: §2.2 `q` row and the "`q` while a write runs" paragraph; §2.6a `q` row; §2.1 bullet
  "The hint bar is a ranked selection"; §2.3c
- `docs/reviews/m8-13-landing.md` Major 2 (launch sites with lines), Minor 3 (shutdown grace)
- `.scratch/m8-polish-and-ship/issues/13-daily-landing.md`: Rulings 4, "Review fixes"
- `docs/research/ux-principles.md` §3.1
- `crates/graft/MAP.md`: `update::quit`, `ConfirmThen` (`Quit`), `PendingConfirm::yes`, `Stoppable`,
  `cancel_operating` / `stop_write`; `crates/graft-ui/MAP.md`: `HintContext::Banner`, `graph_screen::hints`,
  `ConfirmYes`

## Scope
In:
1. graft `update/quit.rs`: `running_box` words the box by the running write's `Stoppable` (keymap §2.2, exact words).
   `confirmed` for a `Stoppable::No` write sets the wait and the status line. It does not call `stop_write` and does
   not emit `CancelJob`.
2. graft: every answer path of a `Stoppable::No` write calls `quit::write_ended` **after** the answer is applied and
   journaled. If the write is followed by a put-back (edits set aside), call it after the put-back's answer, not the
   write's. Done → quit. Stopped, failed, incomplete or refused → drop the wait; landing and words are unchanged.
3. graft: `Esc` while waiting drops the wait. Order: held-failure dismiss first (§2.2 `Esc`), then the wait, then
   `stop_write`'s line. `q` while waiting: nothing changes (no second box).
4. graft: the waiting status words survive the write's progress updates (rebase `step i of n`, busy lines).
5. graft-ui `banner_list` (`HintContext::Banner`): `q quit` becomes the last hint on the banner bar, the first one
   dropped when the row is narrow.
6. Re-shoot every capture the banner bar change makes stale. Re-assemble the contact sheets they sit in. Update the
   captions and TESTING.md rows that quote the bar.
7. MAP.md rows: graft `update::quit`, `ConfirmThen`, `AppState` field; graft-ui hints, if `banner_list` has a row.

Out:
- the stopped-operation box (`stopped_box`; ruling 2b pending with the user) and its wording (review Minor 4)
- `Ctrl+C` behaviour (see Question)
- whether the resolver (§2.6, modal) answers `q`. Not asked here.

## Signatures
These are crate-private. The scout may correct a name to match the code; MAP.md records the final shape.
```rust
// graft, update/quit.rs — what [ Quit ] / y does; replaces `ConfirmThen::Quit { stops: Option<JobId> }`
pub(crate) enum QuitThen {
    Now,             // nothing running (today's `stops: None`)
    Stop(JobId),     // Stoppable::Yes — stop it, then quit (ticket 13, unchanged)
    WaitFor(JobId),  // Stoppable::No — quit once it ends done
}
// ConfirmThen::Quit { then: QuitThen }

// graft, AppState
pub(crate) quit_when_done: Option<JobId>,

// graft, update/quit.rs — called by each Stoppable::No answer path, after apply + journal
pub(crate) fn write_ended(state: &mut AppState, job: JobId, done: bool);
// Esc chain: true when it dropped a wait (so the Esc goes no further)
pub(crate) fn drop_wait(state: &mut AppState) -> bool;
```
`done` is true only for a complete answer (`Operated::Done`, "up to date" included). An Incomplete answer is not done.

## Touches (check each one, and say how in the report)
- `update/quit.rs` `running_box`, `confirmed`; `ConfirmThen::takes_y` and `PendingConfirm::yes` stay `ConfirmYes::Quit`.
  No new `ConfirmYes`, so the graft-ui `COVERAGE_SLOTS` count does not change.
- Answer paths of a `Stoppable::No` launch. The review lists the launch sites: `operate.rs:116-131`,
  `banner/resume.rs:46`, `preview/answer.rs:271`, `undo.rs:507`. Each one's answer must reach `write_ended`. A test
  per path, or one choke point shown by the scout.
- Set-aside put-back chains (ADR-0045): quit after the last answer.
- The `Esc` chain (`escape_key`, `stop_write`, the held failure) and the commit editor's `Esc` through `stop_write`.
- Status priority: the waiting line must not be overwritten by a busy or progress line, and must not outlive the wait.
- `?` help `IN PROGRESS` / `HERE` for the banner (§2.12): lists `q` if it lists the banner's keys.
- graft-ui: `hints` banner context tests, `the_bare_keys_are_a_tail_not_a_scatter`, the bar-fragment test.
  graft: `every_bound_action_is_hinted_live_or_is_named_here`.
- Captures that draw the banner bar. Grep `docs/shots` for `M mergetool  e edit  ⇥ graph` and `a abort`: m6
  `m6-06-relaunch-sticky-80x24.txt`, `m6-contact-sheet-2.txt` among them. Also their captions (`*-captions.tsv`) and
  TESTING.md rows.
- `m8-quit-while-running-80x24.txt` (fetch box): unchanged words. Confirm it still matches.
- Journal and undo: no new entry. The quit waits for the write's entry.

## Acceptance
Each tick names its test inline.
- [ ] `q` during a `Stoppable::No` write (a merge) → box `Quit?`, lines exactly `<desc> is still running · quit
      waits for it to finish` and `if it stops at a conflict or fails, you stay to see it`, buttons
      `[ Cancel ] [ Quit ]`, cursor on Cancel, bar leads `y quit`.
- [ ] `y` → box closed, app running, no `CancelJob` emitted, status `<desc> · quitting when it finishes · esc stays`.
- [ ] Then the write answers done → app quits; the write's journal entry exists (read the journal after the quit).
- [ ] Answers stopped at a conflict → no quit; resolver open on the first conflicted file; wait gone (a later done
      write does not quit).
- [ ] Answers failed → no quit; the failure's own words on the status row.
- [ ] Answers incomplete → no quit; its warning kept.
- [ ] `Esc` while waiting → wait dropped, write runs on, then answers done → no quit.
- [ ] A progress update while waiting keeps the waiting words.
- [ ] `q` during a `Stoppable::Yes` write is unchanged: ticket 13's `quit::tests` still pass, names listed.
- [ ] `q` on the focused banner with a `c` continue running → this box (extends
      `q_on_the_banner_while_a_write_runs_names_the_write`).
- [ ] Banner bar: `q quit` is the last hint at 187 columns. At 80 columns it is dropped before any other banner hint.
      graft-ui hints test.
- [ ] Captures (colour **unverified**): `docs/shots/m8-quit-waits-80x24.txt` (the box over a running merge, then
      the waiting status row); `docs/shots/m8-banner-bar-q-187x50.txt` and `-80x24.txt` (focused banner after a
      conflict stop). A capture drawn from a synthetic running state, as `capture_quit_while_a_fetch_runs` is, says so.
- [ ] Stale captures re-shot and contact sheets re-assembled; list them.

## Question (out of scope, no line verified: rule 7)
`Ctrl+C` during a `Stoppable::No` write. The review says `shutdown` cancels every job (`event_loop/startup.rs:205`,
read, not observed). Does that signal a git child halfway through a rebase? The builder files what the scout finds,
with lines. The conductor decides.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
One context, ~40k tokens. Crates: graft and graft-ui (a two-file touch in graft-ui).
