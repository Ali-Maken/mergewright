# ADR-0056: `?` is the palette opened on this place; the tour is a practice repo

Status: accepted (user, 2026-10-02: "go ahead with your recommendation" — the M8 note's Proposed changes 1, 2, 3).
Amends DESIGN.md §1.15 (tour paragraph), §1.16, §2.2, §2.12, §2.13; PROMPT.md §5 ("A 30-second first-run tour").

## Context

- The M6 usability walk: "`?` does nothing", `⌃P` and `:` too (KNOWN_GAPS M6/u2). PROMPT §9: a new user does the seven
  daily tasks "using only the hint bar and the command palette".
- He: "I'll definitely need a tutorial when the architecture is finished" (field notes, M5).
- Research (`first-run.md`, `keymap-help-remap.md`): lazygit's `?` menu is the best-liked help, and users ask for it
  to be "contextual to the current mode" (#3141). Hands-on tours (vimtutor, helix `:tutor`, JetBrains' learning
  project) beat callouts: "if you only read the text, you will forget". Complaints: a tour that shows default keys
  (helix #13483), a tour with no visible exit (zellij #5064).
- DESIGN.md had a separate `?` key grid (§1.16), `?` on the empty state replaying the tour (§2.13), and four 30-second
  callouts on his real repo (§1.15).

## Decision

1. **`?` means help on every screen**, the empty state included (same key, same meaning — UX H4).
2. **`?` is the command palette opened on "this place"**: one widget, two entries. `?` opens it filtered to the focused
   place's keys, an `IN PROGRESS` group first while an operation is live, then `HERE`, then `EVERYWHERE`. `⌃P` / `:`
   open it on everything (actions, branches, tags, files, commits, recent). Rows are runnable with `⏎` and show the
   key and the action's config id (ADR-0055). Typing filters names and hidden search words (`aside` finds stash); a
   typed key that is bound ranks its row first.
3. **The tour is a practice repo, done by doing.** Graft builds a throwaway repository in a temp dir and opens it with
   a three-row card: 8 tasks (stage a hunk, commit, undo, stash and back, branch, merge with its preview, resolve a
   conflict, push to a local remote). A task ticks when it is done, never when read. The header reads
   `◇ practice · <tmp> · a throwaway repo, not yours`. `Esc`, when nothing else on screen would take it, leaves to his
   repo with his cursor where it was. Opt-in: the palette's `tour` row, or `t` on the empty state. ~3–5 minutes.
4. **First launch** is one muted status-row line, never a popup: `first time? ? keys here · ⌃P anything · ⌃P "tour" to
   practise`, gone after `?` or `⌃P` is used once or after 5 launches.

## Why not the alternatives

- **A separate key grid for `?` (§1.16):** two surfaces to build and keep in sync; a grid cannot run the action.
- **Callouts on his real repo:** undo and merge cannot be practised safely there; read-only callouts are forgotten.
- **Tour on first launch, automatically:** lazygit-style popups need a dismiss key, a step with no work in it (UX §3.1).

## Consequences

- At 80×24 the list shows ~18 rows at once and scrolls; the old grid showed more keys per screen. Accepted.
- The tour needs the app to close one repository and open another in-process (`Exit::Open`), which the empty state's
  Recent list needs too.
- The practice repo builder must never write outside its temp dir: a path guard and a test.
- `t` is free on the empty state, which has no repository vocabulary (§2.13).
