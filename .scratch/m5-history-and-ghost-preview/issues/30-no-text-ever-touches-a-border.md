# no-text-ever-touches-a-border

Status: ready
Blocked by: **10** (shares `graft-ui` + `graft`).
Crates: `graft-ui`, `graft`.
Build with: **Sonnet.** Small and fully specified. The thinking is done; what
is left is one invariant, one sweep, and a width calculation.
Scout: **not needed** — all three instances are quoted below with their fix.

## Why this exists

Three separate tickets have each fixed one instance of the same fault, and the
third was **created by the fix for the second**:

1. **Ticket 28** — the title ran into its own rule:
   `╭ Credentials — 127.0.0.1:8443─────╮`. Fixed by adding a space.
2. **Ticket 29** — the border ran into the content beside it:
   `╮60af0eb`. Fixed by a gutter in the snap.
3. **Now** — the longest line inside the box runs into the border:

```
●   ╭ Push main → origin/main ─────────────────────────────────────────╮ 60af0eb
●   │ 1 commit · can't be undone from here — origin keeps what you send│ aceb8c5
●   │ git push --progress origin refs/heads/main:refs/heads/main       │ 91900bd
```

Outside the box is now correct. Inside, the longest line touches the right
border while every other line is padded, so the box is **asymmetric**: `│ ` on
the left, `send│` on the right. `docs/shots/m5-ui-confirm-over-graph-80x24.txt`.

**Fixing instance-at-a-time is what produced instance three.** This ticket
replaces the three point fixes with one invariant.

## The decision (conductor; build to it)

> **No text is ever adjacent to a border glyph — on any side, inside or out.**
> A bordered surface reserves one cell of padding inside each border, and one
> blank column outside it, and its required width includes all of them.

Consequences, stated so they are not re-derived:

- **A box's required width is `2 borders + 2 padding + the longest content
  line`.** Today the content measurement omits the inside padding, which is
  why the longest line exactly fills to the border. Fix the measurement, not
  the symptom — do not special-case the longest line.
- **If the snap cannot give that width, the box clears**, per ADR-0040's
  existing final clause. **Never shrink or truncate the content to fit** — that
  trades a legible box for an illegible one.
- Text in a border's *rule* (a title or a footer legend) is the same rule: a
  space each side, which ticket 28 already did for two surfaces. The invariant
  now covers any surface that does it.

This is a **clarification of ADR-0040, not a new decision.** Amend it in place;
it is two days old and has one amendment already. Do not append a second.

## What to build

1. The width calculation, wherever ticket 28 put the snap
   (`crates/graft-ui/src/graph_screen/layout.rs::snap_to_columns` and its
   caller). Content width must include its own padding before the snap runs.
2. **One sweep replacing the three point assertions.** Ticket 28 created
   `crates/graft-ui/tests/border_rules.rs` and ticket 29 extended it. Add the
   invariant there, over the widths those already sweep:

   > For every bordered surface drawn at that width: no cell holding a border
   > glyph has a non-space text cell orthogonally adjacent to it, inside or
   > outside.

   Written that way it catches all three instances **and the fourth**, and it
   is one test rather than three. Give a mutation for each of the three known
   instances and show each goes red.
3. Apply it to **every** bordered surface, not only the confirm box and the
   credential prompt. If a third surface fails the sweep, fix it and say which.
4. Recapture whatever moves. The four `m5-ui-*-over-graph-*` files are
   deterministic (ticket 26) — **keep that**: regenerate twice and `diff -q`.

## Acceptance (every tick names its verifying test inline, at tick time)

- [ ] **No text is adjacent to a border glyph on any side, at any swept
      width** — test: name it. Give three mutations, one reproducing each known
      instance (`8443─────`, `╮60af0eb`, `you send│`), each verified red.
- [ ] The box's required width includes its own padding — test: name it, and
      name the function that now computes it.
- [ ] Where the width cannot be had, the box clears rather than truncating —
      test: name it, and say which width first forces it. Ticket 29 reported
      that no width in 20..=240 forced a clear; say whether that still holds.
- [ ] Every bordered surface is covered, not just the two — evidence: say how
      many the sweep enumerates and how it finds them (a hardcoded list that
      silently misses a new surface is not coverage; say so if that is what it
      is).
- [ ] ADR-0040 amended **in place**, no second appended amendment — evidence:
      quote the amended clause.
- [ ] Captures regenerated and still byte-identical across two runs — evidence:
      `diff -q` output, and quote the padded row.
- [ ] **The sweep includes at least one state with a detail pane open beside
      the graph** — not only full-frame graphs. Review Q1: `view/tests.rs`'s
      `confirm_over_graph` sets `detail.shown = false` (`:2673`) and
      `pane_boundaries.rs` sweeps a full-frame graph, so **no test draws a box
      over a graph that has a pane beside it.** `snap_to_columns` waives the
      gutter only at `bounds.right()` (`graph_screen/layout.rs:520-524`); with a
      pane open, the graph's own right edge is a different boundary, and whether
      the cell it steps onto is m5/19's unpainted gutter or the SHA's last
      character is untested. Draw it at 160 with the pane open and say which it
      is — test: name it. If the box eats a SHA character, that is a defect in
      this ticket's scope; fix it under the same invariant.

Rule 6: you do not tick "looks right". The user does.

---

# PART 2 — the review's Majors, which are the same class

`docs/reviews/m5-23-to-29.md`. These are folded in because every one of them is
**two things that must agree, with nothing testing one against the other** —
the same fault the invariant above exists to kill. Fix them here or they become
a seventh instance.

## M1. Five documents still say the Tab layout has no keys

Ticket 29 fixed it. These still describe it as live:

- `docs/adr/0039-the-hint-bar-degrades-to-a-doorway.md` — `:46-56`, `:101`,
  `:131`, `:141`, `:152`
- `docs/capabilities/…m5:470` — status still `broken`
- `TESTING.md:1627-1634`
- `KNOWN_GAPS.md:494`

**And ADR-0039's "Considered and not taken" lists "A row reserved for the bar in
every pane"** — which reads, to anyone opening it, as rejecting what now ships.

**Be precise about this rather than just deleting the bullet.** The ADR rejected
the *broad* option — a reserved row in every pane at every width — for a real
reason: "costs a line of content at every width to fix a case that only some
widths have." What ticket 29 built is **narrower**: one row, Detail tab only,
where no other bar exists. The rejection still stands for the broad version.

So: keep the rejected bullet, and record beside it what **was** taken and why
it is not the same thing. A reader must be able to see both and tell them
apart. `KNOWN_GAPS.md` is the **conductor's** file — do not edit it; report the
row and the conductor closes it.

## M2. `hints.rs:413`'s doc contradicts its own body

The comment says the bar overflows and the footer cuts it. The body calls
`below_the_floor`. Ticket 28 implemented the floor-yields-to-the-doorway rule
and the comment was not updated. **The comment is the thing that is wrong** —
check the body and describe what it does.

This comment has now misled twice: the conductor quoted it to the user as
current behaviour after it had already stopped being true.

## M3. `banner::graph_screen_on_frame` is a new untested hand mirror

A second copy of layout knowledge with nothing asserting it matches the thing
it mirrors — the exact shape forbidden for badge geometry in ticket 10, and the
shape `update/geometry.rs` already carries as a known gap.

**Either** give it a test that it agrees with the real layout — draw a frame,
compare — **or**, if one implementation can serve both, make it one. Do not
leave a third mirror untested.

## M4. The ladder caption claims `H hide`, the picture has none

`docs/shots/m5-hints-ladder.txt`. Another stale caption — the fourth this
milestone. Read the regenerated file and make the caption describe **it**.

## M5. `KNOWN_GAPS.md:546` still open after ticket 28

Conductor's file. **Report it; do not edit it.**

## Also

Seven Minors and two Questions in the review are **not** in this ticket. The
conductor triages them at the milestone close. Do not chase them.

## Gates

`cargo fmt --check`, `cargo clippy --all-targets --message-format=short
-- -D warnings`, `cargo nextest run`, `cargo test --doc`. **Workspace-wide,
never `-p`.** Measure the baseline yourself and report both numbers.

`cargo test --doc` is vacuous workspace-wide (two `no_run` doctests in
`graft-fixtures`). Run it; do not report it as evidence.

`CARGO_TARGET_DIR=/tmp/graft-target-g30`, deleted before hand-back. One cargo
command at a time. **Never bare `git stash`.** **Never `git checkout` or
`git restore` across your worktree.** Commit, never push.

## Report

How many bordered surfaces the sweep covers and how it enumerates them.
Whether any surface had to clear that did not before. Anything unverified —
which is not broken.
