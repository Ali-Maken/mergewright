# Pre-publish cleanup: index

Started 2026-10-25 (user "yes"). Inputs: the 14 KNOWN_GAPS rows whose expiry says "pre-publish cleanup", and the user's rulings inside them. The Licence/release row in PROGRESS.md is the **next** step, not this one, so nothing here touches licence files, `cargo-dist`, the export allow-list file or the clean-start commit.

No design note: this is no user-facing feature. The one visible change (04 Part B: a preview lane name) follows ticket 17's decided rule (ADR-0064, mock :99).

## For the user: one question, two checks

**Question**
- **U1. Code that moves the old-name folders.** Builds from before the rename kept each repo's undo history and settings in a folder named after the old project, inside that repo's `.git`. The app moves that folder to the new name the first time it opens the repo. The moving code has to spell the old name, so the old name would be in the public code. Your other rule says the public repo must not carry it. Which one wins?
  - **keep** (recommended): any repo you haven't opened since the rename still gets its undo history moved. The old name shows in a few lines of public code, each marked as old-folder handling.
  - **drop**: open each of your repos once first. After that, the code and the old name go.
  - Until you answer, ticket 11 allow-lists those lines (reason `old-generation`). "Drop" becomes a follow-up ticket.

**Checks only you can do** (no ticket; each needs his terminal and eye)
- After 05: in Ghostty, narrow the window until the graph is 3 columns or fewer (05 NOTES gives the exact size). The graph should still draw as text (`●`, `│`), never as a blank column. (gap `ease/29-narrow-graph-text`)
- After 05: in Ghostty (dark), open blame on a file that fails to blame (05 NOTES gives a way). The `⊗` should be red, the same red as in other panes. Captures can't show colour (rule 5). (gap `ease/26-blame-mark-uncoloured`)

**Rulings applied, not asked**
- The m3 contact sheet is deleted, not rebuilt (06). It's the same class as the m6/m8 sheets he ruled on ("delete, not rebuild", 2026-10-19).
- Fixture authors get neutral names of the same length, with no product name in them (12), so a later rename can't touch them again.

## Groups

| # | Group | Gap rows it closes | Build with | Scout first | Blocked by |
|---|---|---|---|---|---|
| 01 | fast-tier-what-grew | ease/28-fast-tier-over-budget | Sonnet, **solo** | no (measurement decides where to look) | none |
| 02 | fork-rows-flake | ease/17-fork-rows-flake (+ ease/06h-fork-rows-flaky if same cause) | **debugger** (Opus) | no (cause is the ticket) | none |
| 03 | 100k-unicode-flake | ease/19-100k-unicode-flake | **debugger** (Opus) | no (cause is the ticket) | none |
| 04 | lane-identity-leftovers | ease/17-pane-focus-by-identity, ease/17-preview-seeds-upstream | Sonnet | **yes** (`scouts/04-lane-identity.md`) | none |
| 05 | small-ui-leftovers | ease/26-blame-mark-uncoloured, ease/29-narrow-graph-text (test half) | Sonnet | **yes** (`scouts/05-blame-ink-narrow-graph.md`) | none |
| 06 | stale-sheets-and-shots | ease/11b-three-pane-shots, ease/12b-m6-sheet-stale, ease/06i-m8-sheets, ease/14-m3-sheet-stale | Sonnet | no (docs only, files named, drift script checks) | none |
| 07 | ratchet-app-src | rename/over400-raised (app `src/` rows) | Sonnet | **yes**, shared 07–10 (`scouts/ratchet-rows.md`) | 01, 02, 03, 04, 05 |
| 08 | ratchet-app-tests | rename/over400-raised (app `tests/` rows) | Sonnet | shared | 01, 02, 03, 04, 05 |
| 09 | ratchet-ops | rename/over400-raised (ops rows) | Sonnet | shared | 01, 02, 03 |
| 10 | ratchet-other-crates | rename/over400-raised (ui, git, graph, term, fixtures, core, capabilities rows) | Sonnet | shared | 01, 02, 03, 05 |
| 11 | codename-leftovers | rename/graft-leftovers (all but fixture identities), rename/m3-pty-script | Sonnet | **yes** (`scouts/11-codename-hits.md`) | 06, 07, 08, 09, 10 |
| 12 | fixture-identity-rename | rename/graft-leftovers (fixture identities) | Sonnet; **runs alone, last** | **yes** (`scouts/12-fixture-identity.md`) | 11 |

All 14 rows are covered: 28 (01) · 17-fork-rows (02) · 19-100k (03) · 17-pane-focus, 17-preview-seeds (04) · 26-blame, 29-narrow (05) · 11b, 12b, 06i, 14-m3 (06) · over400-raised (07–10) · graft-leftovers (11+12) · m3-pty-script (11).

**Fable tier: none.** No ticket meets all three tests. 12 touches every capture, but a normalisation script judges it, so correctness rests on neither the diff nor the eye. Debuggers: 2 (02, 03), per the brief: a flake gets its root cause, never a retry.

## Waves (max two cargo-running helpers, memory "two helpers at a time")

1. **01 alone.** Its numbers are wall times.
2. 02 ‖ 03. Both reproduce under load on purpose; neither times the fast tier.
3. 04 ‖ 05, then 06. Scouts 04/05 can run during wave 2 (they don't run cargo).
4. Shared ratchet scout, then 07 ‖ 09, then 08 ‖ 10. All four edit `crates/mergewright/tests/it/over_400.txt`, each in its own contiguous block of sorted rows. The conductor merges the hunks. If the scout gives any group more than ~25 files, split it by directory (a/b) first.
5. 11 (scout first).
6. 12 alone.

Why this order: the fixes (01–05) land before the sweeps, so no sweep moves code a fix is editing. The ratchet goes before the renames: 11/12 lengthen strings, so they'd raise rows again. Each of them splits what it pushes over, and a split is cheaper on files the ratchet has already cut. 12 rewrites nearly every capture, so it goes last, after 06 deletes the stale ones.

## Every builder

- Read CLAUDE.md first. Don't read PROGRESS.md or KNOWN_GAPS.md: each ticket quotes its rows.
- Worktree from **local main** (memory: agent worktrees base on origin/main), own `CARGO_TARGET_DIR` inside the worktree.
- Gates workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --all-targets --features heavy-tests -- -D warnings`, `cargo nextest run`, `cargo test --doc`, plus `scripts/walks.sh` when the ticket says so.
- **Split, never raise** an `over_400.txt` row (user, 2026-10-18).
- No AI-attribution wording anywhere.
