# app-resolver-whole-files-and-base: `o`/`t` on deleted/added/binary conflicts through git, `B` shows the merged base inside every open block, `d` starts a file over behind a confirm box, and the stash-pop conflict (no banner) walks the same path

Status: ready
Blocked by: **01**, **03**, **04**.
Build with: Sonnet — set by the conductor after the scout (`.scratch/m6-conflicts/scout-app-g05.md`); the keymap gate on line 13 is already met (`02-keymap.md:245-246`, `4dfa409`).
Crates: graft
Needs:
- `app-conflict-resolver` — `ResolverState`, `resolver_key`, `open_resolver`, `conflict_opened`, `resolver_view`, `Command::OpenConflict`, `Product::ConflictOpened`.
- `ops-conflict-operations` — `Operation::{ResolveWholeFile, RestartConflict}`, `conflict::{base_for, BaseError}`, `Refusal::NotMarkerShape`.
- `git-conflict-model` — `UnmergedHow::{words, has_markers}`, `ConflictFile`, `attach_base`.
- `ui-conflict-resolver` — `RegionView::WholeFile`, `BlockView.base`, `DocView.base_shown`, `HintContext::Resolver { base, restart }`.

**Spec change required before the build**: `B` (show/hide base, UPPERCASE = toggles a mode, §2.1) and `d` (start this file over — the discard verb) are Proposed changes 2(b) and 2(c) in the design note. The conductor adds them to `02-keymap.md` §2.6 first; until then the builder ships pieces 1 and 4 only and reports `blocked: keymap` for 2 and 3, and `HintContext::Resolver { base: false, restart: false }` stays.

## Scout first (Sonnet `Explore`, ≤ 1 page) — after G4 merges

- `crates/graft/src/update/resolver.rs` as built: where `ConflictOpened` fills `file`, where the notice is set, how `o`/`t`/`b` are dispatched (to add the whole-file branch before the block branch).
- `crates/graft/src/update/confirm.rs`: `ConfirmView::with_choices` / `ConfirmThen` variants and how the banner's `a` abort box is declared (copy that shape for `d`).
- `crates/graft/src/jobs/`: how G4 declared `OpenConflict`/`TakeSide` (copy for `LoadBase`).
- Report: the exact status-order the walk's stash-pop fixture leaves (`ls-files -u` paths) so the acceptance's "no banner" case uses a real path.

## Read first

- `docs/design-notes/m6-conflicts.md` — New UI "BASE", "Whole-file shapes", Hard-to-reverse choices 3 and 4, Proposed changes 2(b)(c).
- `docs/design/02-keymap.md` §2.6 after the conductor's edit; §2.1 (UPPERCASE toggles).
- `docs/design/06-undo-journal-design.md` §6.4 (`d` folds into the live entry; not undoable while live — the box says so).
- `crates/graft/MAP.md` §update::resolver, §update::confirm, §jobs, §view::resolver (after G4); `crates/graft-ops/MAP.md` §conflict, §operation (whole-file table); `crates/graft-ui/MAP.md` §conflict_resolver.
- `KNOWN_GAPS.md` row 229 (stash-pop conflict: the resolver owns unmerged-index state; no banner, no `in_progress`).

## Scope

In, in order:

1. **Whole-file shapes** — `ConflictOpened(Ok)` with `file: None` no longer refuses: the resolver shows `RegionView::WholeFile { how: <UnmergedHow::words()>, keys, lines: <the file's lines if it exists on disk, else empty> }`; `keys` per shape: `DeletedByThem` → `o keeps the file  t deletes it`; `DeletedByUs` → `o deletes it  t keeps theirs`; `AddedByUs` → `o keeps yours  t drops it`; `AddedByThem` → `o drops it  t keeps theirs`; `BothDeleted` → `o or t removes it`; binary `BothModified`/`BothAdded` → `o keeps ours  t keeps theirs`. `o`/`t` → `Command::Operate(Operation::ResolveWholeFile { path, side, how })` through the existing `operate` launch; `b` → `Sticky("no both for a <how> file · o ours  t theirs")`. On `Operated::Done`: the same `resolved_advance` as `⏎` (the operation also stages). A text `BothModified` reached here (file `Some`) never takes this branch — `NotMarkerShape` is `ops`' guard, asserted.
2. **`B` base** (`ResolverState.base_shown` toggle): first `B` on a file with `file: Some` pushes `Command::LoadBase(id, path, Box<ConflictFile>)` → job calls `base_for` → `Product::BaseLoaded(Result<(Arc<ConflictFile>, usize), String>)`; `Ok((file, n))` → `file` replaced (bases attached), `base_shown = true`, `Status::Done("base shown for <n> conflicts")`; `Err` → `base_shown` stays false, notice `(words, false)`: `no base (added on both sides)` for `NoBase`, `base unavailable after edits` for `Mismatch`, git's words for `Diff3`. Second `B` hides (no job); a later take keeps the attached bases (`take` does not drop `Block.base`); a re-read (`Changed`, hand-over) clears them and `base_shown` — the user presses `B` again. `HintContext::Resolver { base: true, .. }` once the key is bound.
3. **`d` start over** — confirm box: title `Start <path> over?`, lines `the conflict comes back with all its markers`, `takes and edits in this file are lost · not undoable while the <kind> is open`, command `git checkout --merge -- <path>`, `[ Cancel ] [ Start over ]`; `ConfirmThen::RestartConflict { path }` → `Operate(Operation::RestartConflict { path })`; `Operated::Done` → `OpenConflict` again (fresh read, `cursor_block = Some(0)`). Refused in words on a whole-file card when the file is not in the index on both sides (`ops` refuses `NotUnmerged` after `rm` — surface its words). `HintContext::Resolver { restart: true, .. }`.
4. **Stash-pop conflict path** (row 229): `open_resolver` with `in_progress == None` works; the explainer reads `ours = your working tree · theirs = the stash`; `⏎` on the last file closes to the pane with `Status::Done("all conflicts resolved")` and no `c continue`. `ResolveWholeFile` here is its own journal entry (G2) and shows in the journal row as `resolved <path> (ours)` with `← u`.
5. G4's `whole_file_refused_in_words_for_now` test is replaced by the tests below; MAP.md, NOTES.

Out: three-way text merge editing; a base column for taken blocks; rerere; anything on the banner.

## Signatures

```rust
// graft/src/update/mod.rs
pub enum Command { /* existing */ LoadBase(JobId, PathBuf, Box<ConflictFile>) }
pub enum Product { /* existing */ BaseLoaded(JobId, Result<(Arc<ConflictFile>, usize), String>) }
pub enum ConfirmThen { /* existing */ RestartConflict { path: PathBuf } }

// graft/src/update/resolver.rs (additions)
pub fn whole_file_keys(how: UnmergedHow, binary: bool) -> &'static str;
pub fn whole_file_take(state: &mut AppState, commands: &mut Vec<Command>, side: Side, next_id: &mut JobId) -> bool;
pub fn toggle_base(state: &mut AppState, commands: &mut Vec<Command>, next_id: &mut JobId) -> bool;
pub fn base_loaded(state: &mut AppState, id: JobId, result: Result<(Arc<ConflictFile>, usize), String>);
pub fn restart_box(state: &mut AppState) -> bool;   // opens the confirm; the confirm's accept pushes Operate(RestartConflict)
```

## Acceptance

`crates/graft/tests/it/resolver.rs`, fixtures `StoppedMerge` (all shapes) and m5's stash-pop conflict shape.

- [ ] `theirs-deleted.txt`: the card reads `deleted by them · o keeps the file  t deletes it`; `o` → `Operate(ResolveWholeFile { side: Ours, how: DeletedByThem })`; after `Operated::Done`, `ls-files -u` no longer lists it, the file exists, and the resolver has moved to the next conflicted file — `deleted_by_them_o_keeps`; `t` → the file is gone from the tree and index — `deleted_by_them_t_deletes`. One assertion per `(how, side)` row of the table, binary included — `whole_file_table_through_the_keys` (table test).
- [ ] `b` on a whole-file card → `Sticky("no both for a deleted by them file · o ours  t theirs")`, no command — `b_refused_on_whole_file`.
- [ ] `B` on `three.txt` → `LoadBase`; `BaseLoaded(Ok(_, 3))` → `resolver_view().base_shown == true` and every open `BlockView.base.is_some()`; the 120-column `layout` has `Side { Base }` rows; `B` again hides them with no job — `B_shows_and_hides_base`.
- [ ] `B` on `both-added.txt` (AA, text) → notice `no base (added on both sides)`, `base_shown == false` — `B_says_no_base`; after `e` hand-over and a re-read, `base_shown == false` and `B` reloads — `reread_drops_base`.
- [ ] `d` → the confirm box with the four lines above; `Start over` → `Operate(RestartConflict)`; `Operated::Done` → `OpenConflict` and the markers are back in the file (`open() == 1` on `both.txt` after a prior `o`) — `d_starts_over_behind_a_box`; `Cancel` → nothing, resolver unchanged — `d_cancel_is_free`.
- [ ] No journal line is added by `ResolveWholeFile` or `RestartConflict` during a live merge (entry count unchanged) — `whole_file_and_restart_fold_into_live_entry`.
- [ ] Stash-pop conflict: `⏎` on the `CONFLICTS` row opens with the explainer `ours = your working tree · theirs = the stash`; `o` then `⏎` → resolver closes, `Status::Done("all conflicts resolved")`, `ls-files -u` empty, journal's last row `resolved <path> (ours)` (whole-file) or unchanged (block take + `Stage`) — `stash_pop_conflict_resolves_without_a_banner`.
- [ ] `hint_context()` → `Resolver { base: true, restart: true }` once bound; `every_key_the_bar_draws_is_answered` and the app chain test pass — named in the report.
- [ ] MAP.md updated.

## Gates

fmt / clippy / nextest / doc, workspace-wide; `seam_rule` (the app builds no `GitInvocation`: `checkout --merge` is `ops`' invocation).

## Size

One context; ~400 lines. Reads G4's `resolver.rs` (it is the file being extended) and the scout page; nothing else in source beyond `confirm.rs`'s box shape.
