# the-ladder-and-the-stale-banner: shrink before dropping, and a capture that predates its fix

Status: **done**, at a seam. Pieces 2 and 3 merged earlier; piece 1 (the shrink
ladder) built on the corrected rule and committed, plus the `m`/`r` hint fix the
conductor added mid-build. **Four `graft` tests are red and stay red**: the
seam is named under NOTES (second pass) and needs four mechanical edits in a
crate this ticket does not hold.
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

**The rule the ladder serves — corrected 2026-09-21 by the conductor, after
this ticket's first attempt proved the original wrong.** The sentence that was
here said the bar puts *every* hint on its best rung before dropping any hint.
Built and measured, that empties the bar: a row context is 22 hints (227 cells
full, 190 short, 81 key alone), so below 190 columns the list-wide ladder draws
bare keys at 80, 120 and 160 alike — this ADR's own rejected option.

**The rule now (ADR-0039, third amendment):** the bar fills **from the most
relevant end**; each hint takes the best rung that still leaves room for the
hints ahead of it; a hint that does not fit even at its key alone is dropped.
Shrinking is **per hint**, never a step the whole list takes together. ADR-0037
§4's floor stands: the three most relevant never go below their **short** rung,
and the bar is never all hidden.

The monotonicity property must still hold — more room can only improve a hint's
rung or add one, never the reverse.

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

- [ ] **NOT DONE — blocked, see NOTES.** Every hint over four characters has
      the user's short rung, spelled exactly as the table gives it — test:
      none written. The table was implemented and reverted; five labels have
      no cell (NOTES 3).
- [ ] **NOT DONE — blocked, see NOTES.** At a width where the bar must
      economise, every hint is on its shortest informative rung **before**
      any hint is dropped — test: none written.
- [x] `no_hint_bar_context_drops_a_hint_as_the_room_for_it_grows` still holds
      — test: that test itself
      (`crates/graft-ui/src/graph_screen/tests.rs:1022`), green in the
      workspace run below. `hints.rs` is byte-identical to `main`
      (`git diff crates/graft-ui/src/graph_screen/hints.rs` is empty), so it
      holds for the trivial reason that nothing under it changed.
- [x] `capture_for_docs_shots`'s caption describes what `banner_line` now
      draws — test: the three tests that pin the drawn text, all green:
      `graph_screen::tests::the_banner_names_the_state_and_never_the_files`
      (asserts `"REBASING step 3 of 7"` exactly, and no filename, no
      `conflict`, no `(+`), `::the_banner_carries_no_warning_glyph` (no `⚠`),
      `::the_banner_is_worded_the_same_whatever_the_header_it_lands_in` (one
      wording at 100/120/160/200 — the claim "no width tiers" the caption now
      makes). The caption asserts nothing these three do not.
- [x] `docs/shots/m5-ui-banner.txt` shows no `⚠`, no filename and no width
      tiering — evidence: regenerated and read. The banner row, identical in
      all four blocks (`80x10`, `120x10`, `160x10`, long-path `120x10`):

      ```
       graft /code/api    REBASING step 3 of 7
      ```

      The block labels `<100 cols` / `>=100 cols` / `>=160 cols` are gone too
      — they were the tiering claim in the file's own headings. A second
      stale sentence was found and fixed while regenerating: the caption said
      "the *path* is what an `…` lands on", and with the banner this short
      the long path at 120 columns is no longer cut at all, so the picture
      showed no `…`. Reworded to what the picture shows.
- [x] Minor 4 comment added; Minor 3 doc comment added — test: both are
      comment-only, so the gate is that the claims are true and the tree is
      green. Minor 4's claim ("the `None` arm is `wordings` is empty, and
      that cannot happen") is exercised by
      `graph_screen::tests::the_commit_graph_line_spends_the_room_it_is_given`,
      which sweeps width `0..=240` over all four `COMMIT_GRAPH_LINES` and
      asserts width 0 returns the *narrowest* wording — i.e. it runs the
      `narrowest` path for every variant and gets a real wording back.
      Minor 3's doc comment names the four cases by their test names; all
      four exist and are green
      (`a_combined_diff_is_an_error_not_an_empty_success`,
      `a_combined_section_after_a_normal_file_is_an_error_too`,
      `an_unreadable_line_before_the_first_file_is_an_error`,
      `the_diff_tree_commit_id_line_is_still_skipped_and_the_diff_still_parses`).

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

## NOTES (builder, 2026-09-21)

No scout: the ticket said none was needed and quoted every line — correct, it
was enough.

Baseline measured on this worktree before any edit: **2226 passed, 37 skipped,
54.8 s**. After: **2226 passed, 37 skipped**, `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` both clean, `cargo test --doc` ok.

### 1. Piece 1 is blocked: the ladder as specified empties the graph's hint bar

**Built it, ran it, reverted it.** Not arithmetic — observed.

The ticket's rule is "at a given width the bar spends the room it has on the
most informative rung that fits, for every hint, **before it drops any hint**",
and ADR-0039's 2026-09-21 amendment says the same in the same words: shrink
"applies before any hint is dropped at all". Implemented exactly that: three
rungs, the whole list stepping down together, drop only once every hint is on
its key.

What it draws. The row contexts are **22 hints long** — `drawable()` appends
`TAIL`'s screen-wide keys to every row context
(`crates/graft-ui/src/graph_screen/hints.rs:224-230`, m5-03 review Major 3).
Measured widths of the whole list plus `. more`:

| rung | cells |
|---|---|
| full | 227 |
| short (the user's table) | 190 |
| key alone | 81 |

So below **190 columns** no rung but "key alone" fits, and the ladder goes
there before dropping anything. The regenerated capture, with the ladder in,
drew this at 80, 120 **and** 160 columns:

```
 ⏎  c  y  u  S  J  /  q  F2  F3  x  +  j/k  space  1  F  H  f  p  P  M  O  . more
```

The 160-column terminal teaches exactly as much as the 80-column one: nothing.
That is ADR-0039's own **"Considered and not taken — Keys with no verbs
(`space d c`). Fits anywhere, teaches nothing."**

Three tests in `graft` (a crate ticket 25 holds — never edited) pin the old
text and went red, with the drawn bar in the panic message:

- `crates/graft/tests/it/hint_bar.rs:151` — `" ⏎  c  P  p  f  u  S  J  /  q  F2  F3  x  +  j/k  space  1  F  H  y  M  . more"` against `starts_with(" ⏎ open  c commit  P push")`
- `crates/graft/tests/it/hint_bar.rs:168` — same shape, plain row
- `crates/graft/tests/it/hint_bar.rs:186` — same shape, uncommitted row

`crates/graft-ui/src/graph_screen/hints/tests.rs:80`
(`eighty_columns_leads_with_three_and_fills_the_row`) pins the same expectation
inside my own crate.

**Cause, not symptom.** The ladder is right for a short list and wrong for a
long one, and ADR-0039 was written for a short one: its worked example is the
changes pane, 7 hints (`space stage` → `space stg` → `space`), which shrinks
sensibly. `TAIL` landed afterwards and made every row context 22 hints. "Shrink
everything before dropping anything" then means "shrink 22 hints to bare keys
rather than drop the 22nd", which inverts the ADR's own purpose.

I did **not** pick a replacement rule. Three that would work, for the user to
choose between — all of them spec changes, not build decisions:

- (a) Ladder per context list only, `TAIL` excluded — `TAIL` keeps dropping
  from the right as today. Keeps the ADR's example working, keeps the leading
  verbs at 80 columns.
- (b) Ladder applies, but the key-alone rung is never drawn for a list longer
  than N — the bar drops instead. Needs N.
- (c) Shrink from the right, hint by hint, so the leading hints stay full and
  the tail degrades first. **Contradicts the ticket's stated defect condition**
  ("a bar that drops a hint while another hint is still on its full rung"), so
  it needs that sentence rewritten.

Cost of leaving it as it is: none to the user — the bar is exactly what it was
on `main`. The ADR keeps an unbuilt clause.

### 2. The wording table: applied cleanly, 22/22 cells, no collisions

Every cell was implementable as given and no two short forms collide, in any
context or against any other drawn verb. The table is not the problem.

### 3. Five labels the table does not name — reported, not replaced

`hints.rs` has these verbs, none of them in the table and none in the
no-short-form list (`fetch pull push undo drop edit skip pop move open more
sha map focus all quit fold`):

- **`stash`** (5 chars) — `hint("S", "stash")`, **Live**, drawn in `TAIL` and
  in all four row contexts. The one that bites today. Note the table *does*
  name `hide stash` → `hide`, so a short `stash` cannot be `hide`.
- `merge`, `rebase`, `branch`, `resolved` — all `HintState::Planned`, never
  drawn, so they cost nothing until the group that binds them lands.

Also worth the user's eye: the no-short-form list is introduced as "four
characters or under", but `fetch` and `focus` in it are five. I took the
**list** as authoritative over the arithmetic (they have no short form) rather
than shortening two words he wrote out. Confirm or correct.

While the ladder is unbuilt, none of this is live.

### 4. Incidental

`capture_for_docs_shots` crossed clippy's `too_many_lines` when the caption
grew; added `#[allow(clippy::too_many_lines)]` with a reason, matching the
exemption `capture_hint_bar_for_docs_shots` beside it already carries. No lint
was disabled to hide a fault.

**Rule 6: the regenerated capture is not ticked "looks right" here.** The user
judges `docs/shots/m5-ui-banner.txt`.


---

# Continuation: piece 1 only (the ladder), 2026-09-21

Pieces 2 and 3 are merged. **Only the ladder remains.** The rule it serves was
corrected above and in ADR-0039's third amendment: **per hint, from the most
relevant end** — never a step the whole list takes together.

## `stash` — ruled by the user: no short form

It keeps its full label. His reasoning, which is the general rule and not a
one-off:

- **Every other hint is a whole word** — open, commit, push, pull, fetch, undo,
  search, compare, lanes, move, fold, more. A consonant-squeeze for one verb
  makes the bar inconsistent **in kind**, not merely in width. A reader who
  meets `stsh` learns that labels here can be mangled, and then reads every
  other label twice.
- **The ladder already has the mechanism for "no room": the bare-key rung.** If
  a row cannot afford `S stash`, that is the ladder saying to drop `S` into the
  tail with the others. An abbreviation scheme solves the same problem a second
  time and solves it worse.
- `hide` being taken is an argument against `hide`, not for `stsh`.

So: **no short rung for `stash`.** Its middle rung is its full form; when the
room runs out it goes to the bare key like anything else.

The four Planned labels the table does not name — `merge`, `rebase`, `branch`,
`resolved` — cost nothing yet. Leave them full; do not invent short forms.

**My ticket's "four characters or under" was wrong** (it listed `fetch` and
`focus`, which are five). The real rule is the one above: a label has a short
rung only where the user's table gives it one. Everything else is full → bare
key.

## The bare-key rung stays in, and the capture is the deliverable

Build it with the key-alone rung. The user judges whether it earns its place.

**What he is judging, in his words:** *whether a bare key reads as available or
as leftover — a tail of naked letters after three labelled hints can look like
the row ran out of budget rather than like an offer.*

So the capture must let him answer that in one pass:

- **80, 120 and 160 columns.**
- **A variant with a separator or an ordering that makes the tail read as
  deliberate**, beside the plain one, in the same file. Something that says
  "these are also available" rather than "we ran out of room" — a divider
  before the tail, or grouping the bare keys apart. **Propose it; do not agonise
  over it.** One variant is enough, and if you think two are genuinely
  different, show two.
- Each block under its own "what right looks like" line, as the other captures
  in `docs/shots/` do.

**Rule 6: you do not tick this.** He does.

## Still true from the original ticket

- The wording table is the user's. **Do not invent, re-word or improve a cell.**
  A label `hints.rs` has that the table does not name keeps its full form and is
  **reported**, not replaced.
- `no_hint_bar_context_drops_a_hint_as_the_room_for_it_grows` must still hold,
  and so must `eighty_columns_leads_with_three_and_fills_the_row` — the leading
  verbs stay full at 80 columns under the corrected rule.
- Crate set `graft-ui` and `graft-git`. **`graft` is held by ticket 25**, which
  is editing `view/`, `update/` and `jobs/`. The three `graft` tests at
  `tests/it/hint_bar.rs:151,168,186` pin the old bar text — if the corrected
  rule changes what they see, **report it as a seam, do not edit them.**

## Acceptance (every tick names its verifying test inline, at tick time)

- [x] Every hint the user's table names has its short rung, spelled exactly as
      given; `stash` and the Planned labels keep their full form — test:
      `graph_screen::hints::tests::every_short_rung_is_the_users_own_word`
      (the 22 cells asserted verbatim against `short_form`, plus `short_form`
      returning `None` for `stash`, `merge`, `rebase`, `branch`, `tag`,
      `resolved` and every other whole-word verb, plus a walk over
      `every_hint()` proving no drawn verb has a short form the table does not
      give it). Mutation E — `"commit" => "com"` and an invented
      `"stash" => "stsh"` — red.
- [x] The bar fills from the most relevant end: a hint takes the best rung that
      leaves room for those ahead of it, and only a hint that will not fit at
      its bare key is dropped — tests, all swept over every context, both
      dividers, widths 0..=240:
      `::the_bare_keys_are_a_tail_not_a_scatter` (once one hint is at `Key`,
      every hint after it is),
      `::a_tail_hint_gains_its_verb_only_once_the_whole_list_is_drawn` (past
      the floor, a hint rises above its key only when the whole context is on
      the bar — the "leaves room for the hints ahead of it" clause),
      `::a_hint_is_dropped_only_when_its_bare_key_will_not_fit` (adding the
      next hint at its key would overflow), and
      `::a_row_context_keeps_whole_verbs_at_the_relevant_end` (the leading
      three are `Rung::Full` at 80, 120 and 160). Mutations A (list-wide
      ladder), B (plain left-to-right greedy) and F (stop adding at eight
      hints) each red.
- [x] The three most relevant never fall below their short rung, and the bar is
      never all hidden (ADR-0037 §4) — test:
      `::the_three_most_relevant_never_fall_below_their_short_rung` (0..=240
      columns, every context; the floor count and each floor hint's rung
      against its own `floor_rung`), with `::no_context_hides_every_key`
      unchanged for the never-empty half. Mutation D (floor = 0) red on the
      first.
- [x] `no_hint_bar_context_drops_a_hint_as_the_room_for_it_grows` still holds —
      test: that test itself
      (`graph_screen::tests`, unchanged, green in the workspace run below), now
      joined by `::no_hint_loses_ground_as_the_room_for_it_grows`, which adds
      the rung dimension the ladder introduces: 0..=240 columns, every context,
      both dividers, no hint reordered and no hint's rung lower than at one
      cell less. Mutation C (skip a step that does not fit instead of stopping,
      so the result is no longer a prefix) red.
- [x] At 80 columns the leading verbs are still full — tests:
      `::eighty_columns_leads_with_three_and_fills_the_row` (the first three
      by (key, verb), `. more` last, width ≤ 80),
      `::a_row_context_keeps_whole_verbs_at_the_relevant_end`, and
      `graph_screen::tests::footer_eighty_branch_row`, which pins the whole
      drawn row: `" m merge  r rebase  ⏎ open  P  c  p  f  u  S  /  q  F2  F3
      x  +  j/k  . more"`.
- [x] The capture shows 80/120/160 plus the deliberate-tail variant — evidence:
      `docs/shots/m5-hints-ladder.txt` (21 KB), written by
      `graph_screen::tests::capture_the_ladder_for_docs_shots` (`#[ignore]`d).
      Five contexts × three widths as built, then variant A (a `·` between the
      labelled hints and the bare keys, fitted into three fewer columns so the
      dot pays for itself) and variant B (`. more` moved to the front of the
      bare keys — no new glyph, not one cell more). Each block over a ruler
      exactly `columns` cells wide. **Not ticked "looks right" — rule 6.**
- [x] **Added mid-build by the conductor:** `m merge` and `r rebase` are `Live`
      — test: `graph_screen::hints::tests::no_planned_hint_is_ever_drawn` (its
      canary moved from `m merge` to `b branch`) and
      `::eighty_columns_leads_with_three_and_fills_the_row`, whose branch-row
      case now expects `m merge  r rebase  ⏎ open`. **This is the seam: it
      turns four `graft` tests red.** See NOTES.

**Mutation:** make the ladder step the whole list down together (the rule that
was wrong). The sweep property must go red, and the capture must show bare keys
at 160. If it stays green the property is not expressing the corrected rule.

## Gates

Workspace-wide, never `-p`. `CARGO_TARGET_DIR=/tmp/graft-target-g24b`, deleted
before hand-back. Baseline **2226/2226** — check it yourself. Captures under
200 KB. Never `git checkout` across your worktree. Commit, never push.


---

## NOTES (builder, second pass, 2026-09-21)

No scout: the ticket and ADR-0039's third amendment quoted every line, and the
first pass had already mapped `hints.rs`. Reading the file itself (521 lines)
was cheaper than briefing anyone.

**Baseline.** Measured on this worktree: 2226 passed / 37 skipped at `d96433e`
(exit 0), but that run overlapped my first edit, so treat it as agreeing with
the ticket's number rather than as an independent measurement. I then rebased
onto `main` at `547467e` (ticket 25 merged, 2240 tests) — testing the ladder
against a stale `graft` would have hidden the seam below. Final: **2247 tests,
2243 passed, 4 failed, 38 skipped.** `cargo fmt --check`, `cargo clippy
--all-targets -- -D warnings` and `cargo test --doc` all clean.

### Cause, not symptom

The defect was the rule, not the code: ADR-0039's second amendment said shrink
"applies before any hint is dropped at all", which read list-wide empties a
22-hint bar. The third amendment corrects it and this build implements the
corrected rule. Nothing is special-cased and no test was weakened.

### How the ladder is built

`ladder(list, floor)` returns a **fixed sequence of steps**, the same at every
width, and `fit_list` takes the longest prefix that fits. The order is: the
three most relevant to their full verbs; then every remaining hint at its bare
key; then those raised, one hint at a time from the left, short rung then full.
The floor — three hints at their short rung plus `. more` — is drawn before the
first step and whatever the width.

Prefix-of-a-fixed-sequence is what buys monotonicity in *both* dimensions for
free: every prefix costs at least as much as the one before it, so more room
can only add a hint or raise one. A plain left-to-right greedy is **not**
monotone — spending a cell to raise hint 0 can cost hint 5 its verb — and two
other shapes I tried (maximise the count first, then upgrade; interleave add
and raise per hint) each lose one of the two properties. This is the only
shape I found that keeps both and still draws the tail.

### What the bar draws now (`Divider::Hidden`, ordinary commit row)

```
 80: " ⏎ open  r rebase  c commit  y  m  u  S  J  /  q  F2  F3  x  +  j/k  . more"
120: " ⏎ open  r rebase  c commit  y sha  m merge  u  S  J  /  q  F2  F3  x  +  j/k  space  1  F  H  f  p  P  M  O  . more"
160: " ⏎ open  r rebase  c commit  y sha  m merge  u undo  S stash  J journal  / search  q quit  F2 status  F3 rend  x  +  j/k  space  1  F  H  f  p  P  M  O  . more"
```

The short rung shows up exactly where the room runs out mid-hint — `F3 rend`
at 160, `x cmp` on a current-branch row, `c cmt` in a 40-cell changes pane.

### The seam — four `graft` tests red, and the four edits that close them

Flipping `m merge` / `r rebase` to `Live` was the conductor's mid-build
addition. It is correct (`graft::update::preview::is_history_key` binds both)
and it needs four changes in `crates/graft/**`, which this ticket does not
hold and was told not to widen into:

1. `crates/graft/src/update/keys.rs`, `bound_actions()` — add two rows:
   `("m", "merge"),   // is_history_key → preview::history_key` and
   `("r", "rebase"),  // the same`.
   Closes `update::tests::every_hinted_key_is_bound` **and**
   `update::tests::every_planned_hint_is_still_unbound` (the second fails on
   its `Live ⇒ bound` arm, not its planned arm).
2. `crates/graft/tests/it/hint_bar.rs:168`
   (`a_plain_row_hints_the_plain_context`) —
   `starts_with(" ⏎ open  c commit  y sha")` becomes
   `starts_with(" ⏎ open  r rebase  c commit")`.
   The other two rows at :151 and :186 (`the_tip_row_hints_the_current_branch_context`,
   `the_uncommitted_row_hints_the_working_tree_context`) **still pass**: their
   contexts have no `m`/`r` at the head and `ends_with(". more")` is unchanged.
3. `crates/graft/src/view/snapshots/graft__view__tests__renderer-*.snap` —
   accept; the bar row is the only line that moved.
4. `crates/graft/tests/it/hint_bar.rs:370-392`, the caption
   `capture_hint_bar_for_docs_shots` writes into all twelve
   `m5-hints-<context>-<size>.txt` files. It is not false — it says "as many
   keys as fit the row the bar actually has, `. more` last" — but it is now
   **incomplete**: it does not mention rungs, so a reader meeting a tail of
   bare keys has nothing in the caption telling him it is deliberate. The
   twelve captures were regenerated from this worktree (the test writes only
   `docs/shots/`, which is not a crate), so the pictures are current; the words
   above them are one milestone behind. `docs/shots/m5-hints-ladder.txt` says
   it properly and is the file to judge by.

### Captures regenerated (all `docs/shots/`, no crate touched)

- **New:** `m5-hints-ladder.txt` — the deliverable.
- `m5-hints-{branch,plain,uncommitted}-{80x24,120x40,160x48}.txt` (nine) and
  `m5-hints-marked-*.txt` (three) — the bar changed under them.
- `m5-ui-context-menu-verbs-{80x24,160x48}.txt` — its drift check caught the
  same row.
- `m5-hints-changes-*.txt` did **not** change: the changes pane's list fits
  whole at every captured width, so the ladder never runs.

### The wording table: one label reported, not replaced

Every cell applied cleanly, 22 of 22, no collisions. Labels `hints.rs` has that
the table does not name, all left at their full form per the ticket's rule:
`stash` (the user's explicit ruling), `merge` and `rebase` (now live, and
absent from the table for the same reason as `stash`), and the still-planned
`branch`, `tag`, `resolved`. **None replaced, none invented.**

### Unverified

- **Colour** — the footer's hints draw in `Token::TextMuted`; `TestBackend`
  records a character grid, so no capture here proves it (rule 5).
- **Which of the three tails reads right** — plain, variant A or variant B — is
  the user's by eye. Nothing here ticks it (rule 6).
- The 2226 baseline above overlapped my first edit; the post-change 2247-test
  run is the measurement I stand behind.

### One thing for the next pass, not done here

`TAIL` is appended to every row context, so a row context is 22-plus hints and
the tail is long by construction. The ladder makes that readable rather than
fixing it. If the user judges the bare-key tail as leftover, the cheaper answer
is not a separator but a shorter `TAIL` — and that is a design question about
what the bar is for, not a width rule.
