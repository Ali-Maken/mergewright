# say-it-once-and-name-the-outcome: the banner repeats the pane, and "ghost" is a word the UI never defines

Status: done
Blocked by: 18 (`graft`) and 19 (`graft-ui`) — this holds both crates
Build with: **Opus.** Wording the user will read under pressure, plus a
legibility change he has to be able to judge by eye.
Crates: `graft` and `graft-ui`
Scout: **not needed.** Every defect below is quoted from a capture the user
read, with the file it came from.

All of this came from the user's own eye-check of the five M5 captures. Where
a ruling is mine it says so; where the fix is his it says so, and it is not
yours to redesign.

## 1. The banner says what the pane already says — his fix, take it

`docs/shots/m5-ui-banner-conflict-80x24.txt`: the banner reads
`conflict notes.md (+1)`, and four rows below it the pane reads
`CONFLICTS (2)` with both filenames. **The same fact twice on a 24-row
screen.**

Shrink the banner to the state and let the pane name the files. **Keep the
keys row** — a modal that owns the keyboard has to be visible (that was
ticket 16's whole point, do not undo it).

## 2. Drop the `⚠` — conductor's ruling

He asked whether `⚠` earns its place, noting the caption claims "nothing on
the screen reads as an error" while the first character on screen is a warning
glyph. **It does not earn its place. Remove it.**

CLAUDE.md: *a status field must never read as an error when nothing is wrong.*
A stopped merge is **a state to finish, not a failure** — the user chose to
merge, and git stopping on a conflict is the expected middle of that. The word
`MERGING` plus the keys row already says the state, and the banner is
visually distinct by position.

Keeping `⚠` for things that are genuinely wrong is what preserves its force.
If you find `⚠` used elsewhere for real faults, that is the argument for this
change, not against it — say where in your report.

## 3. One wording everywhere — defect 3

The banner drifts: `conflict in notes.md` at 120, `conflict notes.md` at 80
and 160. One wording at every width. With item 1 this mostly dissolves, but
whatever produced two spellings is a cause worth finding and naming.

## 4. The ghosts do not read as an outcome — his diagnosis, one of his fixes

`docs/shots/m5-ui-preview-scrolled-*.txt`. In his words: the five ghost
subjects are identical to the five real rows directly above them, no separator
and no label. Strip colour and the only cue is `○` vs `●` and two em-dashes.
**A first read of "these are in my history twice" is available — the
duplicate-commit confusion M2 exists to prevent.**

What already works, and he said so: **the `[◆ topic]` badge moving onto the
newest ghost.** That is the one cue that says *outcome* rather than *extra
rows*. Build on it; do not weaken it.

He offered three fixes and any one would do: label the block (`after rebase`),
indent it, or change `topic e7e39a7 → ghost` to `topic e7e39a7 → 5 new
commits`.

**Conductor's pick: do the label *and* the summary line, not the indent.**
Reason: the label names the outcome and creates the separator whose absence is
the actual complaint, and the summary line kills the word `ghost` on screen.
Indentation alone is a subtle cue doing a job that needs an explicit one.

**And the deeper point, which is his: `ghost` is our word.** It appears
nowhere else on screen; the user sees `○` and `—` and has to connect them to a
noun the UI never defines. It is a `CONTEXT.md` glossary term that leaked into
the interface. **Keep `ghost` in code, tickets and ADRs; never show it to the
user.** Sweep the user-facing strings for it, not just this one line.

## 5. Captions that are not true — defects 4 and 5

- The banner caption claims `CONFLICTS (2)` sits above `STAGED` and
  `UNSTAGED`. The capture shows `reading…` and neither: **taken mid-load.**
  Make the capture wait for the pane to settle, or stop claiming it. Waiting
  is the right fix — a capture of a half-loaded screen is weak evidence for
  everything else on it too.
- The scrolled caption states `history.top is 14`. True at 160x48, where the
  first visible row is redesign step 10. **At 80x24 the first visible row is
  step 00, so top is 24 there.** Per-width caption — the same fix the context
  menu captions already took.

## 6. Recapture 80x24 with something behind it — his request

He could not judge the banner's row cost from the current capture: *"the pane
holds four rows of content and about thirteen blanks, so the banner crowds out
nothing."* Recapture at 80x24 **with the graph tab and a real history behind
it, or with a conflict set long enough to fill the pane.** A capture that
cannot show the cost cannot answer the question it was taken for.

## Acceptance (every tick names its verifying test inline, at tick time)

- [x] The banner names the state, not the files; the pane names the files —
      test: `graft_ui::graph_screen::tests::the_banner_names_the_state_and_never_the_files`
      (unit) and `it::conflict_interim::the_banner_and_its_keys_are_on_the_frame_at_every_width`
      (real repo, real frame: the banner line holds no filename and no
      `conflict`, and the pane holds `CONFLICTS (2)`).
- [x] No `⚠` on the banner; the keys row is still there at every width —
      test: `graft_ui::graph_screen::tests::the_banner_carries_no_warning_glyph`
      and, for the keys row at 80/120/160,
      `it::conflict_interim::the_banner_and_its_keys_are_on_the_frame_at_every_width`.
- [x] One banner wording at 80, 120 and 160 —
      test: `it::conflict_interim::the_banner_reads_the_same_at_every_width`
      (the drawn frame at all three, wordings compared to each other) and
      `graft_ui::graph_screen::tests::the_banner_is_worded_the_same_whatever_the_header_it_lands_in`.
- [x] The ghost block carries a label, and the summary line names commits
      rather than `ghost` —
      test: `it::preview::the_predicted_block_is_labelled_and_named_in_the_users_words`
      (drawn frame: `after rebase` on its own row, `5 new commits` in the box,
      no `ghost` anywhere, and the rows the label heads still on screen).
- [x] No user-facing string anywhere says `ghost` — a sweep, not a spot
      check — test: `it::preview::no_user_facing_string_says_ghost`, which
      walks every non-test `.rs` file under `crates/graft/src` and
      `crates/graft-ui/src` and checks every string literal, skipping
      comments, attributes and developer assertions.
- [x] The `[◆ topic]` badge still lands on the newest ghost —
      test: `it::preview::the_branch_badge_still_lands_on_the_newest_predicted_row`
      (asserts the badge row is `label + 1` and carries `finish the topic`).
- [x] Captures wait for the pane to settle rather than capturing `reading…` —
      test: `it::conflict_interim::capture_banner_for_docs_shots` (#[ignore]d,
      run by hand), whose own assertion is `!body.contains("reading…")` at
      both wide sizes; the fix is `open_changes_pane` pumping `detail.diff`
      as well as `detail.files`.
- [x] Each caption's stated `history.top` matches its own width —
      test: `it::preview::capture_scrolled_preview_for_docs_shots`
      (#[ignore]d), which opens a session per size, takes `top` from that
      session, and asserts the two tops differ — a single number cannot be
      true of both captions.
- [x] (Item 7) The hint bar counts from its own width, never the screen's —
      test: `it::hint_bar::the_changes_pane_hints_the_keys_that_write`, whose
      regression check is the **drawn** bar at 160x48 with the changes pane
      open beside the graph: `assert!(drawn > 4)` on the hints in that bar.
      Anchored to the frame, not to `hints_for`'s argument.

**Mutation-check each** and report the red-test table.

### Red-test table (each mutation applied alone, tree clean between)

| # | Mutation | Result |
|---|---|---|
| M1 | `banner_line` appends `— conflict in src/auth/token.rs` | RED |
| M2 | `banner_line` prefixes `⚠ ` | RED |
| M3 | `layout::header` words the banner by `width >= 100` | RED |
| M4 | the label row is never inserted (`if false && ghost …`) | RED |
| M5 | `ref_line` returns `"ghost"` for `Target::Ghost` again | RED |
| M6 | no badge is carried onto a preview row (`badges: Vec::new()`) | RED |
| M7 | `open_changes_pane` stops pumping `detail.diff` | RED |
| M8 | one session photographed at both capture sizes | RED |
| M9 | `fit_list` caps at 6/3 by `columns >= 100` | RED |
| M10 | the 80x24 capture's fixture loses its 30 background commits | RED |

10 mutations, 10 red. M3 and M7 needed a second attempt (first anchors were
wrong: `self.area.width` does not exist in `layout::header`, and
`open_changes_pane`'s tail is rustfmt-wrapped).

## Captures

Recapture all five, and the 80x24 one with real content behind it per item 6.
≤ 200 KB each. **Colour ships unverified — say so** (rule 5). **Do not tick
any visual result** (rule 6): the user judges these by eye, and he has already
sent back one round, so assume he will read them closely.

## Gates

Workspace-wide, never `-p`. `CARGO_TARGET_DIR=/tmp/graft-target-g20`, deleted
before hand-back. Check the baseline on `main` yourself. Commit, never push.

## Report

**Cause or symptom for every fix** — he asked for this explicitly. What you
could not verify. And if you disagree with a ruling here, say so in the report
rather than quietly doing something else.

---

## 7. The hint bar counts from the screen, not from itself — added after ticket 19

Ticket 19 fixed the inverted footer and found the **same miscalibration
unfixed in `hints_for`**: three hints at 160 columns against six at 120. It
stopped and asked rather than changing it, because the 3/6/all numbers trace
to ADR-0037 §4. Right call — and the answer is that they never were spec.

**I have amended ADR-0039** (see its 2026-09-21 amendment, "count from the
bar's own width, never the screen's"). ADR-0037 §4 sets a floor — three most
relevant keys, never all hidden — and says nothing about screen width. The
tiers were invented on top of it.

**Build to the amended rule:** the bar shows as many hints as fit in **the
room the bar actually has**, never fewer than three (or the doorway where a
modal pane draws its own complete list), and it **shrinks before it drops**.

Ticket 19 left `no_hint_bar_context_drops_a_hint_as_the_room_for_it_grows`
passing because `hints_for` is monotone *in its own argument*; the defect is in
what is passed to it. Your test must be anchored to **the drawn bar at a real
screen size with a real pane open**, not to `hints_for`'s argument — otherwise
it will agree with the code exactly as this milestone's other six did.

**While you are there, look for the third instance.** The general form of both
bugs is: *asking how big the screen is instead of how much room this thing
has.* Any place branching on screen width to decide how much to say is
suspect. If you find one, fix it or file it; if you look and find none, say
you looked.

## Recapture is now required, not optional

Ticket 19's three fixes (the gutter, the row carve, the footer wording) move
the 120 and 160 frames. Every capture in item 6 must be re-taken regardless of
what else you change.

---

## NOTES (builder)

**Scout:** none, as the ticket directed — every defect was quoted with its
capture file.

**Cause or symptom, per fix.**

1. **Banner repeats the pane — cause.** `BannerView::more_conflicts` and the
   drawing of `conflict` are gone; `conflict` stays on the struct because
   `hint_context` reads it to decide whether `M mergetool`/`e edit` have
   anything to act on. Cost to the user: none — the pane names the files.
2. **`⚠` — cause.** Removed from `banner_line`. `⚠` still means a real
   finding elsewhere: `GraphScreen::marker_glyph` (Mistake Radar),
   `OverviewStrip`, the Map screen. That is the argument for the change.
3. **Two spellings — cause, and it is the same cause as item 7.** A
   `width >= 100` tier inside `banner_line`, fed the banner's **own region**
   rather than the screen: 120 cells at 120 columns (pane below) and ~92 at
   160 (pane beside). So the wider terminal drew the terser banner. The
   width parameter is gone; the text is short enough to fit everywhere.
4. **Ghost block — cause.** `PreviewState::outcome_label` (`after rebase`)
   drawn as its own row by `view::preview::label_entry`, with a new
   `NodeKind::Absent` so nothing on the row claims to be a commit and real
   pass-through lanes keep an unbroken `│`. The box's ref line now counts:
   `ref_line` → `new_commits(new_commits_at(plan, at))`, walking `Target::Ghost`
   parents rather than assuming `at + 1` (a two-chain plan would otherwise
   count the other chain in). `preview_entries` builds the whole labelled
   list **first** and windows over that — windowing first and inserting the
   label after would re-break ticket 17's Critical.
5. **`reading…` — cause.** `open_changes_pane` never pumped `detail.diff`,
   only `detail.files`; `body_view` draws `reading…` while either is in
   flight. Now it waits for both.
6. **Per-width `history.top` — cause.** One state was photographed at two
   sizes. `state.size` drives the layout, the graph's row budget and the
   bar's room, so that state was a picture of at most one of them. A session
   per size now, scrolled at that size. The same fault was in the banner
   captures and in `capture_hint_bar_for_docs_shots`; both now resize before
   drawing.
7. **Hint bar — cause.** `width_tier` was a **cap** laid over a `columns`
   that was already the bar's own room. Deleted, along with `NARROW`/`MEDIUM`.
   `fit_list` alone answers "as many as fit", with ADR-0037 §4's floor of
   three.

**The third instance of "asks the screen, not its own room": found, filed,
not fixed.** `crates/graft/src/view/frame.rs:226` —
`screen.compaction.max_lanes = max_lanes_for(state.size.columns, session.max_lanes)`
is computed from the **screen** width before `render_graph_with_detail`
narrows the graph to its own region. At 160 columns with the pane beside, the
graph has ~92 cells but is given the lane budget of 160. Not fixed here: it
changes the drawn graph, which is a visual result the user judges by eye
(rule 6) and wants its own capture. Weaker candidates I looked at and do not
believe are instances: `update/remote.rs:541`, `event_loop/graphics.rs:141`
and `:203`, `update/map.rs:123` — all branch on the terminal's real size for
things that genuinely occupy it.

**ADR-0039's "shrink before it drops" is still unbuilt.** `fit_list` drops
from the right; it never shortens `space stage` to `space stg`. Building it
means choosing a short form for every verb — wording the user reads, so a
spec decision, not a build one. Everything else in the amendment is built.

**Two defects found while in here, neither in scope, both unfixed:**

- **The changes pane's body draws nothing for an unmerged file.** With the
  diff settled (item 5), `detail.diff` is `Loading::Ready` with **zero
  hunks** for a conflicted path, so the body area is blank — eleven blank
  rows at 80x24 on the Detail tab, and the whole right-hand 90 columns at
  120x40. Probed directly: `PROBE diff=ready hunks=0 files_cursor=Some((1, 18))`.
  Likely `git diff`'s combined (`diff --cc`) output for an unmerged path not
  being parsed. `body_view` has no empty-diff arm either, so it renders as
  silence rather than as anything. M6's territory, but it is a real hole now.
- **`the_tab_layout_still_hides_every_key` still fails under `--ignored`** —
  pre-existing known gap (m5-03 fix round), untouched here. It is the only
  failure in the `--ignored` run.

**What I could not verify.** Colour, by construction: the captures are
TestBackend character grids with no colour attributes (rule 5). Whether any
of this *reads* right — the banner row, the `after rebase` label, the longer
hint bars — is the user's call (rule 6); no visual result is ticked above.

**Spec edit made.** `docs/design/01-view-mockups.md` §1.13 still showed
`⚠ REBASING  step 3 of 7 — conflict in src/auth/token.rs` and the
`[ Continue ] [ Skip ] [ Abort ]` row that m5/16 removed. Rewritten to the
shipped banner with the reasoning. Flagging it because it is a design-doc
change, not a code one.

**Incident.** My mutation-check script reverted mutations with
`git checkout -- <path>` against an **uncommitted** working tree, which
threw away the ticket's source changes in five files. Rebuilt them from the
session's own reading of those files and re-verified green before
committing; the mutation table above was then re-run against the committed
tree. No work is missing, but the rebuilt `update/preview.rs` uses
`PreviewSubject::operation().kind()` for the label where the first pass had
read it off the undo note — the same answer by a shorter route.

**Captures.** All five re-taken, plus the fifteen `m5-hints-*` (their caption
still taught the deleted 3/6/all tiers, and they were drawn without resizing).
The 80x24 banner capture now stands on the **graph** tab with 30 commits
behind it and asserts it has no blank rows: on that tab the banner shares the
header row and its keys take over the hint bar, so it costs **no** rows of
history. The Detail tab is where it takes a row of its own, and that tab
cannot show a cost at 24 rows while the body draws nothing (defect above), so
the wide captures carry the pane instead.
