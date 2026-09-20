# Review: M5 group 04 — ghost rows, preview box, N-choice confirm, banner, CONFLICTS, name prompt; graft seam + seeded pulse

Range: `3b9d855..8305e8f`, restricted to `fad9f9b` (graft-ui widgets) and `190a4ed` (graft seam + pulse).
Ticket: `.scratch/m5-history-and-ghost-preview/issues/04-ui-ghost-banner-conflict.md`.
Reviewer ran **no cargo command** (another builder holds graft-ui + graft). Gates are the conductor's at the merge: `fmt ok · clippy ok · 1885/1885 · doctests ok`. Everything below is by reading; where only a run settles it, the command is named.

**Not run, must still run at the milestone review:** heavy tier (`cargo test --features heavy-tests -- --test-threads=2`), release perf (`cargo test --release -p graft --features heavy-tests --test it -- frames:: heavy::budgets_hold --test-threads=1 --nocapture`), heavy clippy (`cargo clippy --all-targets --features heavy-tests -- -D warnings`). No numbers reported here.

Counts: **1 Critical, 7 Major, 12 Minor.**

## The user's conditions (ADR-0038)

- **§1 Cancel first, cursor on Cancel, every box.** Holds at every construction site I could find: `ConfirmView::new` (Cancel/Confirm, 0), `with_choices` (resets 0), `with_labels` (via `with_choices`); `remote::push_box` both arms `("Cancel","Push")`/`("Cancel","Force push")`; `stash::request_drop` `("Keep","Drop")`; `undo::drift_box` `("Stop", verb)`; `operate::request_discard` `("Cancel", verb)`; `operate::request_discard_at_diff_cursor` (`new`). The app acts on `selected() != 0`. Two holes: the app wires `←` to a wrapping `toggle()` (Major 1) and `PreviewBox` has no constructor pinning anything (Major 5). Labels are not literally "Cancel" everywhere (`Keep`, `Stop`) — the safe one is still first and under the cursor; flagging for the user's call, not as a defect.
- **§2 Keystroke registers, response encodes no length.** Both halves hold by reading `text_input.rs` and the two named tests. `SecretField::new(pulse_seed)` is the only constructor; no `impl Default for SecretField` remains (the `Default` derive at `text_input.rs:86` is `TextField`). `graft-ui/Cargo.toml` names no RNG. One production site: `PromptField::new` ← `credential_needed` ← `Progress::CredentialNeeded { pulse_seed }` ← `runner.rs` calling `askpass::pulse_seed()`. The fallback bit is wrong on macOS (Major 2).
- **§5 Anchor looks like an anchor at every tier.** Node glyph: Unicode `○`/`◎` vs real `●`/`◉`/`◇`/`◆`/`■`; ASCII `o`/`O` vs `*` — unambiguous at both tiers, tested (`ghost_paint_dashes_every_straight_edge_and_forces_the_anchor_glyph`, `ghost_row_ascii_glyphs_are_colon_dot_o_capital_o`). Edges: a ghost row dashes *real* lanes too (Major 4); corners never dash (Minor 6); no NO_COLOR/16-colour capture (Minor 7). The "preview says so when the part that matters is outside the slice" half is owned by ticket 08 (its line 139), not this group; `PreviewBox.lines` is enough to carry it.

## Tick spot-check

11 ticks across both sessions; 10 checked by reading the named test (the workspace-gate tick I cannot check without running). **7 held.** 1 holds as worded and misses the requirement (Critical 1). 1 names a capture whose caption describes a rendering that does not exist (Major 7). 1 asserts a user judgement I cannot see (Major 6).

| Tick | Named test / artefact | Verdict |
|---|---|---|
| piece-9 tests + widgets inside their Rect | `preview_box_stays_inside_its_rect`, `name_prompt_stays_inside_its_rect`, `credential_prompt::tests::stays_inside_its_rect` (exists, proptest), wildcards for ghost/banner/CONFLICTS, `confirm::tests::area_never_exceeds_the_frame` | **held as worded, misses the docking requirement** — see Critical 1; wildcards are not named tests (Major 6) |
| confirm frame at 80×24 and 160×48 | `confirm::tests::confirm_draws_its_frame` | held: asserts `╭` and title in the first non-blank row at both sizes |
| captures | six `docs/shots/m5-ui-*.txt` | exist; preview capture filtered (Critical 1), confirm caption false (Major 7) |
| MAP.md | entries present | held; `name_prompt` entry drifted from code (Minor 3) |
| conflicted path refused | `space_on_a_conflicted_file_says_why_instead_of_staging_it`, `d_in_the_diff_region_of_a_conflicted_file_says_why_instead_of_asking` | held: both assert `Status::Sticky` with "conflicted file" and no command/box |
| every keystroke registers | `every_keystroke_visibly_changes_the_masked_row` | held: 8 inserts, backspace, delete, `Ctrl+U`, both seeds, exactly one cell moves `•`↔`◦` |
| resting row not length | `the_resting_row_is_decided_by_the_seed_not_the_length` | held: 3 and 30 chars produce the same two rows across the two seeds |
| seed from app, no Default | compile-enforced, `view::tests::credential_state_seeded` exists | held by grep |
| captures judged by the user | — | **cannot verify**; ticked by the builder (Major 6) |
| MAP both crates | — | held |

## Critical

### C1. The preview box cannot draw at its own height, the docking test hides it, and the capture is filtered so the user cannot see it
`crates/graft-ui/src/preview_box.rs:166` checks `area.height < PREVIEW_MIN.1` (12) against the **rect the widget is docked into**, but `preview_rows` (line 81) for the fixture is 7. So a caller that docks the box at its natural height always gets `preview needs 40x12 · esc closes`; a caller that hands it ≥12 rows gets the title, 5 content rows, then borderless blank rows (`draw_body` stops writing at the buttons row, lines 119–146, and the rows between were only cleared to spaces at line 178), then the footer at `area.bottom()-1`. Neither is the box the design shows.

`preview_box/tests.rs:55,87,124` work around it: every test sizes the rect `preview_rows(..).max(PREVIEW_MIN.1)` and asserts only the first and last rows. `capture_for_docs_shots` (line 188) then drops every blank row with `if !line.trim().is_empty()`, so `docs/shots/m5-ui-preview-box.txt` shows a tidy 7-row box at "ready, 80x24" when the widget actually drew 16 empty rows between `[ Cancel ]    Confirm` and the footer. The user judges by eye (rule 6) a picture the widget does not draw.

Why it matters: this is the group's central widget, ticked as done and tested; group 08 will dock it and hit one of the two failure modes on day one. The tick "preview box docking (last row of the area is the box's footer …)" holds as worded and misses the requirement — the pattern the conductor warned about.

Fix: check `PREVIEW_MIN` against the *screen* (pass the frame size in, or have the caller decide and hand the widget a one-row rect for the message, as `name_prompt_area` does); draw `│ │` on every interior row so a too-tall rect is still a box; make the docking test assert every interior row starts and ends with `│`; remove the blank-line filter from the capture (or capture at `preview_rows` height). Until then the tick is false.

## Major

### M1. `←` on Cancel wraps onto the acting choice; the clamped `left`/`right` are dead in the app
`crates/graft/src/update/confirm.rs:57`: `KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => pending.view.toggle()`. `toggle` wraps (`confirm.rs:121`). On a two-choice box `←` from Cancel lands on Confirm — exactly the "one key-repeat away from the dangerous one" the widget's `left()` doc (line 106) exists to prevent. On the three-choice box `←` moves right. Pre-existing wiring, but the seam session says every call site was fixed to the new shapes and the ADR §1 rationale is the user's. Fix: `Left => left()`, `Right => right()`, `Tab | BackTab => toggle()`, plus a test that `←` on index 0 stays on 0.

### M2. `pulse_seed`'s fallback is a constant on macOS
`crates/graft/src/jobs/askpass.rs:58`: `since.subsec_nanos() & 1`. macOS's realtime clock is microsecond-granular; on this machine `python3 -c "import time; print([time.time_ns()%1000 for _ in range(8)])"` printed `[0,0,0,0,0,0,0,0]` (CPython and Rust `SystemTime::now` read the same `clock_gettime(CLOCK_REALTIME)`). So the low nanosecond bit is always 0 and the fallback is the constant seed the doc comment says it avoids ("A constant fallback would have put the length leak straight back"). Reachability: only when `getrandom::fill` fails, which on macOS (`getentropy`) is essentially never — hence Major, not Critical. I agree with the conductor that the ignore is genuine under rule 3; the *fallback value* is wrong, not the ignore. Fix: `(since.subsec_nanos().count_ones() ^ since.as_secs() as u32) & 1 == 1`, or `subsec_micros() & 1`; verify when cargo is free with a test that draws the fallback 64 times and sees both values (can't be pinned, so an `#[ignore]`d probe is fine).

### M3. `d` on a CONFLICTS file row is not refused — it opens a discard that resets the file to HEAD
`crates/graft/src/update/operate.rs:562–599` `request_discard` branches on `entry.untracked / is_staged / is_unstaged`, never on the section. A `UU` entry (staged and unstaged both set) reaches `discard_consequence`'s "Both the staged and the unstaged change go back to HEAD" and `git restore --staged --worktree --source=HEAD`, which throws away the user's in-progress resolution. The refusal the ticket claims ("a conflicted path is refused in words, not ignored") covers `Space` (line 168) and hunk-`d` (line 436) only. Latent today (the CONFLICTS section is never populated, KNOWN_GAPS row 474) but live the moment group 09 fills it, and the design gives a conflicted row `M`/`e`/`space resolved`, not `d`. Fix: in `request_discard`, `if section == Unmerged { status = Sticky(conflict_refusal("discard")); return true }` + a test beside the two existing ones. Exact git behaviour on a `UU` path with `--source=HEAD` not verified by running — the refusal is right either way.

### M4. A ghost row dashes and mutes real lanes passing through it — the design shows them solid
`crates/graft-ui/src/graph_column/mod.rs:290–302`: on a `Ghost` row every owned cell becomes `┊`/`┈` + `TextMuted`, whichever lane owns it. DESIGN §1.7's mockup row `┊│┊●b┈  (ghost)` keeps the real lane `│` solid through the ghost row, and `05-visualization.md:80–86` says real nodes stay solid "so the before/after reads as one continuous picture, not two separate graphs". With two ghost rows a real branch's lane breaks twice and reads as ghost. The widget cannot do better with a per-*row* paint; it needs a per-lane/cell ghost mask from the caller (group 08) or a decision accepting the break. The ticket's wording ("edges dashed … a ghost row is dashed regardless") let this through; it is not in the ticket's NOTES as a deviation. Fix: decide (ADR or design-doc line), then either add `ghost_lanes: &[usize]` to `GraphLine` or amend §1.7.

### M5. `PreviewBox` has no constructor: nothing pins Cancel at index 0 or the cursor at 0
`preview_box.rs:38–55` is a plain struct with pub `choices`/`cursor`; `left/right/selected` exist but nothing creates the safe default. `ConfirmView` is also fully pub, so a struct literal bypasses `new`. ADR §1 is enforced only by discipline in pass 2. Fix: `PreviewBox::new(title)` seeding `["Cancel","Confirm"]`/0 and `with_choices` resetting to 0, mirroring `ConfirmView`; consider making `cursor` private on both (`selected()` already exists).

### M6. Tick evidence: wildcards instead of named tests, geometry instead of drawing, and a user judgement ticked by the builder
- Piece-9 tick names `graph_column::tests::*`, `graph_screen::tests::*`, `detail_pane::tests::*` for the ghost/banner/CONFLICTS "draws nothing outside its Rect" claim — no such test exists for any of the three; a wildcard is not a named test.
- `confirm::tests::area_never_exceeds_the_frame` (confirm.rs:383) checks `confirm_area`'s rect, not drawn cells. Holds by reading (`border_row`/`content_row` are exactly `width` cells) but the tick names the wrong evidence.
- Follow-on tick "Captures regenerated and judged by eye by the user (not by me, rule 6)" is ticked. If the user has not looked, that is a false tick — Critical by rule. Conductor to say which.

### M7. The confirm capture's caption describes a reversed cursor button that the widget never draws, and unselected choices lose their brackets
`confirm.rs:500–509` caption: "the cursor's reversed button always on `Cancel` … judge the reversed-button glyphs". `Confirm::render` (lines 185–240) applies no `REVERSED` anywhere; the only cursor cue is `[ … ]` vs two spaces (`button`, line 168). The capture shows `[ Cancel ]    Undo locally      Re-push old tip` — the design note and DESIGN §1.7 draw every choice bracketed (`[ Cancel ]  [ Undo locally ]  [ Re-push old tip ]`); a bare "Re-push old tip" does not read as a button, and on the box whose whole job is to say what the third choice does, that matters. Fix: bracket every choice, mark the cursor by `REVERSED` (colour, unverified — say so) plus a text cue that survives NO_COLOR (e.g. `▸`), and rewrite the caption to what is drawn.

## Minor

1. `frame_box.rs:24` doc says "label, a space, dashes" but no space is written after the label: `╭ Preview: … main───` vs the mockups' `main ───`.
2. `preview_box.rs:82` `let _ = width;` — name the parameter `_width` (or wire it) rather than discarding inside the body.
3. `crates/graft-ui/MAP.md` `name_prompt` entry says `option_label`, `option_on`, `footer`, `field: &'a FieldView`; code is `option: Option<(SanitizedText, bool)>`, `field: FieldView<'a>`, no `footer`.
4. `name_prompt.rs:191` footer always reads `⇥ toggle`, including the tag prompt where `option` is `None`.
5. `name_prompt.rs:104` writes `│ ▏` (3 cells) before the `width <= 4` guard — bleeds past a rect narrower than 3 if a caller bypasses `name_prompt_area`.
6. Ghost corners/junctions never dash at either tier (`○┈╯`, ASCII `o./` — the `/` is a real row's glyph); ASCII ghost vertical `:` is also the fold RAIL's glyph. The ticket asked for corners "drawn with the dashed set"; `style.rs:22–25` explains the choice in a comment but the ticket NOTES do not list it as a deviation.
7. No ghost test or capture at `Palette16`/`NO_COLOR`, where real odd-column lanes already draw `╎`/`╌` beside ghost `┊`/`┈` with no colour to separate them. Nodes still differ; edges overlap. One `render` at `ColorDepth::Palette16` in `graph_column/tests.rs` and a third block in the ghost capture would close it.
8. `○` (ghost) and `◌` (uncommitted row, `mod.rs:119`) are both hollow circles. Not a parent, so §5 holds; worth one look by eye in group 08's capture.
9. `stage_at_diff_cursor`'s `Unmerged` branch (`operate.rs:387`) has no test; the two named tests cover `stage_selected` and `request_discard_at_diff_cursor`.
10. `SecretField::left/right/home/end` do not pulse and the masked field draws no cursor, so a cursor key in the password field shows nothing. Defensible (nothing changed); state it in the `pulse` doc.
11. KNOWN_GAPS has rows 472/474/475 for the pulse limit, the unpopulated CONFLICTS section and the stale confirm capture, but none for the ticket's gap 3 (stale `m4-03-credential-masked-*` / `m4-04-credential-username-*` captures still showing a 28-glyph mask) or for M3 above.
12. `Confirm`/`PreviewBox` clear their area with `Style::default()` (terminal default background) rather than a theme token; pre-existing convention from `credential_prompt`, noted only so a future theme pass sees all three together.

## Architecture and readability

- Crate rules hold: `getrandom` lives in `graft` (already a dependency since M3), not `graft-ui`; `update` stays pure with the seed on the event; `frame_box` is a private shared helper. `ARCHITECTURE.md` does not enumerate widget modules, so nothing there is stale.
- `graph_screen/layout.rs::push_walk_status` takes both a `push` closure and the `spans` it pushes into — the closure-with-its-own-target shape reads oddly; pre-existing pattern, left alone.
- Doc comments are good throughout; the `SecretField::pulse` comment states the remaining observer leak plainly, which is right.
- No `unwrap`/`expect` outside tests in the diff. No git work on the UI thread introduced. All new strings drawn are `SanitizedText` or literals.
