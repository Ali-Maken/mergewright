# pre-publish: readability ratchet, part 2 of 4. Split the app `tests/` files the rename raised

Status: ready
Blocked by: 01, 02, 03, 04, 05 (02 edits `m7_walk` helpers, 03 the `frames` tests, 04/05 add app tests)
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: as 07: mechanical moves, judged by the compiler, tier counts and unchanged captures.
Scout: **yes**, the shared one (`.scratch/pre-publish-cleanup/scouts/ratchet-rows.md`, written before 07). Use this group's rows only. Over ~25 files → the conductor splits by directory (08a `m*_walk/`, 08b the rest).

## Closes (in part)
- `rename/over400-raised` (**broken**): "Rename raised 95 over_400 rows (+466 lines), all rustfmt reflow from longer names; user rule is split, not raise".
This ticket: the rows under `crates/mergewright/tests/`.

## Rules
**07's Rules 1–10 apply unchanged.** Read them in `07-ratchet-app-src.md`. Three weigh most here:
- **Rule 6 (tier):** most files here are walks (`m3_walk`, `m5_walk/*`, `m7_walk/*`, `tour_walk`) or pty tests (`pty.rs`, `quit_waits/*`, `outside_repo.rs`). A moved test stays under its parent module path, so `scripts/walks.sh --check` counts don't move.
- **Rule 7 (snapshots and captures):** many tests here write captures under `docs/shots/` by name. A capture's name must not change when its test moves. If a name derives from the module path, keep the path or keep the explicit name.
- **Rule 8 (scanners):** also check `tests/it/main.rs` (or whatever lists the `it` modules) and any test listing walk modules by path.

## Stop conditions
As 07. Plus: a shared test helper (`app_harness.rs`, `m5_walk/fixture.rs`, `support`) can only be split in a way that changes its public names. Report it before renaming any helper.

## Scope
In: this group's rows, new child modules, `crates/mergewright/tests/it/over_400.txt` (this block), `tests/it` module wiring.
Out: as 07.

## Signatures
None change.

## Acceptance (tick each with its test, inline)
- [ ] Every in-scope row ≤ its pre-rename ceiling, NOTES table as in 07 ← `file_size` green
- [ ] No row raised anywhere ← `git diff <start> -- crates/mergewright/tests/it/over_400.txt`
- [ ] Test counts per tier unchanged ← `scripts/walks.sh --check` before and after, quoted
- [ ] Walk tier green after the moves ← `scripts/walks.sh` SUMMARY quoted
- [ ] No capture name or content changed; snapshot moves are 100% renames ← `git diff --stat docs/shots` empty; `git diff -M --stat -- '*.snap'`
- [ ] Each new file passes the scanners ← `no_literal_keys`, `map_coverage`, `published_repo` green

## Touches (check each, say how)
As 07, plus: `tests/it/main.rs` module list, walk-module paths in `.config/nextest.toml` filters (read only; must still match), acceptance scripts selecting walk modules by path (`scripts/ease-accept.sh`, `m7-accept.sh`, `m8-accept.sh`: still select > 0, same count).

## Read first
- This ticket, `07-ratchet-app-src.md` § Rules, then the scout file (this group's rows).
- `crates/mergewright/MAP.md` § tests/it layout.
- `.config/nextest.toml` (the walk filter).

## Gates
Workspace-wide: the four CLAUDE.md gates, heavy clippy, `scripts/walks.sh` (in full: walks move here).

## Size
≤ ~25 files per context, ~70k tokens.
