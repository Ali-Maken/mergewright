# Review — m6-conflicts ticket 16 (`Ctrl+O`/`Ctrl+T`: one side in every open block, one write)

Reviewed read-only at `d96144a1`. The commit's parent is `c08bb8a2`, so the diff read is `git show d96144a1` (36 files, +908 −133). The conductor's range `8be759cf..d96144a1` also shows doc-only commits from main, reversed; those are not this ticket's.
Gates: none rerun; the conductor ran them. The heavy tier and the release perf run were not run for this amend review. Every finding below rests on the lines it cites; none was observed running.

**Counts: Critical 0 · Major 3 · Minor 7.**

## What was verified, and how

- **Ticks: all 14 ticked items checked, by reading each test named against its criterion. 12 hold in full. 2 hold only with a gap the builder disclosed.**
  - graft-git (2/2 hold). `take_all_equals_taking_each_block` covers all 7 `MARKER_FILES` × 3 takes, checking the count and the render. `take_all_keeps_blocks_already_taken` checks the return of 2, block 0 still `Ours`, and `len == 3`.
  - graft-ops (2/2 hold). `take_all_never_clobbers_a_changed_file` checks `Changed { now == blob_id(edited) }` and that the bytes are unchanged. `apply_take_all_after_a_take_keeps_it` checks the render, `len == 3`, and the disk re-read.
  - graft update (8/8 hold). `ctrl_t_waits_behind_a_queued_take`, `a_bulk_take_is_one_queued_entry` (with its `[Block(1), Every]` → `2 queued takes dropped` half, which survives the re-read), `ctrl_t_with_nothing_open_writes_nothing`, `ctrl_t_notice_counts_the_blocks` (3, 2 and 1 blocks; cursor and `top` unchanged at the press), `ctrl_t_notice_waits_while_takes_are_queued`, `ctrl_t_on_a_card_is_one_choice` (text and binary), `ctrl_t_on_a_done_file_is_refused` (the can and cannot words), and `ctrl_alt_t_does_nothing`.
  - Real git (1, partial). `it::resolver::take_all::ctrl_t_takes_theirs_everywhere_then_enter_resolves` checks the bytes and that `ls-files -u` is empty. The "differ from `checkout --theirs`" clause is not exercised; this is disclosed (Minor 6).
  - graft-ui (1, partial). `resolver_offers_take_all` passes, but the criterion's short rung `^O/^T all` was not built (Major 2).
  - Capture (holds). `it::resolver::take_all::take_all_capture` pins `fix-take-all-120x40.txt`. The three headers read `took theirs · feature`, the notice line matches the words, and the bar shows `^T all theirs`. I did not judge the look; that is the user's call (rule 6).
- **D3 order holds** (`update/resolver.rs:318-340`). The done gate runs first: `done::refuses` includes control `o`/`t` without alt (`done.rs:69-79`). The `act` arms come next, ahead of the swallow. Inside `take_all_key` (`take.rs:92-135`) the order is: card, then `take_refusal`, then `pending_file`, then `open() == 0`, then `take_side`.
- **D1 holds.** `apply_take_all` (`graft-ops/src/conflict.rs:126-142`) clones the document, calls `take_all`, then makes one `write_back` against `file.fingerprint`. `Changed` maps to `TakeError::Changed`. No user data is at risk: the write is refused on a fingerprint mismatch, and the test above shows this.
- **D2 holds.** `TakeAt` is threaded through `in_flight`, `queued`, `TakeRequest.at`, `pending_file` (replays `Every` as `take_all`), `take_landed` (sends the front's `at`) and `jobs/conflict.rs:305-313`. `Every` is one entry against the cap.
- **No git work on the UI thread.** The write runs in the `TakeSide` job (`jobs/runner.rs:576-583`).
- **Escape sequences are sanitised.** The new notice carries a branch name and a path, and it goes through `sanitize_line` (`view/resolver.rs:74-77`).
- **No new `unwrap`/`expect` outside tests.** There is one new `unreachable!` (Minor 5).
- **No swallowed `Result` in the new code.** Two silent early returns remain:
  - `take_all_key`'s `None => return` carries a comment saying why.
  - `take_side`'s `file.clone() else return` was moved, not added.
- **Keys match `02-keymap.md`.**
  - §2.6 row `:263`: `Ctrl+O`/`Ctrl+T`, and on a whole-file card or a done file, refused in words.
  - §2.1 `:142`: bulk variants use a modifier.
  - `:31-32`: `O`/`T` stay the P6 exception and are not taken here.
- **`BOUND_ACTIONS` (`keys.rs:819-826`) and `update/tests.rs` map `^O`/`^T` to control presses.** So `every_key_the_bar_draws_is_answered` presses the drawn key.
- **Pipeline.**
  - The note has Need (`:225`, `:414`), Prior art (`:237` row 246, citing CR §1.5) and Journeys (`:268`, `:290`: `three-conflicts-all-theirs` P3:2).
  - The journey tick is correctly left open for ticket 21.
  - The conductor-owed note edits are present: `:323` and `:330`, and ticket 19 `:42`.
- **MAP.md** is updated for all four crates.

## Critical

None.

## Major

### 1. After an `Every` landing, the footer can show a stale count, contradicting the notice

- **Where.** `crates/graft/src/update/resolver/answers.rs:309-314`. Only a `Block` landing sets `state.status`, via the `side` returned by `landed_words` (`:242-248`). An `Every` landing leaves `state.status` as it was.
- **Sequence.**
  1. Press `o`, then `Ctrl+T` before the first write lands. This is the ticket's own queued case, D4's second bullet.
  2. `o` lands. Status becomes `Done("took yours · main in three.txt · 2 of 3 open")`.
  3. `Every` lands in a later update, with no key in between. Nothing clears the status: `Status::Done` is cleared only by a key (`update/keys.rs:83-85`), and the `Taken` arm (`update/mod.rs:239-243`) does not touch it.
- **What the user sees.** The resolver footer draws `Status::Done` in place of the hints (`view/frame/bars.rs:273-275` via `draw_status_or_hints`):
  - The footer reads `took yours · main in three.txt · 2 of 3 open ✓`.
  - The notice reads `took theirs · feature in 2 blocks · ⏎ marks three.txt done`.
  - The identity row reads `0 open`.
  - Three rows give two counts, and one of them is wrong.
- **Why it matters.** A wrong number is on screen until the next key, in the "keep this one, theirs for the rest" flow this feature exists for. The builder's NOTES say D4 "names only the notice". D4 does not say to leave an earlier landing's status standing.
- **Status:** read, not observed. `ctrl_t_waits_behind_a_queued_take` does not assert `status`.
- **Fix.** On an `Every` landing, either set `state.status` to the same `k of m open` form or reset a `Status::Done` to `Ready`. Add the assertion to `ctrl_t_waits_behind_a_queued_take`. This fixes the cause.

### 2. The graft-ui acceptance tick is `[x]` against a criterion it does not meet

- **Where.** Ticket `16-take-every-open-block.md:127`. The criterion reads "the short rung is `^O/^T all`". What was built is per-hint `^O all  ^T all`: `hints.rs:180-182` and `hints/tests.rs:1324-1330`. The builder disclosed this and flagged it for the conductor.
- **Why it matters.** A box ticked with an unmet clause reads as done in any roll-up. The approval quoted (P3, user, 2026-09-23) is for `^O/^T all`, not for the six new rows. Yet the six rows were added to `every_short_rung_is_the_users_own_word` (`hints/tests.rs:465-471`), a test whose name and MAP entry say it holds *the user's own table*.
- **Also wrong: "not expressible".** The builder's claim goes further than the code:
  - One `Hint { key: "^O/^T", verb: "all" }` is expressible; `j/k move` is exactly that shape (`keys.rs:725`).
  - What is not expressible is *two* full hints that collapse into *one* short hint. That is the real constraint, and a ruling on it is owed.
- **Fix.** Put the tick back to open, or mark it "deviation — conductor ruling pending". Then either:
  - the conductor rules per-hint `all` in, and the six test rows cite that ruling rather than P3; or
  - the spec's combined form is built as one `^O/^T` hint with its own full wording.

### 3. Two production files over 6.2's ~400 lines grew, and no size row names them

- **Where.**
  - `crates/graft/src/update/keys.rs`: 848 → 856 lines.
  - `crates/graft/src/update/events.rs`: 673 → 684 lines.
  - Counted with `git show <rev>:<path> | wc -l`, at `c08bb8a2` and at `d96144a1`.
- **Why it matters.** PROMPT.md 6.2 (`:436`) sets the limit, and readability is a hard rule. Review m6-amend-11-12 M2 filed the same pattern for three other files. The growth here is small and necessary: a table's rows and one enum. But neither file has a KNOWN_GAPS row, so the debt is invisible.
- **Fix.** Add one KNOWN_GAPS row naming both files and their counts, or split them. `BOUND_ACTIONS` could move to its own `keys/bound.rs`, and `TakeAt`/`TakeRequest` could sit with the resolver's own types.

## Minor

### 1. Key glyph: `^O`/`^T` breaks the on-screen `⌃` convention

- **The convention.** Every modifier the app draws uses `⌃`:
  - `hints.rs:75`, the `Hint.key` doc: `` (`m`, `⏎`, `⌃A`, `.`) ``.
  - `keys.rs:722-725`: `⌃A` for `Ctrl+A`.
  - `lists.rs:241`: `hint("⌃A", "all")`, the same verb as the new hints.
  - `commit_editor.rs:171`, `credential.rs:59`.
  - The design mockups: `01-view-mockups.md:25` `⌃P`, `:117` `⌃A stage all`, `:405` `⌃Z`.
- **Where `^` comes from.** It appears only in the resolver mockups (`01-view-mockups.md:173`, note `:661`, `:692`) and in the ticket's Scope 4. `02-keymap.md` spells keys in its tables as `Ctrl+O` and prescribes no screen glyph.
- **Assessment.** Textual drift in the spec, followed faithfully. The same bar now draws `⌃A all` on the changes pane and `^O all yours` in the resolver.
- **Fix.** Conductor ruling. Suggested: `⌃O`/`⌃T` in the hint, in `BOUND_ACTIONS` and in the three design lines.

### 2. The resolver bar lost verbs at 120 and at 80 columns (words only; the look is the user's call)

The seven regenerated captures and what each lost:

| Capture | Before → after |
|---|---|
| `fix-card-names-numbers-120x40` | `d start over` → `d`; `esc close` → `esc` |
| `fix-file-list-many-120x40` | the same |
| `fix-stop-lands-in-resolver-120x40` | the same |
| `fix-file-list-rebase-step2-120x40` | the same, **and** `M mergetool` → `M` |
| `fix-stop-lands-in-resolver-187x50` | nothing lost |
| `fix-card-names-numbers-80x24` | `B base`, `e edit`, `n next`, `N prev` → bare; `^T` at its short rung (`^T all`) |
| `fix-stop-lands-in-resolver-80x24` | the same |

- **Hints the design draws in full at 120.**
  - `esc close`: note `:164`, ticket 02's 120x40 mockup, `… M mergetool  esc close`.
  - The way-out verb: note `:692` / `01-view-mockups.md:190`, the 120x40 resolver bar, ends `e edit  esc banner` in full. That mockup draws only `^T all theirs` at 120, not both bulk hints.
- **Nothing is lost outright.** Every key is still drawn, so ux-principles H3 ("Esc leaves it", `ux-principles.md:53`) holds by key, though not by verb. No design line requires `d start over` in full on the *document* bar at 120. The done-file bar (`d start over  e edit  esc close`, note `:705`, `:831`) is a separate list, and 16 left it unchanged.
- **Cause.** The ladder raises hints left to right (`hints/tests.rs:1126-1129`), and the two long bulk hints sit fourth and fifth. The combined short rung of Major 2 would not change 120: the ladder raises `^O` in full before it reaches the tail.
- **Fix, if the user wants the tail back.** Order `^O`/`^T` after `⏎ resolved` in `resolver_list` (`lists.rs:264-265`), or draw one bulk hint at 120 as the mockup does.
- **Stale copies.** Three committed `m5-06-stop-lands-in-resolver-*` captures still show the pre-16 bar. The heavy walk writes them rather than checking them (`m5_walk/fixture.rs:143`), so they refresh on the next heavy run. TESTING.md `:2019` says only "the bar starting `o yours`", which still holds.

### 3. `landed_words` returns a pair of `Option`s, only one of which is ever `Some`

- **Where.** `answers.rs:227-250`: `(Option<(String, usize)>, Option<String>)`.
- **Also.** `map_or(0, …)` at `:238` stands in for "no document", which cannot happen while a take is in flight. That default would print `in 0 blocks`.
- **Fix.** Return an enum, `Landed::Every { side, blocks } | Landed::Block { side } | Landed::Nothing`, and name the impossible case rather than defaulting it.

### 4. `all_of` is a `const fn` matching role bytes, with a silent `_ => "all"` (`lists.rs:286-296`)

- **Risk.** A new role would draw `^O all` with no role word. `every_key_the_bar_draws_is_answered` would catch that, because `("^O", "all")` is not bound, so the risk is guarded.
- **Fix.** Plainer: give `SideRoles` `all_ours()`/`all_theirs()` returning the static verbs beside `ours()`/`theirs()`.

### 5. `take_all` uses `mem::replace` plus `let … else unreachable!` (`graft-git/src/conflict/document.rs:90-95`)

- **Assessment.** It follows `take`'s precedent (`:75-79`), but the panic path is avoidable.
- **Fix.** `*region = match mem::replace(region, Region::Plain(vec![])) { Region::Conflict(b) => { changed += 1; Region::Resolved { block: b, take } } other => other };`

### 6. The real-git test's oracle is `take_all` itself, and "differs from `checkout --theirs`" is unexercised

- **Where.** `tests/it/resolver/take_all.rs:9-14`.
- **Assessment.** It holds transitively, because graft-git's test checks `take_all` against per-block `take`. But the research §1.5 case (a clean hunk beside a conflicted one) is never built, and `three.txt` gives equal bytes. The builder disclosed this.
- **Fix.** Use or add a corpus file with a cleanly merged hunk next to a conflict, and assert that the result is not equal to `git checkout --theirs`.

### 7. `take_landed` is 61 lines (`answers.rs:255-315`), at 6.2's ~60

- **Fix.** Move the notice `match` (`:288-300`) into a helper beside `all_taken`.
