# pre-publish: readability ratchet, part 1 of 4. Split the app `src/` files the rename raised

Status: ready
Blocked by: 01, 02, 03, 04, 05 (they edit app files. A sweep that moves code goes after the fixes that edit it)
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: mechanical moves of whole items into child modules. The compiler, the test counts and an unchanged capture set judge it.
Scout: **yes**, one shared by 07–10, run before 07. It writes `.scratch/pre-publish-cleanup/scouts/ratchet-rows.md`:
- the rows the rename raised, by crate and directory, each with: pre-rename ceiling, ceiling after the rename, current ceiling, target, and one candidate seam per file (a group of tests sharing a subject or fixture; a type with its impls; a helper family; a `mod tests` block not yet in its own file);
- the scanners that pick files by name or path (see Rules).
Method: `git show 507c52e2:<over_400 path then>` (the rename's base; paths then under `crates/graft*`; find the file with `git ls-tree -r --name-only 507c52e2 | grep over_400`). Map `graft` → `mergewright` in each path, then compare with `git show 34861856:crates/mergewright/tests/it/over_400.txt` (gates after the rename) and with current main.
If this group's share is over ~25 files, the conductor splits it by directory (07a/07b) before dispatch.

## Closes (in part; 07–10 together close it)
- `rename/over400-raised` (**broken**): "Rename raised 95 over_400 rows (+466 lines), all rustfmt reflow from longer names; user rule is split, not raise".
This ticket: the rows under `crates/mergewright/src/`.

## Rules (07–10 share these; 08–10 point here)
1. **In scope:** each row the scout lists for this group. **Target:** the file's current length ≤ its pre-rename ceiling. A row already at or under the target needs nothing; list it in the NOTES table.
2. **Split, never raise** (user, 2026-10-18). Move one coherent group into a child module named for what it holds: tests sharing a fixture or subject, a type and its impls, a helper family. No `part2.rs`, no numbered files, no file split mid-item. Prefer one meaningful split that gets the file well under its target, or under 400 so the row goes, over trimming to the exact number.
3. **Not allowed:** rustfmt config changes, joining lines, deleting comments, blank lines or docs to make the number, `#[rustfmt::skip]`, raising **any** row (in scope or not), adding a row for a new file over 400.
4. **Allowed:** deleting code proven dead (compiler warning or grep shown in NOTES), listed per file.
5. **Pub items moved in `src`** are re-exported from the old path, so no caller changes. Their MAP.md line is updated (`map_coverage`).
6. **Tests keep their tier.** The walk tier is chosen by module path (`*_walk`, pty). A moved test stays under the same parent module path. `scripts/walks.sh --check` counts per tier are the same before and after.
7. **Snapshots keep their content.** insta names snapshot files by module path. Prefer moving tests without `assert_snapshot!`. If one moves, its `.snap` is renamed with identical content (`git diff -M` at 100%), and no `.snap.new` is committed.
8. **Scanners that pick files by name or path.** Check each before naming a new file:
   - `no_literal_keys` (it skips `*_tests.rs` only: the 07a gate failure);
   - `map_coverage`;
   - `published_repo`;
   - the hint and caption scanners;
   - any test that lists source paths (e.g. 06t's `stop_ladder_readers.rs:57` path list).
   The scout names them; NOTES says how each new file passes.
9. **`over_400.txt`:** lower each row to the file's new length, and remove the row when the file is ≤400. Edit only this group's block of sorted rows, so 07–10 merge as separate hunks.
10. **Size guard:** after ~25 files, or if context runs short, commit at a file boundary, update the NOTES table, and report `partial` with the remaining rows.

## Stop conditions (report, don't work around)
- A file has no coherent seam (one item longer than the target). List it with its biggest item; the planner rules (it may be a real readability defect).
- A move changes a capture's bytes, a test count per tier, or a public path: report what and why.
- A row in scope was raised again by later work, so its current ceiling is above even the post-rename one: still target the pre-rename ceiling, and note it.

## Scope
In: the files in this group's rows, their new child modules, `crates/mergewright/tests/it/over_400.txt` (this block), MAP.md lines.
Out: behaviour; other groups' rows; renaming anything (11, 12); rows not raised by the rename (`ease/06-over-400`, `ease/07b-ratchet-raised`, `M8/59-over-400-grew` keep their own expiries; lowering one in passing is fine, targeting it isn't).

## Signatures
None change. Public items keep their paths through re-exports.

## Acceptance (tick each with its test, inline)
- [ ] Every in-scope row ≤ its pre-rename ceiling. NOTES table: path, pre, after-rename, before this ticket, now, new files ← `file_size` (tests/it/file_size.rs) green
- [ ] No row raised anywhere: the `over_400.txt` diff has only decreases and removals ← `git diff <start> -- crates/mergewright/tests/it/over_400.txt` quoted in summary form
- [ ] Test counts per tier unchanged ← `scripts/walks.sh --check` before and after, both lines quoted
- [ ] No capture changed, and snapshot moves are renames at 100% ← `git diff --stat docs/shots` empty; `git diff -M --stat -- '*.snap'`
- [ ] Each new file passes the scanners (rule 8) ← `no_literal_keys`, `map_coverage`, `published_repo` by name, green
- [ ] Dead code deleted, if any, with its proof ← NOTES

## Touches (check each, say how)
- `over_400.txt`: 07–10 share it, by block.
- MAP.md for moved public items.
- The scanners in rule 8.
- Walk partition, insta snapshot names, captures.
- Callers of moved pub items (re-export: none change, compiler shows).
- Keymap, journal, undo: none.

## Read first
- This ticket, then the scout file (your group's rows only).
- `crates/mergewright/MAP.md` for the directories in your rows.
- PROMPT.md §6.2 (readability).
- Each file as you split it, in outline first (`grep -n '^\s*\(pub \)\?\(fn\|mod\|struct\|enum\|impl\)'`), not whole.

## Gates
Workspace-wide: the four CLAUDE.md gates, heavy clippy, `scripts/walks.sh --check`. Run `scripts/walks.sh` in full once before hand-in if any walk-module test moved.

## Size
≤ ~25 files per context, ~70k tokens. The scout's count decides the a/b split.
