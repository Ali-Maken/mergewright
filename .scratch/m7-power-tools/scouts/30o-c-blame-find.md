# Scout 30o-c: blame find (keymap §2.8) at HEAD f522b6f3

Paths relative to `crates/`. Keymap §2.8 (`docs/design/02-keymap.md:416-417`): "`/` Find in the file", "`n`/`N` Next / previous match, as the graph's search".

1. **`/`**: `graft/src/update/place/keys.rs:60` in `place_key`: `blame.find_prompt = Some(String::new())`. State lives on `BlameState`: `find_prompt: Option<String>` (the typing buffer) and `find: Option<String>` (the kept find). No separate struct, no cursor-in-field, no saved cursor.
2. **Typing**: `prompt_key` (`keys.rs:88-116`). **Not incremental.** Chars push (`:96`) and the cursor does not move until Enter. On Enter, the search starts at the current cursor (`at >= blame.cursor`, `:107`) and wraps to the first match (`:108`). Case folding is done by hand with `to_lowercase().contains` (`place/blame_state.rs:30-40`). It does **not** use `search::Query`.
3. **Keys**:
   - Enter (`:99-112`): an empty query closes the prompt and keeps no find. Otherwise it sets `find` and jumps.
   - Esc while typing (`:98`): `find_prompt = None`. There is nothing to restore because the cursor never moved. Any earlier kept `find` is untouched.
   - Esc with a kept find: `escape` (`:143-155`) runs `find.take()` and does nothing else. The next Esc clears a Failed/Sticky status, then runs `trail_back`.
   - n/N: `step_find` (`:120-140`) wraps both ways and does nothing without a find (`:121`).
   - Ctrl+U: **no handler**. A Ctrl char is excluded by the guard at `:96`, so it is ignored.
   - Backspace: `pop` (`:97`).
   - Paste: `update/mod.rs:1127-1131` routes to credential, then `name_prompt_paste` (`update/name_prompt.rs:293`, which only covers `state.name_prompt`), then finish, then commit_editor. **Blame find paste goes nowhere** and is dropped.
4. **No match**: the cursor does not move (`keys.rs:105-110`, only `if let Some`). The title tail reads ` · find "<q>" · no match` (`graft-ui/src/blame.rs:282`).
5. **`FindTitle`** (`graft-ui/src/blame.rs:117-126`) has fields `query: &str`, `at: usize` (1-based), `of: usize`. It is built in `blame_state.rs:66-74`, where `at` = the count of matches `<= cursor`, `.max(1)`. It is formatted in `fn title` (`blame.rs:270-290`) as ` BLAME · <path> · <rev>` plus either ` · find "<q>" <at> of <of>` or ` · find "<q>" · no match`.
   - FindTitle exists only for the kept `find`, so there is no difference between typing and kept in the title.
   - While typing, the bar row shows ` /<typed>` (sanitized, Text colour) in place of the hints (`graft/src/view/frame/overlays.rs:176-184`). **No cursor glyph** is drawn.
6. **Hints**: `blame_place_list` (`graft-ui/src/graph_screen/hints/lists.rs:206-223`).
   - No find: `⏎ that commit`, `^ before it`, `L history of these lines`, `/ find`, `space ages`, `w whitespace`, [`i ignore list`], `esc back`.
   - Kept find: the same, with `n N match` inserted after `/ find` (`:214-216`).
   - Without lines: `⏎ ^ L n N` are removed (`blame_bar`, `:228-234`).
   - Typing: no hint list at all, only the ` /<typed>` row (item 5).
   - Context comes from `graft/src/view/rows.rs:660-664` (`finding: blame.find.is_some()`).
7. **Wiring**: `text_field_open` (`graft/src/update/state.rs:806`) calls `place::prompt_open` (`graft/src/update/place.rs:436-441`, `find_prompt.is_some()`). `modal_owns_keys` (`state.rs:835-845`) includes `text_field_open()` and also `place::owns_keys` (`place.rs:431`, `trail.place_shown()`), so the place owns keys whether or not the prompt is typing.
8. **Tests**:
   - `graft/src/update/place/tests/keys.rs`: `slash_in_blame_finds_in_the_file_not_the_graph` (:6), `find_and_n_move_between_matches` (:66), `n_without_a_find_does_nothing` (:91), `esc_clears_the_find_before_popping` (:100).
   - `graft-ui/src/blame/tests.rs:350`: `find_title_counts_matches`.
   - Capture: `graft/tests/it/blame_place.rs:145` `capture_blame_find_187x50`.

**Contradictions/could not confirm**: none checked against a ticket (no ticket text was given). One mismatch with the keymap: §2.8 says n/N work "as the graph's search", but the blame find does not use `search::Query`. Whether the graph search folds case identically: *could not confirm* (not read).
