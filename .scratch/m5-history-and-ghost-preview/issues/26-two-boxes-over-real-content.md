# two-boxes-over-real-content: a capture taken in the state the question is about

Status: ready
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

- [ ] The confirm box capture has real graph content beside and behind the box
      at 80 and 160 — evidence (quote a row showing both box and graph):
- [ ] The credential prompt capture shows what is behind it at both widths —
      evidence:
- [ ] Neither widget's drawing changed — test:
- [ ] No plaintext password anywhere in either capture; the mask is 8 glyphs —
      test:
- [ ] Both captures regenerate byte-identically twice — evidence:
- [ ] Each caption describes its own picture, checked by reading the file —
      evidence:

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
