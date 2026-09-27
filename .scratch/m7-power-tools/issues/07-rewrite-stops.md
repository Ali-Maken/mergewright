# rewrite-stops — a stop says where he is and what's left; an `edit` stop hands him the graph; `Esc` never aborts

Status: draft (respecced after scout 07, planner, 2026-09-27). **Three parts, filed as 07a / 07b / 07c** (below): each  **Split into 07a / 07b / 07c (conductor, 2026-09-27); build those, not this file.**
has its own Files, Build with, Needs, Acceptance. They run in order a → b → c; the file sets are what make them
sequential with the other tickets, not the size.
Blocked by: 02-rewrite-engine, 02b-rewrite-engine-b, 03-land-on-result (-a merged cad5ed86); see "Order".
Source: note "Pass 2a" R4 (`docs/design-notes/m7-power-tools.md:600-651`); journeys 4, 5; keymap §2.6a (already
amended: `02-keymap.md:317-351`); scout `.scratch/m7-power-tools/scouts/07.md`.
Scout: done (`scouts/07.md`). No second scout.

## What the scout changed (settled)
1. **`Esc` already never aborts.** Banner: `banner_key` `banner.rs:340` → `hand_the_keyboard_back` `banner.rs:295`.
   Resolver: `resolver/leave.rs:40` `escape` → `close` `:50`. Graph `Esc` (`keys.rs:204`) cancels only stoppable work;
   `resume` is `Stoppable::No` (`banner.rs:576-613`). So `esc_never_aborts` **cannot go red**: it is a **regression
   test**, written green, said so in NOTES. KNOWN_GAPS row **:545** (not :549) is stale → **the conductor closes it**
   (cite the three lines above), not the builder.
2. **The banner text is graft-ui `banner_line`** (`graph_screen/mod.rs:253`, `BannerView` `:205`), with no ladder (doc
   `:233-251`: "nothing left to tier"). `stop_wordings` (`stop.rs:204`) is the **footer** stop line — **unchanged**.
   Decided: `banner_line` + `BannerView` **move** to a new file `crates/graft-ui/src/graph_screen/banner.rs` (a
   move-only first commit), and the ladder and chain words are added there. `mod.rs` keeps `mod banner; pub use
   banner::{banner_line, BannerView, RewriteBannerView};`. Why a file: `mod.rs` is shared with 09 (GraphEntry); a
   separate file keeps 07's banner work disjoint and testable alone. The stale doc `:233-251` is rewritten there.
3. **Focus: every stop focuses the banner today** — `operate.rs:1043`, `stop.rs:66` (`stopped_again`),
   `relaunch.rs:80`, all `banner_focusable(kind)` (`banner.rs:229`). There is no "nothing unmerged" branch, and 07
   doesn't add one. The exception keys on **the edit-stop fact only** (`Stop.rewrite.edit.is_some()`), through one
   new fn `stop_takes_focus`, called at those three sites. Every non-edit stop keeps today's focus, byte for byte.
   (Keymap `:317-318` says "a stop with nothing unmerged focuses the banner"; the code is wider. Doc question for
   the conductor, not a build change.)
4. **P14's refusal is `check_conflict_state`, `graft-ops/src/operation.rs:1435-1446`** (`Amend` not in the `waits`
   exemptions), not `commit.rs`. `RepoFacts` (`operation.rs:312`) has no edit-stop fact; 07a adds one. 10b, 11a and
   03b also hold `operation.rs` → 07a runs after 03b (see "Order").
5. **`rebase-merge/amend` is unconfirmed** (no code reads it). 07a's **first test** checks it against real git
   (`git_writes_rebase_merge_amend_at_an_edit_stop`). Planner's belief, **unverified**: git's sequencer writes HEAD's
   oid there at an `edit` stop and removes it on `--continue`. Fallback if the test shows git doesn't: below, 07a.

## Order (against running / next)
- **10b** (graft-ops `operation.rs`; graft `update/operate.rs`) — overlaps 07a and 07b.
- **16b** (graft `update/events.rs`, `events/command.rs`, `jobs/runner.rs`, `update/mod.rs`, `event_loop/label.rs`,
  `update/place/write.rs`) — overlaps 07b (products) and 07c (hand-over).
- **09** (graft `update/keys/commit.rs`, `update/commit_editor.rs`, the JOBS set, `update/mod.rs`; graft-ui
  `graph_screen` GraphEntry file + render, `RowTag`) — overlaps 07b and 07c.
- **11a → 03b** (graft-ops `operation.rs`; 03b also the pull banner words and the m7 walk harness) — overlaps 07a
  (and 07b if 03b's banner words are in `update/banner.rs`).
- **21** (being respecced) — no known overlap; the conductor checks its Files line against 07b/07c's
  (`graph_screen` render, `update/mod.rs` are the likely ones).
- **Sequence:** 10b ∥ 16b → 09 ∥ 11a → 03b → **07a** (may run beside 09: 09 holds no graft-ops file) → **07b** (after
  09 and 07a) → **07c**.

## Decided (note R4, unchanged unless marked)
- **Banner text:** `REBASING  step i of n · <action> <sha7> <subject>`, then `· <branch>, branch k of m` only when
  `progress().branch` is `Some` (a shell-begun rebase has none). An `edit` stop adds `· change files, space stages, a
  amends it, c continues · ⇥ banner`.
- **Ladder (new, 07b):** as width shrinks, drop in this order: the edit hint (the hint bar says the same), the
  subject, the chain words, the sha7 + action. `REBASING  step i of n` is never cut. Every non-rewrite banner (merge,
  cherry-pick, revert, pull, stash clash) is **byte-identical** to today.
- **Conflict stop:** the M6 landing (resolver, first file). Only the banner words change.
- **Edit stop:** keyboard to the **graph**; banner drawn unfocused; `⇥` focuses it (its `a` aborts, box, `y`).
  Cursor on HEAD, tagged `HEAD · stopped here`.
- **`a` at an edit stop** (graph focused, cursor on HEAD or the uncommitted row): amends HEAD with what's staged,
  **keeping its message, no editor** (`commit --amend --no-edit` semantics), through `Operation::Amend` and `apply` as
  every amend. Lands: the graph, cursor on the new HEAD (still tagged), status `✓ amended the stopped commit · c
  continues`. Pressed twice (a forgotten file): amends again. On any other row: today's P14 words.
- **`c` in a shell-begun rebase or cherry-pick** (07c): hands the terminal to `git <kind> --continue` as `M` hands
  over to the mergetool; git's editor may ask for a message. On return, one working-tree read, then:
  - finished → graph, status `✓ git <kind> --continue finished`;
  - stopped again → the M6 stop landing (`stopped_again`), footer `stop_wordings`;
  - non-zero exit, still in progress → the stop stays, status `git <kind> --continue exited <code> · still stopped`.
  A shell-begun revert: unchanged (refused in words).
- **`Esc`:** never aborts (settled 1). Regression-tested.
- **Still-to-apply block** (07c): boxed between the banner and the graph rows, titled `still to apply · not commits
  yet`; one row per `pending` pick-like line **newest first** (action word, sha7, subject); then
  `<branch> (<author of its tip>): N commits follow, on top` per dependent still to come; ≤ 6 rows then `+N more`;
  drawn at any rewrite stop while `pending` is non-empty; never ghost rows in the graph (field notes :38).
- **Abort landing:** as 03 defines; status `✓ rewrite aborted · every branch is as before`.
- **A failed progress read** (rule 3): the stop still lands; banner falls back to today's `REBASING step i of n`;
  focus = banner (the safe default: its `a` is behind a box); status `couldn't read the rewrite's progress: <error>`.

## Signatures
```rust
// graft-ops (07a) — rewrite.rs, beside RewriteProgress (:278)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditStop { pub stopped: Oid }          // the rewritten commit git stopped on (rebase-merge/amend)
// rewrite/progress.rs — same repo/git/stop parameter types as `progress` (:39)
pub fn edit_stop(repo: &Repository, git: &Git, stop: &StopFlag) -> Result<Option<EditStop>, RewriteError>;
// operation.rs — RepoFacts (:312) gains:
pub edit_stop: Option<EditStop>,                  // filled by facts::read_facts; None outside a rebase
// check_conflict_state (:1435): Amend is allowed iff facts.edit_stop is Some(e) and HEAD^1 == e.stopped^1
// (HEAD is the stopped commit or an amend of it). Anything else live: P14 stands.

// graft-ui (07b) — graph_screen/banner.rs
pub struct RewriteBannerView { pub action: String, pub sha7: String, pub subject: String,
    pub chain: Option<(String, usize, usize)>, pub edit_hint: bool }
// BannerView gains: pub rewrite: Option<RewriteBannerView>
pub fn banner_line(view: &BannerView, width: u16) -> Line<'static>;   // keep today's params if they differ; add width

// graft (07b) — update/banner/stop.rs
pub(crate) struct RewriteStop { pub progress: graft_ops::rewrite::RewriteProgress,
    pub edit: Option<graft_ops::rewrite::EditStop> }
// Stop gains: pub rewrite: Option<Result<RewriteStop, String>>   // Err = the failed read, shown, never dropped
// update/banner.rs
pub(crate) fn stop_takes_focus(kind: InProgress, stop: &Stop) -> bool; // banner_focusable(kind) && not an edit stop

// graft-ui (07c) — still_to_apply.rs
pub fn draw_still_to_apply(frame: &mut Frame, area: Rect, view: &PendingView);
pub struct PendingView { pub rows: Vec<PendingRowView>, pub more: usize,
    pub followers: Vec<(String, String, usize)> }    // (branch, author of its tip, commits)
pub struct PendingRowView { pub action: String, pub sha7: String, pub subject: String }
// graft (07c) — update/events/command.rs, update/events.rs
Command::ContinueInGit { id: JobId, kind: InProgress }   // Rebase | CherryPick only
Product::ContinuedInGit(Result<std::process::ExitStatus, String>)
// update/banner.rs
pub(crate) fn pending_view(state: &AppState) -> Option<graft_ui::PendingView>;
```
Names of parameter types (`Repository`, `Git`, `StopFlag`, `JobId`, `InProgress`) are the crate's existing ones; use
what `progress` / `read_facts` / the `M` hand-over already take. Name any rename in NOTES.

---

## 07a — the edit-stop fact and P14's exception (graft-ops)
Build with: **Opus**. Small, but it verifies git's behaviour first and chooses between a rule and its fallback.
Needs: `rewrite::progress` (02). Blocked by: 03b (holds `operation.rs`), which is after 10b and 11a.
Files: `crates/graft-ops/src/operation.rs` (`RepoFacts` :312, `check_conflict_state` :1435-1446 only);
`crates/graft-ops/src/facts.rs` (MAP `## facts`: `read_facts` fills `edit_stop`; if the module is another file, name
it in NOTES); `crates/graft-ops/src/rewrite.rs` (`EditStop`); `crates/graft-ops/src/rewrite/progress.rs`
(`edit_stop`); `crates/graft-ops/tests/it/edit_stop.rs` (**new**; the conductor adds the `mod` line in the seam
commit); every `RepoFacts {` literal in graft-ops, literal-only (`edit_stop: None`); `crates/graft-ops/MAP.md`.
Conditional: `crates/graft-ops/src/apply/engine.rs` only if check 2 below fails. A `RepoFacts {` literal in graft →
grep first; literal-only edits there are granted **unless** the file is one 09 holds while 09 runs → `blocked`.

Steps, in order:
1. **First test, against real git:** `git_writes_rebase_merge_amend_at_an_edit_stop` — a rebase with one `edit` row;
   at the stop, `rebase-merge/amend` exists and holds HEAD's oid; after `--continue`, gone. Record git's version.
   - **Holds →** `edit_stop` = `Some(EditStop { stopped: <amend's oid> })` iff that file exists and the last pick-like
     line of `rebase-merge/done` is `edit` (rules out any other writer of the file).
   - **Doesn't →** fallback: `edit_stop` = `Some` iff the last pick-like `done` line is `edit`, `stopped` = HEAD at
     read time; and the check drops the parent test (amend allowed at any edit stop — `git commit --amend`'s own
     shell behaviour). Say which branch was taken in NOTES; the conductor tells the planner.
2. **Second check:** `an_edit_stop_comes_back_as_a_stop` — `apply` of a rewrite plan with an `edit` row returns the
   outcome variant a conflict stop returns (the one graft's `operated` `Conflicted` arm takes), with no paths. If it
   comes back as a finish, fix it in `apply/engine.rs` (`PlanRun.edit_stop`, `:121`): the stop must reach the app.
3. The exception in `check_conflict_state`, then `read_facts`.

Acceptance:
- [ ] the two tests above (name the branch taken);
- [ ] `amend_is_allowed_only_at_an_edit_stop_on_the_stopped_commit` (table: edit stop + HEAD = stopped → allowed;
  edit stop + amended once → allowed; edit stop + a new commit on top → refused `InProgress`; conflict stop →
  refused; merge / cherry-pick / revert live → refused);
- [ ] `edit_stop_is_none_outside_a_rebase` and at a conflict stop;
- [ ] existing amend-refusal tests unchanged and green (name them);
- [ ] MAP.md graft-ops (`EditStop`, `edit_stop`, `RepoFacts.edit_stop`, the P14 exception).
Size: ~20k.

## 07b — banner words, the ladder, focus, `a` at an edit stop, `Esc` (graft + graft-ui)
Build with: **Opus**. An amend written inside a live rewrite through `apply` must leave the rewrite's live `Pending`
entry the one `resume_target` finds; wrong here is structural and not visible in a diff.
Needs: 07a (`EditStop`, `edit_stop`, the check); `rewrite::progress` (02); 09's `RowTag`/`RowTags` (the
`HEAD · stopped here` tag rides on them: add `RowTag::StoppedHere`); `land_on_head` (03-a).
Blocked by: 07a, 09, 03b.
Files:
- graft-ui: `src/graph_screen/banner.rs` (**new**, move-only commit first from `graph_screen/mod.rs:205-253`, then
  the ladder); `src/graph_screen/mod.rs` (the move: removal + `mod`/`pub use` lines only); the file declaring 09's
  `RowTag` (one variant + its draw words); every `BannerView {` literal in graft-ui, literal-only; their tests.
- graft src: `update/banner.rs` (`banner_view` :184, `banner_focusable` :229 + `stop_takes_focus`, `a`/Esc arms
  untouched); `update/banner/stop.rs` (`Stop`, `RewriteStop`, `stopped_again` :66); `update/operate.rs` (:1043 only);
  `update/relaunch.rs` (:80 and the relaunch read); `update/events.rs` + `jobs/runner.rs` (the stop-producing
  products — apply, resume, relaunch read — carry `RewriteStop`, read on the job thread by `progress` + `edit_stop`;
  name them in NOTES); `update/keys/commit.rs` (`commit_key` at an edit stop); `update/commit_editor.rs` (the P14 arm,
  `:390-393` per 09, gains the edit-stop exception read from `Stop.rewrite`; `operation_refusal` :133 untouched);
  `view/rows.rs` (bar label `a amend it` at an edit stop); every `Stop {` / `BannerView {` literal, literal-only.
- graft tests: `update/banner/tests.rs`; `update/banner/rewrite_stop_tests.rs` (**new**).
- Docs: MAP.md (graft, graft-ui).

Acceptance:
- [ ] `a_chain_stop_names_the_branch_in_the_banner` (red first);
- [ ] `a_single_branch_stop_has_no_chain_words`; `a_shell_begun_rebase_has_no_chain_words`;
- [ ] graft-ui `banner_line_keeps_step_i_of_n_at_80_columns` (red first) and `banner_ladder_drops_hint_subject_chain_sha_in_order`;
- [ ] `non_rewrite_banners_are_unchanged` (merge, cherry-pick, revert, pull, stash-clash: today's strings);
- [ ] `an_edit_stop_gives_the_keyboard_to_the_graph` (red first: today `operate.rs:1043` focuses the banner) — at all
  three sites: first stop, `stopped_again`, relaunch;
- [ ] `a_conflict_stop_still_focuses_as_today`; `tab_focuses_the_banner_at_an_edit_stop`;
- [ ] `a_at_an_edit_stop_amends_the_stopped_commit` (message kept, no editor); `a_twice_at_an_edit_stop_amends_twice`;
- [ ] `c_after_an_amend_at_an_edit_stop_continues_the_rewrite` (the live entry still found);
- [ ] `a_at_a_conflict_stop_is_still_refused` (app words and ops check);
- [ ] `esc_never_aborts_a_rewrite_stop` — **regression, green on arrival** (settled 1): `Esc` on the banner, in the
  resolver, on the graph; `rebase-merge/` and the live entry still there after each;
- [ ] `a_failed_progress_read_lands_with_todays_banner_and_says_so`;
- [ ] `a_relaunch_mid_rewrite_shows_the_chain` (kill -9 mid-rewrite, relaunch reads the saved plan);
- [ ] hint-bar tests green; `stop_wordings` tests unchanged and green.
Size: ~55k.

## 07c — still-to-apply block, the shell hand-over, walks, captures
Build with: **Sonnet**. The hand-over copies the `M` pattern (`outside.rs:173`, `running.rs:269`,
`update/mod.rs:384`); the block is one pure draw; walks and captures are fenced by counts here.
Needs: 07b (`Stop.rewrite`, `banner_view`); 03b's m7 walk harness.
Blocked by: 07b.
Files:
- graft-ui: `src/still_to_apply.rs` (**new**), `src/lib.rs` (mod + `pub use`), the graph_screen render file (the
  block's area between banner and rows; named after 09 lands).
- graft src: `update/banner.rs` (`pending_view`; `continue_pressed` :429 routes a shell-begun rebase/cherry-pick to
  `Command::ContinueInGit`); `update/commit_editor.rs` (`pane_c_acts` true for a shell-begun rebase or cherry-pick;
  the shell-begun words `:151-159` for keys still refused name `c`: `{kind} begun in the shell — c hands it to git`);
  `view/rows.rs` (`live_c` label); `update/events/command.rs`, `update/events.rs`, `event_loop/running.rs` (:214,
  :269), `event_loop/outside.rs` (beside `mergetool` :173, via `run_outside_terminal`), `jobs/runner.rs` (:1083),
  `update/mod.rs` (route beside `mergetool_returned` :384); forced arms `event_loop/label.rs`, `update/place/write.rs`.
- graft tests: `update/keys/live_c_tests.rs`, `tests/it/resolver/shell_begun.rs`, `update/banner/tests.rs`,
  `tests/it/m7_walk/rewrite_stops.rs` (**new**; conductor adds the `mod` line).
- Docs: MAP.md (graft, graft-ui), TESTING.md "Stops during a rewrite" + captions, contact sheet, `docs/shots/m7/`.

Acceptance:
- [ ] `still_to_apply_is_newest_first_with_followers`, `still_to_apply_caps_at_six_then_more`,
  `no_block_when_nothing_is_pending`, `no_ghost_rows_in_the_graph`;
- [ ] `c_in_a_shell_rebase_hands_over_to_git` and `…_cherry_pick_…`, with a fake hand-over as the `M` tests do
  (`banner/tests.rs:484,503,718`); `a_shell_begun_revert_is_still_refused`;
- [ ] the three return landings (finished / stopped again / non-zero) each a test;
- [ ] **Walks** (`m7_walk`, counted, fail over the fence):
  - `rewrite_stop_to_first_file`: after `y`, 0 keys, resolver on the first file; ≤ `noted_keys("rewrite-stop-to-first-file")`.
  - `edit_stop_lands`: 0 keys to the graph at the stopped commit, block drawn.
  - `edit_stop_amend_continue`: fs write, then `space a c` (3); commit contains the edit, rewrite finished. `space`
    is today's graph `space` (keymap §2.3); if one key from where the stop lands can't stage the edit, stop and
    report the count — don't rebind.
  - `abort_from_a_chain_stop`: `⇥ a y`; every branch back, HEAD on his branch.
- [ ] **Captures** (colour unverified):
  - `docs/shots/m7/rewrite-chain-stop-187x50.txt` — **Right:** `REBASING  step 3 of 5 · pick <sha> <subject> ·
    feature-b, branch 2 of 3`; the resolver below.
  - `docs/shots/m7/rewrite-edit-stop-187x50.txt` — **Right:** `still to apply · not commits yet` boxed; two `pick`
    rows; `feature-b (Sara): 2 commits follow, on top`; `▸` on `HEAD · stopped here`; bar `a amend it  c continue
    space stage`.
  - `docs/shots/m7/rewrite-edit-stop-80x24.txt` — **Right:** `REBASING  step 3 of 7` whole; no edit hint; block cut
    to `+N more`.
Size: ~55k.

## Touches (each part checks its own, reports how)
- **M6 banner and resolver** for merge, pull, stash clash: words and focus unchanged. Check: every m6 banner/resolver
  test and capture byte-identical (`check_shot`), or each change listed in NOTES. (07b, 07c)
- **Footer `stop_wordings`:** unchanged. Check: its tests. (07b)
- **`u`/`U` refusal while live:** unchanged; the amend at an edit stop is refused for `u` like any live step. Check:
  its test. (07b)
- **P14 for merge / cherry-pick / revert and conflict stops:** unchanged. Check: existing amend-refusal tests. (07a, 07b)
- **`resume_target` / `live_entries`:** an amend entry landing inside a live rewrite must not hide the rewrite's
  `Pending` entry. Check: `c_after_an_amend_at_an_edit_stop_continues_the_rewrite`. (07b)
- **`pane_c_acts`** (one place; `commit_key` and `view::rows::live_c` read it): now true for shell-begun rebase and
  cherry-pick. Check: `live_c_tests.rs`, `shell_begun.rs`. (07c)
- **Relaunch after kill -9:** reads the saved plan through `progress`. Check: `a_relaunch_mid_rewrite_shows_the_chain`. (07b)
- **Keymap:** §2.6a already amended; no doc edit here. Check: hint-bar tests.
- **Journal/undo:** the edit-stop amend is an ordinary `Amend` entry; the shell hand-over writes none (git's
  operation, begun outside, as M6). Nothing else new.
- **KNOWN_GAPS :545:** closed by the conductor (settled 1), not by a builder.

## Rules / Gates
As rewrite-engine. Workspace-wide gates. Each part commits at its seam.
