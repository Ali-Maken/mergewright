# review-fix-amend-11-12: count cut mid-number at 187 cols, banner.rs split, doc drift, dead `y`, silent landing drops

Status: ready
Blocked by: none (11, 12, 23 merged)
Was: filed by the conductor 2026-09-24 from `docs/reviews/m6-amend-11-12.md` M1, M2, M3, Minor 1, Minor 3. Minor 1 restores `02-keymap.md:111` ("a key the menu displays must actually work"), which outranks 11's scope item 4 — conductor's ruling.
Build with: **Opus** (medium).
Needs: none.
Crates: graft, graft-ui.

## Read first
- `docs/reviews/m6-amend-11-12.md`: M1, M2, M3, Minor 1, Minor 3 — each names file and line (note: `hints.rs` was split by ticket 23 after that review; `stopped_row` now lives under `graft-ui/src/graph_screen/hints/`).
- `crates/graft/MAP.md`, `crates/graft-ui/MAP.md`.
- `ARCHITECTURE.md` before moving modules; `codebase-design` for the split.

## Scope
In:
1. **M1.** `stopped_row`'s `fit(reason, room)` cuts characters from the end, so 13 files reads `1…` at 187 cols with the pane open. Cut the reason at `·` boundaries: drop whole parts, lowest value first — `begun outside Graft`, then `from a previous run`, then the step. The count part is kept whole or dropped whole, never cut inside. Test at 108 cells with 13 files. Recapture `docs/shots/m6/fix-landing-b-launch-187x50.txt`.
2. **M2.** Move the stop/landing concern out of `update/banner.rs` (`:574-793`: `stopped_again`, `finished_lands_on_head`, `refs_answered`, `walk_started`, `land_on_head`, `stop_reason`, `stop_line`, `one_tab_to_banner`) into `update/banner/stop.rs` or `update/relaunch.rs` — your call, say why. Pure move. `preview.rs`, `state.rs`, `rows.rs`: not in scope — the conductor adds a size row.
3. **M3.** `update/mod.rs:539-542` doc comment back above `took_fact`; `crates/graft/MAP.md:224` corrected (`conflicted_line` gone; `Conflicted` sets `AppState::stopped`); `tests/it/conflict_interim.rs` `capture_banner_for_docs_shots` dead `Tab` round trip removed (re-check the shot unchanged) or its purpose stated.
4. **Minor 1.** `update/preview.rs:459-468` `footer_keys`: set `confirm_key`/`again` only when the plan `acts()` or is `planning()`. Test: a `closing_box` and a can't-act plan draw no `y confirm`; `every_key_the_bar_draws_is_answered` (or equivalent) covers them.
5. **Minor 3.** `banner.rs:717-719`, `:721-723` (moved by item 2): each silent `return` either gets a comment saying why dropping is safe, or stops being silent. Then answer the review's question with a test: `Ctrl+R` straight after `c` (a second refresh replacing the walk before it starts) — is `land_on_head` left owed forever? If the test shows it is, fix at the cause (point `head_landing.walk` at the newest walk, or drop the landing when `awaiting` changes). If it shows it isn't, say so in NOTES with the test name.

Out: every other Minor (they go to KNOWN_GAPS); ticket 13's work.

## Acceptance (each tick names its test, at tick time)
- [ ] M1: 13 files at 108 cells keeps the count whole or drops it whole — test:
- [ ] M1: recaptured 187x50 shows the count — (capture path)
- [ ] M2: stop/landing code moved; `banner.rs` line count in NOTES — (gates green)
- [ ] M3: three drift fixes — (textual; name the lines)
- [ ] Minor 1: no `y confirm` on a closing box or a can't-act plan — test:
- [ ] Minor 3: no silent drop without a reason; Ctrl+R-after-`c` question answered — test:
- [ ] Gates workspace-wide: fmt, clippy `-D warnings`, nextest, doctests

## NOTES
