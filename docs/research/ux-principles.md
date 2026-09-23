# UX principles — standing reference for Graft design passes

Read at feature-pipeline step 4 (Journeys) — CLAUDE.md:32-33. Cite the principle by section number (e.g. "UX 1.3"), not taste.

**Source status (2026-09-23).** Every link was fetched and its claim checked unless marked `[unfetched: why]`. Those that remain: GOMS book (print only), Fitts 1954 and Hick/Hyman (the PDF copies returned 404 or refused the connection; the publisher returned 403), Kelley 1984 (the author's 2018 PDF returned 404). Treat claims resting on them as questions.

---

## 1. Counting steps

### 1.1 Keystroke-Level Model (KLM)
Source: Card, Moran, Newell, "The keystroke-level model for user performance time with interactive systems", CACM 23(7), 1980, 396–410. https://doi.org/10.1145/358886.358895 (paywalled). Read in full from a university course copy: http://iihm.imag.fr/blanch/ens/2010-2011/M1/EIHM/cours/1980-Card-KLM.pdf (Fig. 1 operator table, Fig. 2 M rules). The copy is a scan. The K, P and H values read cleanly. The table cell for M is garbled; 1.35 s is confirmed by the paper's text ("unless it is over 1.35 sec").

| Op | Meaning | Time |
|---|---|---|
| K | key press (Shift/Ctrl count as separate K) | best 0.08 · good 0.12 · average skilled 0.20 · average non-secretary 0.28 · random letters 0.50 · complex codes 0.75 · worst 1.20 s |
| P | point with mouse (excludes the button press, 0.2 s) | 1.1 s |
| H | hand moves keyboard↔mouse | 0.4 s |
| M | mental preparation (decide/recall) | 1.35 s |
| R | system response the user waits on | measured |

M placement (the part that matters; paper Fig. 2): Rule 0: M before every K that isn't part of an argument string, and before every P that selects a command (not an argument); drop M when the next op is fully anticipated (Rule 1); one M per cognitive unit, e.g. a typed command name (Rule 2); drop M before a redundant terminator like Enter (Rule 3); keep M before a variable string, drop before a constant one (Rule 4).

Takeaway for a TUI: **M dominates.** 1.35 s vs ~0.2 s per key — a journey with 3 decisions and 6 keys is ~5.3 s; one with 6 decisions and 3 keys is ~8.7 s. Cut decisions before keys.

### 1.2 GOMS
Goals, Operators, Methods, Selection rules — Card, Moran, Newell, *The Psychology of Human-Computer Interaction*, 1983 `[unfetched: book, no online copy]`. KLM is GOMS's simplest variant. Use for us: a journey = goal; write the **method** as operator list; where two methods exist (key vs mouse vs palette), state the **selection rule** the user will actually follow.

### 1.3 Graft counting rule (assertable in tests)
Count a journey as `steps = keys + decisions`:

- **key** = one chord sent (`g`, `Ctrl-r`, `Enter`, one mouse click). A typed string (commit message, branch name) counts as **1** — its length is the user's content, not our cost.
- **decision** (the M) = each point where the user must look/choose before the next key: picking a target from a list, reading a prompt, confirming. Not counted when the next key is fully anticipated (KLM Rule 1) — e.g. `Enter` after typing a name.
- **movement** (`j/k` to reach a row) counts as **1 key per motion command**, not per repeat, but the design note states the assumed distance ("target is under cursor" vs "N rows away").
- **H** (switch to mouse) counts 1. Mixed key/mouse journeys pay it.
- Start state is named ("in log pane, cursor on HEAD"); end state is "the result is visible".

Test assertion: the acceptance walk (CLAUDE.md:36) replays the journey's key list through the pty harness and fails if `len(keys) > note.keys`. Decisions aren't machine-countable; the reviewer checks them against the note.

### 1.4 Fitts's and Hick's law — where they apply
- **Fitts's law** (Fitts 1954, J. Exp. Psych. 47(6), 381–391) `[unfetched: uiowa.edu copy 404, wexler.free.fr refused connection]`: pointing time grows with distance/target-width. TUI relevance: **mouse only**. Cell-sized targets are small — make clickable things whole rows/whole labels, not a single glyph. Irrelevant for keyboard paths.
- **Hick–Hyman law** (Hick 1952, QJEP 4(1) 11–26, https://doi.org/10.1080/17470215208416600; Hyman 1953, J. Exp. Psych. 45(3) 188–196) `[unfetched: publisher 403; citation and formula from https://en.wikipedia.org/wiki/Hick%27s_law]`: for n equally likely options, choice time T = b·log2(n+1). TUI relevance: menus, hint bars, palettes. A which-key popup of 30 unsorted items costs more than 6 grouped ones. Does **not** say "fewer features"; it says group and order by likelihood.

---

## 2. Nielsen's 10 heuristics
Source (fetched): https://www.nngroup.com/articles/ten-usability-heuristics/ (updated 2024-01-30).

| # | Heuristic | For a keyboard TUI | Graft-shaped example |
|---|---|---|---|
| 1 | Visibility of system status | Long ops show progress in-pane; state (mid-rebase, conflicts, detached) always on screen | Rebase stopped at 3/7 shows `rebasing 3/7 · 2 conflicts` in the status line, not only in `git status` |
| 2 | Match between system and real world | Use the user's words (CONTEXT.md glossary), not git plumbing | "Your branch" / "their branch" beside ours/theirs, since ours/theirs flip in rebase |
| 3 | User control and freedom | Every mode has one obvious exit (`Esc`); undo exists | `Esc` from the conflict view returns to the log, cursor where it was |
| 4 | Consistency and standards | Same key, same meaning, every pane; follow lazygit/vim conventions where they exist | `j/k`, `/`, `?`, `q` mean the same in every pane (docs/design/02-keymap.md) |
| 5 | Error prevention | Show the result before confirming (M5 promise); block the impossible, don't warn after | Rebase preview shows the resulting graph before `Enter` |
| 6 | Recognition rather than recall | Keys visible at the point of use | Hint bar lists the 4–6 keys valid in the focused pane |
| 7 | Flexibility and efficiency | Single-key accelerators for experts; palette/menu for novices; counts, repeats | `c` commits directly; palette finds "commit" for those who forgot |
| 8 | Aesthetic and minimalist design | Every glyph earns its cell; status only when there's news (CLAUDE.md status-field rule) | Show `gfx: <error>` only on error, nothing when fine |
| 9 | Recognize, diagnose, recover from errors | Error says what, why, and the next key | "Push rejected: remote has 2 new commits. `p` pull-rebase, `Esc` cancel" |
| 10 | Help and documentation | `?` shows context help for this pane, searchable | `?` in conflict view lists only conflict-view keys |

---

## 3. Flow

### 3.1 Land the user where the next action is
After any event (op finishes, stops, fails), ask: **what will he do next, and is the cursor already on it?**
- Stops (merge/rebase conflict) → open on the first conflicted file, first hunk; next key resolves it.
- Finishes → cursor on the new commit/branch tip it produced, so the result is visible (heuristic 1) and the follow-up (push, amend) is one key.
- Fails → error in view with the recovery keys (heuristic 9); state unchanged, cursor unchanged.
- Never land on a neutral "done" dialog that needs a dismiss key: that's a step with no work in it.
This is Graft's own rule (CLAUDE.md:32), a direct application of heuristics 1 and 9 plus KLM (a dismiss = K+M wasted).

### 3.2 Modes and modal traps
- Modes are fine when the mode is **visible** and **escapable with one key** (heuristics 1, 3). Vim/helix prove modal keyboards work for experts.
- A **trap**: mode that isn't shown on screen, or whose exit differs per mode, or a modal dialog blocking the info needed to answer it (e.g. "Force push?" covering the graph that shows what'd be lost).
- Rule: every mode has an on-screen label and `Esc` leaves it. NN/g on modes (https://www.nngroup.com/articles/modes/): mode errors come from not showing which mode is active. It recommends redundant, clear indicators of the active mode, and avoiding modes where a mistake would do harm.

### 3.3 Progressive disclosure
Show the common options first; advanced ones one step away. NN/g: https://www.nngroup.com/articles/progressive-disclosure/ (essentials first, advanced options on request). TUI form: default action on the plain key, variants behind a submenu (hypothetical Graft example: `P` push; `P` then `f` force-with-lease). Magit's keyboard-driven menus are built on Transient: https://docs.magit.vc/transient/ ("well-designed transients guide their users to the relevant set of options"). Cost: the variant is +1 key. Only hide what's used rarely.

### 3.4 Discoverability of keys
| Pattern | Example | Link |
|---|---|---|
| Persistent hint bar | lazygit `showBottomLine`: "the bottom line that contains keybinding info" | https://github.com/jesseduffield/lazygit/blob/master/docs/Config.md |
| Keybindings menu (`?`) | lazygit global `?` "Open keybindings menu" | https://github.com/jesseduffield/lazygit/blob/master/docs/keybindings/Keybindings_en.md |
| Popup after prefix key | Emacs which-key "displays the key bindings following your currently entered incomplete command (a prefix) in a popup". Helix has an `auto-info` option ("Whether to display info boxes"). Neither fetched helix page says when info boxes appear; that they appear after `space`/`g` is a question. | https://github.com/justbur/emacs-which-key, https://docs.helix-editor.com/editor.html, https://docs.helix-editor.com/keymap.html |
| Command palette | VS Code `⇧⌘P` / `Ctrl+Shift+P`, "access to all functionality… including keyboard shortcuts". Whether each entry shows its shortcut is not stated in the fetched text; that is a question. | https://code.visualstudio.com/docs/getstarted/userinterface |
Combine: hint bar (recognition, heuristic 6) + palette that **teaches the shortcut** (path from novice to expert, heuristic 7). Hick (1.4): group palette/which-key entries.

### 3.5 Undo permits fewer confirmations
If an op is reliably undoable, ask no confirmation — undo is the safety net and the confirm is a wasted K+M every time. Confirm (or preview) only what undo can't reverse (e.g. push to a shared remote). Heuristics 3 and 5; Graft's M3 promise "you cannot lose work". NN/g (https://www.nngroup.com/articles/confirmation-dialog/): "if you warn people too much, they stop paying attention"; "go to great lengths to provide undo".

---

## 4. Prototyping UX cheaply
Pipeline step 5 (CLAUDE.md:34): smallest thing that shows the journey, before specs.

- **ASCII mockup** — draw each state of the journey as a terminal-sized text block in the design note, one block per event, cursor marked. Cheapest; shows landing points (3.1). Paper prototyping, cheap and early, before development: https://www.nngroup.com/articles/paper-prototyping/.
- **Wizard of Oz** — the user presses keys; a human (or a script of canned screens) shows the next frame. Tests the journey without real git logic. Origin: J. F. Kelley, ACM TOIS 2(1) 26–41, 1984 `[unfetched: citation from a search result; the author's 2018 JUS PDF returned 404]`.
- **Throwaway TUI prototype** — fake data, real keys, no backend; delete after. Findings to an ADR (CLAUDE.md:109).
- **Five users** — Nielsen: 5 users find ~85% of problems. Spend a 15-user budget as 3 rounds of 5, redesigning between rounds. https://www.nngroup.com/articles/why-you-only-need-to-test-with-5-users/. Graft has one user; so: many small rounds with him.

### 4.1 Ten-minute hallway test (one user)
1. (1 min) Give a **task in his words**, not UI words: "you're mid-rebase and it stopped — get it finished." Don't name keys or panes.
2. (6 min) Think-aloud: he says what he's looking for. You **say nothing**; no hints. If stuck >30 s, note it, then help.
3. Watch for: first key he tries (reveals expectation); hesitations (hidden Ms); reading the hint bar vs guessing; `Esc` presses (lost/trapped); where his eyes go after each event (did the landing work?); any reach for the shell (log it — TESTING.md "Times I left").
4. (3 min) Ask: "What did you expect to happen when you pressed X?" "What would you do next here?" "What was confusing?" Avoid "did you like it" — reports preference, not usability.
5. Record: keys actually pressed vs the note's count; each stall with its screen. Into `docs/field-notes.md` same day.

---

## 5. Planner checklist (run on every new screen/journey before specs)
1. After each event (finish / stop / fail), where does the cursor land, and is the next action **one key** away? (3.1)
2. Journey counted as keys + decisions (1.3), next to the best tool's count; any step with no work in it (dismiss, re-navigate) removed?
3. Which step has the most decisions (M)? Can the app pre-select the likely answer?
4. Is every mode visible on screen, and does `Esc` leave it? (3.2)
5. Is system state (op in progress, conflicts, detached, dirty) visible without asking? (H1)
6. Are the keys valid here shown here — hint bar or `?`? (H6, 3.4)
7. Every key checked against `docs/design/02-keymap.md`; same meaning as in other panes? (H4)
8. For each destructive op: undoable → no confirm; not undoable → preview the result before confirm. (3.5, H5)
9. Every error message says what, why, and the recovery key? (H9)
10. Words match CONTEXT.md and the user's own words, not plumbing? (H2)
11. Common path on plain keys, variants one step behind; nothing common hidden? (3.3)
12. Mouse: are targets whole rows/labels, and does a mixed journey avoid repeated H switches? (1.4)
13. Does the screen show anything that isn't needed for this task or status that reads as error when fine? (H8)
14. Is there an ASCII mockup of every state in the journey, and has the user seen it or a prototype? (4)
15. Is the journey count written so the acceptance walk can assert it? (1.3)
