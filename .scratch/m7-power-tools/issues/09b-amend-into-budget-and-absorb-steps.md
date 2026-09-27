# 09b amend-into-budget-and-absorb-steps — 09's two open boxes and its two questions

Status: done (builder 2026-09-28; perf number provisional under load)
Build with: Opus. Why: a measurement, then a fix picked from a fixed decision tree; a menu reorder; words. Every
part is judged by a named test, an oracle or the perf script. No new signature → not Fable.
Needs: none new. Calls what 09 merged (`update/amend_into.rs`, `jobs/rewrite.rs`, graft-ui
`ContextMenu::for_working_tree(staged)`) and graft-ops `rewrite::{plan, dependents, line_of}` (MAP :271-272), all
unchanged in signature.
Blocked by: **03b merged** (holds graft-ops `operation.rs` `sets_aside`/`touches`, `apply.rs` Pull path, the pull
banner, the m7 walk harness). Part C reads the set-aside fact 03b is changing, and Q4's cause may sit in
`operation.rs`. Parts A and B are file-disjoint from 03b: if the conductor wants them sooner, split per § Size.
Scout: none. 09's scout (`scouts/09.md`) and 09's NOTES cover B and C; A's profile (step A1) is its scout.
Source: 09 NOTES (open questions 1–4), `docs/perf/09.txt`, note journeys 2b/7 (:76-82), R2 (:535-577), fence :822;
keymap §2.1 (:158-167); ADR-0047 §4; ux-principles §1.3, §1.4 (Hick–Hyman), §2 H4/H5/H8, §3.2.

## Read first
- 09's ticket: "Decided", "Signatures", "NOTES" (all). `docs/perf/09.txt`. `scripts/m7-09-perf.sh`.
- Note `docs/design-notes/m7-power-tools.md` :69-82 (journeys), :535-577 (R2), :815-840 (fence).
- `docs/design/02-keymap.md` §2.1 :108-167 (the `.` menu rules, Checkout's fixed first place).
- `crates/graft-ops/MAP.md` :258-259 (`plan`, `plan_as_run`), :271-282 (`rewrite::plan`, `dependents`, files).
- `crates/graft-ui/MAP.md` :8 (`context_menu`: `for_row` order, `for_working_tree(staged)`, `dim_first`).
- `crates/graft/MAP.md` :226-233 (`amend_into`, `row_tags`), :241-256 (`update::context_menu`, `slots`).
- `docs/design-notes/m6-stash-put-back.md` :150-160 (the put-back row's words table).

## Rulings

### A. Perf: `a` on HEAD adds 39.2 ms through the dependents check (budget 16.7 ms)
- **Budget stays 16.7 ms** for `a` → the right screen (editor or preview). The check is already off the key path (a
  job; `checking what's built on <branch>…` draws at once, H1). The budget measures what he feels: the editor
  arriving. Relaxing it without a named cause is a symptom fix (rule 2).
- **Rejected: open the editor first, swap in the preview if a dependent turns up.** The screen would change under his
  typing (ux §3.2, a mode he didn't choose), and he'd invest in a message before seeing the result (H5, M5 promise).
- **Rejected: a cached "built on HEAD" answer read at the key.** A stale answer sends `a` to today's amend while a
  dependent exists; the dependent is left behind with no preview. Correctness over 40 ms.
- **Step A1 — profile first (builder, not `debugger`):** nothing behaves wrongly, so this is measurement, not
  diagnosis. Extend `scripts/m7-09-perf.sh` → `docs/perf/09b.txt` (release build; header: git version, machine,
  `sysctl -n vm.loadavg` before and after). Phases, each its own lines, from a measurement test the script runs (no
  instrumentation shipped):
  1. the HEAD-path job end to end, as graft calls it;
  2. each graft-ops call that job makes, timed alone, in the order it makes them;
  3. every git process spawned (`GIT_TRACE2_PERF=<file>`; if Graft's spawn strips the env, say so and time spawns
     from the test instead): argv and wall time.
  3 rounds × 7 runs; each round's median and the spread. Builder reads summary lines only.
- **Step A2 — fix the cause the profile names, by this tree:**
  - **(a) The job does more than the dependents check before it can answer `NothingBuiltOnHead`** (for example the
    whole `plan(AmendInto { HEAD })`, whose AmendInto path runs `write-tree` + `commit-tree` before `PlainAmend`,
    graft-ops MAP :271). Candidate only, not verified — name the line in NOTES before fixing. Fix in
    `jobs/rewrite.rs`: call `dependents` first; `plan` only when it returns a non-empty line or a `Fork`. Red first:
    `a_on_head_with_nothing_built_on_it_writes_no_object` (count loose + packed objects before and after `a`).
  - **(b) The time is inside `rewrite::dependents`** (git spawns). Replace them with reads through the `repo` handle it
    already takes; **same signature, same answers**. Keep today's git-spawning body, test-only, as the oracle:
    `dependents_match_the_git_answer_on_every_fixture` over every existing `dependents` fixture plus: a branch at
    the same commit, a branch behind HEAD, a fork, a line of two, a merge between, the default branch built on HEAD,
    a branch checked out in another worktree, a shallow clone (32). Existing `dependents` tests unchanged, green.
    Needs a signature change → stop, `blocked: needs rewrite-engine signature` (Fable's, ADR-0047).
  - **(c) Elsewhere** (job queue wake, runner, graft's answer path): name it with numbers; fix if in Files, else stop
    `blocked: needs <file>`.
  - After the fix, still over 16.7 ms with no further cause in the profile → stop with the phase table. **Never
    relax the budget in this ticket.**
- **Fixture:** HEAD on `feature-a`, `main`, plus one unrelated local branch that is not the default, so the check
  really runs (ADR-0047 §4 excludes the default branch; a pre-filter may already skip on `main` alone — say which
  in NOTES from the line). Also record, **not gated**, one line on the same fixture with 300 unrelated local
  branches. Over 100 ms there → NOTES; the conductor files the gap.

### B. Absorb steps: 9 keys vs the note's 4
- **Ruling: Absorb takes the uncommitted row's first menu place, always.** Nothing staged → it dims **in place**,
  reading `nothing staged` (09's existing dim), as Checkout dims in place on a commit row (keymap :162-164).
  - H4: on commit rows the row's own verbs lead (Checkout 1st, Cherry-pick 2nd, fixed) and the ambient items
    (Fetch/Pull/Push/Undo, `always_items()`) come last (graft-ui MAP :8). Absorb is the uncommitted row's only row
    verb; today it sits under the ambient ones, against that rule.
  - Hick–Hyman (ux §1.4: order by likelihood): Fetch/Pull/Push/Undo each print a live key (`Push (P)`); the menu is
    where he goes for the verb with no key. Name the fifth old item in NOTES and whether its label carries a key.
  - The five keep their relative order, one place down.
- Journey 2b becomes `k . ⏎ y` = **4** ≤ the note's 4 (start: cursor on HEAD, the uncommitted row one above). No
  motion inside the menu.
- **Rejected: a direct key.** Keymap gives none; §2.1 :158 puts rare verbs behind `.`; `A` breaks the UPPERCASE rule
  (:23, places/modes only); `Ctrl+A` is stage-all. A key would be a spec change; none is proposed.
- **Rejected: menu wrap.** Changes every menu, and `k . k ⏎ y` = 5 still misses.
- **Rejected: amending the count to 9.** The note's count is reachable.
- **Cursor on open:** where the commit-row menu's cursor opens when its first slot is dim (same rule, H4). Find
  which (`slots` `dim_first`/`leading`, MAP :252), match it, name the test that pins it.
- **Spec change (text only, no new key) — the conductor applies it at filing, not the builder:** keymap §2.1 :167,
  after "the menu gains `Put staged changes into their commits (absorb)`" insert "in its first place, dimmed in place
  (`nothing staged`) when nothing is staged, as Checkout's on a commit row".
- **Note text (builder, counts unchanged):** :77 `` `.` motion `⏎ y` (absorb) `` → `` motion `. ⏎ y` (absorb; motion
  = to the uncommitted row) ``; :542-543 "`.` on the uncommitted row, motion to `Put staged…`, `⏎`, `y` = 4" →
  "motion to the uncommitted row, `.` (the first item is `Put staged…`), `⏎`, `y` = 4"; :822 comment
  `. motion ⏎ y` → `motion . ⏎ y`.

### C. 09's questions 3 and 4
- **Q3 (ruled): the landing composes, it doesn't replace.** `landed` keeps the put-back words when the rewrite set
  aside unstaged edits and put them back. For a rewrite they read `your unstaged edits are back` (the staged ones
  went into commits, so plain "your edits" would be false). From the same fact `done_status` reads — never by
  matching its string. Shapes (`done_line` order):
  - `put into "Add OAuth callback" ✓ · your unstaged edits are back · u undoes`
  - `put 2 hunks into 2 commits ✓ · 1 stays staged · your unstaged edits are back · u undoes`
  - Nothing unstaged → no edits clause (H8).
- **Q4 (ruled: defect, textual — stands on the captures):** `amend-into-preview-187x50.txt:44-45` and
  `absorb-preview-120x40.txt:35` say `1 file set aside, put back after` and `put-back: not checked (nothing is put
  back after this)` in one box.
  - First establish the truth by the write: `a_put_in_puts_the_unstaged_file_back` (after `y`: the file byte-equal in
    the worktree, index per ADR-0045/30).
  - Then find the row whose source is wrong. **Name file:line in NOTES before fixing** (rule 7). Fix at that source.
  - Right: the two rows agree. A checked forecast reads per the m6 table (:155). If a rewrite's put-back genuinely
    can't be checked, the reason says why, never "nothing is put back" while a row says it is.
  - Cause in graft-ops `operation.rs` (`sets_aside` for `Operation::Rewrite`) or `preview/put_back.rs`: in Files
    (after 03b). Needs a new `Operation`/`SetAside` shape → stop, `blocked: needs <signature>`.
- **Undo line (ruled):** keep the shared `undo: u takes this back` (every preview's line, H4). The note mock's `u
  undoes all of it` was shorthand; the `branches:` rows already show who follows. TESTING.md's Right line for
  `amend-into-preview` changes to match; the capture test asserts it (09's NOTES exception drops).
- No user questions.

## Files (ADR-0051: outside this list → stop, `blocked: needs <file>`)
- graft: `jobs/rewrite.rs` (+ its tests), `update/amend_into.rs` + `update/amend_into/tests.rs`,
  `update/context_menu/tests.rs`, `tests/it/m7_walk/amend_into.rs` (the walk; the measurement test the script runs,
  as the script names it today). Only if Q4's cause is in the draw: `update/preview.rs`, `update/preview/line_rows.rs`.
- graft-ui: `context_menu.rs` + its tests.
- graft-ops (only as A2(b) / Q4 need, only those lines; say which): `rewrite/dependents.rs` + its tests;
  `operation.rs`; `preview/put_back.rs`.
- Docs: `scripts/m7-09-perf.sh`, `docs/perf/09b.txt`, the note lines in B, TESTING.md rows, `docs/shots/m7/`, MAP.md
  (each crate touched). **Not** `02-keymap.md` (conductor).

## Signatures
None new. Unchanged and asserted by compile: `for_working_tree(staged: bool) -> ContextMenu`; `rewrite::dependents`
(`(repo, git, branch, default_branch: Option<&RefName>, stop)`, MAP :272); `RewritePlanned`, `DoneWords`,
`amend_into::{a_key, absorb_chosen, planned, landed, tags_resolved}` (09). A2(b)'s oracle is `#[cfg(test)]` only.

## Acceptance (each tick names its test; red quoted in NOTES)
- [x] **A. Perf** — `scripts/m7-09-perf.sh` → `docs/perf/09b.txt`, phase table present; summary line
  `m7-09b summary: a to the amend editor, median of 3×7: <x> ms through the dependents check (rounds <a>/<b>/<c>),
  <y> ms today's amend; the check adds <z> ms (budget 16.7 ms); load <before>/<after>` with **z ≤ 16.7**, quoted in
  the tick. Cause named in NOTES with its phase and ms, and which branch of A2. 300-branch line recorded.
  **Tick (test `m7_walk::amend_into::perf_dependents_check` + `perf_dependents_spawns`, via the script):**
  `m7-09b summary: a to the amend editor, median of 3×7: 6.9 ms through the dependents check (rounds 10.4/6.9/5.9),
  0.1 ms today's amend; the check adds 6.8 ms (budget 16.7 ms); load { 7.06 15.04 23.15 }/{ 6.78 14.06 22.46 }`.
  Provisional (machine loaded; conductor re-runs quiet). 300-branch line: 12.3 ms median.
- [x] A2(a)-shaped, not (a)'s candidate: `a_on_head_with_nothing_built_on_it_writes_no_object` — **green on arrival,
  not red**: it refutes the candidate (see NOTES). The real fix's test: `jobs::rewrite::tests::the_git_version_is_read_at_most_once_per_process`
  (red: `cannot find function git_version`). A2(b) not taken; `dependents` untouched.
- [x] **B.** graft-ui: `absorb_is_the_first_item_on_the_uncommitted_row` (red first: index 5),
  `absorb_dims_in_first_place_with_nothing_staged`, `the_uncommitted_row_menu_keeps_its_old_order_below_absorb`
  (09's `the_uncommitted_row_menu_is_unchanged`, renamed and extended, not rewritten). graft: the cursor-on-open
  test named. Walk `absorb_staged` = `k . ⏎ y`, `len(keys) <= noted_keys("absorb-staged")` (red first: the
  landing assertion under today's order; 09's "exactly 9" dropped); 2 hunks placed, 1 still staged; `u` restores
  refs and index.
  **Tick:** graft-ui `context_menu::tests::{absorb_is_the_first_item_on_the_uncommitted_row,
  absorb_dims_in_first_place_with_nothing_staged, for_working_tree_keeps_its_old_order_below_absorb}`; graft
  `update::context_menu::tests::the_uncommitted_row_menu_keeps_its_old_order_below_absorb` (renamed, extended to
  staged/unstaged), cursor-on-open `update::context_menu::tests::the_uncommitted_row_menu_opens_on_absorb`; walk
  `m7_walk::amend_into::absorb_staged` asserts route `["k", ".", "⏎", "y"]` and `within("absorb-staged")`, 2 hunks
  placed, README still staged, `u` refs+index. Walk was **not** red first (edited after the menu moved; see NOTES).
- [x] **C.** `a_put_in_over_unstaged_edits_says_they_are_back`, `absorb_over_unstaged_edits_says_they_are_back`
  (both red first), `a_put_in_with_nothing_unstaged_says_nothing_about_edits`, `a_put_in_puts_the_unstaged_file_back`,
  `the_edits_rows_agree_in_the_put_in_preview`, `the_edits_rows_agree_in_the_absorb_preview` (red first: the lines
  quoted above). **Tick:** the first three in `update::amend_into::tests`, the last three in
  `m7_walk::amend_into`; reds quoted in NOTES.
- [x] **Captures** (colour unverified) — `m7_walk::amend_into::{capture_absorb, capture_amend_into}` re-rendered
  and assert the Right lines; captions tsv updated. No m7 contact sheet exists yet to reassemble (only the
  captions file) — left for the acceptance ticket:
  - `docs/shots/m7/absorb-menu-120x40.txt` (new). **Right:** the cursor on `Put staged changes into their commits
    (absorb)`, first; the five old items below it in their old order.
  - `amend-into-preview-187x50.txt`, `absorb-preview-120x40.txt` re-rendered. **Right:** one statement about his
    edits, no `nothing is put back` beside `set aside`; `undo: u takes this back`.
  - `amend-into-landed-120x40.txt` (fixture with one unstaged file). **Right:** status
    `put into "Add OAuth callback" ✓ · your unstaged edits are back · u undoes`.
  - Contact sheet reassembled.
- [x] Gates workspace-wide: `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`,
  `cargo nextest run --workspace` (4545 passed, 5 skipped), `cargo test --workspace --doc` — all pass.

## Touches (check each, report how)
- **The uncommitted row's `.` menu:** mouse `item_at` clicks, `the_menu_runs_what_the_dropped_keys_run`,
  `every_verb_the_menu_advertises_is_live_while_it_is_open`, any walk doing `. j…` on the uncommitted row (grep
  `m*_walk`), any capture showing that menu (grep `docs/shots` for its item labels; re-render and list each).
- **Commit-row menus:** unchanged. Check: their tests and captures byte-identical.
- **`dependents` callers** (`plan`, `plan_as_run`/`line_of`: `r`, `R`, `a`, absorb): their tests unedited, green;
  the m7 rewrite walks green.
- **The put-back row** (every preview that draws it: stash, `o`, rebase, pull after 03b): a fix at a shared source
  changes them. Check: their captures byte-identical, or each change listed with why.
- **Landing words:** m7 walk landings, TESTING.md rows, 24's expectations (grep `put into` in 24's spec).
- **Journal/undo:** A2(a) removes a pre-check object write, adds none. Check: `u` steps in the three walks.
- **Keymap:** text only, by the conductor (B). No key bound, none freed.
- Out: the `subject_truncated` gap (09 NOTES) stays a gap.

## Size
~70k. Split line if over or if A2(b) is the cause: **09c** = A2's fix (the in-process `dependents` + oracle).
09b keeps A1 (profile + named cause), B and C. If the conductor wants A and B before 03b merges: A+B as 09b, C as
09c after 03b.

## NOTES (builder, 2026-09-28)
- **Did not compact.** Stalled once on the stream watchdog mid-ticket; resumed from `git status`, nothing lost.
- **Scout:** none new, per the ticket; 09's `scouts/09.md` and 09's NOTES used. A1's profile was A's scout.
- **A — cause (phase table, loaded run, load ~24–32):** `a` on HEAD made two git spawns before answering:
  `system_git()` at `jobs/rewrite.rs:88` (`git --version`, phase 2: 14.5 ms wall, 2.0 ms git's own clock) and
  `rewrite::dependents`'s `for-each-ref --contains` (`dependents.rs` `local_branches`, phase 2: 17.4 ms wall, 5.3 ms
  git's clock). Sum ≈ the 30.2 ms the check added. The rest is spawn overhead on a loaded macOS, not git's work.
- **Which branch of A2:** (a)'s candidate is **refuted**: `jobs/rewrite.rs:95-103` asks `dependents` first and returns
  `NothingBuiltOnHead` before `planned` → `rewrite::plan` is never reached; pinned by
  `a_on_head_with_nothing_built_on_it_writes_no_object` (green on arrival). The real extra work before the answer was
  the version spawn, fixed in `jobs/rewrite.rs` (in Files): `git_version()`, read once per process into a `OnceLock`,
  as graft-git blame's `SYSTEM_VERSION` already does (`blame/marks_pass.rs:84`). **Cause fix**, cost: the first plan
  of a session still pays one `git --version`; a git upgraded under a running Graft is not noticed until restart
  (same as blame). After it: one spawn per `a` (phase 3: only `for-each-ref`), check adds 20.8 ms loaded /
  **6.8 ms at load ~7** — under budget, provisional. A2(b) (the `for-each-ref` spawn) not needed at this load; if the
  quiet re-run is over, that spawn is the remaining cause → 09c.
- **Broader (for the conductor, not done):** `jobs/preview.rs:51`, `operate.rs:313`, `undo.rs:339`, `conflict.rs:288/380`
  each spawn `git --version` per job (≈5–15 ms each). A shared cached `system_git` in graft-git would serve all; out of
  my Files.
- **Pre-filter:** none skips on `main` alone: `update/amend_into.rs:146` counts any non-current branch, `main`
  included; the default branch is excluded only inside `dependents` (`dependents.rs:85-89`). So `main` + `feature-a`
  already ran the check.
- **300 branches:** 12.3 ms median (54.5 ms loaded) — under 100 ms; no gap.
- **B:** fifth old item = `Open details (⏎)` (the other four are the ambient `always_items()`); its label carries `⏎`.
  Cursor on open: `for_row` opens at 0 even on a dim Checkout (`graft-ui context_menu.rs` `for_row` `cursor: 0`,
  pinned by `enter_on_the_dim_checkout_slot_keeps_the_menu_and_says_why`); `for_working_tree` matches (cursor 0), pinned
  by `the_uncommitted_row_menu_opens_on_absorb`. Red (graft-ui): `left: OpenDetails right: Absorb` (first item),
  `left: [Fetch, Pull, Push, Undo, Absorb] right: [OpenDetails, …]`. Walk `absorb_staged` **not red first**: I edited it
  after the menu moved.
- **Q4 — source, rule 7:** `your edits: N files set aside` is true (`Operation::sets_aside` wraps a Rewrite,
  `operation.rs:1315`; `apply.rs:370-383` sets aside and puts back around it; `a_put_in_puts_the_unstaged_file_back`
  proves the write). The false row: `graft-ops/src/preview/put_back.rs:157-160` (before the fix) — `landing_of` had no
  `Operation::Rewrite` arm, so a rewrite fell to `_ => "nothing is put back after this"`. Fixed at that source: a
  Rewrite arm lands on `made[0]` (`a`: the commit written from the index, parent HEAD) or `index_base` (absorb: HEAD +
  placed hunks) when the replay forecast is clean; `the rewrite may stop first` when not; an `R`-editor rewrite (neither)
  says `the rewritten line is not built before y`. `operation.rs` **not edited**. Red: `the rows disagree: ["your
  edits: 1 file set aside, put back after", "your edits' put-back: not checked (nothing is put back after this)"]`
  (absorb: `3 files`). Now the box reads `no conflicts expected` (m6 table row 1). Approximation stated: the landed tip
  is taken as HEAD + what went in, exact when the replay is clean.
- **Q3 — deviation to flag:** ruling said "from the same fact `done_status` reads". That fact
  (`entry.put_back.why == SetAside`, `clash.rs:427`) is always true for a put-in (it always has staged files), and
  `landed`'s unchanged signature (`operate.rs:1076` passes only the op) cannot see it. The fact that decides the
  words is whether *unstaged* tracked edits were set aside: recorded in `Asked` when the plan answers
  (`amend_into.rs` `has_unstaged`), the same tree the preview's set-aside row counts. No string matching. Edits made
  between the preview and `y` are not counted (words omit, never claim). Reds: `left: "put into \"commit 2\" ✓ · u
  undoes" right: "… · your unstaged edits are back · u undoes"`; absorb `left: "… ✓ · 1 stays staged · u undoes"`.
- **Touches:** uncommitted-row menu — graft lib tests (mouse `item_at`, `the_menu_runs_what_the_dropped_keys_run`,
  `every_verb_the_menu_advertises_is_live_while_it_is_open`) green unedited; walks grep'd (`on_working_tree` in
  m3_walk/hint_bar/conflict_interim) green; only m7 `absorb_staged`/`capture_absorb` did `. j…`. Captures showing a
  `.` menu under `docs/shots` (m5 verbs, m6 fixed-slots, fix-radar) are all commit-row menus (contain `Copy SHA`) —
  unmodified (`git status`). Put-back row: the fix adds a Rewrite arm only; stash/rebase/checkout/cherry-pick captures
  unmodified (`git status` shows only the three m7 shots + the new menu shot). `dependents` callers: untouched.
  Landing words: TESTING.md rows updated; 24's spec has no `put into`/`edits are back` (grep). Journal/undo: no write
  added or removed (A2(a) had nothing to remove); `u` asserted in `fix_into_older`, `absorb_staged`,
  `amend_with_dependents`. Keymap: not edited. Note: :77, :542-543, :822 edited as ruled; counts unchanged.
- **Perf script:** measurement lines redact the fixture's temp path as `<repo>` (`published_repo` test flagged it).
- **Out:** `subject_truncated` stays a gap.
