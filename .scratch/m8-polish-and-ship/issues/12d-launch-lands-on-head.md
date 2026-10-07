# G-look 2d — launching inside a repository lands on HEAD, as an open does

Status: done (builder 2026-10-08) — was: ready (planner spec-only pass 2026-10-07; from 12's NOTES question, "cursor on the newest row at launch")
Build with: **Opus** (conductor 2026-10-07 after scout `scouts/12d-launch-head.md`: the "moved" case holds — banner/stop.rs:301 land_on_head overwrites a key typed before reads answer; fix sits in banner/stop.rs, update/mod.rs only for the :131 doc comment). Was: Expected **Sonnet** if the scout
confirms the fix is the start path arming the existing landing (one flag); **Opus** if the owed landing yanks a cursor
he already moved (see Decision 3 — touches the landing's consumption in `update`). Not Fable: every box is a pty
oracle (row under `▸` vs HEAD's row).
Blocked by: **scout first** (`scout`, Opus low, ≤1 page, `.scratch/m8-polish-and-ship/scouts/12d-launch-head.md`).
Source: none if the fix stays in `event_loop/places.rs`. **If Decision 3 needs `update/mod.rs`: wait for 17c to merge**
(17c may edit `update/mod.rs:774`). **Capture step serial** with 22c's and 11c's capture runs (all rewrite
`docs/shots`). Two cargo helpers at a time.
Needs: from 08 — `places::drive`, `Arrival { status, on_head }`, `update::lands_on_head_at_open`; `HeadLanding`,
`AppState::{land_on_head, head_landing, live_landing_owed}`; fixtures `FeatureBranches` (HEAD `main`, newest row
`feature/three` — 12 NOTES).
Crate set: graft only.
**Files (expected; scout confirms):** `event_loop/places.rs` (the first place's `Arrival`); `tests/it/outside_repo.rs`;
`tests/it/palette_find.rs` (`repo()`'s workaround only); `MAP.md` (:103, :273). Decision 3 only: the file where an owed
landing is consumed (scout names it). Captures the run rewrites (listed in NOTES).
**Not:** `update/mod.rs` unless Decision 3 and 17c merged; `view/*`, `event_loop/graphics.rs` (17c); graft-ui.

## The defect, with its lines
- Design note `docs/design-notes/m8-polish-and-ship.md` :100 — every journey starts "his repo, graph, cursor on HEAD";
  :160 — "Open / init succeeds → that repo's graph, cursor on HEAD"; :167 clone done → HEAD.
- 08 NOTES (`08-look-outside-repo-screen.md` :190–192): `lands_on_head_at_open` arms the landing for an open from the
  empty state; "a start *inside* a repository still lands on the top row, as before
  (`it::outside_repo::starting_inside_a_repository_remembers_it`)". MAP.md :103, :273 say the same.
- 12 NOTES (Question): launched on `FeatureBranches`, cursor stayed on `feature/three` 20 s; the walks work around it by
  switching the fixture to `feature/three` first.
So the two arrivals disagree by construction, not by a bug in the landing. **Scout confirms at current main**
(rule 7): the line where the first place's `Arrival` is built and its `on_head` value; that `run` (off a tty) never
arms it; how the empty-state open sets it.

## Decisions (planner)
1. **Every arrival on a terminal lands on HEAD: launch, open, init, clone.** The first place `drive` runs gets the same
   `on_head` an `Exit::Open` gets — one path, no second landing. Why: the design note's journey start state (:100) and
   its Lands table (:160, :167); UX row 4 (same event, same result: arriving in a repo); his team works on dependent
   branches (memory) — after a fetch the newest row is often a teammate's tip, not his. Rejected: arming it in
   `opening` (every harness and unit test would owe a landing at start — large knock-on for no user gain); keeping
   the top row (contradicts :100, and 12's walks already had to work around it).
2. **Where on HEAD:** exactly where an empty-state open lands on the same fixture (dirty tree, pinned uncommitted row,
   HEAD below the first window — the existing `HeadLanding` path, unchanged). The test compares the two, so this can
   never drift apart.
3. **A key before the landing arrives.** At launch he types at once (J1: hold `j`). Scout answers with file:line: when
   the landing is owed and a cursor-moving key lands first, does the landing later move the cursor? **Not moved** →
   nothing to do, pin it. **Moved** → in scope: the first key that moves the graph cursor drops the owed launch/open
   landing (one condition where the landing is consumed); `--continue` and operate landings are untouched (they follow
   a write he asked for). Builder confirms by test, not by reading.
4. **Off a tty (`run`) unchanged** — no one sees it; NOTES lists the tests that use `run` (`rg -n 'fn run\b|::run('`).
5. 12's workaround goes: `tests/it/palette_find.rs` `repo()` no longer switches to `feature/three`; the walks start on
   `main` = HEAD, as their fence says. If a walk then fails for another reason: restore the switch, report with
   file:line.

## Scope
In: Decisions 1–5; MAP.md :103/:273 wording ("a place reached by a launch or an open lands on HEAD"); NOTES asks the
conductor for a Lands row in the design note: `Launch inside a repo | graph, cursor on HEAD`.
Out: the tour's return (19: "his repo, cursor where he left it", design note :164 — must not take this path; NOTES
says whether 19's return goes through `Exit::Open`, file:line, as a flag for 19); headless `run`; landing after
`--continue`/operations.

## Signatures
None new. Expected change (scout confirms the shape): `places::drive`'s first `run_repo` passes
`Arrival { on_head: true, .. }` (today `false`, 08 NOTES :191). Decision 3, "moved" branch only: no new pub item; the
condition sits where the owed landing is consumed (NOTES: file:line).

## Acceptance (each tick names its test, at tick time)
- [x] Launch lands on HEAD (`it::outside_repo::starting_inside_a_repository_lands_on_head`, red on base: cursor on `[feature/three] three: in progress`): binary on `FeatureBranches` under a pty → `▸` on `main`'s row, not `feature/three`'s; red
      on base first. (`it::outside_repo::starting_inside_a_repository_lands_on_head`)
- [x] Launch and open land alike (`it::outside_repo::launch_and_open_land_alike`, red on base: timed out, cursor on `side 39`; 8/8 runs green): same fixture with a dirty tree and HEAD below the first window — launched, and opened
      from the empty state — the cursor row and pane top are equal. (`it::outside_repo::launch_and_open_land_alike`)
- [x] 08's test (`it::outside_repo::starting_inside_a_repository_remembers_it`, body unchanged; it had no top-row assertion, only its doc comment said so — now says HEAD): `starting_inside_a_repository_remembers_it` green; its "remembers" assertion unchanged; any top-row
      assertion flipped to HEAD (NOTES quotes old → new).
- [x] Decision 3, **moved** branch (`update::banner::tests::arrival_landing::a_key_before_the_launch_landing_keeps_his_cursor`, red on base: cursor oid(2)=HEAD instead of oid(4); plus `with_no_key_the_launch_lands_on_head`, `a_key_does_not_drop_a_continue_landing`) — unit level, not pty, see NOTES: launch on `FeatureBranches`, `j` sent before the walk answers → after it lands, the cursor is where
      `j` put it, not HEAD (or, "not moved" branch, the same assertion pins today's behaviour).
      (`it::outside_repo::a_key_before_the_launch_landing_keeps_his_cursor`)
- [x] 12's walks without the workaround (`walk_seek_branch`, `walk_seek_tag`, `walk_rerun_recent`: 5/5 runs, a shell loop of 5 × `cargo nextest run -E 'test(/walk_seek_branch|walk_seek_tag|walk_rerun_recent/)'`): `walk_seek_branch`, `walk_seek_tag`, `walk_rerun_recent` green, 5/5 runs
      (NOTES: the loop used); `repo()` diff shows the switch removed.
- [x] Captures (full `GRAFT_UPDATE_SHOTS=1 cargo nextest run`, then plain: 5896/5896; `git status docs/shots` empty — no capture changed, see NOTES): one full `GRAFT_UPDATE_SHOTS=1 cargo nextest run`, then plain green; `git diff --stat <base> --
      docs/shots` lists only files whose fixture's HEAD is not the top row; NOTES per file: old → new cursor row (and
      the detail pane that follows it). Any other change → stop. Colour unverified (rule 5).
- [x] MAP.md :103, :273 (`rg -n 'lands on .HEAD' crates/graft/MAP.md` → :103, :273) read the new rule. Check: `rg -n 'lands on .HEAD' crates/graft/MAP.md`.

## Touches
- `places::drive` callers (`main` on a tty) and its `Exit::Open` arm (unchanged — diff shown).
- Harness prelude (`update::prelude` feeds no key while `land_on_head` is owed, MAP :172): pty walks now wait for the
  landing at launch — full nextest shows timing holds; any walk that sends keys before the frame settles is named.
- Recent list (`recent.rs`): launch still remembers the repo (08's test).
- Tour (19), clone (18): both arrive through `Exit::Open`; this ticket does not change that arm.
- Hint bar / header: follow the cursor row (the captures above).
- Keymap: none. Journal/undo: none (`git diff <base> -- crates/graft/src/update/journal* crates/graft/src/update/undo*`
  empty).
- 17c: no shared file unless Decision 3 lands in `update/mod.rs` (then after 17c). 11c, 22c: capture runs serial.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~30k: start path + tests ~12k; Decision 3 (if moved) ~8k; walks + captures ~7k; MAP/NOTES ~3k.

## NOTES (builder, 2026-10-08)
- Scout used (`scouts/12d-launch-head.md`); its lines held at ea870976. No compaction.
- **Decision 1** (cause fix): `event_loop/places.rs` drops the `opened` flag; every `Place::Repo` gets `Arrival { on_head: true, .. }`. `Exit::Open` arm otherwise unchanged (only `opened = true;` removed).
- **Decision 3 = moved** (scout: `banner/stop.rs` `land_on_head` never checked the cursor). Fix: `HeadLanding::gives_way` (pub(crate) field, `update/state.rs`), true only in `update::lands_on_head_at_open`; `banner::ArrivalWatch` (`banner/stop.rs`) notes the graph cursor (`on_working_tree`, selected commit) before a key/click and drops the owed landing if it moved; wired beside `LandingWatch` in `update/mod.rs` `answer`'s Key/Mouse arm. `--continue`, operate and relaunch landings set `gives_way: false` and are untouched (`a_key_does_not_drop_a_continue_landing`). 17c had merged (fd0b66c8), so `update/mod.rs` was open.
- Decision 3's test is unit level (through `update::update`), not a pty walk: on a pty the order of `j` vs the walk's first rows vs the refs answer cannot be fixed, so a pty assertion would be flaky or vacuous. A `j` before any rows exist moves nothing, and the landing still lands on HEAD (by design: "a key that moves the graph cursor").
- `launch_and_open_land_alike` first flaked 4/6: the working-tree read answers on its own time and its pinned row shifts every row. The test now waits for the `Uncommitted changes` row as well as the cursor (a settled-frame wait, not a sleep); 8/8 green after. The full plain run (5896/5896) predates this test-only change; `outside_repo` re-run 8× after it.
- **Captures: none changed.** Some were expected; no capture run reaches `places::drive` with a repo whose HEAD is not the top row (the harness captures start from `opening`, not `drive`). Not checked capture by capture.
- `raster::write_shots` (graft-graph) is red under `GRAFT_UPDATE_SHOTS=1` ("regenerated 10 capture(s)", PNGs), yet `git status docs/` is empty afterwards. Unrelated to this ticket, not investigated; green in the plain run.
- Off a tty (`run`, `src/event_loop/startup.rs:209`, called only from `src/main.rs:91`): unchanged, never calls `lands_on_head_at_open`.
- **Flag for 19 (tour return):** no place produces `Exit::Open` from the graph yet (`src/event_loop/mod.rs:86`). Every place `drive` runs now lands on HEAD, so 19's "cursor where he left it" must not return through `Place::Repo` in `drive` as it stands; it needs its own arrival (e.g. `on_head: false` for a return).
- **Ask conductor:** add the design-note Lands row `Launch inside a repo | graph, cursor on HEAD`.
- Touches: `places::drive` (the only `run_repo(` caller; `main` on a tty); `startup.rs:323` consumer unchanged; recent list (08's test green); harness prelude (`update/prelude.rs:109` still holds keys while owed; full nextest green); journal/undo: no file under `update/journal*`, `update/undo*` in the diff; keymap: none; captures: none changed.
