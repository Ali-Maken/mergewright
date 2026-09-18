# m1-the-graph/11 — Ref badges and ahead/behind

Status: done
Build with: **Opus** (high) — git reads with edge cases (detached, unborn, diverged) off the UI thread
Blocked by: 09
Leaves runnable: yes — branches, remotes, tags, HEAD and stashes show on their commits.

Why: badges are what answer "what is going on here".

## Read first
- PROMPT.md §1.3 (ref badges paragraph only)
- docs/design/01-view-mockups.md §1.1 (badges in the wide mockup) and docs/design/08-architecture.md §8.2 (grep `Branches`)
- KNOWN_GAPS.md M0 row "`ui.icons` has no consumer"
- Code: `crates/graft-git/src/branches/`, 10's row widget, the M0 job runner's public API

## Scope
In:
- Badges for local branches, remote branches, tags, `HEAD` and stashes. In sync → one merged badge (`main ⇄ origin`); otherwise `↑2 ↓1`.
- Ahead/behind computed off the UI thread, cached per (local tip, upstream tip), recomputed when either moves.
- `ui.icons` consumed: badges and node glyphs honour it. Closes the M0 gap.
- Badges truncate before the subject disappears at 80 columns.
- Add a corpus shape only if a badge case below has none (e.g. a stash).

Out:
- 50k-ref scale (30). "Contained in" (M2).

## Acceptance
- [x] One test each: in sync, ahead only, behind only, diverged, detached HEAD, tag-only commit, stash, unborn branch. `crates/graft/src/refs/tests.rs` (pure, one per state) + `graft-git` `history::divergence::tests` (counting) + `repo::tests::ahead_behind_counts_the_ref_states_branches` (real fixture).
- [x] A branch name containing an escape sequence renders inert. `view::tests::a_branch_name_with_an_escape_sequence_renders_inert`.
- [x] `ui.icons` changes the rendering — snapshot pair. Config is `plain`/`nerd`, not a bool. `graph_screen::tests::ui_icons_changes_how_badges_render` (`badges-icons-plain`/`-nerd`).
- [x] Ahead/behind never runs on the UI thread (asserted through the job runner). `jobs::tests::ahead_behind_is_counted_on_a_worker_never_the_loops_thread`.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.

## Notes (m1-11 build)

- Built on Opus. **Compacted once** mid-ticket (after code + gates, before capture/commit); token budget (~80k) exceeded. Nothing left undone.
- Shape: `Command::ReadRefs` job → `refs::read` (peeled refs, HEAD, `RepoHandle::upstream` per branch) → `Product::Refs(RefBadges)` → `AppState::refs` → `view::badge_view` sanitises → `graft_ui::graph_screen::Badge`. Counts via `graft_git::history::ahead_behind`, cached per (local tip, upstream tip) in the runner's `AheadBehindCache`.
- HEAD node gap closed: the row `HEAD` resolves to draws `◆`; newest stash `■`.
- Badges take at most half the subject column, cut with `…` (`badges_are_cut_before_the_subject_disappears_at_80_columns`).
- **Not done, needs user OK:** node glyphs ignore `ui.icons` (Nerd commit glyphs risk double width, breaking lanes). KNOWN_GAPS.
- Only the newest stash entry is badged (no reflog read). KNOWN_GAPS.
- Visual: pty capture `docs/shots/11-ref-badges.txt` (this repo, ref-states 80/120 plain + 120 nerd, stash, detached-head). User judges by eye; Nerd glyphs show blank in the text dump.
- Fixture oddity: `ref_states` builder ends on an orphan `fresh` branch, but the built fixture's `HEAD` is `refs/heads/main`. Tests don't depend on it. KNOWN_GAPS.
