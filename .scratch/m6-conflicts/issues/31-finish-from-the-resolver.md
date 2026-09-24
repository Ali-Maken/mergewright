# fix-finish-from-the-resolver: `c` in the resolver finishes; the resolver stays open after the last file, with the words on its notice row

Status: ready (filed 31 by the conductor 2026-09-24) — was draft (new 2026-09-24, split out of `19-finish-summary-screen.md` Part B, ruling S8). **P8 is approved** (`02-keymap.md:269`, `:275`), so this isn't gated.
Blocked by: 19-finish-summary-screen.md (now 34 + 35, conductor split 2026-09-24) (`open_finish`, `ReturnTo`), 13-resolver-file-list.md (R4/R5 and their landing tests), 17-resolver-file-keys.md (same crate set, ADR-0036: not concurrent)
Build with: blank — brownfield (ADR-0035). `scout-19.md` item 8 covers `resolved_advance`'s tail. **Scout first** on what it doesn't cover: how `status_read::settle` writes `CHECKING` → `every_file_done` (MAP §`status_read`), and whether the notice row survives a re-read. Expected **Sonnet**: every rule is fixed below, and the tests judge it.
Needs: 19 (`open_finish(.., ReturnTo::Resolver, ..)`, `finish_key`'s `Esc`); 13 (`resolved_advance`, `every_file_done`, the tests named below); banner `continue_pressed`, `continue_refusal`.
Crates: graft-ui (`graph_screen::hints` Resolver table only), graft.

## Read first
- `docs/design-notes/m6-conflicts.md` Amendment 2: Journeys J4 and J6 (:447-460), "Where each event lands him" (:467-476), Where I'd differ P8 (:636-638).
- `docs/design/02-keymap.md` §2.6 :269 (`c`), :270 (`Enter`), :273 (`Esc`); §2.6b :323.
- `.scratch/m6-conflicts/issues/13-resolver-file-list.md` rulings R4, R5, Scope 7; `19-finish-summary-screen.md` S6, S8, and item 1.
- `.scratch/m6-conflicts/scout-19.md` items 1, 8, and "Resolver at finish".
- `crates/graft/MAP.md` §`update::resolver` (`resolved_advance`, `every_file_done`, `ResolverState::notice`), §`update::banner` (`continue_pressed`, `continue_refusal`), §`status_read` (`settle`, `CHECKING`).

## Scope
**In.**
1. **One `c` for both places.** `banner::continue_pressed` gains `from: ReturnTo` and is `pub(crate)`. The banner passes `Banner`; the resolver's `c` passes `Resolver`. The checks and their order are unchanged:
   1. The status read is owed → `CHECKING`.
   2. `continue_refusal`.
   3. A merge → `open_finish(.., from, ..)`.
   4. Otherwise → `resume(Continue)`.

   With `from == Resolver`, the words from steps 1 and 2 go on the resolver's notice row, not the status line. Step 2 reads `<continue_refusal's words> · ⏎ marks done`, for example `1 file still conflicted · ⏎ marks done` (design note :474).
2. **The resolver stays after the last file.** `resolved_advance`'s none-left branch (`resolved.rs:~110-135`) no longer closes the resolver or focuses the banner. It stays on the file just done (read-only, ticket 25), and the notice row reads `every_file_done`'s words:
   - `every file done ✓ · c finishes the merge` for a merge;
   - `every file done ✓ · c continues the rebase` (or `the cherry-pick`, `the revert`) otherwise.

   While a status read is out, the notice row reads `CHECKING`'s words, and `settle` writes the verdict to the notice row when the resolver is open (to the status line otherwise, as today).
   - **Stash pop keeps 13's close**: to the graph, with `every file done ✓` on the status line.
   - `Enter`'s row (`02-keymap.md:270`, "or close if none left") changes with this. The text for it is under "For the conductor".
3. **`Esc` from a summary opened here** → back to the resolver on the file it was on, the draft kept (19's `finish_key`, `ReturnTo::Resolver`).
4. **Hints.** The Resolver table (and its done-file list) gains `c finish` last before `esc`, short rung `c`. Add a `BOUND_ACTIONS` row.
5. **Tests to rewrite.**
   - 13's `after_the_last_file_the_banner_says_c_finishes` and `…_a_rebase_says_c_continues` are rewritten to the stay-open landing; the rebase version reads `… · c continues the rebase` on the notice row.
   - `after_the_last_stash_file_the_status_says_every_file_done` stays green, **unchanged**.
   - Remove the comment at `resolved.rs` that says "Ticket 19 Part B keeps the resolver open instead"; this is that change.
6. MAP.md: graft §`update::resolver` and §`update::banner`; graft-ui §hints.

**Out.** The summary itself (19). Any key the resolver doesn't already own. `c` in the banner (19).

## Signatures
```rust
// graft/src/update/banner.rs
pub(crate) fn continue_pressed(state: &mut AppState, commands: &mut Vec<Command>, from: ReturnTo, next_id: &mut impl FnMut() -> JobId) -> bool;
// graft/src/update/resolver/resolved.rs: resolved_advance's signature unchanged; its none-left branch changes (Scope 2)
```

## Acceptance (name the verifying test when you tick; truth from git)
- [ ] `update::resolver::tests::c_in_the_resolver_opens_the_summary`: merge, every file done → `finish` is `Some` with `return_to == Resolver`.
- [ ] `…::the_resolver_stays_after_the_last_file`: the resolver is open, and its notice row (not the status line) reads `every file done ✓ · c finishes the merge`.
- [ ] `…::c_with_files_open_is_refused_in_words`: notice row `1 file still conflicted · ⏎ marks done`, no command.
- [ ] `…::c_in_a_rebase_resolver_continues`: `Resume::Continue`, no `finish`.
- [ ] `…::esc_from_the_summary_returns_to_the_resolver`: the resolver is focused on the same path, and `finish_draft` holds the typed subject.
- [ ] `…::a_status_read_out_shows_checking_on_the_notice_row`, then the verdict once it settles.
- [ ] 13's two landing tests, rewritten (Scope 5); `after_the_last_stash_file_the_status_says_every_file_done` green and unchanged.
- [ ] `hints::tests::every_context_reaches_every_shape`, and `every_bound_action_is_hinted_live_or_is_named_here`, green.
- [ ] Journey `a-finish` = 2 from `every file done` in the resolver (`c ⏎`), counted by ticket 21's walk.
- [ ] Capture `docs/shots/m6/fix-finish-from-the-resolver-120x40.txt`: two-file merge, both files done. **What right looks like:**
  - The resolver is still drawn.
  - The notice row reads `every file done ✓ · c finishes the merge`.
  - The bar contains `c finish`.
  - The user judges by eye.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Small–medium: one branch rewritten, one key, one parameter threaded, words moved, ~8 tests, one capture. One context.

## For the conductor (exact text)
- `docs/design/02-keymap.md:270`, replace with:
  `| \`Enter\` | Mark this file done and move to the next conflicted file; after the last, stay on it with \`every file done ✓ · c finishes …\` (a stash pop closes to the graph) (P8) |`
