# Review — m8-polish-and-ship group "bars and hygiene": 15, 15b, 11f A/B/C, 21b-1, 21b-2

Reviewer, one round, read-only against `65492afb`. Each merge diffed against its first parent: `fdb57ab2` (15),
`93413e52` (15b), `def843b6` (11f A: the merge commit itself, second parent `15bf22c2`), `6b5c66e7` (11f B),
`6514ea26` (11f C, `docs/shots` only), `26f1286f` (21b-1), `65492afb` (21b-2). Read: the four tickets, scout
`15-bar-content.md` by reference only, 02-keymap §2.1 (:38–63), design note Prior art/Journeys headings, `KNOWN_GAPS.md`
rows :1207, :1220–1229, MAP rows for every new item.

**Counts: 0 Critical · 1 Major · 8 Minor · 4 Questions.** Rules 5/6: no colour and no "looks right" judged here; the
captures named below are the user's to judge (`M8/15b-captures-unjudged` already carries them).

## What I ran and what it showed

My own detached worktree at `65492afb` and my own `CARGO_TARGET_DIR` (both in the session scratch area, both deleted
after). One command at a time. Load average was 16–25 on 10 cores throughout: another helper's workspace `nextest`
was live the whole time.

| Check | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass (37 s) |
| `cargo clippy --workspace --all-targets --features heavy-tests -- -D warnings` | pass (17 s) |
| `cargo nextest run --workspace` (profile: 4 threads) | **6084/6084 pass**, 13 skipped, 4 slow; 758 s run, 829 s with the build. Under load: not a fast-tier time |
| `cargo test --workspace --doc` | pass (3 doctests) |
| 5 slowest tests | **not captured**: the profile's `status-level = "fail"` prints no per-test times or SLOW names |
| Peak memory (max RSS, `/usr/bin/time -l`) | clippy 1.17 GB · heavy clippy 1.18 GB · nextest 1.44 GB · release perf 1.21 GB |
| `target/` | mine 6.1 GB (deleted); shared `target/` 81 GB |
| Heavy tier `cargo test --features heavy-tests -- --test-threads=2` | **not run.** A group round, two cargo helpers already at the cap with mine; heavy clippy is clean. Owed by the milestone review |
| Release perf run (`frames:: heavy::budgets_hold`, `--release`, 1 thread) | **red, not a verdict**: 12 passed, **9 failed**, 197 s. Every failure is an input-to-frame p99: Pixel 50k 34.1 ms, Unicode 50k 21.3, diff scroll 20.6, pane open 20.3, Map 20.6, J find 22.8, 100k Pixel 29.4, 100k Unicode 21.3, `G` after idle 23.3 (budget 16). Passed: first paint 100k realistic 27.7 ms (commit-graph) / 38.7 ms (none); Pixel `j` held peak RSS 245 MB (40 MB/s) and 254 MB (free). The run's own control (`the_machine_alone_after_idle_and_warm`) shows 2–10 ms oversleeps on an idle machine — this is load. Same shape as `docs/reviews/m8-doorway-words.md`; not attributed to this group (`fit_order` and `fitted_tail` are a few small allocations per frame; not bisected) |
| Fuzz, benches | not run (group round) |

## Ticks — spot-checked

25 checked, **25 held**. For each: the named test exists at `65492afb`, and I read what it asserts against the box.

| Ticket | Checked | Held | Which |
|---|---|---|---|
| 15 | 9 of 10 (A3 is a cut, no test) | 9 | A1 ×2, A2 ×3, A4, A5, palette, captures (four files exist; each test writes and asserts its file) |
| 15b | 1 of 1 | 1 | `search_hint::after_enter_the_bar_is_the_rows_keys_and_the_search_is_in_the_header` (`tests/it/search_hint.rs:117`) |
| 11f | 7 of 13 | 7 | A pin, A spelled, A pane = banner (both tests), A sizes (`wc -l`), B1 (my own line-set check, below), C1/C3 (footer rows quoted, `grep` at HEAD) |
| 21b-1 | 4 of 4 | 4 | 1 (`keys_text_*` + `rg join(` empty), 2, 3, 4 (line-set check) |
| 21b-2 | 4 of 4 | 4 | 1 (`rg` empty + the three title tests exist), 2, 3 (`rg` empty), 4 (line-set check) |

11f B2 is still `[ ]`, correctly: its remainder was done by 21b-1 item 1 (Minor 7 below).

## Move-only splits

Checked by script on each split commit (removed lines vs added lines, whitespace-stripped, `use`/`mod`/`//!`/brace
lines and `pub(..)` prefixes ignored): `fe0bda29`, `ec427ea5`, `4827304e`, `5530e354` (21b-1); `0f991a99`, `c9b1996c`,
`f047446d`, `5259a592`, `84772b79` (21b-2); `6a7d0216`, `11e564dc` (11f B1). **Every one is a move.** The only
residue is `fmt` re-wraps, `use`-list re-wraps, `super::` → `super::super::` one module deeper, and one doc link
gaining a path (`graph_column/kinds.rs:98`) — all as each builder's NOTES say. Every production file the items name is
≤ 400 by `wc -l` at `65492afb`: `hints.rs` 149, `hint.rs` 323, `doorway.rs` 118, `filters.rs` 350, `fitting.rs` 316,
`fitting/stop.rs` 223, `fitting/place_footer.rs` 102, `map_screen.rs` 129, `map_screen/render.rs` 389,
`render/footer.rs` 84, `stash_screen.rs` 383, `stash_screen/view.rs` 92, `blame.rs` 314, `blame/line.rs` 262,
`graph_column/mod.rs` 340, `kinds.rs` 114, `palette.rs` 326, `palette/row.rs` 120, `hints/lists.rs` 377 (+ `live` 100,
`rows` 92, `editors` 51), `context_menu.rs` 387, `context_menu/draw.rs` 53. Two new *test* files are over (Minor 6).

## Keymap (§2.1)

- **`u` fitted second** (02-keymap.md:48–49) is implemented once, in `fit_list`
  (`graft-ui/src/graph_screen/hints/fitting.rs:144–181`, `fit_order` :189–198), so it holds for every bar that holds
  `u`, not only graph rows. `below_the_floor` (:247) reads only `list.first()`, which `fit_order` never moves, so the
  reorder cannot leak into the sub-floor rung. Pinned by `undo_is_drawn_at_100_and_80_on_every_row_form_that_holds_it`.
- **Marked-row doorway** still ranks after `u undo` and `/ search` (`filters.rs` `marked_list`), now spelled before
  ranking (21b-1); `doorway_last` (`doorway.rs:9–18`) moves either doorway.
- **`REFUSED_WHILE_LIVE`** follows the mode rule (:55–56). Its exception (a shell-begun merge, where `u` acts) is a
  hint withheld, carried as `M8/15-u-hidden-shell-begun-merge`.
- `n`/`N`: `app.next_match`/`app.prev_match`, already bound; no key newly claimed by this group. Spelling differs
  across screens (Question 3).

## Pipeline

The design note has Need, Prior art and Journeys. No journey step count changes in this group (`n`/`N`, `S`, `?`
keep their counts). One aspect has no prior-art row (Minor 8).

## Error handling, sanitising, UI thread

No ignored `Result`, `let _ =`, `unwrap_or_default` or `unwrap`/`expect` added outside tests in any of the seven diffs
(grep of added non-test lines). The header's search item is `sanitize_line(&search.line(..))`
(`graft/src/view/rows.rs:351–361`) drawn as `SanitizedText`. No new git work on the UI thread: A3 was cut precisely
to avoid a per-row spawn; `cursor_is_stash` (`rows.rs:884–891`) reads resident refs.

---

## Critical

None.

## Major

### 1. This group grew production files already over 400 lines, and pushed one over (PROMPT.md 6.2)

`wc -l`, at `fdb57ab2^1` → `65492afb`:

| File | Before | After | Grown by |
|---|---|---|---|
| `graft-ui/src/graph_screen/layout.rs` (production to :608) | 710 | 828 | 15b header tail (`search_item`, `tail_items`, `Drop`, `TailItem`, `fitted_tail`, :152–303) |
| `graft/src/view/rows.rs` (production to :1103) | 1133 | 1215 | 15 `cursor_is_stash`, 15b `search_tail`, `search_wrapped` |
| `graft-ui/src/graph_screen/hints/catalogue.rs` (no tests) | 567 | 634 | 15b `searching_form`, `over_rewritable` |
| `graft-ui/src/graph_screen/footer.rs` | 387 | **430** | 15b's two replacement tests (inline `mod tests` from :284) |

- **What's wrong:** the same window spent two parts (21b-1, 21b-2) bringing files *under* 400, while 15/15b added
  ~280 lines to four that were over or at the line. `footer.rs` crossed it in this group. The class row
  `M8/files-over-400-unrowed` (`KNOWN_GAPS.md:1242`) cites `view/rows.rs` only as an example; the other three are
  unnamed.
- **Why it matters:** PROMPT.md 6.2 is a hard rule here, and each later hygiene round pays for growth no feature
  ticket was asked to avoid (11f ruled "Part A is built not to grow an over-400 file"; 15 and 15b had no such line).
- **Fix:**
  - `layout.rs`: the header tail (:263–303 plus its three types) is one seam → `graph_screen/header_tail.rs`.
  - `footer.rs`: inline tests → `footer/tests.rs` (11f B1's method).
  - `rows.rs`: the hint-context classification (`cursor_row_context`, `cursor_is_stash`, `search_wrapped`,
    `live_wrapped`, `back_to_place`) is one seam → a `view/rows/` child.
  - Add a planner line to feature tickets: "built not to grow an over-400 file" (the 11f wording).

## Minor

### 1. A stash row's `.` menu still names `(m)`/`(r)` and opens their preview — the half of A4 the bar can't see

- **Where:** `graft/src/update/context_menu/slots.rs:98–101` calls `merge_and_rebase_verbs` for a stash row.
  `verbs.rs:253–273` labels them `Merge <sha> into main (m)` and `Rebase main onto <sha> (r)`. Chosen, they go
  `context_menu.rs:334–345` → `preview::open_preview`. The only `is_stash` check on that route is the keys'
  (`update/preview/history.rs:108`; `grep is_stash crates/graft/src` finds no other in `update/preview*`).
- **What's wrong:** A4 took `m`/`r` off the stash row's bar because they refuse there. The `.` menu on the same row
  names those keys anyway, against its own rule ("a label names a key only where one is live", `verbs.rs:228–229`).
  Its items then preview the merge or rebase of the stash's internal commit that the keys refuse.
- **What was verified:** read from code only; no test drives it. The menu test `a_stash_row_menu_is_unchanged`
  (`context_menu/tests.rs:880`) uses a fixture with no merge/rebase items.
- **Why only Minor:** this predates the group, and a Ghost Preview stands before any write.
  `KNOWN_GAPS.md:1058` (`M7/u-m4`) is still open, but it names only the keys' path (`history.rs:92–106`).
- **Fix:** skip `merge_and_rebase_verbs` on a stash row, or give `open_preview` the one stash check. Add a test that
  a stash row's menu holds no `(m)`/`(r)` item. Amend `M7/u-m4` to name the menu path so it isn't closed on the
  keys alone.

### 2. `S stash` on a stash row opens the list at `stash@{0}`, not at the stash he was on

- **Where:** `graft/src/update/stash.rs:175–178` (`next.stash.selected = 0`). The new hint is
  `graft-ui/src/graph_screen/hints/lists/rows.rs:81`, and `context.rs:65` documents it as "the door to the stash's
  own verbs".
- **What's wrong:** the door goes to the newest stash, not to this one. On `stash@{2}`, `S` then `p` pops
  `stash@{0}`.
- **Why it matters:** this is the journey A4 creates. He either takes extra `j` steps, or pops a different stash than
  the row he was on (H4/H5). It is recoverable through the journal.
- **Fix:** when `S` is pressed on a stash row, select that entry (`RefBadges` already knows the index). Pin it with a
  test, and count the step in the note.

### 3. Map `⏎ zoom to commits` is offered in Empty and Failed, where `⏎` only closes

- **Where:** `graft-ui/src/map_screen/render/footer.rs:61` always adds `map.seek`. Only `j/k` is gated on rows
  (`:56–60`). `graft/src/update/map.rs:191–193`: with no selected tip, `⏎` closes the view.
- **What's wrong:** with no row to select, the clause names an action that doesn't happen there (keymap :55–56).
- **When:** not introduced here. 11f B and 21b-1 rewrote this function and kept it.
- **Fix:** gate `map.seek` on `has_rows`, as `j/k` is, and extend `footer_never_draws_a_fragment`'s cases.

### 4. The stash-row sweep's doc claims a live wrap the test never asserts

- **Where:** `graft/src/update/tests.rs:4187–4198`. The doc says the frame classifies the row `StashRow` "bare, and
  inside a live operation's wrap". The test asserts `view.hint_context == HintContext::StashRow` for every state
  `states_for(&HintContext::StashRow, ..)` returns. That is the bare form only.
- **Fix:** drop the clause, or add the `OperationLive { row: StashRow }` states and assert them.

### 5. 21b-2's graft pin cannot see the literal it exists to prevent

- **Where:** `graft/src/update/place/blame_state.rs:116–132` asserts
  `view.default_ignored == BlameOptions::default().ignore_whitespace`. The default is `true` today, so a literal
  `true` at `blame_state.rs:98` stays green. The builder's NOTES say so.
- **What's wrong:** the test pins only the wiring's shape today.
- **Fix:** none cheap. Record it in the row that closes `M8/blame-ws-default-twice`, so it isn't read as a guard.

### 6. Two new test files are over 400

- **Where:** `graft-ui/src/context_menu/tests.rs` 441 (21b-2) and `graft-ui/src/stash_screen/tests.rs` 424 (11f B,
  grown by 21b-1).
- **What's wrong:** each was created to bring its parent under the line, and arrived over it.
- **Fix:** split each by topic, or name both in `M8/files-over-400-unrowed`.

### 7. Stale wording left by this group (textual)

- **Status line:** the 11f ticket's (`11f-doorway-words-review-fixes.md:3`) reads "Parts B, C open". C is done, and
  B2's remainder was closed by 21b-1 item 1. Note that on the B2 box; it stays unticked.
- **Caption vs frame:** the caption at `docs/shots/m8/doorway-stash-120x36.txt:7` (written from
  `graft/src/update/tests.rs:6964`) says `… space files  esc close  ? keys`. The frame under it (:44) draws
  `S/esc close`. The m3 headers `m3-stash-list.txt:13` and `m3-stash-empty.txt:9` say the same. Already routed as
  21b-4 box 5e (open); listed here so it isn't lost if 21b-4 slips.

### 8. 15 A2 (where a standing search lives) has no prior-art row

- **Where:** the design note's Prior art (`docs/design-notes/m8-polish-and-ship.md:39–115`) has "words in the bar"
  (:58), but nothing on where a search's status sits once the field closes. 15 A2 invented "the header's tail".
  Search status lives at the bottom in lazygit, and in vim's and less's command line. None of that is cited, taken
  or rejected.
- **Why it matters:** pipeline step 3 says invent only where nothing found fits, and say why.
- **Fix:** add one Prior art row for A2's placement (take / reject / invent, with sources) before the usability walk
  judges it.

## Questions

1. **Does `c`'s refusal keep naming `space marks resolved` after he moves off the CONFLICTS row?**
   - The words are computed once, at the press: `banner/pressed.rs:37` → `resolve_words.rs:22–30`.
   - The status is `Sticky`, which only `Esc` clears (`update/keys.rs:276–279`; `state.rs:1180–1181`: "dismissed by
     hand, never by a later job").
   - So one `j` onto `b.rs` would leave the line naming `space`, while `space` now stages `b.rs`. That is the case
     11f's ruling rejected ("it would stage another file").
   - Reasoned from those lines, not driven. A test would press `c` on row 1, `j`, then assert the keys.
2. **Can a standing search vanish from both rows?**
   - `fitted_tail` drops the search whole before the banner (`layout.rs:263–290`), and the footer no longer shows it.
   - On a narrow header (a long path or branch name, or a live banner taking the room, `:161`), the search can be
     invisible while it stands.
   - With zero hits the bar's `n N` are withheld too (`rows.rs:371–378`), so nothing on screen says a search is on.
   - For the usability walk (H1 visibility). Not observed.
3. **One key pair, three spellings:**
   - Graph bar: `n next · N prev` (`hints/filters.rs:257–260`).
   - The header's search line, beside it: `n/N next/prev` (`graft/src/search.rs:360`).
   - Blame: `n/N match` (`hints/lists.rs:218`).
   - A2 named the first. Is this one verb (ux H4)? For the usability walk, beside `close` vs `back` (row
     `M8/stash-footer-close-key`).
4. **`b branch` on a stash row** (in `docs/shots/m8-bar-stash-row-120x36.txt`):
   - On the graph, `b` names a branch at the stash's internal commit. In the `S` list, `b` runs `git stash branch`.
   - Same key, same stash, two different results.
   - Intended? 15 NOTES found no refusal line for it (`name_prompt.rs`). Not driven.

## Knock-on effects checked

- **`SearchLine::Status` readers:** compiler-enforced; `footer.rs` only.
- **`HintContext` exhaustive matches:** catalogue, hints, graft `update::tests` all extended (`every_context` sweeps
  `StashRow`, `Searching`).
- **Palette/help:** does not read `HintContext` (grep, as 15 NOTES).
- **Place footers sharing `place_footer_text`:** J, S and Map. Their tests pass in the run above.
- **Captures against code:**
  - J: `m3-08`, `m4-07`, `J/esc close  ? keys`.
  - S: `m3-06`, `m3-stash-*`, `m8/doorway-stash-*`, `S/esc close  ? keys`.
  - Map: `m2-15-*`, `j/k select  ⏎ zoom to commits  M/esc back  ? keys`.
  - Bars: the four `m8-bar-*` captures.
  - Each matches the string its code builds, except the captions in Minor 7.
  - Every touched capture is ≤ 200 KB (largest `m7/m7-contact-sheet-1.txt` 177 576 B), with no home path.
- **The `.` menu and the `S` overlay** draw the same stash row: Minor 1, Minor 2.
- **Docs:** no file:line reference to a split file is left in `crates/**` (grep). `ARCHITECTURE.md`'s two
  references (`palette.rs`, `context_menu.rs`) still name existing files.
