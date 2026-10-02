# G-daily 2 — reword HEAD with dependents, forks follow, stash rows refuse, honest counts

Status: partial (3, 5, 6 done; 1, 4 ops half, wiring blocked; 2 split off — see NOTES)
Build with: Opus (conductor, after scout `scouts/14-daily-ops.md`; file set adds graft `update/clash.rs` for item 4 wording)
Blocked by: 13
Needs: none in code. Crates graft-ops (rewrite, put-back words, reflog fold), graft-git (divergence), graft
`update/preview/history.rs` only — disjoint from `daily-landing`'s files.

## Read first
- `docs/adr/0047-rewrite-engine-and-dependent-branches.md` — Decision section only
- `docs/design-notes/m8-polish-and-ship.md`: Need row "Two daily rulings"; Journey J19
- `crates/graft-ops/MAP.md` `## rewrite`, `## restore`, `## journal`; `crates/graft-git/MAP.md` history/divergence;
  `crates/graft/MAP.md` `update::preview` (`history_key`)

## Rulings and the rows they close
1. **`a` on HEAD with a dependent branch and nothing staged is refused** (`graft-ops rewrite/plan.rs:178-185`
   `NothingStagedFor`); before m7/09 it opened the message-only amend editor. **His ruling: let him edit the message.**
   → opens the commit editor on HEAD's message; `⏎` rewords HEAD; dependent branches follow as an amend's do;
   previewed when dependents exist (as today for an amend with dependents).
2. **A branch forked mid-chain stays on the old commit after `a`** (the preview names it, m7/37). Ruling: it follows
   like a dependent when its replay forecasts clean; when it would conflict, it stays and the preview says
   `<b> stays: would conflict`. Undo restores every moved ref (one journal entry, as today).
3. **`m`/`r` on a stash row preview a merge/rebase onto the stash's internal commit**, named by sha, nothing refuses
   it (`crates/graft/src/update/preview/history.rs:92-106`). Ruling: refuse in words `a stash isn't a branch · ⏎ opens
   it`. The bar half (not offering `m r R` on stash rows) is `bar-content-pass`'s: hint lists are its files.
4. **A staged-new file he deleted from disk comes back after a put-back, silently**, while the line says "your edits
   are back" (git stash behaviour). Ruling: keep git's result (nothing lost) and say it: the landing line names the
   files that came back from the index (`notes.md came back from the index`).
5. **Ahead/behind overcounts when commits share a committer timestamp** (`main ↑6 ↓1` where rev-list says `1 1`;
   `crates/graft-git/src/history/divergence.rs:108-110`, `:161-167`): a commit popped as one side's only is counted at
   once; the later re-visit fixes flags, not the count. Fix: count after the walk settles, from flags.
6. **Question: `fold_rebases` misses git's `rebase: fast-forward` reflog line** (no brackets). Find it, test with a
   real fast-forward rebase's reflog; fix if J mis-groups, else close with the test as evidence.

## Scope
In: 1–6 (3's refusal only). Out: cursor landings (G-daily 1); squash message order (his ruling: keep).

## Signatures
None public expected; a new `RewriteKind::Reword` or a plan flag is the builder's, recorded in MAP.md graft-ops.

## Acceptance
- [ ] J19 `reword-head` = 3 (`a` edit `⏎`) with a dependent branch, nothing staged: HEAD's message changed, the
      dependent rebased on it, `u` restores both (walk test, truth read with git).
      **Ops half only, not ticked:** `graft-ops tests/it/rewrite_run.rs`
      `reword_head_rewords_moves_the_line_and_undoes_as_one` (message, line, `u`, read with git),
      `reword_head_plans_one_reword_row_with_the_line`, `reword_head_with_nothing_built_on_it_is_a_plain_amend`.
      The key → editor → `⏎` wiring and the walk test are in graft files outside this builder's set (NOTES).
- [ ] Fork mid-chain: clean → follows; conflicting → stays and the preview says so (two tests, truth from git).
      **Split off, not built** (ticket's own Size rule): bigger than a plan change (NOTES).
- [x] `m`/`r` on a stash row refused in words, no preview opened — `graft src/update/preview/tests.rs`
      `m_and_r_on_a_stash_row_are_refused_in_words` (seen red without the check, green with it).
- [ ] Put-back with a deleted staged-new file: line names it (test).
      **Ops half only, not ticked:** `graft-ops tests/it/set_aside.rs`
      `a_staged_new_file_deleted_from_disk_is_named_as_back_from_the_index` (rebase and checkout: the read names
      `notes.md`, not the staged file still on disk; words `notes.md came back from the index`). The landing line
      does not show it yet: wiring is in graft files outside the set (NOTES).
- [x] Equal-timestamp fixture: badge counts equal `git rev-list --left-right --count` — `graft-git
      tests/it/equal_dates.rs` `ahead_behind_on_one_date_matches_rev_list` (6 seeds × fork, merge, criss-cross,
      96 pairs vs rev-list); unit `history::divergence::tests::one_date_*` (3, seen red before the fix: `↑2 ↓1`
      where truth was `1 1`).
- [x] `rebase: fast-forward` reflog case — `graft-ops tests/it/journal_list_oracle.rs`
      `a_rebase_that_fast_forwards_is_one_row` (real `rebase -i --update-refs` on git 2.54.0; seen red: 4 `$` rows,
      now one `$ rebased fb onto main`).

## Touches
- Rewrite preview words and ghost rows; journal entry shape for a reword (undo/redo of it); radar ahead/behind
  readers; J grouping; m7 walks covering `a` (grep `NothingStagedFor`); MAP.md graft-ops, graft-git, graft.

## Scout questions
- How are dependents chosen in `rewrite/plan.rs`, and why is a mid-chain fork not one (file:line)?
- Where is `fold_rebases` and what pattern does it match (file:line)?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~70k tokens. If 2 proves larger than a plan change, it splits off and 1, 3–6 land first.

## NOTES (builder, 2026-10-02) — partial
Scouts: `scouts/14-daily-ops.md` (conductor's), `scouts/14b-reword-fork.md` (mine, items 1–2). No compaction.

Done (cause fixes):
- 5: `graft-git history/divergence.rs` `count` — counted from the settled marks after the walk, and the walk runs
  on past `unshared == 0` while a queued commit is as new as the oldest one-sided pop (same-second commits are
  ordered only by id, so a parent could pop before the child that makes it shared).
- 6: `graft-ops journal_list/steps.rs` `is_rebase` also matches `rebase: ` / `rebase -i: ` (git writes
  `rebase: fast-forward` for picks after an `update-ref` line). `scratch::rebase_step` still treats it as no step.
- 3: `graft update/preview/history.rs` `history_operation` refuses a stash row (`RefBadges::is_stash`) before
  anything else reads the row. The bar half (not offering `m r R`) stays `bar-content-pass`'s.

Ops half, wiring blocked (needs graft files outside the set):
- 1: `graft_ops::rewrite::reword_head(repo, git, message: &str, facts, default_branch, stop) -> Result<RewriteParams, RewriteError>`
  (a free fn: adding a `RewriteRequest` variant breaks exhaustive matches in graft `jobs/rewrite_open.rs:112` and
  `jobs/rewrite.rs:157`). Wiring: `jobs/rewrite.rs` `plan` (~:97-103) — on the `HEAD` path with dependents and
  nothing staged, answer a new `RewritePlanned::RewordHead` instead of planning `AmendInto` (which refuses
  `NothingStagedFor`, `graft-ops rewrite/plan.rs:223-231`); `update/amend_into.rs` `answer` (~:299-306) opens the
  commit editor on HEAD's message in a reword-with-line mode; its `⏎` (`update/commit_editor.rs` ~:157) runs
  `reword_head` in a job and opens the preview of `Operation::Rewrite(params)` as an amend with dependents does.
  Then the J19 walk test (`tests/it/m7_walk/amend_into.rs` beside `amend_with_dependents`).
- 4: `graft_ops::apply::came_back_from_index(repo, stash, worktree_before) -> Result<Vec<PathBuf>, RunError>` and
  `came_back_words(&[PathBuf]) -> Option<String>`. Wiring options, both outside the set: (a) a field on
  `graft_ops::apply::Applied` filled in `apply/set_aside.rs` `close` on `Outcome::Back` — `Applied` is built by
  struct literal in ~15 places, many in graft tests; (b) graft `update/clash.rs` `edits_back(entry)` (:416) can't
  read git (update is pure), so the read must ride the job's answer: `update/operate/outcome.rs:66,84,90` and
  `update/banner/resume.rs:285` pass it in. (a) then a `edits_back(entry, came_back)` in clash.rs is my pick.
  `clash.rs` was not changed: a words fn with no caller fails clippy's dead-code gate.

Split off:
- 2: a branch forked below the rewritten commit is not in the replayed range, so an `update-ref` line cannot move
  it (ADR-0047 §1 :34 — todo holds no `label`/`reset`; §4 — one line, a fork never guessed). Following it needs
  `label`/`reset` lines or a second replay inside the same entry, plus undo of the extra ref: an ADR-0047
  amendment and an engine change. A clean-replay forecast exists (`preview/chain.rs` `chain`, :71).

Touches: `ahead_behind` readers (radar badges, `refs::badges`) — same signature, counts only change when dates
tie; whole nextest run green. `is_rebase` callers: `fold_rebases` and `scratch::rebase_step` (grep; the latter
unchanged for the bracketless form, lost-scan tests green). `history_key` callers: `update/keys.rs:377` only
(grep); stash refusal is before `current_branch`, the diverged-upstream path reads no row and is unchanged.
`reword_head`/`came_back_*`: no callers yet. `NothingStagedFor` unchanged (m7 walks untouched). MAP.md: graft-ops,
graft-git, graft.
