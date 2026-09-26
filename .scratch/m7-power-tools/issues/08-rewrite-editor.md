# rewrite-editor — `R` opens the rewrite editor on the cursor's row; the result and the message show before `y`

Status: ready
Blocked by: 02-rewrite-engine, 02b-rewrite-engine-b, 05-rewrite-preview
Crates:
- graft (`update/rewrite_editor.rs` new, `GraphOverlay`, `text_field_open`/`modal_owns_keys`, the key route, the walks,
  the `journeys` parser);
- graft-ui (`rewrite_editor.rs` new).

The docs it owns: `docs/shots/m7`, `TESTING.md`, the captions TSV.
Source: note "Pass 2a" R1; journeys 1 and 3; the keymap diff (§2.3 `R`, §2.7); Decision 5; field notes :69.
Build with: Opus. A new overlay on the established `GraphOverlay` pattern; the look is fixed in the note; walks and
captures judge it.
Scout: `scout` first (ADR-0035).
Needs: rewrite-engine (`plan`, `RewriteParams`, `squash_groups`, `default_message`, `validate`), rewrite-preview (the
AFTER block data, `messages`, `StopsAt`). Doc rows: the conductor applies the keymap diff (§2.3 `R`, §2.7) before the
build.

## Decided (note R1; not open)
- `R` on a graph row → `Command::PlanRewrite(RewriteRequest::Open{from})`, run as a job.
  - A refusal → the status line with the note's words, and no overlay.
  - Success → `GraphOverlay::Rewrite`, the cursor on the `from` row (HEAD's row from the uncommitted row).
- **Rows:** newest first (the todo reversed), plus the `─ main` base row (not selectable for actions: `p s f d e r`
  there say `the base stays · it isn't yours to rewrite`).
- **Keys:**
  - `p s f d e`: set the action and move one row older.
  - `r`: the message field (a text field).
  - `Alt+↑/↓`: move the row, and the row shows `↕ moved`.
  - `⏎`: show / hide the row's changes on the right.
  - `j/k`.
  - `Tab` (<120 columns): swap rows ↔ AFTER.
  - `y`: start, once the plan has landed. Any edit re-plans the preview through `preview::run_preview`, stoppable; the
    last edit wins.
  - `Esc`: close, nothing written. In the field, `Esc` drops the field's edit only.
  - Mouse: a click selects the row; a drag moves it.
- **The message block:** `default_message` per squash group, shown under the rows (≥180) or under AFTER (<180), `┊`
  gutter, `MESSAGE of the squashed commit · r on "<kept subject>" changes it`.
  - `r` on the kept row edits its subject line; `Ctrl+E` hands the whole message to `$EDITOR` (the existing `Handover`).
  - The edited message goes in `RewriteParams::messages`.
- **Glyphs and words:** exactly the note's R1 distinctions table: `↓` squash/fixup, `✎` reword, `⏸` edit, `✗` drop
  (subject muted), bold action word when not pick, `│` rule, `○ new`/`● new`, `built on yours · follows`,
  `marked by its name`.
- **Refusals in the editor:** `✗ nothing below to meld into` on the row + `y` refuses; `every commit dropped · the
  branch would be main again`; `feature-a moved since this opened · esc, then R again` (CAS at apply).
- **`y`** → `operate::launch(Operation::Rewrite(params))`. It lands as `land-on-result` defines. Until that ticket
  lands, today's `Done` landing; the walk asserts the cursor only after it.
- **Hint bar:** `y rewrite  ⏎ changes  p pick  r reword  e edit  s squash  f fixup  d drop  alt+↑↓ move  esc cancel`.
- **The graph's bar gains `R rewrite`** on a commit row of his branch (field notes :69: found only after the keys were
  listed).

## Read first
- Note "Pass 2a" R1 (whole), "Journey counts", "Keymap: proposed diff" §2.1 / §2.3 / §2.7.
- `docs/design/02-keymap.md` §2.1 (modal panes, the hint bar rule), §2.3, §2.3b, §2.6b (`⏎` shows a diff), §2.7, §2.10.
- `docs/shots/m7-proto-1-rewrite.txt` (rev 1, what he saw).
- `crates/graft/MAP.md`: `GraphOverlay`, `text_field_open`, `modal_owns_keys`, `update::preview`, `update::operate`,
  `update::commit_editor` (the field and `Handover`), `jobs`. `crates/graft-ui/MAP.md`: the preview box, the commit
  editor (layout precedents).

## Scout brief (≤ 1 page)
1. How a `GraphOverlay` variant is opened, keyed, drawn and closed (take `Journal` or `Stash`). Every match on
   `GraphOverlay`.
2. `text_field_open` / `modal_owns_keys`: where each is consulted, and what a new text field must register.
3. How the preview job is started and how a newer request supersedes a running one (`preview::run_preview`, `stop`).
4. What `R`, `Alt+↑/↓` and `Tab` do on the graph today (grep the key route). Keymap: `R` unclaimed in the doc.
5. `m5_walk::journeys::parse_counts` / `noted_keys`: which note it reads, and how an id without a scenario is treated.
6. The hint-bar tests (`every_key_the_bar_draws_is_answered`, `every_planned_hint_is_still_unbound`,
   `every_bound_action_is_hinted_live_or_is_named_here`): what a new overlay must add.

## Signatures
```rust
// graft
pub fn open_rewrite(state: &mut AppState, from: Oid) -> Vec<Command>;
pub fn rewrite_key(state: &mut AppState, key: KeyEvent) -> Vec<Command>;
// GraphOverlay::Rewrite(Box<RewriteEditor>); Command::PlanRewrite(RewriteRequest);
// Msg::RewritePlanned(Result<RewriteParams, RewriteError>)
// graft-ui
pub fn draw_rewrite_editor(frame: &mut Frame, area: Rect, view: &RewriteEditorView);
```

## Acceptance
- [ ] **App** `update::rewrite_editor::tests`:
  - `r_opens_on_the_cursor_row` (red first);
  - `s_marks_and_moves_one_older`;
  - `a_bottom_squash_is_refused_in_words`;
  - `alt_up_moves_the_row_and_marks_it`;
  - `r_opens_a_field_that_owns_every_key`;
  - `esc_in_the_field_drops_only_the_edit`;
  - `esc_writes_nothing`;
  - `y_before_the_plan_lands_does_nothing`;
  - `y_launches_the_rewrite`;
  - `an_edit_replans_and_the_last_wins`;
  - `a_refused_open_shows_the_words_and_no_overlay`;
  - `the_base_row_refuses_actions`.
- [ ] **UI** `graft_ui::rewrite_editor` snapshots at 187×50, 140×40 and 100×30. Each asserts the non-colour carriers:
      `↓ ✎ ⏸ ✗`, `│`, `○ new`, the `┊` message gutter, and `built on yours · follows`.
- [ ] **Walks** (`crates/graft/tests/it/m7_walk/`):
  - `squash_last_three` = `R s s y` ≤ `noted_keys("squash-last-three")`: after it, one commit where there were three,
    its message = the note's order, and feature-b moved;
  - `swap_adjacent` = `R Alt+↑ y` ≤ `noted_keys("swap-adjacent")`;
  - `rewrite_look_and_leave_writes_nothing` = `R s esc`: refs, index and files unchanged.

  `parse_counts` extended to read this note's `journey-counts` fence, failing on a duplicate id: test
  `m7_ids_are_parsed_and_unique`.
- [ ] **Captures** (rule 6; colour **unverified**), through `check_shot`:
  - `docs/shots/m7/rewrite-editor-187x50.txt`. **Right:**
    - `REWRITE feature-a · 5 commits since it left main · newest first`;
    - two `squash ↓` rows over `pick 5d4c3b2`;
    - the `│` rule;
    - `AFTER y · not written yet`;
    - `3 in one`;
    - `MESSAGE of the squashed commit`, with `wip: tests` first, then `Handle expired tokens`, then `wip`;
    - the bar starts `y rewrite`.
  - `docs/shots/m7/rewrite-editor-140x40.txt`. **Right:** the AFTER block below the rows, labelled.
  - `docs/shots/m7/rewrite-editor-100x30.txt`. **Right:** the rows only; `tab after` on the bar.
  - `docs/shots/m7/rewrite-refused-fork-120x40.txt`. **Right:** the status line `feature-b and feature-c are both built
    on yours · Graft moves one line`; no overlay.
- [ ] The three hint-bar tests green. `R` is on the graph's bar for a commit row of his branch.
- [ ] MAP.md for graft and graft-ui. TESTING.md: a section "The rewrite editor" with the four capture rows, the journey
      rows, and what the walk does not prove (colour). Contact-sheet rows + captions.

## Touches (check each, report how)
- **Graph keys `R`, `Alt+↑/↓`, `Tab`:** scout Q4. The keymap doc gives `R`. Check: grep + the hint-bar tests.
- **Other overlays:** a new `GraphOverlay` variant. Check: every match site compiles; the Journal/Stash/Map overlay
  tests green.
- **The preview job:** shared with every preview. Check: the existing preview tests + captures byte-identical.
- **The commit editor's `Handover` for `Ctrl+E`:** reused, not changed. Check: the commit-editor tests green.
- **The `.` menu:** unchanged here.
- **Journal/undo:** through the engine. Check: `u` after `squash_last_three` restores 3 commits (walk step).
- **Captures of the graph bar:** adding `R rewrite` to the bar changes every capture that shows a commit row's bar.
  Check: run `check_shot` over all shots and list every changed capture. The conductor re-takes them, or this ticket
  does with `GRAFT_UPDATE_SHOTS=1`, then a second pass without it.
- **The journeys parser:** now reads two notes. Check: `journey_counts_match_the_note` green for M6's ids.

## Rules / Gates
As rewrite-engine.

## Size
Large (~75k). Split if needed:
- **-a:** app state, keys, the parser extension, the unit tests;
- **-b:** graft-ui draw, walks, captures, bar.
