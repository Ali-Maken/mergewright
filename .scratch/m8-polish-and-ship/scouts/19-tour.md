# Scout 19 — tour / practice repo

## What exists
- `Exit::Open(PathBuf)` — `crates/graft/src/event_loop/mod.rs:87`. No cursor, no return target.
- `drive` loop handles Open — `event_loop/places.rs:107-160`: on `Exit::Open(next)` stops pool (`stop_for_the_next_place`), `locate(&next, config)` (`places.rs:67`), loops. Survives re-entry: `Carried` (`places.rs:112`, keymap, problems, first_launch, notice, theme via `Carried::first`). `term` reused (no re-pairing). Comment `places.rs:109-111`: **every place lands on HEAD**.
- Recents: every `Place::Repo` calls `remember_opened` unconditionally (`places.rs:116`, `263`) → `recent::remember`.
- Header practice form exists: `HeaderView::practice: bool` (`graft-ui/src/graph_screen/header.rs:38`), glyph `◇` (`:42-43`), test `:282`.
- Keymap context `Tour` exists, binds nothing (`graft-ui/src/keymap.rs:77`). §2.18 (`docs/design/02-keymap.md:565`) claims only `Esc`; `t` on empty state is in the doc (`02-keymap.md:559`).
- Empty state: `EmptyStateView` (`graft-ui/src/empty_state.rs:102`), `bar_keys` (`:149`); doc comment `:17` mentions `t` tour, `:218` says tour clause joins when palette has a tour row. No `t` handler found in `graft/src/empty_state.rs` (grep).
- Palette: `AppAction { Theme(ThemeKind) }` only (`graft/src/update/palette/mod.rs:218`); listing `listing.rs:254`, run `run.rs:46`, empty state `graft/src/empty_state.rs:398`.
- Journal: `OperationKind` (`graft-core/src/journal.rs:32`): `Stage`, `Commit`, `StashPush/Apply/Pop`, `Push`, `Merge` (one entry across conflicts, `:57-59`), `Undo`/`Redo` with `reverses: Option<EntryId>` (`:304`). No branch-create kind confirmed in lines read. `Touches` (`:161`) = refs, index bool, worktree bool — **no hunk count**.
- Fixtures: `graft-fixtures::git::run/run_at/run_isolated` (`git.rs:88,144,165`) shell out to `git`; `scratch.rs` uses `tempfile::TempDir` (`:13,63-81`). graft-fixtures and tempfile are **dev-deps** of graft-ops (`Cargo.toml:27-34`).

## Contradictions
1. Ticket: Stage entry "covering one hunk" — `Touches` holds no hunk info (`journal.rs:161-169`); only `description` string. Could not confirm any field identifying one hunk.
2. "Leave → cursor on same oid": `drive` lands every place on HEAD (`places.rs:109`); `locate` takes only a path.
3. "Recents never remember practice": `remember_opened` runs for every repo (`places.rs:116`).
4. `build_practice_repo` in graft-ops runtime cannot use graft-fixtures/tempfile (dev-only, `Cargo.toml:27`).
5. Branch-create kind: could not confirm in `OperationKind`.
6. "Finished after the resolver" — Merge is one entry; distinguishing conflict-resolved merge from clean: could not confirm a field.

## Knock-ons
`main.rs:59` matches `Exit::Open(_)`; `places/tests.rs:208`; `exit_wait.rs:161`, `startup.rs:189` docs; palette app rows + empty-state `Edited::Run`; `bar_keys`; header callers (`header.rs:204`); keymap sweep test `update/tests/keymap_edge/sweep.rs` (mentions practice); empty-state captures in `docs/shots/` likely change with `t` (not enumerated).

## Gap M8/lands-row-launch-in-repo (`KNOWN_GAPS.md:1184`)
Known landing sites: `drive` first place + every `Exit::Open` (`places.rs:113-128`), `Place::Outside` → `leave_outside` (`:147`). Others in `startup.rs`: could not confirm.
