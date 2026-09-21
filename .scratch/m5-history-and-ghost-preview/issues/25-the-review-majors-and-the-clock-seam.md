# the-review-majors-and-the-clock-seam: five Majors, a hole in the clock rule, and two approved wording changes

Status: ready
Blocked by: none — **`graft` only.** Ticket 24 holds `graft-ui` and `graft-git`
and runs alongside you. Need either one: commit at the seam, report
`blocked: needs <crate>`, stop. Never wait in place.
Build with: **Opus.** Two of these are silent-wrong-answer shapes on the undo
and push path, which is the failure class this project ranks above all others.
Scout: **not needed.** `docs/reviews/m5-14-to-22-eye-check-round.md` names every
line. Read that file's Major section, not the tree.

All five Majors are from that review. It found **0 Critical and 22 of 22 ticks
holding**, so these are the milestone's real remainder, not noise.

## M1 — the tab banner's rows are unknown to the pure geometry (the fourth instance, vertical)

- `view/frame.rs:436` — `split_for_detail` calls `draw_tab_banner`, taking one
  row above the strip and a second at the bottom while focused
  (`frame.rs:483-499`), **before** `split_area` runs.
- `update/geometry.rs:89-100` — `detail_split`'s `Tab` arm still puts content at
  `y: strip` with `height: size.1 - strip`. The banner rows are not subtracted.
- `update/state.rs:277-288` — `graph_rows()` sums `CHROME_ROWS`,
  `banner_rows()`, the commit-graph line and `preview_rows_taken`. The tab
  banner's row is not a term.

`geometry.rs` exists to mirror the view for hit-testing — its own doc says so.
With a merge stopped at 80 columns and the Detail tab shown, **every click lands
one row above the row the user sees** (two while focused), and on the Graph tab
the cursor can be moved onto a row the graph never drew.

**This is ticket 19's principle applied vertically** — count from what you were
given, not from the screen. Three instances were width; this is the fourth, in
height. The reviewer was asked to look for a fourth in width, found none, and
found this instead.

The review proposes `update::banner::tab_rows(state, width) -> u16` (0/1/2 by
`layout == Tab`, `banner_view`, `banner_focus`) used in both places. Take it or
better it — **say which and why**. The test it proposes is a
`pane_boundaries.rs`-style sweep asserting `geometry::detail_split` equals the
rects `split_for_detail` actually drew, with and without a focused banner. That
test is the more valuable half of this piece: it is the class test for the
vertical axis.

## M2 — on the Graph tab under 100 columns the banner draws twice, keys included

- `view/frame.rs:436` — the tab banner draws for `Tab { shown }` whichever side
  is shown.
- `view/rows.rs:40,57` — `banner_op` goes into `GraphView` unconditionally and
  `graph_screen::layout.rs` draws it on the header row; `rows.rs:49` also feeds
  `footer_context` the banner, so the graph's hint bar draws the banner keys.

So at 80 columns with a live operation, the Detail pane open and the **Graph**
tab shown: row 0 is the tab banner, row 2 carries the same banner text, and
while focused the graph footer and the row under it both list
`c continue  a abort …`.

**Read from the code, not observed** (the reviewer says so — rule 7). Confirm it
before fixing. **No test covers this tab at all**: `conflict_interim.rs` has no
case with the Graph tab shown under a banner, and
`the_tab_layout_banner_costs_one_row_drawn_and_two_focused`
(`conflict_interim.rs:1142`) locates the pane by `"UNCOMMITTED"` (`:1162`) —
the Detail tab only. **Ticket 16's "drawn once, at every width" tick is
therefore verified for one tab of two.** That is not a false tick, but it is a
tick narrower than it reads; say so in your report.

Add the missing test whichever way you fix it.

## M3 — the keymap and the code disagree about a focused banner and `u`

- `docs/design/02-keymap.md:245-246` — "**`u` and `U` are refused while an
  operation is live**, focused or not: `finish or abort the <kind> first ·
  ⇥ banner`".
- `update/banner.rs:246-248` — with the banner focused, `u`/`U` fall into
  `_ => {}` and `banner_key` returns `true`; **nothing is drawn.** `keys.rs:78`
  runs `banner_key` before the undo path, so `undo.rs:84`'s
  `in_progress_refusal` — which draws exactly the doc's words — is reached only
  when the banner is *not* focused.

Ticket 15 wrote that doc line and ticked "doc and code agree"; its test drives
the unfocused case only.

**Fix it in the code, not the doc.** The user pressed `u` on purpose and got
nothing back — a key that silently does nothing is the thing CLAUDE.md's status
rule exists to prevent. The focused arm answers with the same `Status::Sticky`.
Minor 9 asks for the swallow arm's comment (`banner.rs:242-246`) to say the
swallow is silent and point at §2.6; if you fix the swallow, rewrite that
comment to match rather than deleting it.

## M4 — `u` after a push of **another** branch names the wrong branch

- `update/remote.rs:527-535` — the push filter is
  `kind == Push && status == Committed && id > target.id`. **Nothing checks the
  push touched the undone entry's branch.**
- `update/remote.rs:814-832` — `pushed_since` uses the same filter, and
  `pushed_to(push)` (`:836`) names *that* push's remote side.

Scenario: commit on `feature`, push `main`, press `u`. The box opens saying
`commit "…" is on origin/feature` — false, nothing on `feature` was pushed.
After a local undo the status reads `origin/main still has it` — also false.

**The error is conservative** (it asks when it need not; it never skips an ask),
so there is no data-loss risk. **The words are still wrong, and M5's promise is
"you see the result before you confirm".** A confirm box that misnames what
happened is the promise failing quietly.

This matters more than its severity suggests: the user works on **dependent
branches in a team**, so "committed here, pushed there" is his normal day, not
a corner case.

Fix: filter on the undone branch's tracking ref (`entry.touches.refs`, or the
push's own `branch` field) and derive `remote_side` from the **matching** push.

## M5 — a failed remote read silently becomes a plain undo (rule 3)

- `update/remote.rs:517` — `let facts = facts(state).ok()?;`. `facts()`
  (`:237-245`) returns `Err` for `Loading::Failed`, `Idle` **and** `Pending`.

When the remote facts failed to load — `git remote show` refused, network down —
`undo_after_push` answers `None` and `undo_key` proceeds **as if no push had
happened**: no box, no `still has it` line. An error standing in for "not
pushed" is rule 3 exactly, and it is on the one path where being wrong costs the
user a push they did not know about.

**The journal already knows the push happened** — the filter at `:527` needs
only the journal. Only `old_tip`/`expected` need the facts.

Two honest answers; pick one and say why:
1. Open the box anyway, with `Re-push old tip` disabled and a line saying why
   (`remote not read: <error>`).
2. Refuse `u` with a sticky naming the failed read.

(1) keeps the user moving and tells the truth; (2) is safer and more annoying.
**Note `Pending` is not `Failed`** — a read still in flight is not an error, and
whatever you choose must not turn "still loading" into a scary line.

Test: `Loading::Failed` facts + a committed push → `u` does not undo silently.

## The clock seam — decided by the conductor, from ticket 23

`jobs/operate.rs:270` and `jobs/undo.rs:297` read `SystemTime::now()` directly
for the journal timestamp.

Ticket 23 reported this as contradicting `session.rs:44`. **It does not** — that
doc scopes itself to "the date path", and a journal timestamp is not the date
path. **The defect is that the rule's scope leaves a hole and captures fall into
it.** Three captures whose repository the app writes mid-capture cannot be
pinned test-side: the m3/m4 walk examples, `remote_by_key`, and the preview
time-machine. Pinning render-only would draw them as a literal `future`
(`columns.rs:61-63`).

**One seam fixes all three.** Route the journal timestamp through the same
`Clock` the date path uses, and widen `session.rs:44`'s doc so it describes the
whole rule rather than one path.

**Do not regenerate the walk captures** — that is group 11's, and doing both at
once makes the diff unreadable. This piece makes them pinnable; it does not pin
them.

## Two approved wording changes

- **`(forecast)` breaks the trailing-paren rule.** `update/preview.rs:877,879`:
  `"no conflicts expected (forecast)"` and `"may conflict (forecast): …"`.
  `02-keymap.md` §2.1, extended 2026-09-21: *trailing parens carry a key, not a
  word — anywhere the user might press what's inside them.* And `forecast` is
  our word, not the user's (CONTEXT.md, "These are our words"). **Drop the
  parenthetical** — the line already says what it is. If dropping it loses
  something real, say what, and propose rather than decide.
- **Blank the re-push command from the first box** (user's decision, 2026-09-21).
  The undo-after-push box's default answer is `Cancel` and its second is `Undo
  locally`; showing the dangerous command beside a safe default reads as "Enter
  runs this". The command belongs in the **second** box — the lease confirm,
  where it is what you are about to run. Check it is actually there before
  removing it from the first.

## Minors from the same review, in your crate

- **Minor 1** — `tests/it/hint_bar.rs:238`: `assert!(drawn > 4)` at 160×48 would
  still pass if half the write keys vanished. Assert the named keys.
- **Minor 2** — `update/context_menu/verbs/tests.rs:479`:
  `const LIVE_KEYS: [&str; 2] = ["m", "r"]` is hand-kept, so a new `(x)` verb
  that is not live passes until someone edits the list. Derive it from
  `bound_actions`/the menu.
- **Minor 8** — `tests/it/conflict_interim.rs:1142-1175`: nothing asserts the
  pane's height shrank by one while focused. Add
  `pane_bottom(&focused) == pane_bottom(&unfocused) - 1`.

## Acceptance (every tick names its verifying test inline, at tick time)

- [ ] `detail_split` and `graph_rows()` account for the tab banner's rows; a
      click lands on the row the user sees, with and without focus, under 100
      columns — test:
- [ ] The geometry sweep asserts `detail_split` equals the rects
      `split_for_detail` drew — the class test, swept, not two sizes — test:
- [ ] The banner and its keys draw once on the Graph tab under 100 columns —
      test (name the one that covers the **Graph** tab):
- [ ] A focused banner answers `u`/`U` instead of swallowing them — test:
- [ ] A push of another branch does not make `u` name that branch — test:
- [ ] A failed remote read does not turn `u` into a silent plain undo, and a
      *pending* read is not reported as a failure — test:
- [ ] The journal timestamp comes through the `Clock` seam — test:
- [ ] No user-facing string says `forecast`; the first box shows no push
      command — test:
- [ ] Minors 1, 2, 8 — test:

**Mutation-check each**, per the section below. The two that matter: restore
the unfiltered push filter (M4) and restore `facts(state).ok()?` (M5), and name
the tests that convict you. If none does for either, the tick is false —
**Critical**.

## How to run a mutation check

- **Per mutation, run only the tests that tick names**
  (`cargo nextest run -E 'test(<name>)'`). Seconds, not a minute. Narrowing
  cannot produce a false green.
- **Once**, on the mutation that restores the actual reported fault, run the
  full workspace suite and report the blast radius — "n red, and only these n".
- **Never run the same suite twice to parse it twice.**

## Gates

Workspace-wide, never `-p`. `CARGO_TARGET_DIR=/tmp/graft-target-g25`, deleted
before hand-back. Baseline **2226/2226** — check it yourself.

**Never `git checkout` or `git restore` across your worktree** — a builder
destroyed five uncommitted files that way this milestone.

**Captures stay under 200 KB each.** Commit, never push.

## Report

Cause or symptom per fix. Which answer you took for M5 and why. Whether M2
reproduced as the reviewer read it. Anything unverified — which is not broken.
