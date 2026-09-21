# the-ladder-and-the-stale-banner: shrink before dropping, and a capture that predates its fix

Status: ready
Blocked by: none — `graft-ui` and `graft-git` only. Ticket 25 holds `graft`
and runs alongside you. **Never edit outside your crate set**; need `graft`,
commit at the seam, report `blocked: needs graft`, stop.
Build with: **Opus.** Piece 1 is a user-approved wording table that must not
drift; piece 2 is a stale picture that would otherwise reach the contact sheet.
Scout: **not needed.** Every line is quoted below.

## 1. The shrink ladder — ADR-0039's "shrink before dropping", unbuilt since M5/03

ADR-0039 says the hint bar shrinks a hint before it drops it. Only the drop is
built. The ladder is three rungs — **full → short → key alone** — and the
wording below is **the user's, approved 2026-09-21**. Take it as given.

Only labels over four characters need a middle rung. Everything at four or
under (`fetch pull push undo drop edit skip pop move open more sha map focus
all quit fold`) has no short form: its middle rung *is* its full form.

| full | short | full | short |
|---|---|---|---|
| first-parent | first | continue | cont |
| mergetool | tool | discard | disc |
| hide stash | hide | journal | log |
| overview | over | renderer | rend |
| explain | why | search | find |
| changes | diff | status | stat |
| compare | cmp | resize | size |
| commit | cmt | revert | rev |
| stage | stg | abort | stop |
| apply | use | graph | grph |
| close | shut | lanes | lane |

**Do not invent, re-word or "improve" any of these.** If one cannot be applied —
a label in `hints.rs` this table does not name, or a collision it creates —
**report it, do not choose a replacement.** The wording is the user's call and
he has already made it.

The hints live in `crates/graft-ui/src/graph_screen/hints.rs`
(`hint(<key>, <label>)`). The existing drop behaviour and its monotonicity
property are pinned by `no_hint_bar_context_drops_a_hint_as_the_room_for_it_grows`
(ticket 19) — **that property must still hold** after the ladder lands.

**The rule the ladder serves:** at a given width the bar spends the room it has
on the most informative rung that fits, for every hint, before it drops any
hint. A bar that drops a hint while another hint is still on its full rung is
the defect.

## 2. M6 from the review — a capture contradicting ticket 20 still ships

`docs/shots/m5-ui-banner.txt:17` (and its two other width blocks) reads:

```
 graft /code/api    ⚠ REBASING 3/7 — conflict src/auth/token.rs (+1)
```

Ticket 20 removed the `⚠`, removed the filename and removed the width tiers —
`layout.rs`'s `banner_line(banner)` takes no width now. It re-captured
`m5-ui-banner-conflict-80x24.txt` **but not this file**, and not the caption
that generates it:

`crates/graft-ui/src/graph_screen/tests.rs:1333-1334` — `capture_for_docs_shots`
still claims "`⚠ REBASING step 3 of 7 — conflict in <path> (+N)` — spelled out
at >=100 cols, abbreviated (`3/7`, path only) under 100".

**Fix the caption first, then regenerate.** A caption that describes behaviour
the code no longer has is the "capture claims what the picture does not show"
failure the user has already hit twice this milestone. Check the regenerated
file actually shows `REBASING` without the glyph or the filename — read it,
don't assume.

**Rule 6: you do not tick "looks right".** The user does.

## 3. Two Minors from the same review, both in your crates

- **Minor 4** — `crates/graft-ui/src/graph_screen/footer.rs:137`:
  `.unwrap_or_default()` on the narrowest-segment index. It is a legitimate
  "index 0 when nothing fits", **not** a swallowed failure, but it has rule 3's
  shape and a reader cannot tell. One comment saying why it is safe.
- **Minor 3** — `crates/graft-git/tests/it/diff_cases.rs:383`: the proptest
  re-implements `is_bare_object_id` (`unified.rs:94`) as its own
  `is_a_bare_object_id`. A property mirroring the code's predicate agrees with
  the code by construction. **Keep it** — but say in its doc comment that the
  four hand-written cases beside it carry the real expected error strings, so
  nobody later "simplifies" them away.

## Acceptance (every tick names its verifying test inline, at tick time)

- [ ] Every hint over four characters has the user's short rung, spelled
      exactly as the table gives it — test:
- [ ] At a width where the bar must economise, every hint is on its shortest
      informative rung **before** any hint is dropped — the property, swept
      over widths, not a case — test:
- [ ] `no_hint_bar_context_drops_a_hint_as_the_room_for_it_grows` still holds —
      test:
- [ ] `capture_for_docs_shots`'s caption describes what `banner_line` now draws
      — test:
- [ ] `docs/shots/m5-ui-banner.txt` shows no `⚠`, no filename and no width
      tiering — evidence (read the file, quote the row):
- [ ] Minor 4 comment added; Minor 3 doc comment added — test:

**Mutation-check per the section below.** The one that matters: make the bar
drop a hint while another is still on its full rung, and show the ladder
property going red.

## How to run a mutation check

- **Per mutation, run only the tests that tick names**
  (`cargo nextest run -E 'test(<name>)'`). Seconds, not a minute. Narrowing
  cannot produce a false green.
- **Once**, on the mutation that restores the actual reported fault, run the
  full workspace suite and report the blast radius — "n red, and only these n".
- **Never run the same suite twice to parse it twice.**

## Gates

Workspace-wide, never `-p`. `CARGO_TARGET_DIR=/tmp/graft-target-g24`, deleted
before hand-back. Baseline **2226/2226** — check it yourself.

**Never `git checkout` or `git restore` across your worktree.** Revert a
mutation by editing the file back.

**Captures stay under 200 KB each.** Commit, never push.

## Report

Cause or symptom per fix. Any table row you could not apply, and why — **named,
not replaced**. What `m5-ui-banner.txt` says now. Anything unverified, which is
not broken.
