# M2 design pass: details and diffs (2026-09-15) — Tickets

[See also: m2-details-and-diffs/README.md](README.md) for the full section index.
Previous: [Genuinely risky tickets](07-genuinely-risky-tickets.md)

---

## Tickets

**Pass 1 (written):** `git-runner` (O) · `diff-parser` (S) · `diff-reads` (S) · `syntax-highlighting` (S) · `mistake-radar-detection` (F sig) · `detail-model-and-jobs` (O).

**Pass 2a (written):** `detail-pane-frame` (S) · `diff-widget` (S) · `side-by-side-view` (S) · `detail-keys-and-focus` (S) · `compare-two-commits` (S) · `context-actions-menu` (S).

**Pass 2b (written):** `radar-context-and-markers` (S: `first_parent_chain`/`previous_tips` in git, `RadarContext` job, `view::radar` facts + `findings_for`/`marker_for`/`radar_lines_for`, marker column, `[radar]` config, pane radar line rendering) ·
`radar-probe-evil-merge` (O: `probe_evil_merge` — `git show --remerge-diff --format= --name-only`, `Bounded` cache, unchecked/checking/clean/not-checkable) ·
`radar-probe-force-push` (O: remote-tracking reflog via `previous_tips`, `merge-base --is-ancestor` via `git-runner`, `NotCheckable` on an empty reflog) ·
`radar-probe-duplicates` (S: `log --all -n 500 -p` piped as stdin into `patch-id --stable` via `GitInvocation::stdin`, groups, "checked newest 500") ·
`explain-panel` (S: `ExplainView` from `explain`, the evidence sketch, `DetailState::explain_requested` consumed, graph highlight/fade, `Esc` priority).

**Pass 2c (written):**
`map-view-data` (S: `graft::map` — `MapData`/`build_map`, one date walk per local branch, default-chain exclusion, caps; `jobs::map`; `update::map` with `M`/`Esc`/`↑↓`/`Enter`/resize) ·
`map-view-screen` (S: `graft-ui::map_screen` — `MapView`/`MapScreen`/`ramp_glyph`; `view::map`; frame swap; captures) ·
`overview-strip` (S: `graft-ui::overview_strip` — `StripMarks`/`cell_for`/`row_for_cell`/widget; `view::strip`; `AppState::strip_shown` + `O`; the graph area's last column; click) ·
`refresh-reopens-walk` (S: `RepoStore::refresh -> Refreshed { table_rebound }`, walk restart re-anchored on the selection, `Written` footer hint; closes KG 54) ·
`docs-sweep` (S: bare `cargo run --` → `-p graft` in README/TESTING/clone script, KG closures and expiries, five new gap rows, README/CHANGELOG M2 entries) ·
`acceptance-details-and-radar` (F: `Harness::prelude`, gallery `--group details` with 19 stops, TESTING.md "Details and diffs", `docs/shots/m2-*` at 160×40/100×30/80×24, matrix; **two contexts by design** — (a) harness + group, (b) entries + captures).

**Spec-only pass (2026-09-17, written):** `working-tree-row-selectable` (S: `AppState::on_working_tree` + `uncommitted_row_shown`/`selected_spec`, `update::selection` rules, key/click arms, highlight; Needs `detail-model-and-jobs`; goes before `detail-pane-frame`, `compare-two-commits`, `context-actions-menu`, whose Needs now name it).

**Can't fit under 80k:** only `acceptance-details-and-radar`, and it says so with its seam; the conductor files part (b) when (a) hands off.

**Spec fix (2026-09-17, ADR-0028):** detail-pane resize keys are `<`/`>` (shrink/grow the pane on the focused divider), not `Ctrl+arrows`; the stored size is per orientation (`PaneSize { right_columns, below_rows }`), since a column count from `Right` means nothing as a row count in `Below`. Only `detail-keys-and-focus` changes.

**Spec fix (2026-09-17, working-tree row):** 06 shipped `DetailSpec::WorkingTree` with no way to select the row it belongs to (KNOWN_GAPS m2/06). Options: (a) defer to M3 — leaves a visible dead end in M2 (a row `Enter` ignores) and dead `WorkingTree` code, and M3's `space`/`d` on that row would need the same selection anyway; (b) make the virtual row a `History` row — shifts every row index; (c) a one-bit selection beside `History`'s cursor, M1's placement rule kept. **(c)**, one Sonnet ticket (`working-tree-row-selectable`), ~150 lines in `graft` only. Cost accepted: the row is reachable only while HEAD is the top visible history row (M1's rule), so with a newer branch above HEAD the user scrolls HEAD to the top first — a KNOWN_GAPS row for M3 to decide whether to pin the row above HEAD wherever it sits. Fable build count unchanged (1 + acceptance). Touched specs: `detail-pane-frame` (`UNCOMMITTED` header from `selected_spec()`), `compare-two-commits` (`x` no-op on the row; live compare keeps the base), `context-actions-menu` (`for_working_tree()`, `oid: Option<Oid>`), `acceptance-details-and-radar` (one TESTING.md entry, no gallery stop).

**Spec fix (2026-09-18, `?` help): deferred to M8, no new ticket.** 10b's group-2 line "`?` help" was a slip of mine: it named a screen this note never designed, and two builders confirmed no help surface exists. Options: (a) build DESIGN §1.16 now as a hand-typed key list per screen; (b) M8. **(b)**: PROMPT §8 already puts "keybinding help and remapping" in M8, and §1.16 defines the overlay as the §2 keymap table *filtered to the focused pane* — that table is the remapping data structure M8 builds, so (a) is a throwaway list rewritten in M8, in a read-only milestone already at 25 tickets with two partials. Cost: until M8 the keys the 80-column footer can't fit (`Space` `1` `H` `x` `w W v [ ]` `<>` `L` `M` `O`) are learned from README/TESTING.md; the acceptance ticket's TESTING.md entries carry them. Rule while the gap stands: **no footer or mockup-derived hint shows `? help`** (none does today, checked against `docs/shots/`, README, TESTING.md); `?` stays unbound rather than bound to nothing. Touched: `detail-keys-and-focus`/10b's `?` line closes as deferred; KNOWN_GAPS rows 199 ("M1, with the help overlay") and m2/10b re-owned to M8. For M8's designer: `crates/graft/src/update/` has a purity guard that rejects any mention of ratatui or `graft_ui` types in that module, doc comments included (10b hit it) — the overlay's geometry and widget live in `graft-ui` (`help_overlay`, sibling of `map_screen`); `update` holds only a `help_shown: bool`-shaped state and the focused-pane id, and the keymap table itself sits in `graft-core` or `graft-ui`, never in `update`.
