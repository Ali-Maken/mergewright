# a-capture-that-drifts-is-not-evidence

Status: done
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

- [x] No marker glyph abuts the subject text in any capture — test:
      `graft-ui::graph_screen::tests::a_marker_glyph_never_touches_the_subject`
      (new; draws a marked row at 80/100/120/160, both severities, cursor on
      and off the row, and asserts the cell after the glyph is a space).
      Proved it bites: with `MARKER_WIDTH` put back to 2 it fails with
      `# @ |!Commit 2` (rule, warning, subject, no gap); at 3 it passes.
      Before/after, from `docs/shots/m2-evil-merge.txt`:
      `"@-.  ! Merge branch 'feature'"` -> `"@-. |! Merge branch 'feature'"`,
      and from `docs/shots/m5-ui-drop-menu-80x24.txt`:
      `  # |i[# feature] finish the feature` ->
      `  # |i [# feature] finish the feature`.
      (Glyphs transliterated here so this file stays greppable; read the
      captures for the real ones.)
- [x] Byte-identical across two consecutive regenerations — checked by
      hashing `git diff -- docs/shots` after the green run and after each of
      **two** further full `GRAFT_UPDATE_SHOTS=1 cargo nextest run` passes:
      all three hashes are `9413925bbd71548b50036bdbed911eb496c29128`. Every
      capture is also asserted on every plain run now, so drift is a red run
      rather than something anyone has to check by hand.
      **The ticket names the wrong file:** `m4-credential-prompt-in-app.txt`
      has no temp path in it (`grep -c '/var/folders'` = 0); the random
      scratch path was in `docs/shots/m5-ui-undo-after-push-80x24.txt`
      (` graft /var/folders/z4/.../T/graft-remote-idICFn/wor...`). Fixed at
      the seam rather than in that one file: `captures::frame_at` now sets
      `repository.path` to `/code/api` **before** the frame is drawn, so it
      holds for every capture drawn through it.
      Other non-determinism found and fixed: two run-invented commit ids —
      `m5-ui-undo-after-push-80x24.txt` (the pushed commit) and
      `m5-ui-preview-time-machine-{80x24,160x48}.txt` (the base commit,
      `627ee72` one run, `2078db7` the next). Both go through
      `captures::without_invented_id`, which zeroes every prefix of the id at
      the same width, so no column shifts. **What I did not fix** is under
      Findings.
- [x] Every capture generator **asserts by default and fails on drift** —
      mechanism: `graft_fixtures::capture` (new module), built on the repo's
      existing `GRAFT_UPDATE_SHOTS` convention rather than a second one. Its
      own unit tests are `graft-fixtures::capture::tests` (18), of which
      `a_normal_run_fails_on_drift_and_rewrites_nothing` is the one that
      pins assert-by-default. Proof by breaking it: changed `Ship` to `Sh1p`
      in `docs/shots/m2-duplicate-commit.txt` and
      `graft::view::tests::capture_m2_duplicate_commit` failed with

      ```
      capture docs/shots/m2-duplicate-commit.txt does not match what the code draws:
        line 17, column 11:
          committed  "...Sh1p the release notes ..."
          drawn      "...Ship the release notes ..."
      ```

      then restored the file and it passed. Nothing was made tolerant: the
      comparison is byte-for-byte.
- [x] Blessing is explicit and documented — `GRAFT_UPDATE_SHOTS=1 cargo
      nextest run`, and **a regeneration is always a red run**: the file is
      rewritten *and* the test fails, naming it, so a changed capture cannot
      reach a commit unlooked-at. Documented in `TESTING.md` under a new
      **Captures** heading, immediately after "The three gates" — where a
      builder is already reading — and in `crates/graft-fixtures/MAP.md`.
      Test: `graft-fixtures::capture::tests::a_regeneration_rewrites_the_file_and_is_still_red`.
- [x] Any generator left `#[ignore]`d is listed — **one**, and it is not a
      capture generator: `graft-graph::it::strip_cache::strip_height_numbers`,
      a timing tool that prints numbers and asserts nothing, run by hand in
      release when choosing the strip height. `cargo nextest list --workspace
      --run-ignored ignored-only` returns exactly that one test.
- [x] All regenerated and the contact sheet rebuilt — `git diff --stat` under
      `docs/shots/`: **68 files changed, 1267 insertions(+), 1245
      deletions(-)** (51 with grid changes, 16 caption-only, plus the sheet).
      `scripts/contact-sheet.sh m5` reports `62 captures, 0 without a
      recorded caption`; the sheet is **281,376 bytes**.
- [x] `grep -c 'NOT RECORDED'` on the sheet is **0**.
- [x] Whole suite green — **before 2356 passed, 37 skipped; after 2415
      passed, 1 skipped** (`cargo nextest run --workspace`). 36 of the 37
      were capture generators and now run; the 37th is the timing tool above.
      The other new tests are 18 for `graft_fixtures::capture`, 5 for
      `without_invented_id`, and the marker-gap test. All four gates pass
      workspace-wide (`cargo test --doc`: 2 passed — vacuous, as the ticket
      says).

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

## Findings

**Still non-deterministic, and not fixed here: 160 capture files under
`docs/shots/` carry machine-specific values.** They are the pty/gallery and
acceptance-walk captures, written by *scripts*, not by any Rust test, so
nothing in this ticket's mechanism reaches them:

| What | Files | Example |
|---|---|---|
| `$HOME` | 113 | ` graft /Users/devuser/dev/sandbox/gitter  137 commits` (`09-graph-screen.txt`) |
| macOS temp dir | 47 | ` graft /var/folders/z4/.../T/graft-trivial-TMtlqo  4 commits` (`m3-01-stage-file-160x48.txt`) |
| ephemeral port | 5 | `Credentials - 127.0.0.1:54372` (`m4-03-credential-masked-160x48.txt`) |

`docs/shots/37-pty-drive.py:9` also hardcodes
`/Users/devuser/dev/sandbox/gitter/target/debug/graft` as its default
binary. **The repo may be published** (CLAUDE.md), so this is one person's
home directory and machine layout shipped in 160 files, and it means no
acceptance-walk capture can be reproduced or asserted either. Fixing it needs
the pty harness changed and every walk re-run on a real terminal — out of this
ticket, and it is the same fix at a different seam.

**`docs/shots/m5-contact-sheet.txt` is 281,376 bytes, over CLAUDE.md's 200 KB
hard cap.** Pre-existing: the committed sheet was already 280,779 bytes before
this ticket; 62 captures concatenated do not fit. Not introduced here and not
fixed here.

**The ticket's counts.** It says 35 generators and 42 captures. There are
**36** generators (the 37th skipped test is a timing tool), and the fixes
touched **68** files under `docs/shots/` — 51 with real grid changes, 16 whose
only change is a caption line that no longer says `#[ignore]d, run by hand`,
and the sheet.

**`insta` cannot carry these.** It is in the repo for `.snap` files and it was
the first thing tried. It writes a YAML `source:`/`expression:` header into
every file and forces the `.snap` extension, but `docs/shots/*.txt` names are
consumed by `scripts/contact-sheet.sh`, by the coverage-matrix test and by the
gallery test (ADR-0037), and the `.png` and `.esc` captures cannot live at a
`.snap` path at all. A capture is a picture the user reads, not a serialised
value. So: a comparator, `graft_fixtures::capture`, which reports the first
differing line and column rather than a whole-file diff.

**Two defects this ticket surfaced rather than caused.**

1. `graft::it::context_menu::a_verb_key_the_menu_advertises_closes_it_and_runs_the_verb`
   forked one `AppState` and ran both branches over **one** job pool. A
   finished job's event goes to whichever branch is pumping, so the
   topological walk landed on one of them and the other kept saying
   `date order, sorting...` forever. It was invisible because the header used
   to be filled by the temp path; shortening it to `/code/api` made room for
   the status and the two frames stopped matching. Fixed by giving each branch
   its own `App` over the same repository and letting each settle before
   comparing — not by relaxing the comparison.
2. The first cut of the mechanism panicked on the first mismatch, so a
   generator that writes several captures aborted before reaching the rest and
   one regeneration pass could not rewrite them all. Fixed with
   `graft_fixtures::Captures`: collect every mismatch, fail once at the end.
   Eleven generators use it.

**Unverified (not broken).** Colour: the captures are character grids with no
colour attributes (CLAUDE.md rule 5), so nothing here says the cursor rule or
the marker is the right colour. And rule 6 — whether the new three-cell slot
*looks* right is the user's call, on `docs/shots/m5-contact-sheet.txt`.
