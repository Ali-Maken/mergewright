# the-short-form-table-becomes-prefixes-only

Status: ready
Blocked by: none. Crate set: **`graft-ui` + `graft`.** Nothing else is running.
Build with: **Sonnet.** Mechanical: a table deletion, one rule sentence, an ADR
restatement, two assertions and three capture regenerations. Every judgement
call in it has been made already and is written below.
Scout: **not needed.** Every file and line is named.

## Why this exists

The user ruled the short-form table down to prefixes only. His reasoning, which
the rule sentence has to preserve:

> A squeeze breaks recoverability. A substitution gives one verb two names,
> which is the thing the keymap doc forbids. On that axis substitution is worse
> than squeezing — `stsh` at least points at stash; `find` points at nothing and
> competes with a word already on screen.

It is not hypothetical. `docs/design/02-keymap.md:12` says **"Mnemonic first,
one meaning per verb."** `docs/shots/m5-hints-marked-160x48.txt` draws
`/ search` and `S stash`; `docs/shots/m5-ui-banner-conflict-80x24.txt` draws
` c continue  a abort  M mergetool  e edit  ⇥ graph`. The short rung would
rename those same verbs to `find`, `stop` and `tool` — so the user renames his
own verbs by resizing the terminal.

Two of them break a second rule outright. `docs/design/02-keymap.md:45` says the
banner binds `c s a` because they "match the buttons' initials exactly" — `a`
for **stop** does not, and `M` for **tool** does not. And `rungs()`
(`crates/graft-ui/src/graph_screen/hints.rs:106-112`) selects rungs from the
**verb**, with no context test, so the banner degrades through the short rung
like every other context. This is reachable, not theoretical.

## What to build

### 1. The table becomes prefixes only

`crates/graft-ui/src/graph_screen/hints.rs`, `short_form()` at `:139-164`.

**Delete these thirteen rows.** Four squeezes, two suffixes, seven
substitutions:

```
"compare" => "cmp",      "commit" => "cmt",       "stage" => "stg",
"graph" => "grph",       "mergetool" => "tool",   "resize" => "size",
"journal" => "log",      "explain" => "why",      "search" => "find",
"changes" => "diff",     "abort" => "stop",       "apply" => "use",
"close" => "shut",
```

**Keep these nine, unchanged.** Every one keeps the beginning of its label:

```
"first-parent" => "first",  "continue" => "cont",   "discard" => "disc",
"hide stash" => "hide",     "overview" => "over",   "renderer" => "rend",
"status" => "stat",         "revert" => "rev",      "lanes" => "lane",
```

Those thirteen verbs now have two rungs, not three (`rungs()` already does this
for a verb the table does not name — no change needed there), and their
`floor_rung()` becomes `Rung::Full`. Both follow from the deletion; do not
special-case either.

### 2. The rule, beside the table

The doc comment above `short_form()` currently argues the `stash` case at
length (`:126-138`). **Replace that argument with the rule**, because the
argument is now the rule's corollary rather than the reason. The user's
sentence, to be preserved in substance:

> **The short form is a prefix of the full label, or there is no short form.**

Name the three kinds it rules on so the next reader cannot misapply it in
either direction — a **prefix** keeps the beginning, so the reader recovers the
word; a **squeeze** removes the middle (`stsh`, `cmt`) and they cannot; a
**substitution** replaces the word (`find` for search, `tool` for mergetool) and
gives one verb two names, which `docs/design/02-keymap.md:12` forbids. One kind
permitted. Say that a verb the table does not name simply has no short rung, so
adding a row is never required to make a label fit — the bare key is the
ladder's answer to "no room" (ADR-0039).

Keep the existing note that `merge`, `rebase` and `stash` are live and absent,
and that `branch`, `tag` and `resolved` are `Planned`.

### 3. The floor has to still be satisfiable

Thirteen verbs' floor moves from `Rung::Short` to `Rung::Full`, so ADR-0037 §4's
"the three most relevant are never below their floor, and never all hidden"
now costs whole words where it used to cost short ones.

**Assert it, do not assume it.** A test that sweeps every `HintContext` across
widths **20..=240** and checks the floor holds: the three most relevant hints
are drawn at or above their floor rung, and the bar is never empty. The banner
context is the tight one — `c continue  a abort  M mergetool` is about 34 cells
plus separators.

**If any context cannot meet its floor at any width in that sweep, stop and
report it. Do not widen the floor, drop a hint from the top three, or
re-add a short form to make it fit.** That is a spec question for the
conductor, and the answer is not a builder's to pick.

### 4. Regenerate the three captures that draw a deleted short form

Exactly three files draw one today — verified by grep before this ticket was
filed:

- `docs/shots/m5-hints-ladder.txt`
- `docs/shots/m5-hints-branch-160x48.txt`
- `docs/shots/35-coverage-matrix.txt`

Removing a rung only moves a hint that was standing on it, so the cascade stays
inside those bars. **Check that claim rather than trusting it:** regenerate
every capture whose writer you touch, and `git status` at the end — if a fourth
file changed, say so in your report and say why.

**Their captions change with them** (three stale captions were found in one
round in this milestone; `KNOWN_GAPS.md` carries the class as open). A caption
that still explains a short form the picture no longer draws is a false picture
under rule 6. Read each regenerated file and confirm its caption against it.

### 5. The ladder capture, taken after — not before

`docs/shots/m5-hints-ladder.txt` is the capture the user judges the bare-key
tail on, and he ruled explicitly that it is taken **after** this change:

> Dropping those pushes more keys into the bare-key tail at the tight widths.
> That is the exact thing the other capture is meant to judge. Take the
> bare-key capture after this lands, not before — otherwise I'm judging a tail
> that's about to grow, and we do it twice.

Keep its existing shape: **as-built, plus variant A** (a `·` separator before
the tail, which costs 3 cells and is paid honestly) **and variant B** (`. more`
leads the tail). Same three widths, **80 / 120 / 160**.

Two things the caption must put in front of him, because they are the open
questions:

- **Does a bare key read as *available* or as *leftover*?** A tail of naked
  letters after the labelled hints can look like the row ran out of budget
  rather than like an offer. That is what the variants exist to let him
  compare.
- **`rev`.** `revert` keeps its short form under the rule — `rev` is a clean
  prefix. But `rev` is also a git word meaning *revision* (`rev-parse`,
  `rev-list`). Make sure `rev` is **visible in at least one of the three
  widths**, and say in the caption that it is there to be judged. Do not delete
  it; the table is the user's.

Do not tick "looks right" on any of this (rule 6).

### 6. ADR-0039, restated once

`docs/adr/0039-the-hint-bar-degrades-to-a-doorway.md` now carries **three**
amendments, the third correcting the second. CLAUDE.md says an ADR is a
decision, not an autopsy, and four amendments read in sequence is an autopsy.
The user agreed and said it is the call he would make.

**Rewrite `## The decision` so it states the current rule once, correctly**, and
**keep every amendment below it, unedited, under a history heading.** Nothing is
deleted and no trail is lost — the decision simply reads straight.

The current rule, which the amendments collectively arrive at:

- The bar fills from the **most relevant end**. Each hint takes the best rung
  that still leaves room for the hints ahead of it; a hint that will not fit
  even at its bare key is dropped. **Per hint, never a step the whole list takes
  together.**
- The three most relevant never fall below their floor rung, and the bar is
  never all hidden.
- A modal pane draws its own keys, not the doorway.
- Widths are counted from **the bar's own width, never the screen's**.

Add the abbreviation rule from §2 as part of the decision, in one line: the
ladder's bare-key rung is the mechanism for "no room", so no abbreviation
scheme is needed or permitted.

### 7. One pending note, in the milestone's design note

`docs/design-notes/m5-history-and-ghost-preview.md`. Add a short section
recording that the **border/background distinction** — a bordered box makes its
edge explicit, so content beside it reads as background rather than as part of
the box — is **accepted with a clause**: it holds *only while whole columns
survive beside the box*. Fragment a column and the border does not save you.

Mark it as **pointing at ticket 28**, which builds the geometry rule and writes
its ADR. Do not write the geometry rule here and do not implement any of it —
this section is one paragraph recording where the decision lives.

## Acceptance (every tick names its verifying test inline, at tick time)

- [ ] Thirteen rows gone, nine remain, and the nine are exactly the prefixes —
      test: name it.
- [ ] No context can produce a label that is not a prefix of its full verb —
      test: name a test that asserts this over the whole table, so a future
      `stsh` fails rather than being reviewed.
- [ ] The floor holds for every context across widths 20..=240 and the bar is
      never empty — test: name it. Say in your report which context was
      tightest and at what width.
- [ ] The banner never draws `a stop` or `M tool` at any width — test: name it.
      This is the `02-keymap.md:45` defect; it must fail before the change.
- [ ] The three captures regenerated, each caption read against its own
      regenerated file — evidence: quote one row per file you checked.
- [ ] `git status` shows no fourth capture changed, or your report says which
      and why — evidence: paste the list.
- [ ] The ladder capture has as-built, variant A and variant B at 80/120/160,
      and `rev` is visible in at least one — evidence: quote the row.
- [ ] ADR-0039 states its decision once with the amendments intact below —
      evidence: the new heading structure.
- [ ] The design-note section exists and points at ticket 28 — evidence: quote it.

## Gates

`cargo fmt --check`, `cargo clippy --all-targets --message-format=short
-- -D warnings`, `cargo nextest run`, `cargo test --doc`. **Workspace-wide,
never `-p`.** Baseline **2247/2247** — measure it yourself first, then report
both numbers.

`CARGO_TARGET_DIR=/tmp/graft-target-g27`, deleted before hand-back. One cargo
command at a time in this worktree.

**Never bare `git stash`** — the stack is shared across worktrees. Use a WIP
commit.

**Never `git checkout` or `git restore` across your worktree.**

Captures stay under 200 KB each. **Commit, never push.**

## Report

Which context was tightest in the floor sweep and at what width. Whether a
fourth capture moved. Anything unverified — which is not broken. Whether you
used a scout, and if not, why.
