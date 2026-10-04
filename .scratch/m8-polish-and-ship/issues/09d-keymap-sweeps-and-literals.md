# G-keymap 1d — sweeps that prove something; the last literal keys spell from the table

Status: draft (planner spec-only pass, 2026-10-04; split from `09c-keymap-review-fixes.md` — its D4, Majors 3–4 and
Minors 2, 8, 12 — plus 10c's five allow-listed literals)
Build with: **Opus** — the oracle and every literal's row are decided here; held by named tests, not by eye. Not Fable:
no structural choice is left open.
Blocked by: 09c (removed ids, `borrows`, `reachable`)
Needs: from 09c — `ContextId::borrows`, `Keymap::reachable`, `KeySwitches`, `Clash`, the golden list without
`finish.commit`/`rewrite.start`/`empty_state.confirm`/`preview.confirm`; from 09b — `outside_edge`, `edge::unanswered`;
from 10/10c — `HintKey::{Key, Fixed}`, `Keymap::{key_verb, after, spell_keys}`, the `StashScreen` widget pattern
(`pub keys: &'a Keymap`), `keymap::sweep` helpers, `graft-ui/tests/no_literal_keys.rs`.
Crate set: graft-ui (`detail_pane/header.rs`, `detail_pane/line_render.rs`, `graph_screen/banner.rs`,
`graph_screen/badges.rs`, `rewrite_editor/right.rs`, their widget structs, `tests/no_literal_keys.rs`) + graft
(`update/tests/keymap_edge*` rewrite and split, `empty_state` tests, `view/status` test, the view call sites of
those four widgets). Not `keymap/*` rows or resolution (09c).

## Read first
- `docs/reviews/m8-09-keymap.md` Majors 3–4 (inside :58–148), Minors 2, 8, 12 (:158, :200)
- `09c-keymap-review-fixes.md` D1, D3 and **Rulings on 10c** (R4's rule). `10c-keymap-spell-footers.md` NOTES
  "Beyond the listed sites"
- `docs/design/02-keymap.md` §2.3c (:289); `x` row (:250); `L` row (:314); §2.6a (:347–360); rewrite `y`/`Tab` rows
  (:429–431)
- `crates/graft-ui/MAP.md` `## keymap`, `detail_pane`, `graph_screen::badges`, `GraphView::banner`, `BannerView`,
  `TabWord`, `rewrite_editor`; `crates/graft/MAP.md` `update::edge`, `outside_edge`, `update::tests`,
  `AppState::text_field_open`, `AppState::rewrite`
- Source after the maps: `update/tests/keymap_edge.rs` (whole — you rewrite it), each literal's line (10c NOTES)

## Decisions (planner; not the builder's)

**D4 — The behavioural sweep's oracle is behaviour, not a copy of `resolve`** (Major 3; moved verbatim from 09c).
Delete `acting_default` (`keymap_edge.rs:160–171`). Expected = what the row's default chord does under the default
keymap. Which rows a place shadows comes from a hand-written `SHADOWED: &[(ContextId, &str /*id*/, &str /*chord*/,
&str /*by id*/, &str /*why: § or file:line*/)]`, never computed. graft-ui's `default_holder`
(`keymap/tests/translate.rs:11–17`) may stay for the pure sweep; 09c's pinned D1 cases are what catch an order bug
there.

**D5 — The five literals 10c allow-listed as KNOWN spell from the row that answers them** (10's decision 8: spelling
at render, widget takes `&Keymap`). An unbound key's part is left out (10's decision 2); never `—`.
| Literal (file) | Spells from | Why |
|---|---|---|
| `x clears it` (`detail_pane/header.rs`) | `Key(graph.compare)`; unbound → clause dropped | §2.3 `x` (:250) marks/clears the compare base |
| `L load all` (`detail_pane/line_render.rs`) | `Key(diff.load_all)`; unbound → clause dropped | §2.5 `L` (:314) |
| `space stages, a amends it, c continues · ⇥ banner` (`graph_screen/banner.rs`) | `space` → the Graph row `space` translates to at an `edit` stop; `a` → `graph.amend`; `c` → `graph.commit`; `⇥` → `app.next_pane`; each clause dropped if unbound | an `edit` stop gives the keyboard to the graph (§2.6a :356–358); `c` there is the banner's `c` (§2.3b :303) |
| `⏎ back to AFTER` (`rewrite_editor/right.rs`) | drawn only while the `r` field is open (`text_field_open`, graft MAP `AppState::rewrite`) → `Fixed(<the row the field answers>)`; drawn outside it → `Key(<row>)` | R4's rule (09c): inside a text field only shipped keys are spelled |
| `x base` (`graph_screen/badges.rs`) | `Key(graph.compare)`; unbound → `base` | the badge names how to clear the base |
- The rows above are the planner's reading of the design, **not checked against code** (rule 7): the builder pins
  each with `Keymap::shipped().translate(<live ctx>, <default chord>) == <id>`. A mismatch is a stop: NOTES names the
  handler's `file:line` and the row it found; report it, don't pick.
- `⏎ back to AFTER` with no row the field answers: a text field's key never moves, so the literal is what the shipped
  spell prints. It stays on the allow-list with reason `text field, no row: <file:line>` — not a `KNOWN` entry.
- No new table rows here (09c's table). A literal that needs one: stop, NOTES, report.

## Scope
In: D4, D5, Majors 3–4, Minors 2, 8, 12. Out: resolution, rows, warnings (09c); `?`/palette (11); ranking (15).

## Signatures
```rust
// graft-ui — the 10c pattern: `pub keys: &'a Keymap` on the widget, or a trailing `keys: &Keymap` on a free draw
// fn, for the detail pane (header, line_render), graph_screen::badges, the graph banner, rewrite_editor::right.
// MAP.md records each name.
// graft::update::tests — sweep tables
const SHADOWED: &[(ContextId, &str, &str, &str, &str)];
const SWALLOWED_BY_DESIGN: &[(ContextId, &str /*id*/, &str /*why*/)];
const NO_EFFECT_IN_FIXTURE: &[(ContextId, &str /*id*/, &str /*why*/)];
```

## Acceptance (each tick names its test, at tick time)
- [ ] **Major 3.** Sweep over `reachable(ctx)` for every context with a fixture (all rows, not own + Global). Overlays
  given rows to move over; pane cursors mid-list. Every case either **acts** (default press ≠ F20) or is listed with
  a reason (`SWALLOWED_BY_DESIGN` — Preview/Confirm/Banner swallows, §2.3c; `NO_EFFECT_IN_FIXTURE`; `SHADOWED`). A
  listed entry that acts, or a shadow that doesn't hold, fails the test. Test prints acting/listed counts; asserts
  acting ≥ the count at landing (written in the test with the date).
- [ ] **Major 4.** EmptyState swept like Major 3 through `outside_edge`, three fixtures: plain, init box open (closes
  Minor 12: `y`/`n`/`esc` under a remap — `y` is `confirm.yes` after 09c), prompt open (every key typed). F2
  sanitising pinned: the reviewer's probe `"\u001b[31mmerge"` → F2 draws `graph.␛[31mmerge is not an action`
  (Minor 2).
- [ ] `keymap_edge.rs` split under 400 lines per file (Minor 8). 09c's `keymap_rules.rs` left as is.
- [ ] **D5 literals.** Test `each_former_literal_names_the_row_that_answers_it` (shipped translate per row of the
  table). Under 10's sweep keymap each of the five lines holds the remapped label and no default label of a moved
  action as a whole token (10c sweep (c) helpers); the `r` field's line holds the shipped label whatever the sweep
  says. `graph.compare = "none"` → no `x` in the header or the badge, no `—`.
- [ ] **Grep test.** `no_literal_keys.rs::no_user_facing_literal_spells_a_default_key` green with **no `KNOWN`
  entry left**; any remaining allow-list entry names its `file:line` and reason.
- [ ] Captures: default config → no capture changes (the spelled labels equal the literals). If a token changes
  (e.g. `space`'s label), it is a decision-3 change: re-shoot (`GRAFT_UPDATE_SHOTS=1`), NOTES lists file and token,
  re-assemble the affected contact sheets. Colour unverified (rule 5).
- [ ] MAP.md graft-ui: each widget now taking `&Keymap`; graft `update::tests` sweep tables.

## Touches
- The four widgets' graft view callers (compiler); every capture drawing a compare base badge, the detail header's
  compare line, a truncated diff, an `edit` stop banner, or the rewrite editor's message block (default keymap: no
  change expected — the capture run shows it).
- Badge width: `x base` is measured in the graph row's layout — no column shift under the default keymap (capture
  run); a longer remapped label is cut like any badge (sweep test).
- 09b's sweeps and J8 tests (rewritten here; J8's words pinned by 09c stay green). `empty_state` tests.
- Journal/undo: none (render and tests only) — show the empty diff over `crates/graft/src/update/journal*` and
  `crates/graft-ops`. KNOWN_GAPS: 09b tick 1's EmptyState row closes with the Major 4 box (conductor).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~58k: Major 3 sweep rewrite ~25k; EmptyState + F2 ~10k; split ~5k; D5 literals + tests ~12k; captures + MAP ~6k.
Order: literals first (small, independent), then sweeps. Seam if over: commit after Major 3, report
`blocked: Major 4`.
