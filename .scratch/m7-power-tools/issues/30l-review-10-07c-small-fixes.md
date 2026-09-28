# 30l: small code fixes from review 10..07c (Minors 2, 8, 9)

Status: done
Build with: Sonnet. Three local changes, each named with its line in the review.
Blocked by: 08-a (holds graft `update/operate.rs`, `update/preview.rs`).
Scout: none. The review names every line (`docs/reviews/m7-10-to-07c.md`, Minors 2, 8, 9).
Files: graft `src/update/operate.rs` (only `stopped`, ~:1008-1020, and its callers' argument), `src/update/preview.rs`
(:976 only), `src/event_loop/running.rs` (comments only); graft-ops `src/preview.rs`, a new child for `shown_aside`; tests beside each; MAP.md per crate.

## Fixes (decided)
1. **Minor 2:** `stopped(stop: Operated, …)` returns `Vec::new()` on a non-`Conflicted` value. Pass the `Conflicted`
   fields (or a struct) so the type carries the invariant. No silent `else`.
2. **Minor 8:** the unborn-HEAD take words (`update/preview.rs:976`) → `check_pick`'s words, as ticket 11 Decided. If
   unreachable, say why in a comment, with the line of the earlier `not_a_commit`.
3. **Minor 3 (source part):** stale comments in graft `src/event_loop/running.rs:210` ("These three"), `:232` ("one of
   the three"), `:284` ("exactly these four") — the match at `:212-217` has six. Word them from the code.
4. **Minor 9:** move `shown_aside` (graft-ops `preview.rs:404-424`) beside `SetAside` in a child; `preview.rs` ≤ ~400.

## Acceptance
- [x] Minor 2: private tuple `Stop` (the variant's six fields) is `stopped`'s argument; `operated` destructures
  `Operated::Conflicted` in its arm, no `else`. Green: `update::operate` tests `conflicted_entry_id_survives`,
  `a_conflict_in_one_file_counts_one_file`, `a_stop_with_nothing_unmerged_does_not_claim_a_conflict`,
  `a_conflicted_merge_lands_in_the_resolver`, `an_edit_stop_with_nothing_unmerged_keeps_the_banner`.
- [x] Minor 8: reachable (orphan branch: HEAD unborn, other branches' rows drawn), so not a comment: words now
  `Refusal::BranchChanged.explain()`, what `check_pick` → `head_is` returns for an unborn HEAD. Test
  `context_menu::tests::o_on_an_unborn_head_says_check_picks_words`.
- [x] Minor 3: `grep -n "three\|four\|six" running.rs` → :210 "These six", :232 "one of the six", :284 "exactly these
  six"; the arms at :212-217 and `handed_over`'s :262-281 are six.
- [x] Minor 9: `preview.rs` 450 → 430; `shown_aside` moved verbatim to new `preview/aside.rs`; no test changed.
- [x] MAP.md: no public item changed (Stop, incomplete_answered, aside are private) — none edited. Gates
  workspace-wide: fmt, clippy, nextest 4727/4727, test --doc pass.

## Touches (check each, say how)
Callers of `stopped`; every preview title test; `shown_aside` callers.

## NOTES (builder)
- Minor 2 pushed `operated` to 64 lines (clippy too_many_lines 60). Cause fixed, not allowed: the `Incomplete`
  arm's put-in forget moved into `incomplete_answered` (same code). `Stop` is a tuple so the arm stays short.
- Added a test in `update/context_menu/tests.rs` (where every `o` test lives) — outside the Files line's paths but
  "tests beside each"; 07d does not hold it.
- No scout: the review named every line.
