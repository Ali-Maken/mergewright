# rebase-preview-keeps-drops — `R` in a rebase preview opens the rewrite editor with the suggested drops marked

Status: draft
Blocked by: 05-rewrite-preview, 08-rewrite-editor
Crates: graft only (`update/preview.rs`, the rewrite editor's open path, JOBS for the new `RewriteRequest` arm).
Build with: Opus. Seeds 08's editor from a rebase plan (a new entry into an established overlay); walk and capture
judge it. Scout first (ADR-0035): brownfield across 05's rows and 08's overlay.
Needs: 05 (`AlreadyIn`, `RebaseParams::drops`, `line`), 08 (`open_rewrite`, `GraphOverlay::Rewrite`), 09
(`Command::PlanRewrite`, `RewriteRequest`).

## Why
Scout 05: the note's `R to keep them` had no keymap slot. Decided (planner, 2026-09-27): `R` keeps its one meaning,
"open the rewrite editor" (§2.1 UPPERCASE opens a place; §2.3 `R`). From a rebase preview it opens the editor on that
same rebase, drops pre-marked `d`, so he sees what he keeps before `y`. No new letter.

## Decided
- Before the build the conductor applies the keymap row written in `05-rewrite-preview.md` ("Keymap row") to
  `docs/design/02-keymap.md` §2.3b.
- `R` in a rebase preview whose plan has `already_in: Some(_)` → `Command::PlanRewrite(RewriteRequest::FromRebase(
  params))` → `GraphOverlay::Rewrite`, rows = the rebase's todo (every branch in `line`), base row = `onto`, the
  `AlreadyIn` commits marked `d`. `p` on one keeps it (08's key). `y` starts; `Esc` closes to the graph, nothing
  written. Any other preview: `R` is not bound and not offered.
- 05's row gains ` · R to keep them` exactly when `R` is live (same predicate as the binding).
- Hint bar in that preview: `R keep` after `y`.

## Signatures
```rust
// graft (JOBS)
RewriteRequest::FromRebase(RebaseParams)            // new arm on 09's enum; → Msg::RewritePlanned as Open
```

## Acceptance
- [ ] `update/preview/tests.rs`: `r_in_a_squash_merged_preview_opens_the_editor_with_drops_marked` (red first);
      `r_is_unbound_in_other_previews`; `the_row_names_r_only_when_live`.
- [ ] Walk `m7_walk/rewrite.rs::keep_a_suggested_drop` = `r R p y`: the kept commit is on the rebased branch; `u`
      restores both refs.
- [ ] The three hint-bar tests green.
- [ ] Capture re-taken: `docs/shots/m7/rebase-squash-merged-preview-120x40.txt`. **Right:** the row ends
      `· dropped · R to keep them`; `R keep` on the bar. New: `docs/shots/m7/rebase-keep-drops-editor-120x40.txt`.
      **Right:** the three matched rows show `✗ drop`, subject muted (colour **unverified**), base row `─ main`.

## Touches
- 05's captures and TESTING.md row for the squash-merged preview (caption changes). 08's editor open path (a second
  entry). JOBS. HINTS (bar). Journal/undo via the engine (walk step). Keymap §2.3b.

## Size
Small-medium (~35k).
