# two-boxes-over-real-content: a capture taken in the state the question is about

Status: done
Blocked by: none. **`graft` only.** Nothing else is running.
Build with: **Opus.** Small, but the whole value is in getting the *state*
right, and the last capture that got it wrong cost a round trip.
Scout: **not needed.** Both fixtures are named below.

## Why this exists

The user has an open question from ticket 13, and he has now framed exactly
what would answer it:

> The menu had no border, so surviving content merged into its rows and read as
> one garbled line. A bordered box has an explicit edge, so content beside it
> reads as background — and for the confirm box that is arguably right, since
> you are about to act on the graph and the graph stays visible. **What decides
> it is whether the surviving content is aligned and legible or fragmentary.**
> Don't pre-emptively band-clear either one.

**The capture we have cannot answer it.** `docs/shots/m5-ui-confirm.txt` draws
the box on an **empty screen**: the rows above and below the frame are blank and
the eight columns to its left are spaces. There is no surviving content in it,
so there is nothing to judge aligned-or-fragmentary.

This is the user's own lesson from earlier in this milestone — *a capture must
be taken in the state the question is about* — and it applies to us again.

## What to build

**Captures only. No behaviour change.** Do **not** band-clear either box, do not
centre them differently, do not touch the widgets. If you find yourself editing
a widget, stop and report.

### 1. The confirm box over a real graph

The existing writer is the `m5-ui-confirm.txt` fixture. Give it a state with
**real history behind it** — the same kind of AppState the banner captures use
(`tests/it/captures.rs`'s `frame`/`write_shot` helpers, and
`conflict_interim.rs` for how a real repository is built). At minimum:

- **80 and 160 columns**, both with enough history that the graph fills the rows
  beside and behind the box.
- A box whose choices are the interesting ones — the push confirm is fine, and
  the undo-after-push box is better if it is reachable from the same fixture.

What the user is judging, put in the caption in these words: **is the surviving
content beside the box aligned and legible, or fragmentary?**

### 2. The credential prompt in the same state

`capture_m4_credential_prompt_in_app` (`crates/graft/src/view/tests.rs`) draws
the prompt over a running push. Its caption already says "the line behind it
still says the push is running", so it has *some* surviving content — but the
user's expectation is stated and opposite:

> The credential prompt is where I expect the opposite: nothing there needs to
> stay visible.

So the capture must show what is actually behind it clearly enough to judge.
Same two widths. **Keep ADR-0038 §2 exactly as it is** — fixed 8 mask glyphs,
never the typed length, no plaintext anywhere. Passwords must not reach argv,
env, journal, log, capture, panic message or `Debug`. **Check the regenerated
file for plaintext yourself before committing it.**

### 3. Determinism

Ticket 23 made every capture byte-identical across runs. **Whatever fixture you
build must keep that** — use the fixed clock (`graft_fixtures::git::run_at`,
`Clock::Fixed`) like the captures beside it. Regenerate twice and diff; a
capture that differs run to run is a finding, not a shrug.

### 4. The caption rule, which we broke three times this round

Three stale captions were found in one round, each describing behaviour the code
no longer had. **Your caption describes what your picture shows, and you read
the regenerated file to confirm it before you commit.** Quote the row you
checked in your report.

## Acceptance (every tick names its verifying test inline, at tick time)

- [x] The confirm box capture has real graph content beside and behind the box
      at 80 and 160 — evidence: `docs/shots/m5-ui-confirm-over-graph-80x24.txt`
      row `●   Comm│ 1 commit · can't be undone from here — origin keeps what you │ 60af0eb`
      (graph left of the border, a whole sha right of it); the 160x48 file's
      row `●   Commit 182 … ╭ Push main → origin/main …╮ … GF 2d   d212b3e`.
      Asserted by `tests/it/two_boxes_over_content.rs::check`
      (`rows_with_content_beside_the_box >= 3` and `contains("Commit 19")`),
      which fails on the blank-screen capture this ticket replaces.
- [x] The credential prompt capture shows what is behind it at both widths —
      evidence: `m5-ui-credential-over-graph-80x24.txt` row
      `●   ╭ Credentials — 127.0.0.1:8443─…╮b8c5` (one graph glyph and four
      sha characters survive); `…-160x48.txt` row `●   Commit 180 … GF 2d   e95ccd4`
      (the row whole on both sides). Same `check`.
- [x] Neither widget's drawing changed — test: `git status` for this ticket
      lists only `tests/it/main.rs`, the new test module and the four new
      capture files; `graft-ui/src/confirm.rs`, `graft-ui/src/credential_prompt.rs`
      and `src/view/frame.rs` are untouched, and the whole suite is
      2247/2247 (`cargo nextest run`), the same count as the baseline.
- [x] No plaintext password anywhere in either capture; the mask is 8 glyphs —
      test: `check` asserts `!body.contains(TYPED)` for both frames and
      `matches(['•','◦']).count() == graft_ui::text_input::SECRET_GLYPHS`;
      checked again by hand with `grep -c hunter2` over all four files (0 each,
      and none of them appears in `grep -rl hunter2 docs/`).
- [x] Both captures regenerate byte-identically twice — evidence: regenerated,
      copied aside, regenerated again, `diff -q` clean on all four files. This
      needed two fixes: `ready` now waits for `Phase::Complete` **and**
      `promotion.is_none()` (a streaming walk draws `N rows so far  date order,
      sorting…`, and `N` differed run to run), and the fixture pushes an
      existing corpus commit rather than making one (a commit Graft makes is
      stamped from the wall clock, so its oid moved every run).
- [x] Each caption describes its own picture, checked by reading the file —
      evidence: read all four regenerated files. Quoting the row checked
      against the "200 commits of ordinary history, the tip one commit ahead of
      `origin/main`" claim: ` graft /code/api  200 commits` and
      `◆ ⓘ [◆ main ↑1] Commit 200                                          2d   d5ae5fd`
      over `●   [origin/main] Commit 199`. The "what survives beside it" line
      is no longer written by hand at all: `what_survives` reads it back out of
      the frame, so it cannot go stale the way three captions did this round.

**Rule 6: you do not tick "looks right".** The user does, and this capture
exists solely so he can.

## Gates

Workspace-wide, never `-p`. `CARGO_TARGET_DIR=/tmp/graft-target-g26`, deleted
before hand-back. Baseline **2247/2247** — measure it yourself.

**Do not use bare `git stash`** — the stash stack is shared across worktrees and
another agent can pop yours. Use a temporary WIP commit instead.

**Never `git checkout` or `git restore` across your worktree.**

**Captures stay under 200 KB each.** Commit, never push.

## Report

What each capture now shows behind the box. Whether anything had to change
outside the fixtures (it should not). Anything unverified — which is not broken.

## NOTES (builder)

**No scout.** The ticket said one was not needed and named both fixtures.

**What was built.** One new `#[ignore]`d writer,
`crates/graft/tests/it/two_boxes_over_content.rs`, and four files:
`docs/shots/m5-ui-{confirm,credential}-over-graph-{80x24,160x48}.txt`.
Regenerate with
`cargo test -p graft --test it two_boxes_over_content -- --ignored --nocapture`.

**Nothing outside the fixtures changed.** No widget, no view code. The only
edit outside the new file is the `mod` line in `tests/it/main.rs`.

**The existing `m5-ui-confirm.txt` was left alone.** Its writer is
`graft-ui/src/confirm.rs::capture_for_docs_shots`, outside this ticket's crate
set, and `graft-ui` has no `AppState` to stand a box on — a box over real
content can only be drawn from the `graft` crate. The new files stand beside
it rather than replacing it.

**The undo-after-push box is not in these captures.** It was tried and
dropped: reaching it needs a commit Graft makes during the run (`u` decides
from the journal, so a pre-made commit gives it nothing to undo), and git
stamps that commit from the wall clock, so the oid the box and the header both
draw moved every run. Determinism won. The same box over the same corpus is
already in `docs/shots/m5-ui-undo-after-push-80x24.txt`, at 80 only, and the
new caption points at it.

**The credential prompt is opened, not driven.** The push is real (`P`,
confirmed, a real job over a real `file://` remote) and the prompt is opened on
that job through `update::credential::credential_needed` — the entry point
`Progress::CredentialNeeded` uses — with the host pinned to `127.0.0.1:8443`.
A genuinely served HTTP fixture (`tests/it/credential_by_key.rs`) answers on a
port the OS picks, which would put a different number in the box title every
run. The caption says this in as many words.

**Unverified, not broken:** colour. TestBackend records a character grid with
no attributes, so the reversed cursor button, the raised-surface tint and the
prompt's focus colour are not in these files (CLAUDE.md rule 5).

**Finding, pre-existing, not touched.** `docs/shots/m4-credential-prompt-in-app.txt`
(and the M4 contact sheet built from it) prints the fixture's literal password
in its **caption** — "with `hunter2` typed". It is a test literal and never a
typed secret reaching a row, so ADR-0038 §2 is not broken, but a capture file
in a repo that may be published reads better without it. The writer is
`crates/graft/src/view/tests.rs::capture_m4_credential_prompt_in_app`.
