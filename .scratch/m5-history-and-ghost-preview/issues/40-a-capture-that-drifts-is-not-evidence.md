# a-capture-that-drifts-is-not-evidence

Status: ready
Blocked by: nothing (branch off `main` at `b4840ca` or later).
Crates: `graft`, `graft-ui`, `graft-graph`.
Build with: **Opus.** The third piece changes how every capture in the repo is
verified; getting the bless mechanism wrong would make a whole class of
evidence silently fake again.
Scout: **not needed** — every number below was measured by the conductor on
`b4840ca` and the commands that measured it are given.

## Why this exists

CLAUDE.md rule 6: **visual results are judged by eye, by the user.** The thing
he judges is `docs/shots/` and the M5 contact sheet. On 2026-09-22 the
conductor ran every capture generator and found **42 committed captures across
M2, M3, M4 and M5 do not match what the code draws.**

Measured with:

```
cargo test --workspace -- --ignored     # every generator is #[ignore]d
git status --short docs/shots/          # 42 modified
```

**This is the milestone's signature failure at its largest scale** — two
things that must agree, with nothing testing one against the other — and it is
the tenth instance. It is also the most expensive shape of it, because the
thing left unchecked is the evidence the user is asked to trust.

It surfaced because ticket 36 noticed captures it did not touch were stale.
Ticket 35's tick *"Captures regenerated where the grid changed"* was
**incomplete** — it regenerated the five files it knew about and missed the
rest. **That is not the builder's fault and is not filed as one:** nothing in
the gate could have told it. The instrument was missing, which is piece 3.

## Piece 1 — a marker must not touch the subject text

**This one is a real visual regression, and it ships in the contact sheet.**
Ticket 35 split the cursor slot to `▏⚠` so the radar marker stopped eating the
cursor rule. It took the separating space with it:

```
before:  ◉─╮ ⚠ Merge branch 'feature'
after:   ◉─╮ ▏⚠Merge branch 'feature'
         ● │ ▏ⓘShip the release notes
```

**8 lines across the regenerated captures** have a marker glyph abutting the
subject with no gap. This is a textual finding and stands on the capture alone
(rule 7) — and it is the same rule ticket 30 was named for: **no text ever
touches a border**, and two things with no gap between them read as one thing.

**Fix** the slot so the cursor rule, the marker and the subject each keep their
own cell. If the row genuinely has no column to spare at 80, say so and report
what you would drop instead — do not silently shave the gap back off.

## Piece 2 — a capture with a random path in it can never be reproduced

`docs/shots/m4-credential-prompt-in-app.txt` embeds the scratch repo's real
temp directory. Two runs, two different files:

```
- graft /var/folders/z4/…/T/graft-remote-idICFn/wor…
+ graft /var/folders/z4/…/T/graft-remote-4Y1Uzx/wor…
```

Two consequences, and the second is the one that matters:

- The file **cannot** be reproduced, so it can never be verified — it will
  differ on every run and on every machine.
- It carries a **machine-specific path into a repo that may be published**
  (CLAUDE.md, Gates). Not a secret; still not ours to ship.

Ticket 23 (`every-capture-is-reproducible`) fixed the **clock** in every file
that writes to `docs/shots/`. It did not fix the **path**. Finish the job:
normalise the scratch path to a stable placeholder in the captured output.
**Sweep for anything else non-deterministic while you are there** — a PID, a
hostname, a port, an absolute home directory — and say what you found.

## Piece 3 — the gate must compare captures to the code

**This is the piece that closes the class, and the other two are symptoms of
its absence.**

There are **35 capture generators and every one is `#[ignore]`d** ("run by hand
to regenerate"). They are most of the 37 tests the suite reports as skipped, so
a full green run says nothing about any capture. Nobody re-runs them, so they
drift from the day they are written.

**Fix at the cause.** A capture must be **asserted by default** and **written
only on request**: the test compares what the code draws against the committed
file and **fails on any difference**, naming the file; regenerating is an
explicit, obvious act (an env var such as `BLESS=1`, or the existing
`--ignored` path kept solely for writing). The repo already uses `insta`
snapshots elsewhere — if `insta` can carry these, prefer it to a hand-rolled
comparator, since its bless workflow already exists and is understood.

**Do not** make the assertion tolerant to make it pass. A capture that is
allowed to differ is exactly what this ticket is deleting.

If some capture genuinely cannot be asserted — a PNG whose encoder varies, say
— **say which and why**, and leave it `#[ignore]`d with a comment naming what
makes it unassertable. An honest exception beats a fake check; an unexamined
one is how we got here.

## Piece 4 — regenerate everything, then the sheet

With 1 and 2 fixed and 3 in place, regenerate all 42 and rebuild the M5 contact
sheet (`scripts/contact-sheet.sh m5`, ADR-0037). Captures stay ≤ 200 KB each;
the sheet is exempt but stays trimmed. **Raw terminal streams are never
committed.**

**Do not read a capture back into your context** — sizes, `head -3` and
`git diff --stat` only. The conductor has already read the diffs; the numbers
above are yours to use.

## NOT yours

- The 3 gap rows about **bar content** (`space fold`, `1 first-parent`,
  `a amend`). Those are M8's pass, not this.
- The acceptance walk (tickets 37, 38, 39).
- `PROGRESS.md`, `KNOWN_GAPS.md`, `docs/log/` — the conductor's. Report rows.

## Acceptance (every tick names its verifying test inline, at tick time)

- [ ] No marker glyph abuts the subject text in any capture — test: name it,
      and quote one before/after row.
- [ ] `m4-credential-prompt-in-app.txt` is byte-identical across two
      consecutive regenerations — say how you checked, and list anything else
      non-deterministic you found.
- [ ] Every capture generator **asserts by default and fails on drift** —
      name the mechanism, and **prove it**: change one committed capture by a
      single character, show the named failure, put it back.
- [ ] Blessing is explicit and documented — name the command, and say where a
      future builder will read about it.
- [ ] Any generator left `#[ignore]`d is listed with the reason it cannot be
      asserted — give the list, or say "none".
- [ ] All 42 regenerated and the contact sheet rebuilt — give
      `git diff --stat` totals and the sheet's size.
- [ ] `grep -c 'NOT RECORDED'` on the sheet is 0 — give the number.
- [ ] Whole suite green; report before and after. Baseline **2356 passed, 37
      skipped** — and say what the skipped count is now, since piece 3 should
      move most of those 37 into the run.

Rule 6: you do not tick "looks right". The user does — and he is waiting on
this ticket to be able to.

## Gates

`cargo fmt --all --check`, `cargo clippy --all-targets --message-format=short
-- -D warnings`, `cargo nextest run`, `cargo test --doc`. **Workspace-wide,
never `-p`.** (`cargo test --doc` is vacuous workspace-wide — run it, never
offer it as evidence.)

`CARGO_TARGET_DIR=/tmp/graft-target-g40`, deleted before hand-back. One cargo
command at a time. **Never bare `git stash`.** **Never `git checkout` or
`git restore` across your worktree.** Commit, never push.

## Report

What the 42 turned out to be once fixed. Whether `insta` carried the captures
or a comparator was needed. What is still non-deterministic. How many of the
37 skipped now run. Anything unverified — which is not broken.
