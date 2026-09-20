# Keymap reconciliation (2026-09-20)

**The tables this review proposed now live in `docs/design/02-keymap.md`**
(ADR-0037, decision 1 — that doc is the single source of truth for keys).
What follows is the review's findings and reasoning, kept for history; do
not re-derive tables from here, they've been superseded by the doc.

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
never says this for either — now fixed: §2.1 and §2.6/§2.7 in `02-keymap.md` state the
modal rule explicitly.

Blame (M7) is the one exception worth naming: it reuses `n`/`N` but isn't alphabet-dense
(`Enter`, `n`/`N`, `^`, `Space`) and doesn't need the wholesale claim — a normal
focus-scoped pane, same shape as the stash list or journal browser, is enough.

Reflog, submodule display, branch comparison (M7) and final-name/remap UI (M8) don't
appear to need new single-letter bindings from what PROMPT.md §4/§8 says of them — reflog
and branch comparison read as views on the existing graph/detail machinery, submodule
display as a sidebar row (`Enter` to open, per PROMPT.md §4 "appear in the sidebar and
can be opened"). Nothing to reserve now; flag if M7's own design pass finds otherwise.

## 3. What rule governs the namespace

The three conventions the dispatch names were all partly true and none was written down
as a rule anyone could check a new binding against. The proposed rule (now in
`02-keymap.md` §2.1) and its reasoning:

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
  over the *same* screen rather than opening a separate one — kept as the same family,
  named as such, rather than a fourth unstated exception.
- **"UPPERCASE is sometimes the pair of its lowercase"** is true for exactly two letters
  — `u`/`U` (undo/redo) and `n`/`N` (next/previous, whatever "hit" the active scope
  means) — and false everywhere else it looks tempting. `o`/`O` is the tell: `O` is a
  place-toggle (the overview strip) under the *first* rule, and lowercase `o` means
  "take ours" in the conflict resolver and "Open…" on the first-run screen — two
  meanings, neither related to `O`, coexisting only because each lives in a scope the
  other never has focus in. Nothing is broken (the modal/focus-scoping rule covers it),
  but framing "o/O" as a running convention described a coincidence, not a design.
  Retired from `02-keymap.md`'s text for that reason.

**Grandfather list** — what the rule changes and what it leaves alone (now folded into
`02-keymap.md` §2.1; kept here for the reasoning behind each row):

| Binding | Under the new rule | Action taken |
|---|---|---|
| `S` `J` `M` | Rule 2a, place-toggle | Kept, unchanged |
| `O` `F` `H` | Rule 2a, mode-toggle over the same screen | Kept; named as the same family as place-toggles |
| `u`/`U`, `n`/`N` | Rule 2b, the only two direction pairs | Kept, named as the closed set |
| `o` (resolver "ours", first-run "Open") vs `O` (strip) | Not a pair; each in its own modal/focus scope | Kept both; "o/O pair" framing retired from §2.1's text |
| `p`/`P` (pull/push) | Rule 4, box-gated opposite pair | Kept; the two conditions now sit in §2.1 next to the model sentence |
| `B`/`X` (commit-graph offer) | Neither 2a nor 2b — a one-shot decision on a modal banner that appears once per session, not a place and not a direction | Grandfathered explicitly in §2.1 as a named third, narrow exception |
| Interactive rebase `p r e s f d`, resolver `o t b e n N M`, commit editor's whole alphabet, stash save prompt, search prompt | Rule 3, modal reuse | No change to the bindings; the modal-claim sentence is in §2.1, cross-referenced from §2.6/§2.7/§2.9/§2.10/§2.14 |
| `F2`/`F3` (screen/tier diagnostics) | Outside the letter namespace (function keys); not one of `02-keymap.md`'s tables | Left as an acknowledged, intentional gap — low value to formalize, no collision risk |

---

## Since this review: M4 checkpoint (ADR-0037)

Decision 1 made `docs/design/02-keymap.md` the single source of truth for keys and
directed this review's tables to be written in. Done 2026-09-20: `f`/`p`/`P` (fetch/pull/
push, M4, now shipped) and the credential prompt's modal keys are also now in the doc —
the credential prompt table needed one further correction beyond what this review
proposed: `update/credential.rs::credential_key` shows the prompt has exactly one field
open at a time (git asks username, then password, one question per turn), so `Tab`
"move between fields" as originally worded in `02-keymap.md` didn't match the code;
`02-keymap.md` §2.14 now describes the single-field behaviour as built.
