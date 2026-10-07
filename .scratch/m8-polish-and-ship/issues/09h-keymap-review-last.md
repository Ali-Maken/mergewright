# G-keymap 1h — 09g remainder: `ONE_CHORD_IDLE` classes (third class ruled), Diff `Home`/`End`, resolver `⌃Z` under the suspend switch, `AFTER y`, ratchet, `?`-at-Map capture, D

Status: ready
Build with: **Opus** — per-row judgement (each of the ten "acts given state" rows needs the right `given`/`then`; a
wrong `then` hides a defect in a table) and one probe whose result picks the branch of E. Not Fable: every box is a
full oracle (state compared, red first); nothing structural.
Blocked by: **none for source.** Running now: **12** and **10h** — no shared source file (below). **Two cargo helpers
at a time** (memory): this starts when one of 12/10h finishes. **Capture step only after 12 merges** (12 changes
palette row drawing, which the new `?` capture draws) and not while 10h, 17b, 11c or 15 is re-shooting (conductor).
**11c runs after this ticket** (09f's serial order carries over).
Needs: 09g's merged work — `ONE_CHORD_IDLE` (22 rows), `ACTING_AT_LANDING` = 449, the `shipped_as` pattern in
`update/detail/keys.rs` `files_key`, `update::resolver::tests::ctrl_z_answers_in_the_resolver_as_u_does`; graft-ui
`Keymap::{shipped, spell_*, key_verb, after}`.

**Files.** Paths under `crates/graft/src/` unless stated.
- graft: `update/tests/keymap_edge/{sweep,sweep_tables}.rs`; `update/detail/keys.rs` (`diff_key` only) + its test
  file; `update/resolver.rs` (the `asks_undo` arm only, E-a) + the resolver test file 09g used; `tests/it/look_polish.rs`
  (one capture test); `MAP.md` (own entries).
- graft-ui: `rewrite_editor/right.rs` (:143–144), `rewrite_editor/tests.rs`, `tests/no_literal_keys.rs` (pattern only),
  `MAP.md` (`rewrite_editor`, `no_literal_keys` entries).
- `ARCHITECTURE.md` (one entry). `docs/shots/m8/help-at-map-120x36.txt` (new).
- **Not:** `update/mod.rs`, the `Product`/`Command` enums, `update/edge.rs`, graft-ui `keymap/*` (a cause there is a
  stop); `update/credential.rs`, `update/picker.rs`, `commit_editor/*` (R1 here is test-only); `view/credential.rs`,
  `search.rs`, `view/frame/bars.rs`, `view/status.rs`, `update/undo.rs`, `journal/*`, `tests/no_literal_keys/known_*.rs`
  (10h); `update/palette.rs`, graft-ui `palette.rs`, `jobs/*`, `update/state.rs` (12); `update/tests/palette_live/*`.

## Shared with running and queued tickets
- **10h:** no source file. Same test binary `no_literal_keys` (root pattern here, `known_undo.rs` there): R4(a)'s "no
  new hit on the merged tree" is re-run after rebasing on 10h if 10h merges first; NOTES names the base. graft `MAP.md`
  hunks — conductor merges.
- **12:** no source file. graft + graft-ui `MAP.md` hunks (12: `update::palette`, `jobs`, graft-ui `palette`). Capture
  step after 12 merges (above).
- **17b / 17c (were blocked by 09g for `update/mod.rs`):** this ticket does **not** touch `update/mod.rs` (E-b stops
  rather than edit it). 17b may start once this is filed, under its own blocker 12; the only overlaps are graft
  `MAP.md` hunks, the capture step (serial) and `tests/it/look_polish.rs` if 17b puts its capture tests there (additive
  test fns; the conductor serialises the two capture steps). 17c: no shared file.

## Read first
- 09g: **Rulings** R1–R4 (:51–91) and **NOTES** (:179–238). R2–R4 stand; R1 is amended below.
- 09f **Major 3** and **Major 4** (`09f-keymap-review-fixes.md` :96–127)
- `docs/design/02-keymap.md` §2.1 :179–193 (translate first; `⌃Z` is `app.suspend` with the switch), §2.6 :344
  (`Home`/`End` as `g`/`G`), :579 (text-field edit keys)
- `docs/research/ux-principles.md` row 4
- `crates/graft/MAP.md` :205 (`suspend_on_ctrl_z`), :491 (resolver `u`/`U`), :528 (`conflict_editing`, taken when
  `Product::HandedOver` answers — it carries no path), :945–946 (sweep, `palette_live`), :1001 (`Handover::suspend`)
- Source after the maps: `sweep_tables.rs` `ONE_CHORD_IDLE`; `detail/keys.rs` `files_key`/`diff_key`; the lines 09g
  NOTES cites for the ten rows (`picker.rs:141`, `commit_editor/keys.rs:229–230`, `credential.rs:378–382`).

## Rulings (planner)

**R1′ — third class accepted, but observed: `ActsGivenState`. `TypesItself` is dropped.**
- 09g found zero `TypesItself` rows (NOTES :213: Menu is no text field; every other idle chord is non-printable). An
  unconstructed variant is dead code (clippy), and a vacuous assertion proves nothing → **delete the variant and its
  assertion**. If a later row types itself, the class comes back with that row.
- The ten rows act in their handler; the sweep's fixture leaves them nothing to change (caret already at the end,
  selection already 0). `Defect` would be false for them. **Reject** reshaping the sweep's fixture: it is shared by 727
  cases (every count and table moves), and the Credential caret is not drawn (09g NOTES :219), so a frame oracle could
  not see `←`/`Home` act from any caret. **Take** a per-row state oracle: each `ActsGivenState` row carries `given`
  (puts the place where the action has something to change) and `then` (the action's effect, read off state). A row
  whose `then` fails is `Defect`, not a stop. The row stays in `ONE_CHORD_IDLE` (the sweep still sees it idle; the
  table is the exact-match ratchet).
- Expected `given` → `then` (builder confirms each against the handler line; a different effect → NOTES, row `Defect`):

| Place | Action, chord | `given` | `then` |
|---|---|---|---|
| Picker | `app.up` `↑` | ≥2 rows, selection 1 | selection 0 |
| CommitEditor | `app.top` `Home` | subject `abc`, caret 2 | caret 0 |
| CommitEditor | `app.bottom` `End` | subject `abc`, caret 0 | caret 3 |
| Credential | `edit_text` `Delete` | text `abc`, caret 1 | text `ac`, caret 1 |
| Credential | `edit_text` `→` | text `abc`, caret 1 | caret 2 |
| Credential | `edit_text` `End` | text `abc`, caret 1 | caret 3 |
| Credential | `app.bottom` `End` | text `abc`, caret 1 | caret 3 |
| Credential | `edit_text` `←` | text `abc`, caret 3 (fixture's) | caret 2 |
| Credential | `edit_text` `Home` | text `abc`, caret 3 | caret 0 |
| Credential | `app.top` `Home` | text `abc`, caret 3 | caret 0 |

- Whether the Credential caret is drawn: no line → a **question** for the conductor (rule 7), NOTES; not fixed here.

**R5 — Diff pane `Home`/`End`: in scope.** Same defect class as 09g's Changes fix, same file (`detail/keys.rs`
`diff_key` answers `Char('g') if plain` only), §2.6 :344 pairs them with `g`/`G`. Fix the same way (`shipped_as` of
`app.top`/`app.bottom`; no raw `Home`/`End` arm; reuse the copy already in this file, no fifth). The sweep counted it
acting because the graph moved — its frame oracle cannot tell which pane acted. That blind spot is **out** (conductor
files it as a question with `detail/keys.rs` `diff_key` as the line).

**R6 — resolver `⌃Z` with `suspend_on_ctrl_z`: it suspends, and the return marks nothing edited.** §2.1 :191–192 (one
meaning per chord everywhere; ux row 4) — the banner already does this (09g NOTES :197–199). 09g kept it swallowed
because a suspend "would" reach `update/mod.rs:441` and mark the file edited — **inferred, not observed** (rule 7: a
stop, not a premise). So: probe first.
- **Probe** (temporary test, removed after; output in NOTES): resolver open on a conflicted file, switch on, no `e`
  pending (`conflict_editing` `None`); feed the answer a `Suspend` command gets back → is any file's edited state
  changed? Also the same with `conflict_editing` `Some` is **not** the case (a suspend cannot happen inside `$EDITOR`).
- **E-a, probe clean:** remove the swallow — with the switch on, the resolver returns `⌃Z` unclaimed and the one
  suspend path (`update/keys.rs:243`) takes it, as at the banner. Only `update/resolver.rs`'s `asks_undo` arm changes.
- **E-b, probe marks a file:** **stop on E** (cause in `update/mod.rs`, held by 12, queued for 17b). Swallow stays;
  NOTES gives the line and the probe output; the conductor updates `M8/resolver-ctrl-z-swallowed` from inferred to
  observed and files the fix (the answer says what was handed over: editor vs suspend) after 12 and 17b merge. The rest
  of this ticket continues.

**R4 carries over unchanged** (09g :83–88): `AFTER y` per 09f Major 3; `no_literal_keys` extension adds exactly one
hit with the literal restored, none on the merged tree, any other new hit → stop with file:line (no `KNOWN` rows);
captures: expected diff = the new file only, plus 09f's `?`-at-overlay allowance.

**Out:** the 12 `Defect` rows' fixes (gap `M8/one-chord-idle`); the sweep's which-pane blind spot (question); the
Credential caret question; consolidating the four `shipped_as` copies into `update/keys/predicates.rs` (follow-up);
the `published_repo` home-dir detector false positive (09g NOTES :229–231 — conductor's gap); 09f's stash-focus
question; `DEAD_ELSEWHERE`'s 214 rows.

## Signatures
No production signature changes.
```rust
// graft::update::tests::keymap_edge::sweep_tables — test-only. `Fx` = whatever the sweep's place builders return
// (mirror it; NOTES names the type). fn pointers, not closures.
enum IdleWhy {
    /// Acts in its handler; the sweep's fixture leaves it nothing to change. `at`: the handler arm's file:line.
    ActsGivenState { at: &'static str, given: fn(&mut Fx), then: fn(before: &Fx, after: &Fx) -> bool },
    /// Neither acts nor types. `at`: where the chord falls idle. Gap row M8/one-chord-idle.
    Defect { at: &'static str },
}
const ONE_CHORD_IDLE: &[(ContextId, ActionId, KeyChord, IdleWhy)]; // 22 rows: 10 ActsGivenState, 12 Defect (or NOTES)
const ACTING_AT_LANDING: usize; // 449; unchanged by R5 (Diff Home/End already counted acting) — NOTES if it moves
```

## Acceptance (each tick names its test, at tick time)
- [ ] **R1′, table.** `IdleWhy` has exactly `ActsGivenState` and `Defect`; 22 rows, each with `at`; sweep green.
  (`update::tests::keymap_edge::sweep::every_reachable_row_acts_or_is_listed_and_follows_its_remap`)
- [ ] **R1′, acts given state.** Every `ActsGivenState` row: built as the sweep builds its place, `given` applied, the
  idle chord pressed → `then(before, after)` holds and `after != before`. Shown failing once with Picker `⌃Z`
  deliberately classed `ActsGivenState` (temporary, NOTES). (`update::tests::keymap_edge::sweep::every_one_chord_row_that_acts_given_state_does`)
- [ ] **R1′, gap list.** NOTES lists every `Defect` row (place, action, chord, `at`); `rg -c 'Defect \{'
  sweep_tables.rs` = NOTES count (12 expected; more if a `then` failed — each named). (that command)
- [ ] **R5, Diff.** Diff pane focused, document scrolled mid: `Home` → same state as `g`, `End` → as `G`, graph cursor
  unchanged both times; `[keys.app] top = "t"` → `t` tops the diff, `Home` leaves the diff where it was. Red first.
  (`update::detail::tests::home_and_end_in_the_diff_pane_act_as_g_and_capital_g`)
- [ ] **R5, no raw arm.** `git diff <base> | rg '^\+.*(KeyCode::Home|KeyCode::End)'` hits test files only (NOTES).
  (that command)
- [ ] **R6, probe.** NOTES: probe code summary, output, branch taken (E-a or E-b). (the probe, NOTES)
- [ ] **R6, E-a** (if taken). Resolver open, switch on: `⌃Z` → commands `[Suspend]`; the answer fed back → no file's
  edited state changes; switch off → unchanged (as `u`). 09g's test's switch-on half updated to expect `Suspend`
  (old → new assertion in NOTES). Red first. (`update::resolver::tests::ctrl_z_suspends_in_the_resolver_and_marks_nothing_edited`,
  `update::resolver::tests::ctrl_z_answers_in_the_resolver_as_u_does`) — **or E-b:** stop recorded, box unticked,
  NOTES says so.
- [ ] **C, `AFTER y`.** `confirm.yes = "c"` → `AFTER c` outside the field; inside the `r` field the shipped `y`;
  unbound → `AFTER` alone, no `—`. Red first. (`rewrite_editor::tests::the_after_heading_spells_his_confirm_key`)
- [ ] **C, ratchet.** Literal restored → exactly one new hit, red observed (NOTES); merged tree (base named, after 10h
  if merged) → no new hit. (`no_literal_keys::no_user_facing_literal_spells_a_default_key`)
- [ ] **Captures** (after 12 merges). Full `cargo nextest run` with `GRAFT_UPDATE_SHOTS` once, then unset and green;
  `git diff --stat <base> -- docs/shots` = the new file only (or R4's allowance, listed). New
  `docs/shots/m8/help-at-map-120x36.txt` (Map open, graph focused, `?`): HERE = Map's own keys; EVERYWHERE = Global
  keys that act there; no `c commit`, `R`, `m merge`, `/ Find…`. Judged by the user (rule 6); colour unverified (rule 5).
  (`tests/it/look_polish.rs::help_at_the_map_lists_only_live_keys`, via `captures::check_shot`)
- [ ] **D, ARCHITECTURE.md.** "Changing a key, or words that name one" entry after :84–85 per 09f Major 4 (≤15 lines,
  ADR-0055/0058 cited). (`rg -n 'Changing a key' ARCHITECTURE.md`)
- [ ] **D, MAP.md.** graft: :945 sweep (`IdleWhy` classes, `ACTING_AT_LANDING` 449), :946 `DEAD_ELSEWHERE` 214, :491
  resolver `u`/`U` + `⌃Z` (and the switch, per branch taken), blame/changes/diff/banner handlers. graft-ui:
  `rewrite_editor` (`AFTER` spelled), `no_literal_keys` pattern. (`rg -n 'ONE_CHORD_IDLE|ACTING_AT_LANDING|IdleWhy'
  crates/graft/MAP.md`)

## Touches
- Translate/dispatch/keymap: `git diff <base> -- crates/graft/src/update/edge.rs crates/graft/src/update/mod.rs
  crates/graft-ui/src/keymap crates/graft-ui/tests/keymap_ids.txt` empty (shown).
- Sweep counts: `ACTING_AT_LANDING` 449 and `DEAD_ELSEWHERE` 214 before/after (NOTES); `palette_live` exact table green
  (Diff rows at Diff).
- 09g's tests: blame/changes/banner `⌃Z` tests untouched; resolver test per R6.
- Hint bars: no bar draws `Home`/`End`/`⌃Z` for these rows — `rg` the spelled labels in `docs/shots` before/after.
- Journal/undo: `git diff --stat <base> -- crates/graft/src/update/journal* crates/graft/src/update/undo*
  crates/graft-ops` empty (shown). Suspend: `update/keys.rs:243` path unchanged (E-a only stops swallowing).
- Credential/picker/commit editor handlers: test-only reads; `git diff <base> --` those files empty (shown).
- 10h, 12, 17b, 17c, 11c: see Shared. KNOWN_GAPS (conductor): close `M8/09g-third-class`, `M8/diff-home-end-moves-graph`,
  `M8/resolver-ctrl-z-swallowed` (E-a) or mark it observed (E-b); keep `M8/one-chord-idle` (12 rows, broken). Rows
  :1168–1171 are duplicated at :1172–1175 — conductor dedups.
- Contact sheet: re-assembled with the new capture (conductor).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~40k: R1′ classes + state oracle ~12k; R5 handler + test ~4k; R6 probe + branch ~6k; `AFTER` + ratchet ~5k; captures
~4k; ARCHITECTURE + MAP ~6k; NOTES ~3k. Order: R1′ → R5 → R6 → `AFTER` + ratchet → captures (after 12) → D. Past
~65k: stop after the ratchet, hand captures and D to a split (NOTES says where).
