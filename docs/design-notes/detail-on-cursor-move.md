# Should the detail pane follow the cursor? (design investigation, 2026-09-22)

Read: CLAUDE.md; PROMPT.md §2, §3, §6.3, §7.1, §8; DESIGN §1.1–1.4, §2.1–2.5, §5.2;
ADR-0021, 0023, 0028, 0030, 0033, 0039; M2 design note (New UI, Hard-to-reverse);
M2 ticket 10; M5 ticket 35; `crates/graft/MAP.md` (`update::detail`, `update::selection`),
`crates/graft-ui/MAP.md` (`detail`, `detail_pane`); TESTING.md M2 walk; docs/field-notes.md.
No source. Not a ticket spec; the conductor files after the user reads this.

## Recommendation

**Yes — the pane follows the cursor whenever it is shown and the graph has the
keyboard. `Enter` opens it as a viewer and leaves the cursor in the graph; a
second `Enter` steps into it; `Esc` steps back out, then closes.** Loads fire
when the cursor rests (~120 ms), never per keypress. Nothing changes below 100
columns except that the Detail tab shows whatever the cursor names.

In one line: tig's model (parent view drives the child), not Sublime Merge's
(pane always present) and not today's (pane is a place you drill into).

## What the pane is for

Three questions a user asks when moving between rows:

1. **"What is this?"** — reading history. Subject, author, when are already on
   the row. What the row cannot say: which files, how big, the body, parents,
   contained-in. A glance, not a drill-in. **Follow pays here.**
2. **"Where is the change I'm hunting?"** — scanning five or ten commits for a
   file or a hunk. Today that is `Enter`, read, `Esc`, `j`, `Enter` — four keys
   per commit, and the pane blanks between each. **Follow pays most here.**
3. **"Should I act on this?"** — merge/rebase/reset target. Answered by the
   graph's shape and the badges, not the diff. Follow neither helps nor hurts.

The user's field note is (1) and (2). His team works on dependent branches, so
(2) is daily: "which of these commits on the other branch touched the file I'm
about to rebase over?"

Today the pane is a **place** (`Enter` moves focus into Files; `j`/`k` then move
the file cursor; `Esc` closes). Sublime Merge's pane is a **viewer** (the
selection is the subject of the whole window; the pane never takes the keys).
The user's question is really "why is it a place?" — and the honest answer is
that M2 built the drill-in and never built the door to the viewer state. The
state exists: `DetailFocus::Graph` with `shown = true` is reachable by
`Enter`, `Shift+Tab`. Nobody discovers it. The follow path also exists for two
of the three specs (`update::selection::follow_selection`: working-tree row and
live compare; TESTING.md M2 "`j` returns to HEAD's commit and the pane
follows"). **Whether a plain commit→commit move re-targets a shown, graph-focused
pane I could not confirm from MAP.md** — `DetailState::follow` reads as if it
does; `follow_selection`'s doc names only the other two cases. A question, not a
defect: the conductor checks `follow_selection`'s guard before filing.

## What comparable tools do (where they differ is the information)

| Tool | Selection drives pane? | What loads on move | Cost they took | Confidence |
|---|---|---|---|---|
| Sublime Merge | Yes, always shown, cursor stays in the list | Full diff of every file | Slow on big commits; no narrow mode | Recalled, fairly sure |
| Tower | Yes, always shown | Message + files + stacked diffs | Same trade as SM | Recalled, not verified |
| Fork | Yes, always shown | File list; diff per selected file | Extra click for the diff | Recalled; auto-select of first file unsure |
| GitKraken | Yes; panel slides in on select | Message + files; diff replaces graph area | Panel covers graph; keyboard drive unsure | Recalled, not verified |
| lazygit | Yes, while the commits panel has focus; `Enter` drills into files | Whole `git show` per move | Visible lag when holding `j` on big commits; graph is a strip | Recalled, fairly sure |
| tig | **Only while the split is open** (`Enter` opens; parent keeps focus; child follows) | Whole diff per move | Split costs half the screen; no hints | Recalled, fairly sure |
| gitui | No — `Enter` opens a details view; move inside it | — | Read-one-commit-at-a-time | Recalled; newer versions unsure |
| magit | No — `RET` opens a revision buffer in another window; next `RET` replaces it | — | Two keys per commit, like Graft today | Recalled, fairly sure |

Two axes matter:

- **Viewer vs place.** Every GUI and both graph-drawing TUIs (lazygit, tig)
  chose viewer. The two that chose place (gitui, magit) have no side-by-side
  pane at all. Graft has the pane and chose place — the odd one out.
- **What follows.** SM/Tower/lazygit/tig load the whole diff; Fork loads the
  file list and one diff. Graft's data model is already Fork-shaped (M2
  hard-to-reverse #1: list first, hunks per selected file, caches of 8/64/16).
  So Graft can follow at Fork's cost, not Sublime Merge's.

tig is the one that matches Graft's constraints: a keymap where `Enter` opens
and `Esc` closes, width that cannot afford a permanent pane, and a graph that
must keep the keyboard. Its choice — follow only while open — is the one I
recommend.

## Costs, concretely

**Per selection today:** `git diff-tree --raw -z --numstat -z` (file list),
`git diff-tree -p` (first file), `git for-each-ref --contains` (cached 64,
`checking…` while pending), an evil-merge probe on merges (cached 16). Three
spawns, all off the UI thread, all stale-dropped by id, the list cancellable.
Draw cost with the pane open is inside budget: `docs/perf/m3-06-review-fix.txt`
`pane=Open` p99 5.1 ms release at 50k refs (what keys that scenario presses I
could not tell from the summary lines — a question for the ticket).

**Per keypress would be wrong.** Key repeat is ~30 Hz; fork/exec plus git
start-up is 5–15 ms on a large repo; issue-then-cancel at repeat rate is 30–90
processes a second, killed a frame later. That is the laptop-safe rule (PROMPT
§7.1) broken for nothing the user can read. **Rest-then-load:** the header
(subject, author, date, parents — already in memory from the row) redraws on the
same frame; the spawns wait until the cursor has rested ~120 ms, or until any
non-movement key. Holding `j` costs zero spawns until you stop. This is the same
debounce the file watcher already uses (PROMPT Technology), a design choice,
not a sleep hiding a symptom. `update` is pure with no clock, so the shape is:
`update` records the pending spec; the event loop arms a wake; on `Rested`,
`update` issues the commands if `AppState::selected_spec()` still matches.
Whether the loop already has a timed wake (spinner, toasts) is the second
question for the ticket. `for-each-ref --contains` on 50k refs is unmeasured;
if it is slow, it takes a longer rest (~400 ms) of its own.

**Width.** Follow spends no width the pane does not already spend: 42 % of
columns at 160+, 40 % of rows at 100–159, nothing while hidden. The user
opens it when he wants it. A permanently shown pane (Sublime Merge) would take
40 % of the graph's rows at every 100–159-column terminal — the width he
lives in inside Zed — for a pane he may not be reading. Rejected below.

**What breaks at 80 columns, and the rule that stops it.** Below 100 the pane
is a whole-screen tab. If "follow" meant "show the pane", every cursor move
would flip the tab and the next key would land in the wrong pane. So: **follow
never changes the layout, the visible tab, or the focus; it only changes what
an already-visible pane shows.** In the Tab layout with `Graph` up, follow
updates the spec and issues nothing; `Tab` to `Detail` issues the loads for
whatever the cursor named. That is the 80-column behaviour, and it is
unchanged from today except that the Detail tab is never stale.

**Wrong-commit flicker.** ADR-0023's ticket 36 (pinned tooltip showed the wrong
commit) is the failure class. While the loads are pending, the files region
shows the header from the row and `reading…` under it, never the previous
commit's list. The M2 spec already states this ("loading → header from the row
at once, files `reading…`").

## Keys — a spec change, through the conductor

`docs/design/02-keymap.md` §2.2 gives `Enter` "open / drill in" and `Esc` "back
out / close". The proposal keeps both verbs and adds one rung each:

| State | `Enter` | `Esc` |
|---|---|---|
| pane hidden, graph focused | open as viewer; focus stays on Graph | (unchanged) |
| pane shown, graph focused | focus → Files (drill in) | close the pane |
| Files / Diff focused | Files → Diff (unchanged) | focus → Graph, pane stays shown (Explain closes first, unchanged) |

Symmetric: `Enter Enter Esc Esc` = open, enter, back, close. The hint bar's
`⏎ open` becomes `⏎ files` in the second state and `esc close` appears; today's
`enter_opens` (m5/31) already knows whether `⏎` has anything left to open, so
the bar cannot lie. `Tab`/`Shift+Tab` unchanged. Compare (`x`) already follows;
this generalises it to `Commit`. Mouse: click selects → follows after rest;
wheel does not move the cursor → pane unchanged; hover never follows
(ADR-0023). The pane's shown/hidden state joins `PaneSize` in
`<common_dir>/graft/layout`, so a repo where he opened it once stays that way.
First run: hidden — the graph is the product.

## Rejected

- **Always shown, always following (Sublime Merge).** 40 % of the graph's rows
  at 100–159 columns, permanently, for a pane not always being read; impossible
  below 100. Remembered-per-repo gives the same daily result without the tax.
- **Follow after rest, but keep `Enter` = focus Files.** The viewer state stays
  a state nobody reaches; the user keeps pressing `Enter`/`Esc` per commit. The
  focus rung is the fix; the dwell is only how it stays cheap.
- **Load on every move, cancel on the next.** Process churn at key-repeat rate;
  laptop-safe. The cancel path stays as the safety net under the dwell.
- **One-line summary instead of a pane.** Costs a row in every layout forever;
  restates subject/author/when the row already shows (the "same fact twice"
  finding, field-notes 2026-09-21); the facts the row lacks — files, body — do
  not fit a line.
- **Hover drives the pane.** Pointer ≠ cursor; every key acts on the cursor, so
  the pane must show the cursor. ADR-0023 closed this door for good reason.
- **Leave it.** The follow path exists for two of three specs, the data model
  is already the cheap shape, and this is the first design-level finding from
  the daily-use tier (ADR-0030). Two small tickets. Leaving it would be ignoring
  the tier we set up to be heard.

## Reversibility

- **Cheap:** the dwell length; the default (hidden/shown on first run);
  whether contained-in gets its own longer rest; hint-bar wording.
- **Sets a shape:** *viewer vs place* for the commit pane. Every pane after
  this — the M6 conflict resolver, M7 blame and file history — inherits the
  rule "a viewer follows the cursor and never takes the keys; a place claims
  them" (§2.1's modal rule already defines the place side). The resolver is
  modal and must be a place; the commit pane should be a viewer. Decide before
  M6 designs on top of the drill-in model.
- **Sets a shape:** a timed wake in the event loop with `update` staying pure.
  Small, but it is the first clock-driven `update` input; do it once, cleanly.

## Cursor visibility (ticket 35) — independent, but order matters

A following pane makes the pane header the loudest "where am I" signal on
screen. With a cursor row that is "really hard to notice", the user will
navigate by the header and the ~120 ms rest will read as the pane lagging one
row behind him. That does not change the recommendation; it changes the order:
**35 lands first, then this.** A dimly lit cursor plus a following pane is
worse than either alone.

## Where it belongs

**Not inside M6:** the resolver rebuilds the pane's files+body area and focus
model; a builder holding `update::detail` for the resolver cannot share it.
**Before M6's design pass**, as a two-ticket group on the same footing as 34/35
(post-M5 field fixes): (1) the `Enter`/`Esc` rungs + persisted `shown` + hint
words (`graft`, `graft-ui`); (2) follow-on-rest with the timed wake (`graft`,
event loop + `update::detail`/`update::selection`). Both fit one context each.
M6 then designs the resolver against the viewer/place rule instead of
rediscovering it.

## Questions to settle before filing (rule 7 — no line, no defect)

1. Does `update::selection::follow_selection` re-target a shown, graph-focused
   pane on a plain `Commit`→`Commit` move today? Name the guard line.
2. Does the event loop have a timed wake the dwell can use, or does one need
   adding? Name the loop's wake sources.
3. Which keys does the frames test's `pane=Open` scenario press — is scrolling
   with the pane open on a commit already measured?

---

## Conductor's disposition, 2026-09-22

**Parked for M8. Nothing is filed from this note.** ADR-0041 (accepted the same
day): presentation and product-shape research defers to M8, and this note is
its first input. The recommendation inside — build as a two-ticket group before
M6's design pass — is **not taken**, because that is the sequencing the user
had just decided against.

**The one question in it that could not wait, answered.** The note asks whether
a commit→commit move re-targets a shown, graph-focused pane, and calls
`MAP.md` ambiguous. Checked by the conductor rather than left open, because a
pane showing one commit while the cursor sits on another would be the app
lying — which ADR-0041 puts in the never-deferred group:

- `crates/graft/src/update/detail/state.rs:227-228` — `DetailState::spec` is
  *"what the pane is about, **once opened**"*.
- `crates/graft/src/update/keys.rs:382-393` — `move_cursor` pushes only
  `Command::ShowRows`; it never touches `detail`.

So the pane **does** keep showing the commit it was opened on. That is `tig`'s
"open" semantics rather than Sublime Merge's "follow", and it is **deliberate,
not stale** — the pane names its own commit in its header
(`crates/graft-ui/src/detail_pane/header.rs:16,108`, `title_text`), so the user
can always see what they are looking at. **Not a defect; the note's question is
closed.**

One live interaction worth carrying into M8 rather than losing: this reasoning
depends on the user being able to see where the cursor is, and on 2026-09-22 he
reported that he cannot (`docs/field-notes.md`, ticket 35). **Re-read this note
after 35 lands** — an "open" pane is defensible when the cursor is obvious and
much less so when it is not.

**Unverified in this note, by its own marking:** every competitor row is
recalled, not checked. Treat them as prompts for the M8 research, not as facts.
