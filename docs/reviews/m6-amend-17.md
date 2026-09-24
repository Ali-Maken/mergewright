# Review — m6-conflicts ticket 17 (`O`/`T` settle a file, `j`/`k` step files)

Fixed hash `d887b884`; diff `78bd3978..d887b884` (two commits: `59a0de15` build, `d887b884` capture split).
Gates: run by the conductor, not here. Read-only; nothing committed.

**Counts:** Critical 1 · Major 1 · Minor 8.

**Tick spot-check:** 20 ticked boxes, all 20 checked against their named tests and files. 19 held; 1 failed
(Captures, part (b), the last bullet). Two boxes hold but assert weakly (Minor 4). The journeys box is correctly
left open for ticket 21.

**Keys against `02-keymap.md` §2.6 (:264–:268):** all match. `O`/`T` → `settle_key` (`update/resolver.rs:378-379`).
On a card, `O`/`T` act as `o`/`t` (`take.rs:152-158`). On a done file they are refused (`done.rs:78`). `n`/`N` on a
done file cross (`walk.rs:31-40`). `j`/`k`/`↓`/`↑` step between files (`resolver.rs:342-347`). `PgUp`/`PgDn`,
`g`/`G` and `Home`/`End` still scroll (`scroll.rs:131-143`).

**Can `settling` get stuck?** Not found. Traced from the code, not observed at runtime:
- A landing always passes through `settle_landed` (`answers.rs:321`).
- Every other path that clears `in_flight` goes through `drop_takes`, which also clears `settling`
  (`resolver.rs:245-247`). That covers a failed take, a `Changed` re-read (`answers.rs:198`) and `handed_back`
  (`answers.rs:375`).
- `take_side`'s early return when there is no document (`take.rs:208-210`) can't be hit from `settle_key`, because
  `pending_file` returns `None` first (`take.rs:167-169`, `:282`).
- If `mark_resolved` refuses at the landing (`resolved.rs:33-43`), `settling` has already been taken and the refusal
  words are shown, so nothing is left stuck.

**The two done-file captures moved from `k` to `PgUp`** (`it/resolver/file_list.rs:354`,
`file_list/start_over.rs:28,35`): they still show their scenarios. Their pinned files
(`fix-file-list-done-open-*`, `fix-done-start-over-*`) are byte-for-byte unchanged in the diff. The assertions
(`▸ gone.txt · done ✓`, bar `e edit  esc close`) are unchanged too.

**Main checkout after the builder's `sed` mishap:** `crates/graft/tests/it/resolver.rs` and
`…/resolver/file_list.rs` match `HEAD` (`git diff --quiet HEAD`, run 2026-09-24).

---

## Critical

### C1. False tick: the Captures box (b) says the card bar has no `O/T`/`j/k`, but neither after-frame draws a card bar
- **Where:** ticket `17-resolver-file-keys.md`, Captures box (b), last bullet ("The card bar has no `O/T`/`j/k`
  (D7)"). The captures are `docs/shots/m6/fix-file-keys-120x40.txt:40` and `fix-file-keys-200x50.txt:50`. The test
  is `crates/graft/tests/it/resolver/file_keys.rs:161-179`.
- **What's wrong:**
  - In both after-frames the bottom row is the Stage's status line, ` stage notes.txt ✓`, not a bar.
  - `file_keys_capture` asserts nothing about the bar after `T`. The 200×50 after-frame has no assertion at all,
    only `check_shot`.
  - The same commit's NOTES still say "The bottom row at the landing is also the Stage's status line … not the bar".
  - The TSV caption says so too (`m6-contact-sheet-captions.tsv`, the `fix-file-keys-120x40` row).
  - So the builder knew the bullet was not shown, and ticked it anyway.
- **What is true:** the behaviour itself is right. The card list has no `O/T`/`j/k`
  (`graft-ui/…/hints/tests.rs`, `resolver_offers_file_keys`, the `whole_file` branch). But that test is not the one
  the tick names, and the capture the user judges by eye does not show it.
- **Why it matters:** CLAUDE.md, "a false tick is Critical, whatever its size". The capture box exists so the user
  can see the card bar after `T`. The contact sheet shows him a status line instead.
- **Fix, either of:**
  - After `landed_on`, press a key that does nothing, as `done_open_capture` does (`file_list.rs:354`, `PgUp`), so
    the bar row comes back. Assert that `frame.last()` has no `O/T` or `j/k` and does have `o ` and `esc` at both
    widths. Regenerate both after-frames and the sheet.
  - Or have the conductor strike the bullet, and untick until one of the two is done.
  - Either way, add the same assertions (strip, identity row, notice) to the 200×50 after-frame.

## Major

### M1. Two files crossed ~400 lines and one function crossed ~60 lines (PROMPT.md 6.2)
- **Where:**
  - `crates/graft/src/update/resolver.rs`: 394 → 428 lines. `take_keys` was added at `:366-383`, and `settling` at
    `:199-204`.
  - `crates/graft/src/update/resolver/answers.rs`: 373 → 406 lines. `settle_landed` was added at `:324-348`.
  - `walk::walk_blocks` (`walk.rs:23-95`): 60 → 73 lines. The done-file branch was added at `:29-41`.
- **Why it matters:** these are hard readability rules, and this ticket is what pushed all three over. A clippy pass
  doesn't show this: `too_many_lines` counts statements, not lines.
- **Fix:**
  - Put `settle_key` and `settle_landed` together in a new child module, `update/resolver/settle.rs`. They are the
  two halves of one feature: the press and the landing.
  - Move `take_keys` into `take.rs`, which already owns every other take entry point.
  - Pull the done-file branch out of `walk_blocks` as `cross_from_done(state, commands, forward, next_id)`.
  - The suite count stays the same.

## Minor

1. **Stale NOTES contradict the tick.** In the ticket's `## NOTES (builder)`, the "Captures (resolved by conductor
   ruling)" bullet still carries "**The bar bullets cannot hold as specced** … Question for the conductor" next to
   "ticked". Fix: rewrite it to say what the after-frames show, or drop it once C1 is settled.
2. **The TSV caption is wrong for `fix-file-keys-before-120x40`.** It says the bar offers `O/T` and `j/k` "(bare at
   120)". The capture's bar (line 40) shows `O/T file` in full; only `j/k` is bare, and `⏎` is bare too. Fix: "`O/T
   file` full, `j/k` and `⏎` bare at 120".
3. **After an `O`/`T` landing, the notice offers `⏎` while the Stage is already running.**
   - `take_landed` sets `took <side> in N blocks · ⏎ marks <file> done` (`answers.rs:299-301`), or `all_taken`
     (`:304`). Then `settle_landed` launches the Stage in the same update (`:345-347`).
   - Until the Stage lands, the notice tells the user to press a key the app is already acting on. Pressing `⏎` then
     is refused by `write_refusal`.
   - Fix: skip the `⏎` suffix when `settling.is_some()`. For example, check `settling` before building `settled`.
4. **Two ticked tests assert weakly.**
   - `j_while_a_take_is_in_flight_is_refused` (`tests/file_keys.rs:238-248`) compares the status to `select_file`'s
     but never checks that it is a refusal. If both left the status unchanged, it would still pass. Fix: also assert
     `matches!(after.status, Status::Sticky { .. })`.
   - `page_keys_scroll_the_document` (`:251-282`) asserts `page > 1` and `G` > page, not "a page" (view height − 1)
     and "the bottom" (`max_top`). Fix: assert the exact values, as `a_done_file_refuses_in_words` does
     (`file_list.rs:816-821`, `41`).
5. **`Home`/`End` have no resolver test.** Keymap §2.6 now names them ("`Home`/`End` as `g`/`G`", conductor
   2026-09-24), and `scroll.rs:133,138` binds them, but no test presses them in the resolver. Fix: add them to
   `page_keys_scroll_the_document`.
6. **The code has small readability problems (6.2).**
   - `settle_key` maps `_ => KeyCode::Char('t')` (`take.rs:153-156`), so a `Take::Both` would silently become `t`.
     Match `Take::Theirs` and put `Take::Both` in an arm that says why it can't happen, or pass `KeyCode` in from
     `take_keys`.
   - `step_file` returns a `bool` that is always `true`, and its only caller ignores it (`files.rs:200-216`,
     `resolver.rs:343,346`). Drop the return.
   - `settle_key` returns `()` where the Signatures section says `-> bool`. That is harmless, but it isn't recorded.
   - `let Some(_) = open.settling.take() else` (`answers.rs:339`) would read more plainly as
     `if open.settling.take().is_none()`.
7. **The invariant is only written down, never checked.** `settling.is_some() ⇒ in_flight.is_some()` appears only
   in a doc comment (`resolver.rs:199-203`). Fix: add a `debug_assert!` at the end of `settle_key` and of
   `take_landed`.
8. **Some text is stale.**
   - The `walk.rs:1-11` module doc lists the three rules but not the done-file cross.
   - The `done_refusal` doc (`done.rs:53-54`) still says a ticket binding `O`/`T` "calls it first too", but they now
     sit in `refuses`.
   - The resolver's own notice for ⏎ still reads `no conflict markers · ⏎ marks it resolved` (`answers.rs:117`).
     The bar's `⏎` is now `done`, and CONTEXT.md **Done (file)** says to avoid "resolved". This is outside D7's
     table scope, so it may be a follow-up.
