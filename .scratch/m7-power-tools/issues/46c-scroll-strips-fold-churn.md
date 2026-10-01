# scroll-strips-fold-churn: confirm the fold re-sends strips, then make the Pixel gate measure the right thing

Status: ready
Build with: Opus. Step 0 is a premise check, and its result decides everything after it; reading a red result
correctly needs judgement. The work is test-only (no signature, no structural choice), so it is not a Fable job.
Needs: 46 merged at 92c12351 or later: `PixelRows::{backdrop, lane_columns, marker}`, `StripRows::with_layers`,
`Graphics::new(replies, &Capabilities)`, `event_loop::tests::pixel_scroll` and its helpers, `fan` (`pub(crate)` in
`view/strips/tests.rs`), `held_key`'s `_pixel_deflate` / `_pixel_iterm2` variants, `KittyTransmit::{width_px,
deflated}`.
Blocked by: 46
Scout: none needed. 46's NOTES and its scout cover these files, and this ticket changes no product code.

File set (crate `graft`, tests and docs only):
- `src/event_loop/tests/pixel_scroll.rs`
- `src/view/strips/tests.rs` (one new fixture beside `fan`)
- `tests/it/held_key.rs` (Pixel verdict, one reading)
- `TESTING.md` ("Held keys" paragraph)
- `crates/graft/MAP.md` (`held_key` line)
- `docs/perf/fix-scroll-held-key.txt` (append)
- `scripts/fix-scroll-held-key.sh` (only if its summary selects tests by name)

**No product code:** do not edit `view/rows.rs`, `view/strips.rs` (`plan`, `keep_for`) or graft-graph's fold. If you
need one of them, stop and report `blocked: needs <file:line>`.

## Read first
- `46-scroll-strips-wiring.md`: "Planner ruling 2026-10-01 (46 return)" (whole), NOTES 1–3.
- `docs/adr/0025-the-lane-cap-is-a-budget-and-pins-are-priorities.md`, Decision 2.
- `docs/adr/0053-selection-paint-is-its-own-layer.md`, Decision (the key).
- `42-scroll-held-key-gate.md`: Decided "Readings" and "Verdict".
- `crates/graft/MAP.md`: `## view::strips`, and the `held_key` line (`:907`).

## Scope
- **In:**
  - Prove or disprove that per-`j` strip re-sends on a capped graph are fold changes.
  - If proved: guard ADR-0053's real invariant in the fast tier, restate `held_key`'s strip bound, and run the gate.
- **Out:**
  - Any change to the fold, the lane cap or ADR-0025 (ruling (a), rejected 1–4).
  - Deflate off (`scroll-kitty-without-deflate`).
  - 46's NOTES 5.

## Decided
1. **Step 0 comes before any verdict edit, red first.** Add a fixture `capped` beside `fan` in `view/strips/tests.rs`.
   It must meet three conditions:
   - The window holds more open lanes than `max_lanes_for` allows at the test's width.
   - Moving the cursor one row with `j` changes the thread's columns (`Ancestry::columns`).
   - At least one of those columns is not a node column in the window, so class 5 decides a fold.

   Write `pixel_scroll::a_capped_graph_resends_strips_only_when_the_fold_changes`. First give it 46's **old**
   assertion: 0 backdrop transmits on any `j` inside the resident strips. **Run it red and keep the output.** Each red
   frame must be one whose fold plan differs from the previous frame's. The fold plan is the kept column set as
   `pixel_rows` / `plan` has it, plus `lane_columns` and `marker`, read through `pub(crate)`. Print, per frame: the
   plan changed (y/n), which `Keep` class differs (`selected` / `preferred` / other), and the backdrop transmits.
2. **Step 0's outcomes:**
   - **(i) Every red frame is a fold-change frame:** the premise is confirmed. Go on to Decided 3.
   - **(ii) A backdrop transmits on a frame whose plan did not change:** stop. The old bound stands, and that frame is
     a defect (paint or something else per-`j` in the backdrop key). Report `blocked: backdrop re-sent with the fold
     unchanged`, with the printed frame, for `debugger`.
   - **(iii) No `j` changes the fold:** the fixture is wrong. Rebuild it. Never weaken the three conditions in
     Decided 1.
3. **Fast-tier guard (the real invariant):** the same test, now asserting all three of these:
   - (a) A frame whose fold plan and strip window are unchanged has 0 backdrop transmits.
   - (b) Within the script, at least one frame changes the fold. This proves the fixture is capped.
   - (c) Every frame has at least 1 emphasis transmit where its `Lit` changed.

   46's `j_inside_resident_strips_sends_no_strip` (uncapped, 0 on every `j`) stays as it is.
4. **`held_key` Pixel verdict.** Replace `strip_transmits_held ≤ ceil(rows_scrolled/32) + 2` with
   `strip_transmits_held ≤ ceil(rows_scrolled/32) + 2 + fold_changes_held × strips_per_frame`, where
   `strips_per_frame = ceil(screen_rows/32) + 1`. Add `fold_changes_held` to the SUMMARY line: the number of held
   frames whose fold plan differs from the previous frame's.
   - **Only if** `held_key` can read the drawn fold per `Drawn` through what `frames::{run, paced}` already exposes.
   - If it can't, add **no** public product API for it. Print `fold_changes_held=unread` and print
     `strip_transmits_held` without asserting it; Decided 3 carries the guard. Say which case applied, with the line
     that shows it.
   - Every other limit is unchanged.
5. **Gate run** via `scripts/fix-scroll-held-key.sh`, appended under `# rev fold-churn`:
   - **Must be `ok`:** `_pixel_deflate` A and B, `_pixel_iterm2`, and the `_unicode` pair.
   - **Expected red:** the deflate-off `_pixel` pair (ruling (b)). Quote their lines and don't touch their verdict.

## Signatures
```rust
// crates/graft/src/view/strips/tests.rs (test-only, pub(crate) like `fan`)
pub(crate) fn capped(/* chosen by the builder; doc comment states lanes, cap, width */) -> /* `fan`'s return type */;

// crates/graft/src/event_loop/tests/pixel_scroll.rs
#[test] fn a_capped_graph_resends_strips_only_when_the_fold_changes();

// crates/graft/tests/it/held_key.rs: SUMMARY gains `fold_changes_held=<n|unread>`; no new test fn
```

## Acceptance (each box names its test)
- [ ] Step 0 red under the old assertion, every red frame a fold-change frame, output quoted in NOTES with the class
      that changed. — `pixel_scroll::a_capped_graph_resends_strips_only_when_the_fold_changes` (red revision noted).
- [ ] The same test green with Decided 3's (a)–(c). — same test.
- [ ] `fixture capped` really is capped: open lanes > `max_lanes_for(width)` asserted in the test. — same test.
- [ ] 46's `pixel_scroll::j_inside_resident_strips_sends_no_strip` still green and unchanged. — that test.
- [ ] `held_key` Pixel verdict per Decided 4 (state which case applied). — `held_key::held_j_stops_on_release_pixel_deflate`.
- [ ] Release gate: `_pixel_deflate` A/B, `_pixel_iterm2` and `_unicode` A/B every verdict item `ok`; deflate-off
      lines quoted, red. — `scripts/fix-scroll-held-key.sh` → `docs/perf/fix-scroll-held-key.txt` `# rev fold-churn`.
- [ ] `TESTING.md` "Held keys": the restated bound and why (fold follows the selection on capped graphs, ADR-0025;
      ADR-0053 keeps the paint out), the fast-tier guard by name, and deflate off is red by known gap.
      — no test; diff quoted.
- [ ] `crates/graft/MAP.md` `held_key` line: `fold_changes_held`; `pixel_scroll` line names the new test.
      — no test; diff.

## Touches (report each on the **Touches:** line)
- 47 (`:37-39`, `:59`): its quote of the bound goes stale. Name it, but don't edit it; the conductor amends 47.
- `scripts/m7-perf-key.sh`: its filters must still select no `held_key::` (grep, quote).
- The text tiers: no product code changes, so no capture moves. Prove it with `git diff --stat` limited to the file
  set.

## Gates
Run workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
`cargo test --doc`. Also run `cargo clippy --all-targets --features heavy-tests -- -D warnings`. Release runs go
through the script only.

## Size
About 35k tokens.
