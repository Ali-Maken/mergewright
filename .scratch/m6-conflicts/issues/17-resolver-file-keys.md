# fix-resolver-file-keys: `O`/`T` settle the whole file in one key and move on; `j`/`k` move between files

Status: done (builder 2026-09-24; journeys box open for ticket 21, see NOTES). **Revised 2026-09-24 in place (planner, spec-only)** from `.scratch/m6-conflicts/scout-17.md` (main `721547f6`, 16 merged). Filed 2026-09-23. P6 and P7 are approved: the keymap rows are written (`02-keymap.md:264`, `:268`) and the §2.1 exception is named (`:29–:32`).
- Scout item 1: the path is `update/resolver.rs`, not `resolver/mod.rs`. Fixed in Signatures.
- Item 2: `file_cursor` is a **function**, `file_cursor(&AppState) -> usize` (`files.rs:142`). It is not a field. See D4.
- Item 3: the bar's `⏎ resolved` becomes `⏎ done`, per the mockups (design note :661, :692), CONTEXT.md **Done (file)** (`Avoid: resolved`) and keymap :270. See D5.
- Item 4: `BOUND_ACTIONS` is in `graft/src/update/keys.rs:755`. It is not in graft-ui. See Scope.
- Item 5: `O`/`T` do **not** go through `take_all_key`, which stops at `nothing open in <file>` (`take.rs:125`). They get their own function. See D1.
- Scout item 6 (`O` on a card advances through `staging` → `resolved_advance`, `resolved.rs:66`) is consistent. It is used as-is.

Blocked by: none. 13, 15 and 16 are merged.
Build with: **Sonnet** (scouted: `scout-17.md`). Both keys chain functions that are already built (`take_side`, `take_landed`, `mark_resolved`, `resolved_advance`, `select_file`, `cross_target`). Every order of checks and every set of words is fixed below, and the tests judge it. Not Fable: nothing later is built on it.
Needs:
- 16: `take::take_all_key` (for its check order, not a call), `take_side(.., TakeAt::Every, ..)`, `pending_file`, `take_refusal`, `take_landed`.
- 13 and 25: `listed_files`, `file_cursor(&AppState) -> usize`, `select_file(state, commands, index, next_id) -> bool` with its `switch_refusal`.
- 15: `in_flight`/`queued`, and `walk::cross_target(&AppState, forward) -> Option<usize>`.
- 28: `done::refuses`, `done_refusal`.
- `resolved`: the private `mark_resolved` (⏎) and `resolved_advance`.

Crates: graft (`update/resolver.rs` and its children, `update/keys.rs`, `it` tests), graft-ui (`graph_screen/hints/lists.rs` `resolver_list` and its tests). Docs: `crates/graft/MAP.md`, `crates/graft-ui/MAP.md`, `TESTING.md`.

## Read first
- This ticket. `.scratch/m6-conflicts/scout-17.md` (whole).
- `docs/design-notes/m6-conflicts.md`, Amendment 2:
  - Journeys `:439–:505`: J4, J5, J8, J9, "Where each event lands him", and the `journey-counts` block.
  - Where I'd differ P6 and P7 `:626–:635`.
  - New UI: the two bars at `:661` and `:692`, and the done-file line `:702–:705`.
- `docs/design/02-keymap.md`:
  - §2.1 `:17–:37` (the UPPERCASE rule and the `O`/`T` exception) and `:38–:56` (the hint bar is a ranked selection).
  - §2.6 `:252–:276`, especially `:264`, `:267`, `:268` and `:270`.
- `CONTEXT.md` **Done (file)** (`:118`).
- `crates/graft/MAP.md` §`update::resolver` (`:307–:336`). `crates/graft-ui/MAP.md` `graph_screen::hints` (`:22`): the `Resolver` table, and `Map`'s `j/k move`, which is the precedent for a hint with a two-key label.
- Ticket 16: D3 (check order), D4 (landing notice), and its NOTES on bar cost and per-hint rungs.

## Decisions
- **D1: `O` / `T` are their own function, `take::settle_key`.** It does not go through `take_all_key`. `O`/`T` are matched the way `B` is matched in `act`. Checks run in this order, and the first that applies answers:
  1. **Done file.** `done::refuses` gains `O` and `T`, so they get `done_refusal`'s words, both the can and the cannot form (28 R5). Nothing is written.
  2. **Whole-file or binary card** (`whole_file.is_some()`). Call exactly what `o` (for `O`) or `t` (for `T`) calls on that card. That already sets `staging`, and `resolved_advance` moves on (scout item 6). Nothing new is added.
  3. **`take_refusal`**, with its words as today: a read, a base load, a git write, or the cap. It gains one case: `settling.is_some()` → `still writing the last take`. So while a file settles, `o t b Ctrl+O Ctrl+T O T` all refuse with those words.
  4. **Open blocks left.** `pending_file(..).open() > 0` → `take_side(.., TakeAt::Every, take, ..)`, which sends the entry or queues it as one entry (16 D2). Set `settling = Some(take)`.
  5. **Nothing open, nothing in flight** (`open() == 0`, `in_flight.is_none()`) → call `mark_resolved`, the function `⏎` calls. So every refusal `⏎` has applies here with its own words. `settling` is not set.
  6. **Nothing open, but a take is in flight** (the queued takes already close every block) → `settling = Some(take)`, and no entry is added. The Stage follows the last landing (D2).
- **D2: the Stage is sent at a landing.**
  - **Invariant:** `settling.is_some()` ⇒ `in_flight.is_some()`, except inside the single update that sends the Stage. So every refusal keyed on `in_flight` also covers a settling file: `handover`'s `e`/`M`/`Esc`, `select_file`'s `switch_refusal`, and `d`.
  - **Where the Stage is sent:** in `take_landed`, after its existing work. If the landing sent nothing next (the queue is empty) and `settling.is_some()`: take `settling`. If the landed document's `open() == 0`, call `mark_resolved`.
  - **The other case** (`open() > 0`) cannot happen, because every entry ahead of it was in `pending_file`. If it does happen, clear `settling`, stage nothing, and add no new words.
  - **Where he lands:** the `Stage` answer goes through `resolved_advance` unchanged. That gives the notice `<file> done ✓ · ▸ <next>` and the next file not done, in list order, wrapping. On the last file it does what `resolved_advance` does at the time of the build (the banner today, or ticket 31's landing once 31 has merged).
- **D3: a failure while settling.** A failed take, or a `Changed` re-read, drops the queue as it does today. It also clears `settling` and appends ` · nothing staged` to the notice that path sets today, with the dropped count included. It sends no Stage.
- **D4: `j`/`k`, `↓`/`↑` step between files.** This is `files::step_file`.
  - It takes `i = file_cursor(state)` and calls `select_file(state, commands, i + 1, ..)` (`j`/`↓`), or `select_file(.., i - 1, ..)` (`k`/`↑`).
  - **At the ends** (`i + 1 == listed_files(state).len()`, or `i == 0`): nothing is sent, the words are unchanged, and it returns `true`. There is no wrap (§2.2 "move selection"). The list's `▸` on the end row is what tells him.
  - **Refusals are `select_file`'s own** (`switch_refusal`), with no new words: a take in flight, a settling file, a read, a base load, a write.
  - **A done file is not refused.** `j`/`k` are not in `done::refuses`, so they move from a done file like from any other.
- **D5: where scrolling goes** (keymap `:268`, which decides it).
  - `scroll_key` (`scroll.rs:128`) drops `j`/`k`/`↓`/`↑`.
  - `PgUp`/`PgDn`, `g`/`G` and the wheel are unchanged. `Home`/`End` are unchanged too: they are bound today, and :268 does not name them. That is a question for the conductor, below.
  - `n`/`N` still walk the blocks, and the cursor block still scrolls itself into view.
  - There is no line-by-line scroll key after this ticket. That is P7's approved trade-off (design note `:634`).
- **D6: `n`/`N` from a done file cross** (moved here by 25 C3, `:71`). A done file has no blocks, so it is "past the end" (keymap `:267`).
  - `done::refuses` drops `n`/`N`.
  - On a done file, `n`/`N` use rule 1 only: `walk::cross_target(state, forward)` → `select_file`, with notice `▸ <file>`, exactly as a cross does today.
  - If `cross_target` returns `None`, the notice is `no more conflicts` (rule 3's words without `⏎ marks`, since the file is already done). The cursor does not move.
- **D7: the bar.**
  - `resolver_list` gains two hints, each **one hint with a two-key label**, the way `Map` has `j/k move`:
    - `O/T file`, right after `^T all <theirs role>`.
    - `j/k file`, right after `⏎ done`.
  - `⏎ resolved` → `⏎ done`, in `resolver_list` and in `BOUND_ACTIONS` (`keys.rs:829`).
  - No `short_form` rows: the Key rung is `O/T` and `j/k`, and `file` and `done` have no short form.
  - Neither hint goes into `resolver_card_list` or `resolver_done_list`. 25 and 28 pinned those bars exactly. Whether `j/k file` belongs there is a question for the conductor.
  - The table is not otherwise reordered. The mockup's order (`⏎` before `n`, and so on) is out of scope.

## Scope
In:
1. `update/resolver.rs`: the `ResolverState.settling` field. `act` sends `O`/`T` to `settle_key`, and `j`/`k`/`↓`/`↑` to `step_file`.
2. `update/resolver/take.rs`: `settle_key` (D1), and the settling case in `take_refusal`.
3. `update/resolver/answers.rs` (`take_landed`, and the paths for a failed take and a `Changed` answer): D2 and D3.
4. `update/resolver/resolved.rs`: `mark_resolved` becomes `pub(super)`. Its body does not change.
5. `update/resolver/files.rs`: `step_file` (D4).
6. `update/resolver/scroll.rs`: D5.
7. `update/resolver/done.rs`: `refuses` gains `O`/`T` and drops `n`/`N`. `update/resolver/walk.rs`: D6, with `cross_target` made `pub(super)` if another child needs it.
8. `update/keys.rs` `BOUND_ACTIONS` (`:755`): rows for `O`/`T` and `j`/`k` (and `↓`/`↑` if the table lists arrows separately, as `Map`'s rows do), plus the rename at `:829`.
9. graft-ui `hints/lists.rs` `resolver_list` (`:259`): D7.
10. `TESTING.md` "Conflicts walk", and the scripts and tests that press the four keys inside the resolver. See the acceptance boxes for how to find them.
11. The MAP.md entries for everything above.

Out:
- `Ctrl+O`/`Ctrl+T` themselves (16).
- `c` in the resolver (31).
- `AppState::choices` (19). `O`/`T` land through `take_landed` as `TakeAt::Every`, which 19 records.
- Any key outside the resolver.
- Reordering the bar.
- The card and done bars (D7).

## Signatures
```rust
// graft/src/update/resolver.rs
pub struct ResolverState { /* existing */, pub settling: Option<Take> }
// Some from an accepted O/T until its Stage is sent. Invariant: Some ⇒ in_flight.is_some() (D2).

// graft/src/update/resolver/take.rs
pub(super) fn settle_key(state: &mut AppState, commands: &mut Vec<Command>, take: Take, next_id: /* take_all_key's type */) -> bool;

// graft/src/update/resolver/files.rs
pub(super) fn step_file(state: &mut AppState, commands: &mut Vec<Command>, forward: bool, next_id: /* select_file's type */) -> bool;
// file_cursor(state) ± 1 → select_file; at either end: nothing sent, true

// graft/src/update/resolver/resolved.rs
// mark_resolved: private → pub(super); signature and body unchanged
// resolver_key, select_file, file_cursor, take_landed, resolved_advance: signatures unchanged
```

## Acceptance (name the verifying test when you tick; real-git truth lives in `it::resolver`, per 16's NOTES)
- [x] `it::resolver::file_keys::shift_t_takes_theirs_marks_done_and_moves_on`. Two-file fixture, on `notes.txt`, press `T`:
  - The bytes equal per-block theirs.
  - `git ls-files -u notes.txt` is empty.
  - The open file is `gone.txt`.
  - The notice reads `notes.txt done ✓ · ▸ gone.txt`.
- [x] `it::resolver::file_keys::shift_o_on_a_whole_file_card_keeps_yours_and_moves_on`. `O` on `gone.txt`: the file exists with `HEAD`'s blob, it is staged, and the open file is `notes.txt`.
- [x] `it::resolver::file_keys::shift_t_with_every_block_taken_only_stages`. `t` on each block and let them land, then `T`: exactly one `Stage` and no take (D1 step 5), and then the same landing as `⏎`.
- [x] `update::resolver::tests::file_keys::shift_t_waits_behind_a_queued_take`. `o` is in flight, then `T`:
  - `queued == [(Every, Theirs)]` and `settling == Some(Theirs)`.
  - No Stage goes out on the first landing. The Every entry is sent then.
  - One Stage goes out on the second landing, and `settling` is `None`.
- [x] `…::shift_t_with_nothing_open_but_a_take_in_flight_stages_after_it` (D1 step 6). The last open block's `t` is in flight, then `T`: no entry, `settling` is set, and the Stage goes out when that take lands.
- [x] `…::takes_refuse_while_a_file_settles`. After `T` with `settling` set, each of `o t b Ctrl+O Ctrl+T O T` gives `still writing the last take`, with no command and `queued` unchanged.
- [x] `…::a_changed_file_is_not_staged` and `…::a_failed_take_stages_nothing` (D3). No Stage, `settling` is `None`, and the notice ends with ` · nothing staged` and keeps the dropped count.
- [x] `…::shift_t_on_a_done_file_is_refused`. Both forms of `done_refusal`, and no command.
- [x] `…::shift_t_on_a_binary_card_is_the_cards_t`. The commands and state are equal to what `t` on that card produces.
- [x] `…::j_and_k_move_between_files_without_wrapping`. Three-file list:
  - `j` goes from row 0 → 1 → 2.
  - `j` at row 2 sends nothing, stays on 2, and leaves the words unchanged.
  - `k` walks back up. `k` at row 0 sends nothing.
  - `↓`/`↑` do the same.
- [x] `…::j_from_a_done_file_moves`. It is not refused.
- [x] `…::j_while_a_take_is_in_flight_is_refused`. Nothing is sent, and the words equal what `select_file` gives in that state (built by calling it, not by copying the literal).
- [x] `…::page_keys_scroll_the_document`:
  - `PgDn`/`PgUp` move `top` by a page.
  - `g`/`G` go to the top and bottom.
  - On a one-file list, `j`/`k` leave `top` unchanged.
- [x] `…::n_from_a_done_file_crosses_to_the_next_open_file`, and `…::n_from_a_done_file_with_nothing_open_says_no_more` (D6).
- [x] graft-ui `graph_screen::hints::tests::resolver_offers_file_keys`:
  - `O/T file` comes right after `^T all …`, and `j/k file` right after `⏎ done`.
  - Neither is in the card list or the done list.
  - `resolved` is not a verb anywhere in the resolver tables.
- [x] `update::tests::every_bound_action_is_hinted_live_or_is_named_here` and `every_key_the_bar_draws_is_answered` pass, and `NEVER_HINTED` is unchanged. Name both tests.
- [x] **Bar at each width, reported.** Pinned by `graph_screen::hints::tests::resolver_offers_file_keys`; strings in NOTES. A test pins the resolver bar string for an open text file at 80, 120 and 200 columns. Paste all three strings into NOTES.
  - At 200 it must contain `O/T file` and `j/k file`.
  - At every width, anything that was on the bar before this ticket and is now missing (`esc` above all) is listed in NOTES for the conductor. **Do not reorder the table to save it.**
- [x] **Existing tests that press these keys.** Listed in NOTES; re-pointed ones green in `file_list::a_done_file_refuses_in_words`, `it::resolver::file_list::done_open_capture`, `…::start_over::done_start_over_capture`. The scout could not confirm what `update/resolver/tests/file_list.rs:813` expects of `j`.
  - Grep the graft tests for `j`/`k`/`↓`/`↑` pressed while the resolver is open, and list each one in NOTES with the assertion it made, quoted.
  - One that asserted a scroll is re-pointed to `PgDn`/`PgUp`, with the expectation re-derived.
  - One that asserted anything else is **not rewritten**. Report it as a question.
- [x] **Scripts and TESTING.md.** none found; commands in NOTES. The scout could not confirm which scenarios use `↑`/`↓`.
  - Grep `TESTING.md` "Conflicts walk" and the pty walk scripts under `scripts/` for those four keys sent inside the resolver. The resolver is open from a `⏎` on a conflicted row until `Esc`.
  - List every hit in NOTES (file:line and the scenario name), with the grep command.
  - Change the scroll uses to `PgUp`/`PgDn`. Where a hit is ambiguous (for example the `k ⏎ j…` prelude that 15's NOTES mention), report it and leave it unchanged.
  - If there are no hits, write `none found` and the command.
- [ ] Journeys at their `P6:` counts in the acceptance walk (ticket 21): `one-file-one-conflict` 1, `three-conflicts-all-theirs` 1, `two-files-in-order` 2, `two-files-out-of-order` 3. This box waits until the conductor has moved those numbers into `keys`. Until then, leave it unticked and say so.
- [x] Captures, pinned by `it::resolver::file_keys::file_keys_capture` (revised per conductor ruling 2026-09-24: two frames per width). The look is the user's to judge (rule 6). Colour is unverified (rule 5).
  - (a) **Before** `T`, on `notes.txt`: `docs/shots/m6/fix-file-keys-before-120x40.txt`, where the bar has `O/T` and `j/k` at some rung; `…-before-200x50.txt`, where the bar has `O/T file`, `⏎ done` and `j/k file`.
  - (b) **After** `T`, landed on the `gone.txt` card: `fix-file-keys-120x40.txt` and `fix-file-keys-200x50.txt`. The strip has `notes.txt · done ✓` and `▸ gone.txt`, the identity row starts `▸ gone.txt`, and the notice reads `notes.txt done ✓ · ▸ gone.txt`. The card bar has no `O/T`/`j/k` (D7).
  - Regenerated captures are listed in NOTES; the TSV is updated and the contact sheet rebuilt.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium: one chained key, one step key, one done-file cross, the bar rows and one rename, ~17 tests, 2 new captures plus regenerated ones. One context. If it will not fit, split `j`/`k` (D4, D5, the TESTING.md box) from `O`/`T` (D1–D3), with D6 and D7 going to whichever half runs second.

## For the conductor (outside this ticket; exact text)
- `docs/design-notes/m6-conflicts.md:572`: replace `// click; `j`/`k` if P7;` with `// click; `j`/`k`/`↓`/`↑` step file_cursor(state) ± 1, no wrap (ticket 17 D4);`.
- `docs/design-notes/m6-conflicts.md:565`: append `, pub settling: Option<Take> /* ticket 17: an O/T waiting for its Stage; Some ⇒ in_flight.is_some() */` inside the `ResolverState` braces.
- `docs/design-notes/m6-conflicts.md`, "Where each event lands him" (`:470`): append ` A failed or changed-on-disk take under `O`/`T` → the same file, notice `… · nothing staged` (ticket 17 D3).`
- **Question: `Home`/`End`.** `02-keymap.md:268` does not name them, and they scroll today. Proposed: append ` (`Home`/`End` as `g`/`G`)` after `g`/`G`. Otherwise they are unbound in the resolver, and that is a spec change.
- **Question: the card and done bars** have no `j/k file`, though `j`/`k` work there (D7). The bars are pinned exactly by 25 and 28. Adding it means editing `:705` (`d start over  j/k file  e edit  esc close`) and 05's card bar.
- `02-keymap.md:267`: append ` On a done file, `n`/`N` cross straight to the next file not done (ticket 17 D6).`

## NOTES (builder)
Scout used: `scout-17.md`. No compaction. Ticks name their test inline (the box's own test name).

- **Captures (resolved by conductor ruling):** split into before/after frames; ticked. Earlier note: `it::resolver::file_keys::file_keys_capture` pins both files. The strip, identity row and notice hold. **The bar bullets cannot hold as specced:** after `T` the resolver lands on `gone.txt`, which is a **card**, and D7 keeps `O/T`/`j/k` off the card bar. The bottom row at the landing is also the Stage's status line `stage notes.txt ✓`, not the bar. Question for the conductor: change the steps (land on a text file), or drop those bullets. Identity row: `▸ gone.txt · modified on yours · main, deleted on theirs · feature`.
- **Open box: journeys, left for ticket 21.** No test parses the note's journey-counts block yet (`grep -rl journey-counts crates` finds none), so there is nothing to name. The counts are the conductor's.
- Ticket 32 owns the 120-column bar (`M mergetool`, `esc close` lose their words); not fixed here.
- Bar strings (open text file, merge, no B/d), from `resolver_offers_file_keys`:
  - 80: ` o yours  t theirs  b both  ^O all yours  ^T all  O/T  e  n  N  ⏎  j/k  M  esc`
  - 120: ` o yours  t theirs  b both  ^O all yours  ^T all theirs  O/T file  e edit  n next  N prev  ⏎ done  j/k file  M  esc`
  - 200: ` o yours  t theirs  b both  ^O all yours  ^T all theirs  O/T file  e edit  n next  N prev  ⏎ done  j/k file  M mergetool  esc close`
- What each width lost against the same table before this ticket. `esc` is still drawn at every width.
  - 80: `^T all theirs` → `^T all`, and `e edit` → `e`.
  - 120: `M mergetool` → `M` and `esc close` → `esc`. The old 113-cell line fitted whole.
  - With `B` and `d` (the committed captures), 80: `^T all` → `^T`.
- Existing tests that pressed `j`/`k`/`↓`/`↑` in the resolver. Found with `grep -rn "Char('k')\|Char('j')\|KeyCode::Up\b\|KeyCode::Down\b" crates/graft/tests/it/resolver* crates/graft/src/update/resolver/tests* crates/graft/tests/it/conflict_interim*`:
  - `update/resolver/tests/file_list.rs:813` (`a_done_file_refuses_in_words`) asserted `top, 1, "j scrolls"`. That is a scroll, so it now presses `PgDn`. The expectation is re-derived as `41` (the window height less one row). Per D6 its `n`/`N` rows are removed, and `O`/`T` are added.
  - `it/resolver/file_list.rs:353` (`done_open_capture`) and `it/resolver/file_list/start_over.rs:27,34`: the comment says `k` "at the top scrolls nowhere and gives it back" (it clears the status row). That is a no-op scroll, so they now press `PgUp`. `done_start_over_capture` failed under `k`, because `k` now steps files. It passes with `PgUp`.
  - `it/resolver/file_list.rs:563` and `it/conflict_interim.rs:247,300` press these keys on the graph, not in the resolver. Not changed.
- Scripts and TESTING.md "Conflicts walk": none found. Commands: an `awk` extract of the section piped to `grep -nE '(^|[^a-zA-Z])(j|k|↓|↑|Up|Down)([^a-zA-Z]|$)'`, and `grep -rn "conflict\|resolver" scripts/*.sh`. No pty script drives the resolver.
- Regenerated captures (only the bar line changed in each): `fix-card-names-numbers-{120x40,80x24}`, `fix-file-list-many-120x40`, `fix-file-list-rebase-step2-120x40`, `fix-stop-lands-in-resolver-{80x24,120x40,187x50}`, `fix-take-all-120x40`. New: `fix-file-keys-{120x40,200x50}`. Both new ones are captioned in the TSV, and the contact sheet is rebuilt.
- Fixture: `update::tests::resolver_on_screen` now lists a second unmerged file, `later.rs`, so the bar sweep can press `j`. `resolver::tests::last_file_staged_from_the_pane` pins its own one-file status.
- The take keys moved out of `act` into `resolver::take_keys`, because clippy's cognitive-complexity check failed on `act`.
- **Mishap, reverted:** two `sed -i` edits hit the **main checkout** (`crates/graft/tests/it/resolver.rs`, `crates/graft/tests/it/resolver/file_list.rs`). I undid them at once with the inverse `sed`. I could not run `git` there to confirm it is clean; please check.
