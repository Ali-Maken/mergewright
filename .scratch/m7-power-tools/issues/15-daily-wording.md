# daily-wording — a rejected push names the right keys, no `subject empty` on open, no radar finding for a branch without upstream, the commit-graph offer only after a slow sort, keys drawn once, single-`Close` boxes offer only `esc`

Status: superseded by 36 (planner 2026-10-01: folded; line numbers predate the preview.rs split)
Blocked by: none
Crates: graft-ops (`apply/remote.rs`, `mistake_radar/`), graft (`update/commit_editor.rs`, `view/commit_editor.rs`,
`update/commit_graph.rs`, the sort job's timing, `update/preview.rs`, `view/radar.rs`, and the tests that pin the
old words), graft-ui (`name_prompt.rs`, `preview_box.rs`), graft-core (one doc line, `mistake_radar.rs:43`).
**Cannot be disjoint from graft:** graft's own tests pin the graft-ops words (:774 list below), and :776, :778, :860
live in graft.
Source: note "Pass 2a" R7. Respecified from scout `.scratch/m7-power-tools/scouts/15.md` (read at 9f8dde7b).
Build with: Sonnet. Words and conditions; each fix is named below with its line.
Scout: done (15). No second scout.
Needs: none.

## Rows closed, not built (the conductor moves them to `docs/gaps/closed.md`)
- **:775 "commit or stash first" — already fixed** (m6-conflicts/85 D). No such string in any `src` (scout grep).
  `graft-ops/src/apply/remote.rs:259-262` now reads `pull refused: your edits to <paths> would be overwritten`, and the
  app adds `OVERWRITE_KEYS = "S s stashes them first"` (`graft/src/update/remote.rs:164`, used
  `update/remote/in_flight.rs:205-208`). Pinned by `graft-ops/src/apply/remote/tests.rs:137-150`.
- **:779 `(M5)` / `shell:` on screen — already fixed.** No `(M5)` or `shell:` in any `src` (scout grep). With the
  tracking tip known the app shows `<b> and <tracking> have diverged` + `DIVERGED_KEYS "m merge it  r rebase onto it"`
  (`in_flight.rs:187-201`, `update/remote.rs:168`). The ops line `pull stopped: <b> and <remote> diverged — merge or
  rebase` (`apply/remote.rs:250-253`) shows only when no tracking tip is known (`in_flight.rs:189-192`), where `m`/`r`
  have nothing to aim at (`RemoteState::diverged_upstream` needs an `Oid`, `crates/graft/MAP.md:454`), so naming the
  keys there would offer keys that refuse. Correct as is.

## Decided (exact words)
- **:774** `graft-ops/src/apply/remote.rs:239-242` → `push rejected: <remote_side> has commits you don't · p pull,
  then m or r · P force with lease`. **No count:** `explain` has no ahead/behind at hand (`apply/remote.rs:222-227`
  takes entry, failure, stderr, names; scout 15 §1), so the ticket's own fallback applies. The split at the first
  ` · ` (`in_flight.rs:217-222`) keeps the reason and keys apart; `p pull, then m or r · P force with lease` is the
  keys part.
- **:776** `subject empty` (`update/commit_editor.rs:332-333`, drawn every frame from `view/commit_editor.rs:21-25`)
  shows only after `Commit` was pressed (`⏎` in the subject, `Ctrl+Enter`, or the button) with an empty subject. It
  clears on the next typed character. Nothing on open. New editor state: one flag, set by the refused commit, cleared
  by an edit to the subject.
- **:777** The rule goes: `graft-ops/src/mistake_radar/rules.rs:323-330` (`UpstreamFacts::None if !b.is_default`),
  its variant (`mistake_radar/mod.rs:231`), its explain text (`explain.rs:270,359`), its drawn words
  (`graft/src/view/radar.rs:232`), and the doc mention in `graft-core/src/mistake_radar.rs:43`. Its tests go with it.
  **Not** the pull refusal `has no upstream — P pushes` (`remote_ops.rs:350`, `operation.rs:853`): different thing,
  untouched.
- **:778** The offer shows only when **this run's** topological sort took ≥ 300 ms. Nothing records it today (scout 15
  §3). The builder times the `history::topological_order` job (one `Instant` around it, in the job), carries the
  `Duration` on its answer, and `update/commit_graph.rs` decides when that answer lands, not when facts arrive
  (`:13-36`, `:31-33`). No sort ran (the order was never provisional) or the sort stopped/failed (`Promotion::Kept`)
  → no offer. The decision is a pure fn of (fact, duration) so tests feed the duration. The once-per-repo declined
  flag and `harness.offer_commit_graph` are unchanged. *Not persisted across runs*: "the last open" is this open.
- **:856** The name prompt's border keys go (`graft-ui/src/name_prompt.rs:194`, `⏎ create  ⇥ toggle  esc cancel`); the
  bar keeps them (`graph_screen/hints/lists.rs:72-`, `HintContext::NamePrompt`).
- **:860** Two box types draw a single `[ Close ]` under the full `←→ choose  ⏎ act  esc cancel` footer
  (`graft-ui/src/preview_box.rs:42,67-69`):
  - `closing_box` (`graft/src/update/preview.rs:849-857`; callers `:825` refused, `:826-828` preview failed,
    `:830-836` undrawable) — only the stash put-back gets `back_only` today (`:855`);
  - `ready_box` nothing-to-do (`preview.rs:862-863`).
  Fix at the cause: `preview_box` derives its footer from its choices — **one choice → the `back_only` footer**
  (87's words, copied, not rewritten). The `back_only` flag then has no job: remove it and its setter. Confirm boxes
  never have one choice (`graft-ui/src/confirm.rs:81`, `after_push.rs:217`): untouched.

## Read first
- Note "Pass 2a" R7; CLAUDE.md "A status field must never read as an error when nothing is wrong".
- `docs/design/02-keymap.md` §2.1 (trailing parens carry a key), §2.3c, §2.10.
- Scout 15 (all).
- `crates/graft-ops/MAP.md` `apply` (remote), `mistake_radar`; `crates/graft/MAP.md` `update::commit_editor`,
  `CommitGraphOffer`, `AppState::commit_graph`, `history::topological_order` / `Promotion`, `update::preview`,
  `update::remote::in_flight`; `crates/graft-ui/MAP.md` `preview_box`, `commit_editor`, `name_prompt`.

## Signatures
```rust
// graft::update::commit_graph — the decision, pure.
pub(crate) fn offer_after_sort(fact: &CommitGraphFact, sort_took: Duration) -> bool; // true iff Absent{declined:false} && sort_took >= SLOW_SORT
pub(crate) const SLOW_SORT: Duration = Duration::from_millis(300);
// graft-ops::mistake_radar: the no-upstream variant removed (name it in NOTES). graft-ui::preview_box: `back_only`
// removed; footer from choice count. No other pub API.
```

## Acceptance (each box names its test at tick time; red first where marked)
- [ ] `rejected_push_names_pull_then_m_or_r` (:774, red first). The old-word tests updated, each named in NOTES:
      `update/remote/tests/lease.rs:35`, `update/remote/tests.rs:475,480`, `view/tests/bottom_row.rs:15`,
      `tests/it/remote_by_key.rs:589`, `tests/it/m4_walk/push.rs:263,345`, `tests/it/box_owns_the_bar.rs:121`.
- [ ] `a_fresh_commit_editor_shows_no_refusal`, `commit_with_empty_subject_says_subject_empty`,
      `typing_clears_subject_empty` (:776, red first). `update/commit_editor/tests.rs:188,400,1015` updated.
- [ ] `a_branch_without_upstream_has_no_finding` (:777, red first). Removed with the rule: `rules.rs:683`, `:~695`
      (keep its unborn-HEAD half if it tests more than the rule — say which), `graft-ops/tests/it/mistake_radar.rs:339`;
      updated: `graft/tests/it/m6_stash_put_back.rs:324,337,614`, `gallery.rs:430,470`, `status_under_a_box.rs:51`,
      the comment at `radar_bookkeeping.rs:191`.
- [ ] `a_fast_sort_gets_no_commit_graph_offer`, `a_slow_sort_gets_the_offer`, `a_stopped_sort_gets_no_offer` (:778).
- [ ] `the_name_prompt_draws_keys_once` (:856, red first).
- [ ] `a_single_close_box_offers_only_esc` over each of the four callers above (:860, red first);
      `the_stash_put_back_box_is_unchanged` (87's capture byte-identical).
- [ ] **Captures:** new `docs/shots/m7/commit-editor-fresh-120x40.txt` — **right:** the editor with an empty subject
      and no `subject empty` anywhere. New `docs/shots/m7/refused-box-single-close-120x40.txt` — **right:** `[ Close ]`
      and a footer reading only `esc back` (87's words). Colour unverified.
- [ ] Older captures: every capture a **fast-tier test** writes is re-taken by that test and listed in NOTES (e.g.
      `m4-01-push-box-plain-160x48`, `m5-hints-uncommitted-160x48` loses the offer). Captures only the heavy tier or a
      past milestone wrote (scout 15 §2's 32 files with `no upstream`: m2-01…m2-14, `fix-*`, `menu-fixed-slots`,
      `landed-after-merge`, `m6-contact-sheet-1/3/4` + `captions.tsv`) are **not hand-edited**: listed in NOTES as
      stale, for ticket 29 / the milestone review's heavy run.
- [ ] Hint-bar tests green. MAP.md for graft-ops, graft, graft-ui.

## Touches (check each, report how)
- **Mistake Radar screen and header counts:** one finding fewer per branch without upstream. Check: radar tests; list
  each count that changed.
- **`p`/`P` after a rejection:** only the words change; `RejectedPush`, the lease rules (`push_params`) untouched.
  Check: m4 push walk green.
- **Commit editor refusals other than `subject empty`** (`Ctrl+Enter` with nothing staged, etc.): unchanged, still
  computed as today. Check: `update/commit_editor/tests.rs` green except the three lines named.
- **Commit-graph offer:** `commit_graph_line`, `graph_rows` subtracting its row, `B`/`X`, `blamable` ("no commit-graph
  offer") all read `AppState::commit_graph`, whose states are unchanged — only *when* it becomes `Offered` moves.
  Check: grep `CommitGraphOffer::Offered`, report each hit.
- **Preview boxes with two or more choices:** footer unchanged. Check: preview-box tests; a Ghost Preview capture
  byte-identical.
- **Name prompt bar:** unchanged. Check: hint tests for `HintContext::NamePrompt`.
- **Keymap:** none. §2.1's trailing-parens rule holds in every new string.
- **Journal/undo:** none.

## Rules / Gates
As rewrite-engine.

## Size
Medium (~50k).
