# G-palette 2 — `⌃P` finds branches, tags, files, commits and what you ran

Status: ready
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Opus**: sources and their `⏎` are decided here; the
widget exists.
Blocked by: 11
Needs: `palette-help-and-actions` (`PaletteView`, `RowKind`, `fuzzy_score`, `update::palette`).

## Read first
- `docs/design/01-view-mockups.md` §1.6; `docs/design/02-keymap.md` §2.12
- `docs/design-notes/m8-polish-and-ship.md`: Prior art "Search by name and by key"; this spec's decisions below
- `crates/graft/MAP.md`: `search` (the `/` index), `update::picker` (`⇥ show it on the graph`, seek), `update::place`
  (opening `F` history), `refs`; `crates/graft-ui/MAP.md`: `palette`

## Decisions (planner)
| Source | Rows | `⏎` |
|---|---|---|
| Branches, tags | local, remote, tags from the resident ref snapshot | move the graph cursor to its tip (the picker's `⇥` path); never switches — `C` does that |
| Commits | the `/` search over laid-out history (subject, sha prefix, author) | move the graph cursor to it |
| Files | tracked paths, read once per open (`git ls-files -z`, a job; `reading…` row until it lands) | open the file's history place (`F`) |
| Recent | the last 8 palette actions run this session, newest first, in memory | run it again, as from the palette (its key's path, preview included) |

Ranking: match quality, then the order above, then recency. At most 8 rows per non-action group, `+N more` row that
widens the group on `⏎`. Empty query: actions only (no flood of refs).

## Scope
In: the four sources, ranking, `+N more`, a file-list read job (cancelled when the palette closes), stale-generation
drop. Out: switching branches from the palette (that is `C`); persistent recent list across runs.

## Signatures
```rust
// graft (pub(crate)) — update::palette
pub(crate) enum PaletteTarget { Action(ActionId), App(AppAction), Seek(Oid), FileHistory(PathBuf), Again(ActionId) }
pub(crate) fn sources(state: &AppState, query: &str) -> Vec<(SanitizedText, Vec<(PaletteRow, PaletteTarget)>)>;
// graft-git (or graft-ops reads): ls-files read for the job — the scout names the nearest existing read
```

## Acceptance
- [ ] `⌃P` `feat` `⏎` on a branch row → cursor on its tip, branch unchanged (test reads HEAD with git).
- [ ] Commit row by sha prefix and by subject word → cursor on it (tests).
- [ ] File row → history place open on that path (test).
- [ ] Recent: run `stash` from the palette, reopen → `RECENT` first group, `⏎` opens the stash prompt again (test).
- [ ] 50k refs fixture: typing a key in the palette updates within one frame budget in release (script →
      `docs/perf/m8-palette.txt`, summary only; no fast-tier timing assert).
- [ ] Capture `docs/shots/m8-palette-find-120x36.txt` (query `rebase`, the §1.6 drawing's groups). **Colour unverified**.

## Touches
- `search` index reuse (no second index); ref snapshot; job pool (file read, cancel); history place opener; picker's
  seek path; MAP.md graft, graft-ui.

## Scout questions
- Can the `/` search be asked for matches without moving the cursor (file:line)? Is there an existing ls-files read?
- Where is the picker's `⇥` seek implemented, and does it handle a tip not yet laid out?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~55k tokens.
