# Scout 03 — ease-vocabulary

Paths relative to `crates/graft-ui/` unless they say `graft/`.

## State today
- `ContextId`: 22 variants, `src/keymap.rs:38-83` (Global…Menu). `scope()` :156, `borrows()` :190 (Preview|Finish|Rewrite borrow `confirm.yes`, :204), `stands_down()` :227 (Map/Stash/Journal only).
- `Token`: `TOKEN_COUNT = 18`, `src/theme/mod.rs:62`; `Token::ALL` :126; arrays sized by it in `theme/mod.rs:260,266`, `theme/chrome.rs:30,137`, `theme/overrides.rs:38`. `contrast_misses` `theme/overrides.rs:89`; contrast tests `theme/tests.rs:368,727,750`.
- `HintContext`: `src/graph_screen/hints/context.rs:47+`. Graph rows today: `WorkingTreeRow`, `BranchRow{current,verbs}`, `PlainRow{verbs}`, `StashRow`, wrappers `Marked`, `Searching`. Also `Banner{..}`, `Preview` (empty list, :97-99), `Resolver{..}`, `Finish{..}` (:192), `Picker`, `ConfirmBox`, etc. No commit-editor variant.

## Where each bar's words come from
- Graph: hint lists `graph_screen/hints/lists/rows.rs` (via `hints.rs`).
- Commit editor: **screen code**, `src/commit_editor.rs:178 footer_subject`, `:192 footer_other`, `:212 footer(focus)` — not a HintContext.
- Preview box: **screen code**, own border footer `src/preview_box/footer.rs` (`footer_label`, `preview_box.rs:28`); `HintContext::Preview` draws nothing.
- Resolver: `hints/lists/resolver.rs:76 resolver_list`. Finish: same file `:148 finish_list(focus, expandable)`.
- Names in words: e.g. `key_verb(&[ActionId(..)], "next")` in `graft/src/update/resolver/resolved.rs:44`; HintContext variants carry state as fields (`Banner{merge}`, `BranchRow{current}`); no variant today carries a `String` name — could not confirm any precedent for passing `<branch>`/`N` into a HintContext.

## Tests pinning counts/ids
- `ContextId::ALL` length: `keymap/tests/rules.rs:383` (`pinned.len() + 3 == ALL.len()`; pinned list :370-382 asserts each `scope()` and empty `stands_down()`); `rules.rs:36,104` iterate ALL; `keymap/clash.rs:21` (conflict check).
- Ids: `keymap/tests/table.rs:7-21` golden vs `tests/keymap_ids.txt` (sorted, one per line).
- Sweeps: `hints/catalogue.rs:42 every_context`, `:365 every_hint`; `hints/tests.rs:1360 every_context_lists_resolver`; `graph_screen/tests.rs:1446 every_hint_context`, `:2651 every_context_reaches_every_shape`; `hints/spell_sweep.rs`, `hints/tests/typed_fields.rs`. Literal-key guard `tests/no_literal_keys.rs`.
- Hint captures: snapshots in `src/graph_screen/snapshots/` and `tests/snapshots/` (not enumerated).

## Who reads the ids today
- `preview.*`: table `keymap/table.rs:205-231`; `preview.confirm_merge/_rebase` read in `graft/src/update/preview/keys.rs:178,182`; `preview.rewrite` in `graft/src/update/rewrite_editor/open.rs:15`; `⏎` hard-matched `preview/keys.rs:67`.
- Resolver keys are hard-coded `KeyCode` matches, `graft/src/update/resolver.rs:363-380` (`esc` :363 → `escape`, `c` :378 finish, `n/N` :372-373, `⏎` :374 mark resolved, `u` via `asks_undo` :379 = `app.undo`).
- Finish: hard-coded `graft/src/update/finish/keys.rs:100-101` (`y` commit, `⏎`/space `toggle_diff`); mouse `finish/mouse.rs:74`.
- Commit toggles: `Field::{Amend,SignOff,NoVerify,AllowEmpty}` focus rows, `src/commit_editor.rs:41-53`, handled in `graft/src/update/commit_editor.rs`; no action ids.

## Released config names?
- `CHANGELOG.md` has only `## [Unreleased]` (:9); no git tags. No release exists, so `finish.toggle_diff` was never released — but table.rs:11 and golden test demand an alias on any rename (ADR-0055 §2) regardless.
- Commit option ids (`commit.toggle_*`) do not exist at all.

## Contradictions with the ticket
1. `resolver.next` does not exist; the id is `resolver.next_block` (`table.rs:323`, golden :122).
2. `resolver.review` on `y` + `c` alias: today `c` is `resolver.finish` (`table.rs:325`). New id or rename of `resolver.finish` — ticket doesn't say.
3. `resolver.pause` "relabel of today's close": there is no resolver close action; `esc` is `app.back` (Global, `table.rs:77`) handled at `resolver.rs:363`.
4. `resolver.undo_pick` / `finish.undo_pick` `u`: `u` is `app.undo` (Global, `table.rs:78`), already answered in resolver (`resolver.rs:379`). A context row on `u` shadows Global — clash check `keymap/clash.rs:21` behaviour under shadowing could not confirm.
5. `finish.look` `⏎` "replaces toggle_diff": toggle_diff is on `⏎` **and** space (`table.rs:362-364`).
6. Q6 "other previews keep ⏎": `ContextId::Preview` is one context; per-kind scoping needs a new mechanism (none exists).
7. Commit editor has no HintContext; its bar is screen code — adding `CommitMessage/Body/Files` moves words out of `commit_editor.rs`.
