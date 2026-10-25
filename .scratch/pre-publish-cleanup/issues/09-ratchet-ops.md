# pre-publish: readability ratchet, part 3 of 4. Split the ops files the rename raised

Status: ready
Blocked by: 01, 02, 03 (01 may fix ops test code; 02's helper may be in fixtures/ops)
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: as 07. Ops holds undo and journal code, but moving whole items changes no behaviour, and the compiler and tests judge it. A split that needs anything beyond moving items is a stop.
Scout: **yes**, the shared one (`.scratch/pre-publish-cleanup/scouts/ratchet-rows.md`). Use this group's rows only. Over ~25 files → the conductor splits into 09a (`src/`) and 09b (`tests/it/`).

## Closes (in part)
- `rename/over400-raised` (**broken**): "Rename raised 95 over_400 rows (+466 lines), all rustfmt reflow from longer names; user rule is split, not raise".
This ticket: the rows under `crates/mergewright-ops/`.

## Rules
**07's Rules 1–10 apply unchanged** (`07-ratchet-app-src.md`). Ops-specific:
- **Undo, journal, apply, restore, rewrite** (`undo.rs`, `journal*.rs`, `apply/*`, `restore/steps.rs`, `operation/history.rs`): move items whole, and never reorder statements inside a function. If the only seam would split a function or widen visibility beyond `pub(crate)`/`pub(super)`, stop.
- The ops MAP.md gets a line for any public item moved (re-exported from its old path).

## Stop conditions
As 07. Plus: a split in undo or journal code would change a type's or function's visibility to `pub`. Report it.

## Scope
In: this group's rows, new child modules, `crates/mergewright/tests/it/over_400.txt` (this block), `crates/mergewright-ops/MAP.md`.
Out: as 07.

## Signatures
None change.

## Acceptance (tick each with its test, inline)
- [ ] Every in-scope row ≤ its pre-rename ceiling, NOTES table as in 07 ← `file_size` green
- [ ] No row raised anywhere ← `git diff <start> -- crates/mergewright/tests/it/over_400.txt`
- [ ] Test counts per tier unchanged ← `scripts/walks.sh --check` before and after
- [ ] No capture or snapshot content changed ← `git diff --stat docs/shots` empty; `git diff -M --stat -- '*.snap'`
- [ ] Public paths unchanged (re-exports), MAP lines updated ← `map_coverage` green; the app crate compiles unchanged (`git diff --stat crates/mergewright/src` empty)
- [ ] Each new file passes the scanners ← `no_literal_keys`, `map_coverage`, `published_repo` green

## Touches (check each, say how)
As 07, plus: callers of ops in `mergewright` (re-exports keep them unchanged, the compiler shows), `git-matrix.sh` filters naming ops test modules (`rewrite_preview::`, `cherry_pick::`: still select the same tests), ops heavy tests.

## Read first
- This ticket, `07-ratchet-app-src.md` § Rules, then the scout file (this group's rows).
- `crates/mergewright-ops/MAP.md` for the directories in your rows.

## Gates
Workspace-wide: the four CLAUDE.md gates, heavy clippy, `scripts/walks.sh --check`.

## Size
≤ ~25 files per context, ~70k tokens.
