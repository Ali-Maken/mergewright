# 20i blame-review-fixes — review m7-20e-20g: Majors 1–4, Minors 1, 2, 4–9

Status: partial (all boxes ticked but one: the `every_key_the_bar_draws_is_answered` reading-state row is blocked on graft-ui; see NOTES)
Blocked by: 20f (building in graft now; same files: `update/place/tests.rs`, `tests/it/main.rs`, the pane-bar sweeps).
Crates: graft (+ `docs/shots/m7/`, graft `MAP.md`, `ARCHITECTURE.md`, `docs/design/02-keymap.md` §2.1, `20-blame-place.md`'s
Navigation tick). **No graft-ui.**
Build with: Opus. Why: the `Esc` rung order crosses `detail/keys.rs`, `keys.rs` and the bar's wrap; a wrong order is a
bar/key lie that tests only catch if they press the key (the defect itself). The rest is mechanical.
Needs: none (no other ticket's new signatures).
Scout: none. The review names every line (`docs/reviews/m7-20e-20g.md`). **Confirm each cited line before editing**; if
one has moved (20f lands first), say where it went in NOTES.
Source: gap rows M7/20eg-major1..4, M7/20eg-m1, m2, m4..m9 (KNOWN_GAPS.md).
Ruling: planner, 2026-09-27.

## Read first
- `docs/reviews/m7-20e-20g.md` — Major 1–4, Minor 1, 2, 4–9, "Also noted".
- `docs/design/02-keymap.md` §2.1 `:78-83` (places) and §2.2 `Esc` row `:175`.
- `docs/design-notes/m7-power-tools.md` P1 `:1147-1164`, B1 `:1083-1119`.
- `crates/graft/MAP.md` `## update::place` (`:568-583`); `crates/graft-fixtures/MAP.md` `Shape::Session` (`:8`) and `git` (`:30`).
- `.scratch/m7-power-tools/issues/20-blame-place.md` Navigation box `:136-148`.

## Ruling — Major 1: `Esc` goes back; the bar stays `esc back to blame`

**Decision: the keymap's reading.** With a trail hidden (after `⏎` from a place), `Esc` on the graph screen takes him back
to the place, same line, **whether the graph or the detail pane is focused and whether the pane is open**. The pane the
`⏎` opened is not on the keymap's close-first list (§2.1 `:80-82`: menu, box, prompt, search, Focus, Explain) and is not
closed first.

`Esc` rungs on the graph screen with a trail hidden, in order (first that applies wins):
1. A held failure (`esc dismiss`) is dismissed (§2.2 `:175`, unchanged).
2. The graph's own open thing closes: `.` menu, a box, a prompt, search, Focus mode, Explain. The trail stays hidden.
3. With the **pane focused**, the pane's own `Esc` rungs that exist today before `detail/keys.rs:67` (unchanged).
4. **The trail shows**: the top place, same line (`escape_key`'s last rung, `update/keys.rs:555`, or `:67`'s — one
   function, called from both; don't duplicate it).

`detail/keys.rs:126` (`KeyCode::Escape if detail.shown => detail.close()`) closes the pane **only with no trail
hidden** — unchanged there.

**Bar:** words unchanged — `esc back to blame` (and, when 21 lands, `esc back to history`) leads **exactly** in the
states where the next `Esc` is rung 4. In rung 1–3 states the bar keeps that state's own `esc` hint as today. If the
wrap at `view/rows.rs:410` leads `esc back to blame` in a rung-2 state (search active, Focus, Explain), that is the same
defect: fix the wrap to ask the same predicate the key asks (one `pub(crate) fn` — e.g.
`place::esc_goes_back(state: &AppState) -> bool` — read by both `escape_key` and the wrap).

**Why:**
- `commit-back-to-blame` is 1 in the note (`:1250`); the bar already promises it (P1 `:1161`). Option "close the pane
  first" makes it 2 after one `Tab` and needs a new bar word.
- H4 (same key, same meaning): pane focused → back; graph focused → back. The other option makes `Esc` depend on focus.
- H3 (one exit, always `Esc`), P1 `:1164`.

**Cost (stated, no user call — this is the approved spec):** while a trail is hidden, `Esc` can't close the pane. It
closes after the trail ends: the last `Esc` restores the pane as it was at `B` (P1 `:1158`), and one more closes it.
20g's Navigation note ("pane closed: with it open the pane's own `Esc` comes first", `20-blame-place.md:137`) is
superseded.

### Keymap amendment (this ticket's builder applies it, in the Major 1 commit)
`docs/design/02-keymap.md` §2.1, the Blame/History bullet, lines `:80-83`
from `` `Esc` goes back one `` through `(M7 pass 2b; ADR-0048).` become:

> `Esc` goes back one
>     place; after `⏎` to the graph, `Esc` there returns to the place (once
>     the graph's own menu, box, prompt, search, Focus or Explain is closed),
>     with the graph or the detail pane focused; an open detail pane is not
>     closed first, and `Esc` closes it again only once the trail has ended.
>     Amended 2026-09-26 (M7 pass 2b; ADR-0048); the pane clause 2026-09-27
>     (m7-power-tools/20i).

(A clarification of the approved reading, not a behaviour change to the spec.) §2.2 `:175` unchanged. Change nothing
else in the keymap.

## Rulings on the folded Minors
- **Minor 1:** `BlameLoad::Reading` keeps the `CameFromRequest`; `rerun` (`w`, `i`) resends it. A `Want::CameFrom`
  answer with `line: None` is **not** line 1: the place becomes `BlameLoad::Failed` with the words
  `can't tell where line <n> came from in <sha7> · esc back` (`<n>` the newer blame's line; if the job carried git's
  words, `can't tell where line <n> came from in <sha7>: <git's first line> · esc back`). `Esc` then pops to the newer
  blame, same line (existing rung). Remove `unwrap_or(1)` at `place.rs:559`.
- **Minor 2:** in `came_from`, a hunk with `new_lines == 0` has the line `new_line <= new_start` **before** it.
- **Minor 4:** `B` on a changes-pane **file row** (files list focused): blames the file on disk, cursor on its first
  changed line (B1 `:1093-1094`, same rule as a commit's file row; H4). If the code lands elsewhere today, fix it.
- **Minor 5:** `BlameLoad::Failed` words go through `sanitize_line` in `view` (as `view/finish.rs:32`).
- **Minor 7:** the ignore line **stays through a re-read**, worded for the option just chosen (the count kept from
  the last answer: the file's list did not change). `i` stays in the bar during the read; `i` during a read reruns,
  as `w` does. If `i` is not answered during a read today, make it answer (via `rerun`) — the bar must not draw a
  dead key.
- **Minor 6, 8, 9:** as the review states.
- **Not folded:** Minor 3 (`B blame` on the changes pane's bar) — it adds a field to graft-ui's `HintContext::ChangesPane`,
  every sweep and form count, and re-shoots changes-pane bars (20h's size, ~35k). Own ticket after 20f. Questions 1–3:
  unverified, not built.

## Scope
**In** (commit order):
1. **Major 4 split, pure move first** — one commit, no behaviour change, test count identical before/after (state both).
   - `update/place/answer.rs`: `blame_arrived`, `BlameAnswer`, `BlameFailure`, `BlameRequest`, `CameFromRequest`.
     Fold "Also noted": `blame_arrived`'s refusal arm becomes two arms, no unreachable `Failed(_)` → `Untracked`
     (`place.rs:572-575`); an impossible arm is `unreachable!` only if the type can't express it — prefer a type that
     can't (say which in NOTES).
   - `update/place/trail.rs`: `trail_back`, `leave`, `after_commands`, `end_for_compare`, `show_again`.
   - `update/place/blame_state.rs`: `BlameState`'s methods out of `keys.rs` (`view`, `ignore_line`, …).
   - `update/place/tests.rs` → `update/place/tests/{mod,open,navigation,pinned_base,keys,hints}.rs`, by 20's
     acceptance boxes; shared builders (`pane_at`, `changes_at`, …) in `mod.rs`.
   - Paths callers use (`update::place::…`) unchanged; re-export where a type moved.
2. **Major 1** per the ruling, red first, and the keymap amendment above.
3. **Major 2**: one test through `update`, red shown by mutation (the wiring exists).
4. **Major 3**: git-backed tests of the blame job + the pure error mapping.
5. **Minors** 1, 2, 4, 5, 7 (red first where the behaviour is wrong; by mutation where it is right), 8 (test), 6 and 9
   (docs).

**Out:** Minor 3; Questions 1–3; any key or bar word not named above; graft-ui; 21's history place (see Touches);
20f's walks and captures.

## Signatures
```rust
// crates/graft/src/update/place/trail.rs (or where the trail's life moves)
/// `Esc` on the graph screen would show the hidden trail now: a trail is hidden and no rung before it applies
/// (held failure; graph's menu, box, prompt, search, Focus, Explain; the focused pane's own rungs).
/// Read by `escape_key`, `detail::keys`' `Esc` arms and `view::rows`' `BackToBlame` wrap — the one place it is decided.
pub(crate) fn esc_goes_back(state: &AppState) -> bool;

// crates/graft/src/update/place/answer.rs (moved; field added)
pub(crate) enum BlameLoad {
    Reading { job: JobId, want: Want, came_from: Option<CameFromRequest> },
    Failed(String),
    Ready { /* unchanged */ },
}

// crates/graft/src/jobs/detail.rs — pulled out of `read_blame` so every arm is unit-tested
fn blame_failure(error: &graft_git::blame::BlameError) -> BlameFailure;
```
Existing types keep their names and fields otherwise. If `esc_goes_back` can't be one predicate (a rung is only known
inside `detail_key`), say why in NOTES and name the two places and the test that ties them.

## Acceptance (each box names its test at tick time; red first — state the red commit or the mutation)
- [x] **Split:** (a3c69c96 pure move, 60 place tests before = 60 after; root renamed `tests.rs` in b5602484; wc in NOTES; every `update::place::` test green) `wc -l crates/graft/src/update/place.rs crates/graft/src/update/place/*.rs crates/graft/src/update/place/tests/*.rs`
      → every file ≤ 400 (paste the output in NOTES). Place test count before = after the move commit (state both). Test:
      the counts, plus the whole `update::place::` suite green.
- [x] **Major 1, red first:** `esc_on_the_graph_with_the_pane_open_returns_to_blame` (red: stub predicate `trail.hidden()`, failed at esc_back.rs:46; 28ba21b4) — the bar test's state
      (`graph_bar_leads_esc_back_to_blame_while_hidden`'s, with the pane **asserted** shown): bar's first hint
      `("esc", "back to blame")`; one `Esc` → place on screen, same line as before `⏎`. Red today (pane closes).
- [x] `tab_then_esc_after_enter_is_one_esc_back_to_blame` (green, esc_back.rs) — `B ⏎ Tab Esc`: blame, same line; counts 1 `Esc`
      (`commit-back-to-blame` = 1, note `:1250`).
- [x] `esc_on_the_graph_with_the_pane_open_closes_search_first` (green, esc_back.rs) — trail hidden, pane open, graph search active: first
      `Esc` clears search (trail still hidden, pane still shown), second shows blame.
- [x] `esc_on_the_graph_with_the_pane_open_and_no_trail_closes_the_pane` (red by mutation: close arm always claimed → failed :92) — regression guard, green before and after
      (red by mutation: route `:126` through `esc_goes_back` unconditionally).
- [x] `the_bar_leads_esc_back_to_blame_exactly_where_esc_goes_back` (8-row table, esc_back.rs; + `the_menu_over_a_hidden_trail_closes_first_and_the_bar_does_not_lead_back`) — a table over: graph focused pane closed · graph
      focused pane open · pane focused on the diff · pane focused on the files · graph search active · Focus mode ·
      Explain open · held failure. Each row asserts `bar leads "back to blame"` == `one Esc shows the place`. Menu, box
      and prompt states excluded (own bar/words; the `.` menu keeps the graph's bar by §2.1 `:117-120`) — say in NOTES
      what the bar leads with the `.` menu open over a hidden trail; file it as a question if it leads `back to blame`.
- [x] `esc_on_the_graph_closes_search_before_returning_to_blame` still green (navigation.rs; pane-close step dropped); its setup no longer needs to close the pane
      (drop that step, or say why not).
- [x] **Keymap:** (grep → 1 line, :85; diff one hunk @@ -82,2 +82,5) §2.1 carries the amendment verbatim. Check: `grep -n "the pane clause 2026-09-27" docs/design/02-keymap.md`
      → 1 line; `git diff docs/design/02-keymap.md` touches only `:80-83`'s bullet text.
- [x] **Major 2:** `u_on_the_graph_ends_a_hidden_trail_and_pins_the_base_again` (pinned_base.rs; red by mutation: call deleted → failed pinned_base.rs:137; fa6fcf35) — through `update` (a key event, not
      `after_commands`): trail hidden, base set aside, `u` on the graph → no trail, base pinned. Red by mutation: delete
      the `place::after_commands(&mut next, &mut commands);` call (`update/mod.rs:140`) → red (state it).
- [x] **Major 3** (`jobs::detail::blame_tests`, a jobs unit test; red by mutation `old_path: None` → landed (1,"l1"); ae015817) (real git, fast tier; `tests/it` if the job path is reachable from there, else a `jobs::detail` test —
      say which): `caret_across_a_rename_lands_where_the_line_came_from` — a test-built repo (`graft_fixtures::git::run_at`):
      commit 1 `a.rs` = `l1`…`l10`; commit 2 `git mv a.rs b.rs`, two lines inserted after `l2`, `l6` → `L6`. Blame
      `b.rs` at commit 2, `^` on line 8 (`L6`) → place blames commit 1 at path `a.rs`, cursor line 6, text `l6`. Also
      assert git's own porcelain `previous` for that line names commit 1 and `a.rs` (the test runs `git blame
      --porcelain` itself).
- [x] `the_blame_job_refuses_a_binary_path` / `the_blame_job_refuses_an_untracked_path` (jobs/detail/blame_tests.rs) — session fixture
      (`assets/pic.bin`, `src/new.rs`) through the job: `BlameFailure::Binary` / `Untracked`, and the words B1 `:1096`.
- [x] `blame_failure_maps_every_error` (jobs/detail/blame_tests.rs) — unit, every `BlameError` arm (`Run`, `Parse`, `Binary`, `Untracked`, `Stopped`,
      `Shallow`) → its `BlameFailure`; `Shallow` → `Failed` with git's/the io error's first line.
- [x] **Minor 1, red first:** (red: keys.rs:163 no came_from; :182 body Ready line 1; 3c3e65a4) `caret_then_w_before_the_answer_keeps_the_came_from_line` (lands on the came-from line,
      not 1) · `a_came_from_answer_without_a_line_says_so` (the words above; `Esc` → newer blame, same line).
- [x] **Minor 2, red first:** (red: left 6, right 4; navigation.rs; 3c3e65a4) `came_from_the_line_before_a_pure_deletion_is_unshifted` — `came_from(&[hunk(5,2,4,0)], 4)
      == 4` and `came_from(&[hunk(5,2,4,0)], 5) == 7`.
- [x] **Minor 4:** `b_on_the_changes_panes_file_row_blames_the_working_tree` (open.rs; lands right today; red by mutation: cursor.rs:79 → last hunk's first change, failed open.rs:266) (files list focused; `Rev::WorkingTree`;
      cursor on the first changed line) — red first if the cursor lands elsewhere today, else red by mutation.
- [x] **Minor 5, red first:** `blame_failure_words_are_sanitised` (keys.rs; red at :256, raw `\u{202e}`/`\x1b` passed through) — words holding `\u{202E}` and `\x1b[31m` reach the
      view neutralised, as `sanitize_line` does.
- [~] **Minor 7** (first two ✓: `i_toggles_ignore_revs_and_the_line_says_which` asserts both wordings through the frame; `the_ignore_line_and_i_stay_through_a_re_read` red at keys.rs:156, green after. Third ✗ BLOCKED: see NOTES) `i_toggles_ignore_revs_and_the_line_says_which` asserts the line's words after the answer (both
      toggles, B1 `:1103-1104`) · `the_ignore_line_and_i_stay_through_a_re_read` (red first: `keys.rs:405-408` hides them)
      · `every_key_the_bar_draws_is_answered` green with a reading-after-`i` state in its states.
- [x] **Minor 8:** `a_click_on_each_drawn_body_row_puts_the_cursor_there` (tests/mouse.rs; red by mutation: `header_rows` 1 → 2, failed mouse.rs row 2 left 1 right 2) — render the place at 120×40 and 80×24, with and
      without the ignore line; click every drawn body row; the `▸` is drawn on that row. Red by mutation: shift
      `place/keys.rs`'s body top by one (state it).
- [x] **Minor 6:** (grep → 0 lines) `20-blame-place.md` Navigation box: drop "; the rename half of `enter_lands_…` not covered" and replace
      "(pane closed: with it open the pane's own `Esc` comes first)" with "(see 20i: with the pane open `Esc` also goes
      back)". Check: `grep -n "not covered\|pane's own .Esc. comes first" 20-blame-place.md` → 0 lines.
- [x] **Minor 9:** (bullet added before the Map view bullet) `ARCHITECTURE.md` "where to change what" (`:80-95`) gains one bullet: blame and places →
      `ui::blame`, `app::update::place` (and its files), `app::view::frame::overlays::draw_place`, `app::jobs::detail`.
- [x] **Capture** `docs/shots/m7/esc-back-to-blame-from-the-graph-120x40.txt` (4.9 KB; `capture_esc_back_to_blame_from_the_graph`) (≤ 200 KB), two frames with "Right:"
      lines — (a) graph focused, pane open, trail hidden: bar leads `esc back to blame`; (b) after one `Esc`: blame, the
      same line under the cursor. Pinned by `graft::it` `…::capture_esc_back_to_blame_from_the_graph` (asserts both
      frames' words). By eye: the user's (rule 6).
- [x] graft `MAP.md` `## update::place` (updated in the docs commit): file list and moved items updated; `esc_goes_back` added; `BlameLoad::Reading`
      field.
- [x] Gates workspace-wide (fmt ok · clippy ok · nextest 4111/4111 · doc ok): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
      `cargo test --doc`.

## Touches (check each, name how, in a **Touches:** line)
- `Esc` callers: `update/keys.rs` `escape_key` (all rungs), `update/detail/keys.rs` `:67`, `:126`, and every other `Esc`
  arm in `detail/keys.rs` (Explain close, diff/file rungs) — grep `Escape` in `update/`; each existing `Esc` test in
  `update/detail/tests*` and `update/place/tests/` green.
- The bar: `view/rows.rs:410` (`BackToBlame` wrap), `back_to_blame`, `place_context`; `every_key_the_bar_draws_is_answered`,
  `every_context_reaches_every_shape`, `every_swept_state_is_the_context_it_claims` green.
- **21 history-place (not built):** the rung is place-generic. Its spec carries the history test and table rows (planner
  note, 2026-09-27, "from 20i"). Don't build history here.
- **20f (merged first):** its `commit-back-to-blame` walk and captures — grep `back to blame` in `docs/shots/`; say for
  each whether its frame is a state whose `Esc` behaviour changed. Bar words don't change under this ruling; a
  capture differs only if it drew a rung-2 state with `esc back to blame` (then it was wrong; re-shoot and say so).
- `tests/it/main.rs` module registration (20f's file, hence Blocked by).
- Journal/undo: no write added. Major 2 pins `u` ending the trail through `update`; `compare_base` stays unjournaled
  (grep `update/undo*`, `jobs/` for `compare_base` → unchanged).
- Mouse: none changed (Minor 8 is a test).
- Keymap §2.1: this ticket applies the amendment above; check the code matches it.
- `20-blame-place.md` (Minor 6), `ARCHITECTURE.md` (Minor 9), graft `MAP.md`.
- KNOWN_GAPS rows closed by this ticket: M7/20eg-major1..4, m1, m2, m4..m9 (conductor moves them; builders don't edit
  KNOWN_GAPS).

## Size
~75k. If it runs over, stop at a green seam after step 3 (split + Majors 1–2 + keymap) and report; the conductor splits
Major 3 and the Minors into 20j with this spec unchanged.

## NOTES (20i builder, 2026-09-27)
- **Compacted mid-ticket** (after Major 3 and Minor 2's code). Remaining: only the blocked sweep row below.
- **Blocked — needs graft-ui:** `every_key_the_bar_draws_is_answered` with a reading-after-`i` state fails: the place's bar
  draws `⏎ that commit` and `^ before it` during any read, and neither answers (`update/place/keys.rs:219`, `:273` return on
  a body that isn't `Ready`). This is older than 20i; no read state was swept before. `i` itself is fine now. Fix needs a
  field on graft-ui `HintContext::Blame { finding, ignore_file, read: bool }` (drop `⏎`/`^` while `read`), plus
  `view::rows::place_context` setting it. The sweep row was left out so the gates stay green. Next step: that seam, then add
  `reading_after_i` to `update/tests.rs`'s `HintContext::Blame` states (~:4485).
- **Deviation, Minor 7 carrier:** `BlameLoad::Reading` gained a second field, `ignore_listed: Option<usize>`. The count has to
  outlive the read, and the ticket's signature names only `came_from`.
- **Minor 1:** no git's-words variant. The job has no carrier for them: a failed diff read fails the whole job with
  git's words. `<n>` = the newer place's cursor final line (new `BlameState::cursor_line`), falling back to `came_from.line`.
- **Split:** the move commit a3c69c96 broke `update_cannot_reach_the_world` for one commit (`tests/mod.rs`). b5602484 fixed it
  by renaming it to `tests.rs`. The refusal arm's two arms plus `refuse()` let the type express it, so there is no `unreachable!`. Extra files: `tests/esc_back.rs`, `tests/mouse.rs` (keys.rs hit 406).
- **Esc decision:** walk rungs (a pending G jump, a seek, promoting, reading) are **not** in `esc_goes_back`. A seek left by
  `⏎` must not block going back. Open question: `show_again` doesn't drop a pending landing or seek, so one could still land under
  the shown place.
- **`.` menu over a hidden trail:** the bar does **not** lead `back to blame`. The first `Esc` closes the menu, then the bar leads back
  (`the_menu_over_a_hidden_trail_closes_first_and_the_bar_does_not_lead_back`). Not filed as a question.
- **Moved lines:** detail/keys.rs `:67` → `:84`, `:126` → `:143`. 20f capture `blame-then-commit-120x40` is a pane-focused
  state; its `Esc` behaviour is unchanged and its check_shot is green.
- `jobs/detail.rs` was 874 lines before this ticket (over 400). Not split here.
- wc -l after:
  place.rs 382 · answer 191 · blame_state 100 · from 147 · keys 392 · rows 66 · tests.rs 244 · trail 141 · write 64 ·
  tests/esc_back 169 · hints 63 · keys 329 · mouse 83 · navigation 307 · open 348 · pinned_base 264.
