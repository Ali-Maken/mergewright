# say-it-once-and-name-the-outcome: the banner repeats the pane, and "ghost" is a word the UI never defines

Status: ready
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

- [ ] The banner names the state, not the files; the pane names the files —
      test:
- [ ] No `⚠` on the banner; the keys row is still there at every width —
      test:
- [ ] One banner wording at 80, 120 and 160 — test:
- [ ] The ghost block carries a label, and the summary line names commits
      rather than `ghost` — test:
- [ ] No user-facing string anywhere says `ghost` — a sweep, not a spot
      check — test:
- [ ] The `[◆ topic]` badge still lands on the newest ghost — test:
- [ ] Captures wait for the pane to settle rather than capturing `reading…` —
      test:
- [ ] Each caption's stated `history.top` matches its own width — test:

**Mutation-check each** and report the red-test table.

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
