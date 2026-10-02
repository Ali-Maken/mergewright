# G-daily 1 — where he lands after pull, undo, redo, a partial answer, and `q`

Status: ready
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Opus**: rulings below; each a landing test.
Blocked by: none
Needs: none in code. Runs beside the scroll lane on **disjoint files** (ADR-0051): `update/operate*`, `update/undo.rs`,
the quit path — not the cursor-motion/detail-follow files `settle-work-at-rest` holds; the conductor checks both
scouts' file lists before running them together.

## Read first
- `docs/research/ux-principles.md` §3.1 (where he lands)
- `docs/design/02-keymap.md` §2.2 (`q` row, `u`/`U` rows)
- `docs/design-notes/m8-polish-and-ship.md`: Need row "Two daily rulings"; Journeys J18, J20; "Where he lands" last row
- `crates/graft/MAP.md`: `update::operate`, `update::undo` (`land_after_undo`, `undo_landing_owed`), `update::banner`
  (`continue_words`); `crates/graft-ops/MAP.md` `Operation::moves_head`, `journal::undoable`

## Rulings (his, 2026-10-02, and planner) and the rows they close
1. **Pull with nothing new: the cursor stays** (his ruling). Today it moves to HEAD (`crates/graft/src/update/
   operate.rs:959,980`; `graft-ops operation.rs:1223`). After a pull whose before equals after on every field: cursor
   where it was, status `up to date ✓`.
2. **Undo and redo land on the result.** After `u` the cursor sat on the stash row; after `U` off the redone commit;
   `land_after_undo` acts only with an operation in progress (`crates/graft/src/update/undo.rs:270-277`). Ruling: after
   any `u`/`U` that answers `Done`, land as `--continue` does: on HEAD when HEAD moved, else on the row of the ref the
   entry touched first, else stay; tag rows that came back as after a finished operation.
3. **A partial answer lands too.** `Operated::Incomplete` (`NotAsTheyWere`) neither lands nor tags recovered rows nor
   drops `error`, unlike the complete answer (`crates/graft/src/update/operate/outcome.rs:26-40` vs `:336-338`).
   Ruling: land and tag as complete; keep its own warning words on the status row.
4. **`q` while a write runs.** Keymap §2.2 "confirms if an operation is in progress", read as: a write job running
   (it would be cancelled). Box: `<op> is still running · quit stops it` `[ Cancel ] [ Quit ]`, `y` quits. A stopped
   git operation (banner showing) quits without a box: relaunch offers continue/abort (PROMPT §9). Wording of §2.2 is
   the conductor's to amend (Proposed in the note).

## Scope
In: rulings 1–4. Out: `a` reword (G-daily 2). Any landing not listed.

## Signatures
None public. `land_after_undo` may be generalised; MAP.md records the shape.

## Acceptance
- [ ] J18 `pull-nothing-new` = 1: `p` on an up-to-date branch, cursor on a non-HEAD row → stays; status `up to date ✓`
      (walk test).
- [ ] J20 `undo-see-result` = 1: `u` after a commit → cursor on the new HEAD (parent); after a stash → on HEAD; `U` →
      on the redone commit (tests, one per operation kind that moves HEAD, plus a branch delete: lands on the
      recreated branch's row).
- [ ] Incomplete answer: lands, tags, keeps its warning (test).
- [ ] `q` during a fetch job → box; `y` quits and the job is cancelled; `q` with a stopped rebase → quits, relaunch
      offers continue (tests).
- [ ] Captures: `docs/shots/m8-landing-undo-120x36.txt`, `m8-quit-while-running-80x24.txt`. **Colour unverified**.

## Touches
- `Operation::moves_head` callers; `undo_landing_owed`; status line priority; confirm box (`y` per M6 rule); the
  journal browser's own undo path (`J` → revert-to) — same landing? check and state; captures after `u` (re-shoot);
  m5/m6/m7 walks asserting cursor after pull or undo (grep `LandedOnHead`); MAP.md graft.

## Scout questions
- Every place the cursor is set after an operation answer (file:line); which read the journal entry's touched refs?
- Is a running write distinguishable from a fetch/read job in `AppState` today?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~50k tokens.
