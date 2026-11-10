# Pre-publish cleanup: index

Started 2026-10-25 (user "yes"). Inputs: the 14 KNOWN_GAPS rows whose expiry says "pre-publish cleanup", and the user's rulings inside them. The Licence/release row in PROGRESS.md is the **next** step, not this one, so nothing here touches licence files, `cargo-dist`, the export allow-list file or the clean-start commit.

No design note: this is no user-facing feature. The one visible change (04 Part B: a preview lane name) follows ticket 17's decided rule (ADR-0064, mock :99).

Spec-only pass 2026-11-10: 11 split into 11b1–11b4 (old-generation code dropped), 11a and 11c. 12 rewritten to its scout. 07–10 cut from this pass.

Spec-only pass 2, same day: 11b2's first build stopped. With no marker, `names_in_force` answered the oldest names (`mergewright-git/src/repo/names.rs:37`), so settling was what kept new repos on the current names. 11b2 now also makes `names_in_force` always answer `CURRENT` (test-first) and deletes every move test, in all crates, in the same commit. It continues in its existing worktree. 11b3/11b4 are amended to match. No new ticket: the names change and the end of settling are only safe together.

Spec-only pass 3, same day: 11b3's build stopped at its fence. Its red test `journal_unmapped.rs` (branch `ppc-11b3` @ c9ce6cea) shows undo's behaviour today:
- A ref-less Stage entry still undoes, from its recorded oids.
- A pruned Discard entry fails only after appending a pending line and a backup ref.

The mapping never caused either. 11b3's fence is now a parity test pinning today's behaviour. Fixing undo is out of this pass, filed as a question.

## For the user

**Answered**
- **U1, 2026-10-25: DROP** (user: "drop it.....code contains nothing but current state of code"). 11b1–11b4 remove all code that moves or reads the old-name folders, refs, messages and env names. Standing rule: no migration or old-generation code for pre-public states.

**Before 11b2 merges, one thing only he can do:** open each of his repos once with today's `mwr`. That moves each repo's undo history and settings to the new folder while the moving code still exists. The conductor asks him and waits for "done" before merging 11b2.

**What dropping costs him** (tell him in these words; none of it needs an answer):
- A repo he hasn't opened since the rename keeps its undo history in the old folder, and the app won't see it.
- Undo of entries from before the rename behaves exactly as it does today; 11b3 pins that with a test. (Separately, an open question: undo of an entry whose saved state git has since cleaned away fails, but leaves a half-started line behind first.)
- Stashes and practice folders made by the old-name builds look like anyone else's: no special label, no automatic clean-up.
- His old config folder isn't read. If he changed settings since the rename, they're already in the new folder.
- Kept on purpose: the journal's "rename" entries in every repo he has opened. Removing that entry kind would make those journals unreadable and lose **all** his undo history, so it stays as a read-only kind with no old name in it (11b3 D2).

**Pending his confirmation:** 07–10 (the line-count ratchet) cut from this pass. Nothing here is blocked by them now.

**Checks only you can do** (no ticket; each needs his terminal and eye)
- After 05: in Ghostty, narrow the window until the graph is 3 columns or fewer (05 NOTES gives the exact size). The graph should still draw as text (`●`, `│`), never as a blank column. (gap `ease/29-narrow-graph-text`)
- After 05: in Ghostty (dark), open blame on a file that fails to blame (05 NOTES gives a way). The `⊗` should be red, the same red as in other panes. Captures can't show colour (rule 5). (gap `ease/26-blame-mark-uncoloured`)
- After 12: `docs/shots/pp-12-identity-review.md`. Every capture whose rows reordered or whose unnamed-lane colour changed when the fixture authors were renamed. He says whether any of them looks wrong.

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
| ~~07–10~~ | ~~ratchet-*~~ | rename/over400-raised: **cut from this pass, pending the user** | — | — | — |
| ~~11~~ | ~~codename-leftovers~~ | superseded by 11b1–11b4, 11a, 11c | — | — | — |
| 11b1 | current-names-and-guards | (step toward rename/graft-leftovers) | Sonnet | done (`scouts/11-codename-hits.md`) | 06 |
| 11b2 | old-gen-app-side (+ git `names_in_force` → `CURRENT`, + every move test deleted) | — | Sonnet; **continue in `.claude/worktrees/ppc-11b2`** | done | 11b1, **user opened each repo** |
| 11b3 | old-gen-ops-side (fence = parity test) | — | Sonnet; **continue on `ppc-11b3`** | done | 11b2 |
| 11b4 | old-gen-delete-table | — | Sonnet | done | 11b3 |
| 11a | codename-app-scripts-docs | rename/graft-leftovers (app, scripts, docs), rename/m3-pty-script | Sonnet | done | 06, 11b4 |
| 11c | codename-other-crates | rename/graft-leftovers (other crates, **ops included**) | Sonnet | done | 06, 11b4 |
| 12 | fixture-identity-rename | rename/graft-leftovers (identities; closes the row) | Sonnet; **runs alone, last** | done (`scouts/12-fixture-identity.md`) | 11a, 11c |

Coverage: 28 (01) · 17-fork-rows (02) · 19-100k (03) · 17-pane-focus, 17-preview-seeds (04) · 26-blame, 29-narrow (05) · 11b, 12b, 06i, 14-m3 (06) · graft-leftovers (11b1–4, 11a, 11c, 12) · m3-pty-script (11a). **over400-raised stays open** while 07–10 are cut.

**Fable tier: none.** No ticket meets all three tests.
- 11b's behavioural risks are fenced by tests written first: fresh repos write current names (11b2 D1); undo of old entries is unchanged (11b3 D3, parity). The `Migrate` kind is kept (11b3 D2).
- 12's bytes are judged by a script and a lane-slot probe, and what they flag goes to the user's eye.
- Debuggers: 2 (02, 03), per the brief: a flake gets its root cause, never a retry.

## Waves (max two cargo-running helpers, memory "two helpers at a time")

1. **01 alone.** Its numbers are wall times.
2. 02 ‖ 03. Both reproduce under load on purpose; neither times the fast tier.
3. 04 ‖ 05, then 06.
4. 11b1 → 11b2 (resumed in its worktree; merged after the user's "done") → 11b3 (resumed on its branch) → 11b4. Strictly in sequence:
   - 11b2 makes the names fixed and stops settling, together;
   - 11b3 then deletes the uncalled move and ports ops off the handle's names;
   - 11b4 deletes the table and the git read once nothing calls them.
5. 11a ‖ 11c. Disjoint files: 11a owns the app and capabilities crates, scripts and docs; 11c owns every other crate.
6. 12 alone. It closes the guard over the full shipped set.
7. Full audit (conductor).

Why this order:
- The fixes (01–05) land before the sweeps, so no sweep moves code a fix is editing.
- 11b goes before the renames: once the old-generation code is gone, nothing needs allow-listing (the user's ruling).
- 12 goes last, after 06 deletes stale captures, because it rewrites nearly every capture.

## Every builder

- Read CLAUDE.md first. Don't read PROGRESS.md or KNOWN_GAPS.md: each ticket quotes its rows.
- Worktree from **local main** (memory: agent worktrees base on origin/main), own `CARGO_TARGET_DIR` inside the worktree. Exceptions: 11b2 and 11b3 continue in their existing worktrees.
- Gates workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --all-targets --features heavy-tests -- -D warnings`, `cargo nextest run`, `cargo test --doc`, plus `scripts/walks.sh` when the ticket says so.
- **Split, never raise** an `over_400.txt` row (user, 2026-10-18).
- No AI-attribution wording anywhere.
