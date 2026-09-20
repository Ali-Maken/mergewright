# banner-visible-and-honest: a modal banner the user cannot see, and a hint bar that will not name its keys

Status: ready
Blocked by: 14 (merged)
Build with: **Opus.** Two judgement calls: where a modal banner lives in the
sub-100-column tab layout, and whether `bound_actions` needs to know its
context. Neither is a find-and-replace.
Crates: **`graft-ui` and `graft`** — both, one builder. Flipping either side
alone fails a test; and the capture defects and the hint defects are the same
screen.
Scout: **not needed.** Ticket 14 §"Found on the way, NOT fixed" names both
offending functions and why they agree. `docs/reviews/m5-09-app-conflict-interim.md`
(`c1fa5df`) names C1 and M1–M4 with file and line. Read those two, not the tree.

## What this ticket is

Group 09's review returned 1 Critical and 5 Majors, all on one screen: the
operation banner. Ticket 14 independently found a sixth. They share files, so
they are one group.

## 1. Critical — the banner is not on screen at 80 columns

`docs/shots/m5-ui-banner-conflict-80x24.txt` on `main` **shows no banner**.
Below 100 columns the layout switches to tabs (DESIGN §1.1), the detail pane
takes the whole screen, and the banner goes with it. A conflict stops the
operation, the banner takes focus, it is **modal** — it owns the keyboard —
and the user cannot see it or any of its keys. `c`, `s`, `a` are live and
invisible.

At 160x48 it truncates to `⚠ MERGING — …`: the button slot is reserved at
≥160 and no buttons are drawn. **Only 120x40 delivers.** Group 09 ticked this
acceptance box; the tick was false, which is Critical whatever its size.

**Conductor's ruling, and the reason for it:** *a modal overlay must be
visible wherever it has focus.* If it claims the keyboard it is on screen.
So in tab layout the banner draws **above the tab line**, outside the pane —
it is not a pane's content and must not travel with one. This is ADR-0039's
principle applied to the thing ADR-0039 did not cover: guarantee the small,
always-needed marker rather than choosing between everything and nothing.

The cost is real and I am accepting it deliberately: 1–3 rows of pane content
below 100 columns, **only while an operation is stopped**. Say in your report
exactly how many rows it cost, because the user judges that by eye.

Fix the ≥160 truncation too: either draw the buttons in the slot that is
reserved for them, or do not reserve it.

## 2. The hint bar will not name the banner's keys

The footer under a focused modal banner reads `<> resize  . more` and mentions
neither `c` nor `a`. Two functions are wrong in the **same direction**:

- `graft_ui::graph_screen::hints::banner_list` marks every entry
  `HintState::Planned`; a planned hint is dropped before the width rule runs.
- `graft::update::keys::bound_actions()` omits `c continue`, `s skip`,
  `a abort`.

The two tests written to catch this — `update::tests::every_hinted_key_is_bound`
and `every_planned_hint_is_still_unbound` — **both pass**, because each is
checked against the other.

**Read that twice.** Green tests are not evidence (rule 1). Part of your job is
leaving a test anchored to *what `banner_key` actually binds*, not to the other
table.

## 3. Majors from the review — read it for the detail

- **M1** Tab cycles Diff↔banner; the graph is unreachable while the pane is
  open (`banner.rs:202-210`, `168-173`).
- **M2** `M` and `e` are dead keys when nothing is unmerged, and the hints
  advertise them anyway. Related to item 2: the changes pane hints the same
  letters for different verbs and `bound_actions` cannot tell two contexts
  apart. Decide it properly — a context parameter or a per-context table is a
  signature change **inside your own two crates**, which is allowed. What you
  may not do is make the tables agree by weakening a test. If you cannot bind
  `M`/`e` truthfully in both contexts, leave them `Planned`, say so, and file
  it: unverified is not broken, **but a hint that lies is**.
- **M3** `finishing_clears_the_focus` feeds `Failed` and never asserts focus.
- **M4** `banner_is_modal_when_focused`'s `Ctrl+C` assertion is vacuous — the
  status is already `Sticky`. Both are tests that cannot go red. Fix them so
  they can, and show the mutation that proves it.

`docs/design/02-keymap.md` §2.6a already names every key here. Bind nothing the
doc does not name; you should need no spec change.

## Acceptance (every tick names its verifying test inline, at tick time)

- [ ] The banner is on the **drawn frame** at 80x24, 120x40 and 160x48, with
      its keys — asserted on the frame, not on a view model — test:
- [ ] At ≥160 the buttons are drawn, or no slot is reserved — test:
- [ ] The footer names the banner's keys while it has focus — on the drawn
      frame — test:
- [ ] A test that goes red if `banner_list`/`bound_actions` drift from what
      `banner_key` binds, rather than from each other — test:
- [ ] Tab reaches the graph with the pane open (M1) — test:
- [ ] `M`/`e` are hinted only where they are live (M2) — test:
- [ ] M3 and M4's tests can now go red — test:
- [ ] The `. more` doorway still draws at every width (ADR-0039) — test:

**Mutation-check each.** Two that matter most: revert `banner_list` to all
`Planned`, and revert the banner to drawing inside the pane. If either turns
no test red, you have rebuilt the original bug and the ticket is not done.

## Re-capture

All three `docs/shots/m5-ui-banner-conflict-*.txt`. Their headers carry the
hint-bar defect as known — **delete that paragraph once it is true**, as
ticket 14 did with its own. ≤ 200 KB each. Colour unverified (rule 5).
**Do not tick the visual result** — the user judges by eye (rule 6), and at
80 columns this capture is the whole point of the ticket.

## Gates

Workspace-wide, never `-p`. `CARGO_TARGET_DIR=/tmp/graft-target-g16`, deleted
before hand-back. Baseline **2156/2156** on `main`. Commit, never push.

## Report

Cause or symptom per fix. How many rows the banner costs below 100 columns.
What your new test is anchored to — the existing pair was anchored to each
other, and that is exactly why this shipped.
