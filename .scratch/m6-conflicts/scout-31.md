# Scout 31 — finish from the resolver (main 293f2b0d)

Paths under `crates/graft/src/update/` unless noted. Ticket 36 may split `finish.rs` / `resolver/answers.rs`; re-find by fn name.

## What exists
- **`resolved_advance`** `resolver/resolved.rs:59`. Next-file branch :102-112 (`reopen`, notice `X done ✓ · ▸ Y`, `reread_set`). None-left tail :114-133: `state.resolver = None` :114; comment "Ticket 19 Part B keeps the resolver open instead" :121; `banner_focus = banner_view(..).is_some()` :122; `state.status` = `Busy(CHECKING)` if `status_read::owed` else `every_file_done` :127-131. No stash-specific branch: stash is distinguished only inside `every_file_done` (`in_progress` None → bare words).
- **`every_file_done`** `resolved.rs:136` → returns `Status::Done`, not a notice tuple. **Filters on `state.banner_focus`** :139-141; without it, no `c …` clause.
- **`status_read`**: `CHECKING = "checking the files"` `status_read.rs:25`; `owed` :33; `settle` :58 runs only if `checking(state)` :75 — i.e. `state.status == Busy(CHECKING)`; writes `state.status` = `every_file_done` or `banner::continue_refused(reason)`. Called only at `mod.rs:658` after an owed `WorkingTree(Ok)`.
- **Banner `c`**: `banner_key` `banner.rs:319` → private `continue_pressed` `banner.rs:395` returning `()`: owed→`Status::Busy(CHECKING)` :402; `continue_refusal` :201 → `continue_refused` :425 (`Sticky`, keys `RESOLVE_KEYS` = `M mergetool  e edit  space marks resolved` :215); merge → `open_finish(.., ReturnTo::Banner, ..)` :416; else `resume(Continue)`.
- **`open_finish`** `finish.rs:133`: `(state, commands, from: ReturnTo, next_id) -> bool`; false if a summary is open; takes `finish_draft`. **`ReturnTo`** `finish.rs:77` {`Banner`, `Resolver`}. `ReturnTo::Resolver` is constructed nowhere today.
- **Summary `Esc`** `finish.rs:581` `escape`: `close_keeping_draft`, sets `banner_focus` only for `Banner`. Does not touch `state.resolver`; key order `keys.rs:120-122` is finish → resolver → banner, so a still-open resolver gets keys back.
- **Resolver keys** `resolver.rs:299 resolver_key` / `act` :331. `Enter` → `mark_resolved` :358. **`c` is unbound** — falls to `_ if plain => scroll_key` :362. Done-file gate `resolver/done.rs:72 refuses` (Enter refused on done; `c` not listed).
- **Notice row**: `ResolverState::notice: Option<(String, bool)>` `resolver.rs:170`. Survives a re-read only if not a problem: `landing_notice` `answers.rs:131` (used `answers.rs:87`, `done.rs:134`). `conflict_set_read` `resolver/files.rs:~260-280` does not touch it. Cleared by scroll `scroll.rs:102`, take `take.rs:210`, restart `restart.rs:47`, walk `walk.rs:133/161`.
- **Hints** (graft-ui): `hints/lists.rs:22-33`; `resolver_list` :261, `resolver_card_list` :298, `resolver_done_list` :313 (`d`?, `e`, `esc`). `BOUND_ACTIONS` is `graft/src/update/keys/bound.rs:33` (graft crate).
- **Tests**: `resolver/tests/file_list.rs:434` `after_the_last_file_the_banner_says_c_finishes`, :445 `…_a_rebase_says_c_continues`, :468 stash one. Also `resolver/tests.rs:451 all_resolved_from_the_pane_offers_c_where_c_continues` asserts the close-to-banner landing (:459-461) — not named by 31.
- Keymap: :269 `c` and :270 `Enter` already carry P8 text (the "For the conductor" replacement is already applied).

## Contradictions with ticket 31
1. Signature says `continue_pressed -> bool`; code returns `()` `banner.rs:395`.
2. `every_file_done` needs `banner_focus` for the `c …` clause `resolved.rs:139-141`; with the resolver open and banner unfocused it yields bare `every file done ✓`. Returns `Status`, not notice.
3. `settle` detects only `state.status == Busy(CHECKING)` `status_read.rs:59,75`; a CHECKING on the notice row is never settled as written.
4. Refusal words: code's `continue_refused` appends `space marks resolved` keys `banner.rs:215,425`; ticket wants `· ⏎ marks done` on the notice row.
5. "Stays on the file just done (read-only, ticket 25)": none-left tail does no `reopen`, so no done-route read and `open.done` stays unset — could not confirm what the document then shows without running.
6. Unnamed test `resolver/tests.rs:451` also asserts the old landing; Scope 5 names only two.
7. Hints scope names Resolver table + done list; whole-file card list `lists.rs:298` unaddressed — question.
