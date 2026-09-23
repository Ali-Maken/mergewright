# Field notes

What real use turned up. **Your daily use is a test tier** (ADR-0029), not a
fallback for when testing fails — it's the only layer that reaches how things
look, repo shapes nobody made a fixture for, and sequences nobody thought to
script. The fast tier covers logic on fixtures we invented; heavy and perf cover
scale at review; the acceptance walk covers the promise along a path someone
wrote down.

**How to use it:** one line, dated, no ceremony. Write it yourself or say it and
the conductor writes it. Half a note beats a forgotten one — "diff looked wrong
on the big repo, Tuesday" is enough to start from.

Unfixed notes feed the next milestone's design pass. Fixed ones get a commit
hash and stay, so the same thing isn't rediscovered.

See also: [`KNOWN_GAPS.md`](../KNOWN_GAPS.md) (what we already know is broken or
unverified — check here first, it may be a known one),
[`docs/log/`](log/) (what was built when).

---

## Open

_(nothing yet — M0–M2 were read-only and never left the fixtures)_

## Fixed

_(nothing yet)_

## 2026-09-21 — the user's eye-check of the five M5 captures

Five captures, five defects he found and three of us missed, plus two design calls and one structural ask. Filed as tickets 19 (his boundary-test design, `graft-ui`) and 20 (wording, ghosts, captions).

- **He could not answer the question the capture was taken for.** The 80x24 banner capture shows a pane holding four rows of content and about thirteen blanks, so the banner crowds out nothing and its row cost is unjudgeable. A capture that cannot show the cost cannot answer the question it was taken for — recapture with a real history behind it. Captures need to be taken in the state the question is about, not the state that is easiest to reach.
- **Same fact twice on a 24-row screen**: the banner reads `conflict notes.md (+1)` and four rows below the pane reads `CONFLICTS (2)` with both filenames. Neither is wrong; together they waste a scarce screen. No test would ever have caught this, because both halves are individually correct.
- **`ghost` is our word.** It appears nowhere else on screen: the user sees `○` and `—` and has to connect them to a noun the UI never defines. A `CONTEXT.md` glossary term leaked into the interface. The glossary exists so *we* use one term — that is not a licence to show it to the user.
- **The ghosts read as duplicate commits.** Five ghost subjects identical to the five real rows above them, no separator, no label; strip colour and the only cue is `○` vs `●` and two em-dashes. He named the failure precisely: *"a first read of 'these are in my history twice' is available — the duplicate-commit confusion M2 exists to prevent."* The one cue that does work is the `[◆ topic]` badge moving onto the newest ghost, because it says outcome rather than extra rows.
- **Footer verbosity inverted**: 120 columns gets the long explanatory line, 160 with forty more columns gets the terse one. Wider terminal, terser text.
- **A caption claimed a screen the capture does not show** — it describes `CONFLICTS (2)` above `STAGED`/`UNSTAGED` while the capture shows `reading…` and neither. Taken mid-load. A capture of a half-loaded screen is weak evidence for everything else on it too.
- **A caption true at one width and false at the other**: `history.top is 14` holds at 160x48 and is 24 at 80x24. Second instance of a per-width caption bug this milestone.
- 2026-09-22 — Preview box over the graph at ~200 columns: **no left border**, and the `u` of `undo:` eaten by a lane glyph. Found by eye in five minutes, on the running app, after ticket 30's border sweep passed green. The invariant is right; the sweep does not reach what ships. Ticket 34.
- 2026-09-22 — Scrolling: fine. No complaint.
- 2026-09-22 — **"really and I mean really hard to notice which row I'm on."** The cursor row does not read as selected at a glance in his theme. This is the most basic affordance in the app and nothing tests it (rule 5: captures carry no attributes). Ticket 35.
- 2026-09-22 — **A drag gives no feedback until you let go.** "when I'm dragging there is no indication that I have held something, until I release and a popup shows up." Both the highlighted target row and `HintContext::Dragging`'s `release drop on a row · esc cancel` bar were built and tested — and neither reached him. Ticket 35.
- 2026-09-22 — **He did not know dragging existed.** Discoverability, not a defect. A tutorial or first-run tour wanted once the architecture settles — his words: "I'll definitely need a tutorial when the architecture is finished." M8.
- 2026-09-22 — Design question, his: moving the cursor between commits does not show that commit's details the way Sublime Merge's right-hand pane does. Wants it investigated as *tool use* — purpose, convenience, what competitors do — not assumed. Handed to planner-fable.
- 2026-09-22 — **"im in dark mode, and the highlighter to show which row im on is also dark."** Second report of the same complaint, and this time with the piece that was missing: it is a **colour** problem, not a shape problem. Root cause, found in minutes once he said "dark": `graph_screen/render.rs:281` fills the cursor row with `SurfaceRaised` — `#232730` on a `#14161B` background in the dark theme, a ~1.3:1 ratio against a 3:1 floor. Ticket 35 diagnosed the same six-percent step in its own code comment and fixed the **fallback** (`▸`, unconditional) rather than the fill. The token is overloaded: `SurfaceRaised` is also a raised panel's background (`view/frame.rs:762`), where a small step is right. One token, two contradictory jobs, nothing testing either. Ticket 44.
- 2026-09-22 — **The `▏` cursor marker works; the fill is the whole problem.** Asked directly, he can see the marker ticket 35 made unconditional. So the cue that survives any palette reaches him, and ticket 44 stays narrow: the row fill, not the marker. Two useful bounds came with it — he is in **Zed's integrated terminal**, and the bar's tail of bare keys (`y  M  O` after `. more`) is ADR-0039's third amendment working as designed, checked before filing and **not** filed.
- 2026-09-22 — **Fixed, confirmed by eye: "yes i can see it now much better."** The cursor row's fill went 1.21:1 → 3.04:1 (dark), via a `selection` token split out of `surface-raised`. Third time this affordance was touched and the first time the user could see the result. What made the difference was one word from him — *"dark"* — which turned "hard to notice" into a measurable number. **Lesson for the next by-eye report: ask what theme and what terminal before filing.** He did not comment on the swallowed lane glyph; unasked is not unbroken, and the gap row says so.
- 2026-09-23 — **First real conflict, in Graft: stuck after the merge stopped.** He had to press `Esc`, move to the uncommitted-changes row and open it before he reached the conflicts — "a human needs not take 5 to 6 steps; whatever is enough is enough." Also, on the resolver: line numbers on the OURS side only; a stray `2` (the marker's line) before `── conflict 1 of 1`; the key bar not visible in his screenshot (unconfirmed — may be below his window). Feeds the conflict-resolution prior-art study (`docs/research/conflict-resolution.md`).
- 2026-09-23 — **Angry, rightly: the best-in-class study he asked for on 2026-09-17 was never done for M4–M6.** "Whenever making a project … one of the steps is researching the best way; if not found, then invent." Now a blocking pipeline in CLAUDE.md ("The feature pipeline"): need → prior art → take/reject/invent → step-counted journeys → prototype → design → build → verify. Araxis Merge added to the tools studied.
- 2026-09-23 — **Prototype walk: "confusing at which file I was talking with."** File identity didn't hold across the resolver (gone.txt → notes.txt). His suggestion: the Araxis/JetBrains layout — left, middle, right, the chosen side lands in the middle — plus a file list to choose from. Asked for an audit of it against the efficient, ideal solution; the study had rejected 3 columns on VS Code's #160806, which this reopens.
- 2026-09-23 — **Finishing a merge shows a summary, approved: "yes build it that way."** `c` opens one screen (merging X → Y · resolved by you, per file and choice · combined by git on its own, each openable · editable message); `⏎` commits, `Esc` back to the resolver. +1 key (6). Not a yes/no confirm: `u` already undoes, and a bare confirm teaches ⏎ without reading. Why: the conflicts are only part of what lands on main; git-combined changes are never seen otherwise.
