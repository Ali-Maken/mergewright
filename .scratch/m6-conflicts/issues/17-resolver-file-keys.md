# fix-resolver-file-keys: `O`/`T` settle the whole file in one key and move on; `j`/`k` move between files

Status: ready (filed 2026-09-23; P1–P8 approved by the user, P9 by the conductor — see docs/field-notes.md)
Blocked by: 16-take-every-open-block.md, 13-resolver-file-list.md, 15-take-advances-to-next-block.md
Was: draft · new 2026-09-23. **Gated on Proposed changes P6 (`O`/`T`) and P7 (`j`/`k` between files)**, per the note's "Where I'd differ". Build only the half that is approved. If neither is approved, do not file this ticket. The keymap rows go into `02-keymap.md` §2.6 before the binding lands; P6 also needs a named §2.1 exception, like the banner's `B`/`X`.
Build with: blank — brownfield (ADR-0035): **scout first**. Expected **Sonnet**: both keys chain functions other tickets built (`apply_take_all`, `Stage`, `resolved_advance`, `select_file`), and every rule is written here.
Needs: `fix-take-every-open-block` (`apply_take_all`, its command and answer path); `fix-resolver-file-list` (`select_file`, `ResolverState.files`/`file_cursor`, list-order advance); `fix-take-advances-to-next-block` (the take queue: `O`/`T` wait behind it).
Crates: graft-ui (`graph_screen::hints` Resolver table only), graft.

## Read first
- `docs/design-notes/m6-conflicts.md`, Amendment 2: Journeys J4, J5, J8, J9 (the `P6:` counts), "Where each event lands him", Where I'd differ P6 and P7, and New UI (the bars in both mockups).
- `docs/design/02-keymap.md` §2.1 (UPPERCASE rule; "Bulk variants use a modifier"), §2.2 (move selection), §2.6.
- `crates/graft/MAP.md` §`update::resolver` (`resolver_key`, `resolved_advance`, `select_file`, the take queue, the `still writing the last take` refusal); `crates/graft-ui/MAP.md` `graph_screen::hints` (`Resolver`, `short_form`, `BOUND_ACTIONS`).
- **Scout questions:**
  1. What `↑`/`↓`, `j`/`k`, `g`/`G`, `PgUp`/`PgDn` and `O`/`T` do in `resolver_key` today.
  2. How `⏎`'s `Stage` answer reaches `resolved_advance`.

## Scope
In:
1. **`O` / `T` (P6).**
   - On an open text file: send `apply_take_all(Ours | Theirs)`. When its answer lands, send `Stage` for the file; when that answer lands, go through `resolved_advance` (the next file not done, in list order, wrapping). `ResolverState.settling` holds the take until the stage is sent.
   - The notice reads `<file> done ✓ · ▸ <next>`, with the file's row in the list turned `done ✓`.
   - No open blocks left (every block taken) → only the `Stage` step.
   - On a whole-file card: exactly `o` / `t` (already one git write that marks the file done), then advance.
   - On a binary card: exactly the card's `o` / `t`.
   - On a `Done` file: `<file> is done · d starts it over`, and nothing is written.
   - Pressed while a take is in flight or queued: the key waits behind the queue (appended as one entry). A failed take drops it with P2's words, and nothing is staged.
   - `apply_take_all` returning `Changed`: re-read, no stage, notice `the file changed on disk · re-read · nothing staged`.
   - `AppState::choices` records the file as P3's `Ctrl+O`/`Ctrl+T` would, so the summary reads `theirs · feature in 2 of 2 conflicts`.
2. **`j` / `k`, `↓` / `↑` (P7):** `select_file(file_cursor ± 1)`, with no wrap at the ends. While a take is in flight: `still writing the last take`.
3. **Document scroll (P7):** `PgUp`/`PgDn` scroll the document a page, `g`/`G` go to its top and bottom, and the wheel is unchanged. `n`/`N` still move between blocks (P2), and the cursor block keeps scrolling itself into view.
4. **Hints.** The Resolver table gains `O/T file` after `^O/^T all` (P6) and `j/k file` after `⏎ done` (P7), per the mockups. Short rungs are `O/T` and `j/k`. Update `BOUND_ACTIONS`.
5. **TESTING.md** "Conflicts walk": each scenario that used `↑`/`↓` to scroll the document switches to `PgUp`/`PgDn`. List which ones in NOTES.

- **From ticket 25 (planner, 2026-09-24):** ~~moving between files walks the list's order (`listed_files`) and starts from a done file too.~~ Superseded 2026-09-24: `n` crossing in list order is ticket 15's (`cross_target`). This ticket keeps only `n` crossing *from* a done file.
Out: `Ctrl+O`/`Ctrl+T` themselves (P3's ticket); `c` in the resolver (P8, `fix-finish-summary-screen` Part B); any key outside the resolver.

## Signatures
```rust
// graft/src/update/resolver/mod.rs
pub struct ResolverState { /* existing */ pub settling: Option<Take> }   // an O/T whose take_all is in flight; Stage follows its answer
// resolver_key's signature unchanged; `select_file` from fix-resolver-file-list, unchanged
```

## Acceptance (name the verifying test when you tick; truth from git)
- [ ] `update::resolver::tests::shift_t_takes_theirs_marks_done_and_moves_on`: two-file fixture, on `notes.txt`, `T`. The bytes equal per-block theirs; `git ls-files -u notes.txt` is empty; the cursor file is `gone.txt`; the notice reads `notes.txt done ✓ · ▸ gone.txt`.
- [ ] `…::shift_o_on_a_whole_file_card_keeps_yours_and_moves_on`: `O` on `gone.txt` → the file exists with `HEAD`'s blob and is staged.
- [ ] `…::a_changed_file_is_not_staged`: the file is changed on disk between the key and the answer → nothing staged, and the notice says so.
- [ ] `…::shift_t_waits_behind_a_queued_take`.
- [ ] `…::j_and_k_move_between_files_without_wrapping`.
- [ ] `…::page_keys_scroll_the_document`.
- [ ] Journeys at their `P6:` counts in the acceptance walk (`one-file-one-conflict` 1, `three-conflicts-all-theirs` 1, `two-files-in-order` 2, `two-files-out-of-order` 3), once the conductor has moved those numbers into `keys`.
- [ ] Capture `docs/shots/m6/fix-file-keys-120x40.txt`: two-file fixture, `T` on `notes.txt`. **What right looks like:**
  - The strip reads `notes.txt · done ✓` and `▸ gone.txt · keep or delete`.
  - The identity row reads `▸ gone.txt · modified on yours · main, deleted on theirs · feature`.
  - The bar contains `O/T file` (and `j/k file` if P7).
  - The user judges by eye.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Small–medium: one chained key, one selection key, hint rows, ~6 tests, one capture. One context.
