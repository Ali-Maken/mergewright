# Scout 12d — launch lands on HEAD (current main, 88c17227)

Paths relative to `crates/graft/`.

## Where the first place's arrival is set
- `src/event_loop/places.rs:114` `let mut opened = false;` — `:119-121` builds `Arrival { status, on_head: opened, .. }` per `Place::Repo`; `:131` sets `opened = true` only after an `Exit::Open`. So first place → `on_head: false`. Confirms ticket.
- `src/event_loop/startup.rs:323-324` `if arrival.on_head { crate::update::lands_on_head_at_open(&mut state); }` — the only consumer; after `seed_keys` (:318), before `pump_with`.
- `src/update/mod.rs:133-143` `lands_on_head_at_open`: `land_on_head = true`, `head_landing = { refs: Pending id, walk: awaiting, on: Head }`. Doc comment :131-132 says a start inside a repo "lands on the top row, as before" — needs rewording.
- Off a tty (`run`): never calls `lands_on_head_at_open` (only caller is startup.rs:324, reached via `drive`). Could not confirm `run`'s file:line beyond MAP.md:103 "`run`, as before".

## Decision 3 — does a key before the landing get yanked? **Yes, moved.**
- Landing is spent in `src/update/banner/stop.rs:301-330` `land_on_head` (called from `refs_answered` :272 and `walk_started` :289) → `history.seek(head, height)`; it checks only `land_on_head` and the two reads, never whether the cursor moved.
- `land_on_head` is cleared only at `stop.rs:306` (spent), `src/update/mod.rs:1073` (refs read failed), and set at `mod.rs:139`, `stop.rs:194`, `stop.rs:255`. No key path clears it (grep `land_on_head\s*=` over `src`, non-test).
- Harness only: `src/update/prelude.rs:105-109` holds keys while `land_on_head` is owed. A real terminal has no prelude, so `j` before the reads answer is overwritten. Ticket's "Opus" branch applies; the consumption site is `banner/stop.rs:301` (not `update/mod.rs`) — the clearing condition could sit in key handling instead; builder picks, conductor notes.

## Knock-on
- Test: `tests/it/outside_repo.rs:308` `starting_inside_a_repository_remembers_it`.
- Workaround: `tests/it/palette_find.rs:57-64` `repo()` switches to `feature/three` (:64).
- MAP.md:103 ("a place reached by an open lands on `HEAD`"), MAP.md:273. Ticket's line numbers hold.
- Captures: 410 files in `docs/shots`; which ones show a launch with HEAD not on the top row **could not confirm** without a run.
- Other `live_landing_owed` users (`update/relaunch.rs:78,171`, `mod.rs:849`, `undo.rs:260`) are a separate flag, untouched.

## Overlaps
- 17c: `update/mod.rs` (possibly :774), MAP.md, `view/*`, `event_loop/graphics.rs`. Shared with 12d: **MAP.md**; `update/mod.rs` only if 12d rewords the :131 doc comment or puts Decision 3 there.
- 11c: graft-ui hints, `tests/it/hint_bar.rs`, `update/tests.rs`, MAP.md, captures. Shared: **MAP.md**, `docs/shots` re-shoot.
- If Decision 3 lands in `banner/stop.rs` or key handling: no file shared with 17c/11c except MAP.md.
