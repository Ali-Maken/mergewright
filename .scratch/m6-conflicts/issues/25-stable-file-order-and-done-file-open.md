# fix-stable-file-order-and-done-file-open: a file keeps its row when it is done, and a done row opens read-only

Status: draft (spec-only pass 2026-09-24 on ticket 13's NOTES D1 and D2; conductor's rulings: D1 stable order, D2 spec stands)
Blocked by: 14-resolver-three-panes.md (same crates graft, graft-ui; ADR-0036: not concurrent). 13 merged.
Build with: blank — brownfield (ADR-0035): **scout first**. Expected **Opus**: an open route that decides on git's state, a new read-only resolver mode across keys, hints and the re-read paths. Every rule is fixed here, and tests can judge it. Not a Fable ticket.
Needs: 13's `listed_files`, `select_file`, `RowState`, `ResolverState::{files, seen}`; the existing `open_conflict`, `restart_landed`, `handed_back`, `HintContext::Resolver`.
Crates: graft-ops, graft, graft-ui.

## Why
- D1. Rows jump when a file becomes done. graft-ops lists unmerged paths first (`graft-ops/src/conflict/set.rs:28-30`), and 13's `listed_files` takes that order as it is. This is the "confusing which file" field note (2026-09-23).
- D2. The spec (13 Scope 6, the design note :61) says a `Done` row opens read-only. `open_conflict` refuses `NotUnmerged`, so 13 refuses the click in words instead.

## Read first
- `.scratch/m6-conflicts/issues/13-resolver-file-list.md`: Scope 2, 6, 7 and NOTES D1, D2.
- `docs/design-notes/m6-conflicts.md` Amendment 2: J4, "Where each event lands him", New UI "Below 180 columns" (:650–:675).
- `crates/graft-ops/MAP.md` §conflict (`open_conflict`, `OpenError`, `ConflictSet`), §operation `RestartParams` (why `d` already works on a path that is no longer unmerged).
- `crates/graft/MAP.md` §`update::resolver` (`ResolverState`, `listed_files`, `select_file`, `conflict_opened`, `handed_back`, `restart_landed`, `resolver_key`); `crates/graft-ui/MAP.md` `conflict_resolver` (`RegionView::WholeFile`), `graph_screen::hints` (`HintContext::Resolver`, `every_context`, `coverage_slot`).
- **Scout questions:**
  1. Which status does the `Command::OpenConflict` job pass to `open_conflict`: one it reads itself, or the app's copy?
  2. Which `Product` variant answers `OpenConflict`, and what `conflict_opened` does with `Refused(NotUnmerged)`.
  3. What `n`/`N` do today on a document with zero blocks.
  4. Every site that reads `ResolverState::whole_file` to decide whether the document is a card.

## Decisions
- **D1 order.** The list is sorted in **git's path order**: byte-wise over the path's bytes (`as_os_str().as_encoded_bytes()`). This is the order of git's index and of `status`. It is **not** `Path`'s `Ord`, which compares by component, so `a/b` sorts before `a.b` there, but after it in git. The sort is enforced **once, in `listed_files`**, over the whole union: the set's paths, the extra unmerged paths and the open path. Why there: only `listed_files` sees the extras, so a sort in graft-ops alone would still let an appended path move once the next re-read lists it. `ConflictSet::paths` is left alone, and no other reader relies on its order for display.
- **D2 document.** A done file shows the **working-tree file**, parsed like a reopened file. It is what his takes produced, what `e` opens and what a reopened file shows today. If the file on disk differs from what is staged, the identity row says so, read from the status and not guessed. A path resolved by removal, with nothing on disk, shows a card. A binary shows a card.
- **D2 route.** One graft-ops function decides between open and done, against a status **it reads itself**. The app never picks the route from its own status copy, which lags a `Stage` (13 NOTES, "Status lag"). Otherwise `restart_landed` would reopen a path that is unmerged again as done, and a click just after `⏎` would reach `open_conflict` and close the resolver.
- **D2 keys on a done file.**
  - `d` starts it over as today: the same box, then `RestartConflict`. `check_restart` already admits a path that is not unmerged, and `prepare_restart` reads its stages from the chain; `NoStagesKept` refuses in ops' words.
  - `n`/`N` cross files as on a card. `e` and `Esc` work as today.
  - `o` `t` `b` `⏎` `Ctrl+O` `Ctrl+T` `B` `M` → refused on the status line: `<path> is done · d starts it over`. No command is pushed.

## Scope
In:
1. graft-ops: `DoneFile`, `Opening`, `open_done`, `open_listed` (Signatures).
   - `open_done` refuses `Refused(Unmerged { paths: vec![path] })` when the status calls the path unmerged.
   - No file on disk → `Removed`.
   - A NUL in the first 8 KiB → `Binary`. Git's rule, as in `graft_git::conflict::read`.
   - Otherwise it parses (`graft_git::conflict::read`) → `Text`.
   - `unstaged` = the status shows a working-tree change for the path.
   - `open_listed` reads the status, then calls `open_conflict` for an unmerged path and `open_done` for any other.
2. graft job: `Command::OpenConflict` calls `open_listed`. Its product carries `Opening` instead of `Opened`.
3. `conflict_opened`:
   - `Opening::Unmerged` → today's path, with `done` cleared.
   - `Opening::Done` → `ResolverState::done = Some(..)`. `Text` sets `file`; `Binary` and `Removed` leave `file` and `whole_file` `None`.
   - The resolver never closes on a listed path.
   - `whole_file`'s invariant becomes: `Some` exactly when `file` is `None`, the read has landed and `done` is `None`. Update every reader from scout question 4.
4. `select_file`: drop 13's `Done` refusal (`<path> is already done`). The in-flight refusals stay.
5. Keys: the `D2 keys` rules above, in `resolver_key`, before any take, stage or base path.
6. Re-reads keep a done file read-only: `handed_back` (after `e`) goes through `OpenConflict` → `open_listed`. `restart_landed` → `Unmerged`, and `done` is cleared.
7. View (`view::resolver`):
   - Identity row: `▸ <path> · done ✓ · d starts it over`, plus ` · edited since done` when `unstaged`.
   - `Binary` / `Removed` draw `RegionView::WholeFile { how: "done ✓ · binary" | "done ✓ · removed", keys: "d starts it over", lines: [], note: None }`. At ≥180 columns this is whatever ticket 14 draws for a card.
   - The row keeps `done ✓` in the list, with `▸` on it.
8. graft-ui: `HintContext::Resolver` gains `done: bool`. When it is true, the table is `d start over`, `n next`, `N prev`, `e edit`, `esc close`, plus `j/k` only if P7's ticket has already bound them.
   - `resolver_context` sets `done` from `ResolverState::done`.
   - Add it to `every_context` and `coverage_slot`.
9. D1: `listed_files` sorts its union in git's path order. Row states, `file_cursor` and `resolved_advance` are unchanged; they read the sorted list.
10. MAP.md: graft-ops §conflict; graft §`update::resolver` (:314 "so a done file moves after the open ones" and "refused … for a `Done` row" become false); graft-ui `HintContext::Resolver`.

Out:
- A done file whose text still has markers, staged from the shell: it is shown as parsed, read-only, with no special words. Gap row.
- `ConflictSet::paths`' own order. A stash pop's done file dropping out at re-read (13's gap row stands). Showing the index blob in place of the disk file.

## Signatures
```rust
// graft-ops/src/conflict/done.rs (new; re-exported from `conflict`)
pub enum DoneFile {
    Text { file: graft_git::conflict::ConflictFile, unstaged: bool },
    Binary { unstaged: bool },
    Removed,
}
pub enum Opening { Unmerged(Opened), Done(DoneFile) }
pub fn open_done(repo: &RepoHandle, status: &WorkingTreeStatus, path: &Path) -> Result<DoneFile, OpenError>;
pub fn open_listed(repo: &RepoHandle, path: &Path) -> Result<Opening, OpenError>;   // reads the status itself
// OpenError gains: Status(<the error graft-git's status reader returns>): the builder names it from graft-git MAP
// open_conflict: signature unchanged

// graft/src/update/resolver (ResolverState)
pub struct ResolverState { /* existing */ pub done: Option<DoneShape> }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum DoneShape { Text { unstaged: bool }, Binary { unstaged: bool }, Removed }
// the Product answering Command::OpenConflict carries Result<graft_ops::conflict::Opening, …> (was Opened)

// graft-ui/src/graph_screen/hints/context.rs
pub enum HintContext { /* … */ Resolver { base: bool, restart: bool, whole_file: bool, done: bool, roles: SideRoles } }
```

## Acceptance (name the verifying test when you tick)
- [ ] graft-ops `conflict::tests::open_done_reads_the_file_on_disk`: throwaway merge, `notes.txt` resolved and `git add`ed → `Text`, `file.open() == 0`, `unstaged == false`. Edited after → `unstaged == true`.
- [ ] graft-ops `conflict::tests::open_done_on_a_removed_path_is_removed` (`DU` resolved by `git rm`) and `…::open_done_on_a_binary_is_binary`.
- [ ] graft-ops `conflict::tests::open_done_refuses_an_unmerged_path` → `Refused(Unmerged { paths: [p] })`.
- [ ] graft-ops `conflict::tests::open_listed_routes_on_gits_status`: the same path gives `Unmerged` before `git add` and `Done` after, with no status passed in.
- [ ] graft `update::resolver::tests::file_list::the_list_is_in_gits_byte_order`: `a/b`, `a.txt`, `B.txt`, `a.b` → `B.txt`, `a.b`, `a.txt`, `a/b`. `Path`'s order would put `a/b` before `a.b`.
- [ ] graft `…::file_list::a_file_keeps_its_row_when_it_is_done`: three files. The middle one becomes `Done`, and a set re-read in ops' order (unmerged first) lands. The row order and `file_cursor` are unchanged.
- [ ] graft `…::file_list::extra_paths_sort_into_place`: a `Ready` set missing an unmerged path, and an open path missing from both. Each sits at its sorted index, not at the end.
- [ ] graft `…::file_list::a_click_on_a_done_file_opens_it_read_only`: pushes `OpenConflict`. The `Done(Text)` answer → resolver open, `done == Some(Text { .. })`, identity row `▸ gone.txt · done ✓ · d starts it over`. With `unstaged`, it ends `· edited since done`.
- [ ] graft `…::file_list::a_done_card_for_removed_and_binary`: `Removed` / `Binary` answers draw `done ✓ · removed` / `done ✓ · binary` with `d starts it over`.
- [ ] graft `…::file_list::a_done_file_refuses_takes_in_words`: each of `o t b ⏎ Ctrl+O Ctrl+T B M` → status `gone.txt is done · d starts it over`, no command pushed.
- [ ] graft `…::file_list::d_on_a_done_file_starts_it_over`: `d` → today's restart box. Accepting it pushes `RestartConflict`. `restart_landed` → `OpenConflict`. An `Unmerged` answer → `done == None`, open blocks shown.
- [ ] graft `…::file_list::a_done_file_handed_back_stays_open`: `e` → hand-back → a `Done` answer. The resolver is still open, and `done` is still `Some`.
- [ ] graft `…::file_list::n_on_a_done_file_crosses_to_another_file` (scout question 3 decides whether this is a change or a pin).
- [ ] graft-ui `graph_screen::hints::tests::a_done_file_offers_start_over_and_moves`: `Resolver { done: true, .. }` → exactly `d`, `n`, `N`, `e`, `esc`. `every_context_reaches_every_shape` still passes.
- [ ] graft `it::resolver::file_list::a_done_file_stays_listed`, **back to the spec's order**. Two-file merge, `o` on `gone.txt` → the list reads `gone.txt · done ✓    ▸ notes.txt …`, with `gone.txt` in the same cell as before the `o`.
- [ ] graft `it::resolver::file_list::a_done_file_opens_read_only_and_d_starts_it_over`, real git:
  - After `o` on `gone.txt`, click its row. The resolver stays open, the identity row reads `▸ gone.txt · done ✓ · d starts it over`, and the row order is unchanged.
  - `d`, then accept the box. `git status` calls `gone.txt` unmerged again. Once the status lands, its row reads `keep or delete`, in the same cell.
- [ ] Captures, pinned by `it::resolver::file_list::file_list_capture` and a new `…::done_open_capture`: re-take `docs/shots/m6/fix-file-list-{120x40,160x48,80x24,200x50}.txt` and take new `fix-file-list-done-open-{120x40,200x50}.txt`. **The look is the user's call (rule 6); colour is unverified (rule 5).** What right looks like:
  - 120x40: `files   gone.txt · done ✓    ▸ notes.txt · 2 of 2 open`.
  - 80x24: `files   ✓ gone.txt   ▸ notes.txt 2/2 open`.
  - 200x50: in the column, `gone.txt   done ✓` comes above `▸ notes.txt  2 of 2 open`.
  - done-open 120x40: `files   ▸ gone.txt · done ✓    notes.txt · 2 of 2 open`, the rule, then `▸ gone.txt · done ✓ · d starts it over`, the file's plain lines, and a bar reading `d start over  n next  N prev  e edit  esc close`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium: one graft-ops function pair with a status read, one job payload, one resolver mode (keys, view, hints), one sort, ~17 tests, 6 captures. One context. If it will not fit, split at the crate seam: graft-ops first (the first 4 boxes), then graft and graft-ui.
