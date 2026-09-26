# 06b absorb-staged-and-replay — review 06's Critical and the replay Majors

Status: done
Blocked by: 28b (holds graft-ops). Before 03b: 03b's pull set-aside uses the same put-back.
Crates: graft-ops only (`apply/set_aside.rs`, `absorb.rs`, their tests). If you need another crate, commit at the seam
and report `blocked: needs <crate>` with the signature.
Build with: Opus (set-aside protocol, ADR-0045; undo must still hold).
Scout: none. The review cites every site at `48839e22`; re-read each before editing.
Split by: conductor, 2026-09-26, from `docs/reviews/m7-06-absorb.md`. No new decision (see Ruling).

## Scope
Each fix is the one the review's section states, at the cause, red first.
- **C1** (§Critical C1): a hunk that stays and lives only in the index is lost. Put back with the index
  (`set_aside.rs:480-490`), so a stay **stays staged**. Includes the mode-only case (§"KNOWN_GAPS M7/06" table).
  Fix `stays_unstaged`'s doc (`absorb.rs:85-89`) to what the code now does.
- **M2** (§M2): a staged rename is half-absorbed.
- **M3** (§M3): a fixup that empties its target stops with "0 unmerged · resolve".
- **M6** (§M6): `add -p`, keep editing, absorb leaves conflict markers in his file.

**Ruling (conductor), review Question 1:** the note's promise is "a hunk that fits no commit **stays staged**" (note
:39, :560). C1's fix makes it true. Build to the note. ADR-0045:131's "comes back unstaged" (M6/0045) is superseded for
absorb: say so in NOTES with the line, and the conductor amends the ADR.

**Touches, required:** every caller of the set-aside put-back (grep). Pull's set-aside (03b, not built yet) and M6's
stash flows must keep their tests green. Name each test.

## Acceptance
- [x] C1: the index-only probe and the mode-only probe as tests, red first. `absorb_staged::an_index_only_stay_survives_absorb`, `absorb_staged::an_index_only_mode_change_survives_absorb` (both red before: index lost), plus the ruling's `absorb_staged::a_stay_stays_staged` (red: status assert failed).
- [x] M2, M3, M6: each probe as a test, red first. M2 `absorb_staged::a_staged_rename_stays_whole` (red: rename's deletion placed); M3 `absorb_staged::a_fixup_that_empties_its_target_drops_it` (red: absorb did not go through) and `absorb_staged::a_fixup_that_empties_a_commit_below_the_top_drops_it` (the review's shape; written after the fix, not seen red); M6 `absorb_staged::absorb_after_editing_on_leaves_no_markers` (red: absorb did not go through).
- [x] `undo_after_absorb_restores_index_and_commits` still green, plus one undo test per fixed case: `absorb_staged::undo_after_absorb_with_an_index_only_stay`, `…_with_an_index_only_mode_change`, `…_with_a_staged_rename`, `undo_after_absorb_that_drops_its_target`, `undo_after_absorb_after_editing_on`. (The two index-only undo tests passed before the fix too: undo already restored the index, as the review said.)
- [x] Every set-aside test in graft-ops and graft green: `cargo nextest run --workspace -E 'test(/set_aside|put_back|stash/)'` 238/238.
- [x] Gates workspace-wide: fmt --check, clippy --all-targets -D warnings, nextest 3912/3912 (1 skipped), test --doc.

## NOTES (builder, 06b)
- **Mechanism (C1 + M6, one cause):** absorb's plan writes `RewriteParams::index_base` = `HEAD` + every placed piece (`commit_pieces`), pinned, and saved in `plan.json` (`SavedPlan::index_base`, serde default) so a resume after relaunch has it. At step 6, when the rewrite went through **and** `HEAD^{tree}` equals the base's tree, `set_aside::from_index_base` rebuilds `S` (worktree tree `S^{tree}`, parent 1 = base, index commit = `S^2^{tree}` on base) and runs `stash apply --index` on it. The index gets base→old-index = exactly the stays (index-only, mode, binary, new files, renames); the worktree merges with base as merge base, so an edit made on past a placed hunk comes back clean. `S` stays the listed copy, dropped by id. Otherwise (other ops, failed/aborted absorb, or a resume that landed elsewhere) the plain ADR-0045 put-back runs.
- **Review's suggested base for M6 was wrong for stays:** with the *old index* as merge base, a stay the working tree holds is removed from the worktree (ours drops it vs base). The right base is old `HEAD` + placed hunks. Said here rather than silently deviating.
- **Ruling, for the conductor to amend:** `docs/adr/0045-stash-put-back-is-recorded-and-one-entry.md:131` ("A put-back loses the staged/unstaged split on the clean path too (no `--index`)") is superseded **for absorb**: `set_aside.rs` `put_back` / `from_index_base`.
- `AbsorbMap::stays_unstaged` removed (always false now; no user outside graft-ops, grep). `SetAside.staged` is 0 for absorb (`operation.rs` `sets_aside`; test `absorb_sets_aside_with_no_staging_lost`).
- M3: a target whose pieces empty it is planned `drop`, no fixup (`absorb::empties`, `plan.rs` `absorb_rows`). When every row would be a drop, the commit below joins as a pick (git keeps it unchanged: no `--force-rebase`). If the emptied commit is the branch's only one, `validate` still refuses "every commit dropped · the branch would be its base again" — truthful, not changed.
- **Unverified:** the resume path with `index_base` (absorb stopped mid-replay, then `c`) — no fixture makes absorb stop; covered only by `SavedPlan` round-trip unit tests. The preview's words for a dropped target (`becomes empty · dropped`) and for `StaysWhy::Renamed` are the app's; no app code reads either yet.
- **Knock-on, not done (scope):** `a` (amend-into) has the same M6 clash and could set `index_base: Some(made)` for the same fix; `r`/`R` still lose index-only content (review C1 knock-on). The put-back forecast (`preview/put_back.rs`) does not forecast rewrites at all, unchanged.
- Scout: none (ticket says so; every site re-read before editing).
- **Touches:** put-back callers (grep `set_aside::close`): `apply.rs` `closed` and `resume.rs` `close_resume` — both pass `index_base`, `None` for every non-absorb op, so rebase/checkout/recover/`r`/`a` put-backs are unchanged (`set_aside`, `stash_clash`, `put_back_forecast`, `rewrite_run`, `recover` suites green). `record.rs` `pins_for` pins the base. `engine.rs` saves it. ADR-0049 guard: `guard_in_the_way` still runs before the engine; `in_the_way.rs` skips `Drop` rows and covers `onto`'s reset — `in_the_way` suite green. Journal/undo: `undo_after_*` tests. App crate: no reader of `stays_unstaged`/`StaysWhy` (grep).
