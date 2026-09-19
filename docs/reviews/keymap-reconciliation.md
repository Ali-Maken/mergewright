# Keymap reconciliation (2026-09-20)

Read: `docs/design/02-keymap.md` (whole), `docs/design/01-view-mockups.md` §1.5–1.13,
PROMPT.md §3, §4, §8; `docs/design-notes/m4-remotes.md`; the key-routing source named in
the dispatch (below). No other source read.

First: a correction to the conductor's extraction, since the whole point of this pass is
not to trust an unverified read of the code.

- `update/strip.rs` binds **only** `O`. There is no `o` in it — the file's one test named
  `other_keys_are_not_handled` presses `o` to prove that. The dispatch's "o O" is wrong.
- `update/stash.rs`'s "c" is `Ctrl+C` passing through unclaimed (the modal-exception check
  at the top of `stash_key`), not a bound `c` verb. Stash has no `c`.
- `update/journal.rs`'s "q" is the journal's wildcard **swallowing** `q` so an open browser
  can't be quit out from under (a real, tested behaviour — `an_open_browser_swallows_
  the_graphs_own_keys`), not a `q` verb the journal defines. `Ctrl+C` still quits through
  it (the `modifiers.control` guard at the top of `journal_key` returns `false` first, so
  a Ctrl-held key is never claimed) — that's correct and consistent with the commit
  editor's own Ctrl+C exception; flagging only because "q" in a key list reads as a bound
  verb and isn't one here.

Same lesson three times: an extraction that lists "what a key press does" without
separating *bound verb* from *modifier passthrough* from *deliberately swallowed* will
keep reporting keys as free, or as claimed, that are neither. `docs/design/02-keymap.md`
itself doesn't distinguish these either — worth a one-line legend when it's next touched.

---

## 1. Where the build has drifted

**Model (given, already slated):** §2.4's `d` is cursor-scoped (file, hunk or line under
the cursor, wherever the changes pane's cursor is) — that's `detail/keys.rs`'s
`request_discard`/`request_discard_at_diff_cursor`, and it's built right for the pane.
§2.3's *other* `d` — "Discard (only on the Uncommitted-changes row)," meant to work
**from the graph row itself**, no pane open — doesn't exist. `graph_key` in `update/
keys.rs` has no `d` arm at all. The same is true of §2.3's `s` ("Stage, alias for Space
in this context"): nothing in `graph_key` stages from the collapsed row either. Both
promises at the graph-pane level are dead until `Enter` opens the detail pane; only the
detail pane's versions are real. One key, one row, one already-open ticket — but it's
two doc lines (`d` and `s`) that are unbuilt at the level the doc puts them, not one.

**New, found here:**

1. **`x` (compare) doesn't exist in the doc at all.** `update/keys.rs::is_compare_key`
   binds plain `x` globally to "mark/complete a compare" (`m2-details-and-diffs/11`) — a
   real, shipped, core feature (PROMPT.md §4 "Diff any two commits"). `02-keymap.md` has
   no table row for it anywhere, and PROMPT.md's own plan for this ("select two rows with
   `Shift` or `Ctrl`+click") is mouse-only and isn't in §2.15's mouse section either. This
   is the one live key with **zero** documentation, not a mismatch — the worst kind,
   because a scout reading the doc would report `x` free.
2. **`L` (load full diff) is unbound in the doc.** `detail/keys.rs::diff_option_key` binds
   it; PROMPT.md §6.3 requires the feature ("truncate the diff, with a Load more
   option"); §2.5's table lists `w W v [ ] < >` and stops there. `L` missing.
3. **`+` (expand folded lanes) is unbound in the doc's tables.** It's real
   (`update/keys.rs::is_lane_toggle`) and even has a design-doc home — `03-graph-layout-
   algorithm.md` §3.5 — but §2.3's table never lists it.
4. **The Map view (`M`) has no §2.x section.** Every other overlay (stash list, journal
   browser, blame, command palette, credential prompt) gets its own numbered table.
   `update/map.rs` binds `j`/`k`/`↑`/`↓`, `Enter`, `Esc`, and `M` again to close — none of
   it is written down. (`Space` to expand a row's Command Log in the **journal** browser
   has the same gap, though it's covered in spirit by §2.2's global "Space … else expand/
   collapse," and the mockup at 1.10 already draws the `▸` marker it toggles — lower
   priority than Map, which has no fallback story at all.)
5. **The stash overlay's save prompt (`s`, inside `S`) isn't in §2.9.** `update/
   stash.rs::shown_key` binds `s` to open a message prompt for a *new* stash; the
   mockup at 1.11 and the table both show only apply/pop/drop/branch/preview, i.e. only
   what to do with stashes that already exist. This isn't wrong, just missing — and it's
   the reason `S` reads two ways: doc principle §2.1's "UPPERCASE opens a place" list
   names `S` as one of the four, but the *place* `S` opens is a list-with-a-save-button,
   never written down as such.
6. **`b` — create a branch from a stash — is allocated and unbuilt.** §2.9's table and
   the 1.11 mockup's footer both promise it; `update/stash.rs::shown_key`'s match has no
   `b` arm. Not wrong to be missing (no milestone has explicitly claimed it — M3's PROMPT
   bullet is "stash save/apply/pop/drop," full stop), but the doc currently reads as if
   it shipped. Recommend M5 take it: it's fundamentally "create a branch," which is M5's
   territory, not M3's or M7's — neither mentions it either.

None of the six are Critical on their own (nothing crashes, nothing silently does the
wrong thing) — `x` is the worst, being both real and completely absent from the document
whose entire job is "what's live." That's precisely last time's failure mode repeating:
a scout or a conductor reading only the doc will call `x` free.

## 2. Does the allocation survive M5–M8?

**Mostly, yes — with one thing the doc needs to say out loud.**

M5 (merge, rebase, cherry-pick, revert, reset, Ghost Preview, Fix actions) needs no new
letters under the doc's own stated policy: `m`/`r`/`b`/`t` are already graph-pane verbs
that read the cursor (merge selected, rebase onto selection, branch/tag *at* selection),
and cherry-pick/revert/reset/Fix are explicitly routed through `.` (§2.1's "rare verbs"
rule) with Ghost Preview's Confirm/Cancel riding the same `ConfirmView` pattern M4's push
box already established (`←→ choose ⏎ act esc cancel`). No collision.

M6 (conflict resolver) and M7 (interactive-rebase editor) are both dense, full-alphabet
reuse panes, and both only work if they **claim the keyboard wholesale** — every key,
not just the ones they name — the same way the commit editor already does
(`commit_editor.rs::editor_key`: "the editor claims every key but `Ctrl+C`"). The doc
never says this for either. §2.1 lists interactive rebase's `p r e s f d` and the
resolver's `o t b e` as *scoped reuse*, which is necessary but not sufficient: `p` is
also global pull, `r` is also global rebase-onto-selection, `f` is also global fetch,
`d` is also the (unbuilt) graph-row discard, `e` is also open-in-$EDITOR, `M` (resolver's
mergetool launch, §2.6) is also global Map toggle, `n`/`N` (resolver's next/prev
conflicted hunk) are also global search-hit stepping. Every one of these only stays a
*reuse* and not a *collision* if the global bindings are not live while the pane has
focus — which is true today only because the commit editor is coded that way and the
doc never states it as the general rule for "claims the keyboard" panes. **Recommend**:
add one sentence to §2.1 — these panes (commit editor, interactive rebase editor,
conflict resolver, and by the same logic the stash save prompt and search prompt, which
already do this in code) are **modal**: they see every key first and answer or discard
it; nothing underneath moves. Name it once, point every affected §2.x section at it,
done — this is a documentation gap, not a design one; the code already behaves this way
everywhere it matters.

Blame (M7) is the one exception worth naming: it reuses `n`/`N` but isn't alphabet-dense
(`Enter`, `n`/`N`, `^`, `Space`) and doesn't need the wholesale claim — a normal
focus-scoped pane, same shape as the stash list or journal browser, is enough.

Reflog, submodule display, branch comparison (M7) and final-name/remap UI (M8) don't
appear to need new single-letter bindings from what PROMPT.md §4/§8 says of them — reflog
and branch comparison read as views on the existing graph/detail machinery, submodule
display as a sidebar row (`Enter` to open, per PROMPT.md §4 "appear in the sidebar and
can be opened"). Nothing to reserve now; flag if M7's own design pass finds otherwise.

**Add now, before M5–M7 planning starts,** so nobody re-discovers them as "free": `x`,
`L`, `+`, `M`(Map)'s own table, `s`(stash save) — see the replacement tables below.

## 3. What rule governs the namespace

The three conventions the dispatch names are all partly true and none is written down as
a rule anyone could check a new binding against. Findings, then a proposed rule.

- **"Lowercase acts on the cursor"** holds for `Space`, `s`, `d`, `y`, `x`, and `e` in the
  detail pane — genuine cursor/selection reads. It does **not** hold for `c`, `a`, `f`,
  `p`, `P`, `i`: these act on ambient state (`HEAD`, the current branch, all remotes) with
  no cursor object at all, and would do the same thing with nothing selected. `m`, `r`,
  `b`, `t` sit in between — they *do* read the selected row, but the verb is "act on my
  ambient context (current branch/`HEAD`) *using* the cursor as the other argument," not
  "act on the thing under the cursor" the way `Space`/`d` do. The doc's own §2.1 already
  has the right frame for this ("one meaning per verb") — the "acts on the cursor" gloss
  in the dispatch is a simplification worth dropping, not a rule worth codifying.
- **"UPPERCASE opens a place"** holds for `S`, `J`, `M` cleanly: each opens an overlay,
  the same letter (or `Esc`) closes it. `O` and `F` are the same shape but toggle a mode
  over the *same* screen rather than opening a separate one — worth keeping as the same
  family, named as such, rather than a fourth unstated exception.
- **"UPPERCASE is sometimes the pair of its lowercase"** is true for exactly two letters
  — `u`/`U` (undo/redo) and `n`/`N` (next/previous, whatever "hit" the active scope
  means) — and false everywhere else it looks tempting. `o`/`O` is the tell: `O` is a
  place-toggle (the overview strip) under the *first* rule, and lowercase `o` means
  "take ours" in the conflict resolver and "Open…" on the first-run screen — two
  meanings, neither related to `O`, coexisting only because each lives in a scope the
  other never has focus in. Nothing is broken (rule 1's modal/focus-scoping covers it),
  but the doc's own framing of "o/O" as one of its three running conventions is
  describing a coincidence, not a design. Retire it explicitly rather than let the next
  reader treat `o`/`O` as a real pair and try to make a fourth binding "complete" it.

**Proposed rule** (for `02-keymap.md` §2.1, replacing the current three-conventions
paragraph):

1. **Lowercase is one verb, chosen for its mnemonic, live everywhere its object exists.**
   Where the verb has a cursor object (a row, file, hunk, line, branch, hunk-conflict),
   it acts on whatever's selected. Where it doesn't (`c` commit, `a` amend, `f` fetch,
   `p` pull, `i` ignore), it acts on ambient state (`HEAD`, current branch, all remotes).
   Never two verbs on one letter while both are live in the same focus.
2. **UPPERCASE is reserved for exactly two jobs, and nothing else:**
   - **Opening/toggling a place or mode** (`S` `J` `M` `O` `F` `H`): the same letter, or
     `Esc`, always closes what it opened.
   - **The reverse direction of a lowercase pair**, limited to `u`/`U` and `n`/`N` — no
     new letter joins this category by resemblance alone; a proposal has to *be* a
     direction-reversal, not just look like one (`o`/`O` never qualified).
3. **A pane that reuses the full alphabet for its own vocabulary (interactive rebase,
   conflict resolver, the commit editor, the stash save prompt, the search prompt) is
   modal: it answers every key itself, control-modified ones included, with a stated
   exception for `Ctrl+C`.** Nothing outside it is live while it has focus. This is
   already true in code everywhere it matters; §2.1 should say so once, and each
   affected §2.x section should point at it instead of re-implying it.
4. **A same-shift-key pair that means two unrelated verbs** (`p` pull / `P` push) is
   allowed **only** when: (a) the pair are natural opposites a git user already holds in
   one hand (pull/push, not e.g. `p`/`P` for "pick"/something unrelated), and (b) the
   irreversible one is never one keystroke from safe — it sits behind a confirm box
   every time (M4's push box; a slipped `Shift` still lands on `[ Cancel ]`, never on a
   running command). Any future pair proposed under this exception clears both bars or
   doesn't get the letters.
5. Numerals (`1`, and any that follow) are outside this rule entirely — they're view
   filters, not verbs, and carry no case.

**Grandfather list** — what the rule above changes and what it leaves alone:

| Binding | Under the new rule | Action |
|---|---|---|
| `S` `J` `M` | Rule 2a, place-toggle | Keep, unchanged |
| `O` `F` `H` | Rule 2a, mode-toggle over the same screen | Keep; note explicitly as the same family as place-toggles, not a third thing |
| `u`/`U`, `n`/`N` | Rule 2b, the only two direction pairs | Keep, name as the closed set |
| `o` (resolver "ours", first-run "Open") vs `O` (strip) | Not a pair; each in its own modal/focus scope | Keep both; retire the "o/O pair" framing from §2.1's text |
| `p`/`P` (pull/push) | Rule 4, box-gated opposite pair | Keep; write rule 4's two conditions into §2.3/§2.14 next to the binding so the exception is legible, not implicit |
| `B`/`X` (commit-graph offer) | Neither 2a nor 2b — a one-shot decision on a modal banner that appears once per session, not a place and not a direction | Grandfather explicitly as a named third, narrow exception: one-shot modal-banner initials, only usable while the banner is the only modal thing on screen (`commit_graph_actionable()` already enforces this) |
| Interactive rebase `p r e s f d`, resolver `o t b e n N M`, commit editor's whole alphabet, stash save prompt, search prompt | Rule 3, modal reuse | No change to the bindings; add the modal-claim sentence to §2.1 and cross-reference it from §2.6/§2.7/§2.9/§2.10 |
| `F2`/`F3` (screen/tier diagnostics) | Outside the letter namespace (function keys); source already says "not one of DESIGN.md's keymap" | Leave as an acknowledged, intentional gap — low value to formalize, no collision risk |

---

## Proposed replacement — `02-keymap.md` tables

Not written into the design doc; the user approves first. Deltas only, against the
current file (line refs are to today's `docs/design/02-keymap.md`).

**§2.1 principles** — replace the three-bullet "Focus-scoped panes…" list and its four
named places with:

> **Lowercase, UPPERCASE and modal reuse follow one rule** (see the keymap
> reconciliation review, 2026-09-20, for the reasoning): lowercase always means one verb
> for its mnemonic, applied to the cursor where the verb has one and to ambient state
> (`HEAD`/current branch/all remotes) where it doesn't; UPPERCASE only opens/toggles a
> place or mode (closed by the same letter or `Esc`) or reverses one of the two
> direction pairs, `u`/`U` and `n`/`N`; a pane dense enough to need the alphabet for its
> own vocabulary (the commit editor, the interactive rebase editor, the conflict
> resolver, the stash save prompt, the search prompt) is **modal** — it answers every
> key itself, `Ctrl+C` excepted, and nothing outside it is live while it has focus.
> `B`/`X` on the commit-graph offer banner are a named, narrow exception: one-shot
> initials on a banner that appears once per session and claims the keyboard the same
> way. `p`/`P` (pull/push) are the model for any future same-shift-key opposite pair: box
> a irreversible half, never let a slipped `Shift` run it.

**§2.3 graph/history pane** — add rows (insert after `y`, keeping the existing ones
unchanged):

| Key | Action |
|---|---|
| `x` | Mark the selected commit as the compare base, or complete/clear a compare against it |
| `+` | Expand/collapse lanes folded behind `+N` (§3.5) |

And correct the two rows already there: `d` and `s` should read *"only reachable once the
detail pane is open (`Enter`) — see §2.4"* rather than implying they act on the collapsed
row directly, until a ticket builds that.

**§2.5 diff pane** — add:

| Key | Action |
|---|---|
| `L` | Load the full file (past the truncation point, PROMPT.md §6.3) |

**New §2.3a — Map view** (insert after §2.3, before §2.4; renumber downstream sections by
one, or letter it 2.3a to avoid a renumber — conductor's call):

| Key | Action |
|---|---|
| `M` | Open (from the graph pane) or close the Map view |
| `↑`/`↓`, `j`/`k` | Move the selected branch row |
| `Enter` | Seek the graph to the selected branch's tip and close the view |
| `Esc` | Close |

**§2.9 stash list** — add, before the existing rows:

| Key | Action |
|---|---|
| `s` | Save a new stash (message prompt; `Tab` switches staged-only vs everything, `Enter` saves, `Esc` cancels) |

and mark `b` *"create a branch from this stash — planned, not yet built; homed at M5"*
rather than presenting it as shipped.

**§2.6 conflict resolver, §2.7 interactive rebase editor** — add one line to each,
pointing at the new §2.1 modal-reuse rule, e.g.: *"This pane is modal (§2.1): every key
is its own, `Ctrl+C` excepted; the global bindings above (`p` pull, `r` rebase, `f`
fetch, `M` Map, `n`/`N` search) are not live while it has focus."*
