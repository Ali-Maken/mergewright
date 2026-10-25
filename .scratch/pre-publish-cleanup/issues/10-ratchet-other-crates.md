# pre-publish: readability ratchet, part 4 of 4. Split the ui, git, graph, term, fixtures, core and capabilities files the rename raised

Status: ready
Blocked by: 01, 02, 03, 05 (05 edits ui `blame`; 02 may edit fixtures helpers; 01 may fix test code anywhere)
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: as 07.
Scout: **yes**, the shared one (`.scratch/pre-publish-cleanup/scouts/ratchet-rows.md`). Use this group's rows only. Over ~25 files → the conductor splits into 10a (`mergewright-ui`) and 10b (the rest).

## Closes (in part)
- `rename/over400-raised` (**broken**): "Rename raised 95 over_400 rows (+466 lines), all rustfmt reflow from longer names; user rule is split, not raise".
This ticket: the rows under `crates/mergewright-{ui,git,graph,term,fixtures,core,capabilities}/`. 07–10 together close the row.

## Rules
**07's Rules 1–10 apply unchanged** (`07-ratchet-app-src.md`). Crate notes:
- **ui:** `graph_screen/hints/tests.rs` is the hints sweep that dominated the fast tier (23, 24). Moving tests doesn't change their cost; don't touch the sweep logic.
- **graph:** raster tests and their insta snapshots (`it__raster__*.snap`). Rule 7 matters most here.
- **term:** Kitty and iTerm2 test files. Move whole tests; byte fixtures stay byte-identical.
- **fixtures:** `capture.rs`, `team.rs`, `remote/tests.rs`. A moved generator item must not change the fixture cache key or any generated object. Fixture determinism tests stay green, and no oid changes (12 does that, on purpose, later).

## Stop conditions
As 07. Plus: a fixtures split changes any generated oid or the cache key. Report it.

## Scope
In: this group's rows, new child modules, `crates/mergewright/tests/it/over_400.txt` (this block), the MAP.md of each crate touched.
Out: as 07.

## Signatures
None change.

## Acceptance (tick each with its test, inline)
- [ ] Every in-scope row ≤ its pre-rename ceiling, NOTES table as in 07 ← `file_size` green
- [ ] No row raised anywhere ← `git diff <start> -- crates/mergewright/tests/it/over_400.txt`
- [ ] Test counts per tier unchanged ← `scripts/walks.sh --check` before and after
- [ ] No capture, PNG or snapshot content changed ← `git diff --stat docs/shots` empty; `git diff -M --stat -- '*.snap'`
- [ ] Fixture objects unchanged ← fixtures determinism test by name green; no oid in any snapshot changed (the snapshot diff above)
- [ ] Each new file passes the scanners; MAP lines for moved public items ← `no_literal_keys`, `map_coverage`, `published_repo` green

## Touches (check each, say how)
As 07, plus: fixture cache key, insta snapshot names in graph, `pane_boundaries` test binary (ui `tests/`), each crate's MAP.md.

## Read first
- This ticket, `07-ratchet-app-src.md` § Rules, then the scout file (this group's rows).
- MAP.md of each crate in your rows.

## Gates
Workspace-wide: the four CLAUDE.md gates, heavy clippy, `scripts/walks.sh --check`.

## Size
≤ ~25 files per context, ~70k tokens.
