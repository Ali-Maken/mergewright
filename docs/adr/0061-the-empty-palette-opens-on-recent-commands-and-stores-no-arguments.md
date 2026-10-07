# ADR-0061 — The empty `⌃P` box opens on recently run commands; a recent row stores no argument

Date: 2026-10-07
Status: **Accepted**, 2026-10-07 (field notes :92, items 2 and 3: "your recommendation"). Design note
`docs/design-notes/m8-polish-and-ship.md`, "Where I'd differ", P-find-1 and P-find-2. Specs
`.scratch/m8-polish-and-ship/issues/12-palette-find-anything.md`, `12b-palette-find-commits-and-files.md`.
Amends: DESIGN §2.12 (`docs/design/02-keymap.md`, the `Ctrl+P` line's order for an empty box) and §1.6
(`docs/design/01-view-mockups.md`, the sample row `git rebase main`). Builds on ADR-0056 (one palette widget).

## Context
- §2.12 opened `⌃P` on "actions, then branches, tags, files, commits, then recently run commands", typed or not.
  On an empty box that is eight branches, tags, files and commits nobody asked for, and the recent group last.
- §1.6 drew the recent group's row as `git rebase main`: a command with its argument.
- Prior art (`docs/research/palette-find.md`, "JetBrains"): Search Everywhere opens on recent items. VS Code's empty
  box is a Question in the study (design note Prior art, "Find: empty box"); rerunning the last command there is 3
  keys (design note J31).

## Decision
1. **Empty box (P-find-1).** `⌃P` / `:` with nothing typed lists `COMMANDS RUN RECENTLY` (when any), then `ACTIONS`.
   Branches, tags, files and commits appear only once he types. The cursor is on the first runnable row, so `⌃P ⏎`
   reruns the newest command. The empty field reads, muted, `type to find actions, branches, tags, files, commits`;
   gone on the first character. Typed: §2.12's order stands (actions, branches, tags, files, commits, then recently
   run commands). `?` mode is unchanged (no find groups, no recent group).
2. **No stored argument (P-find-2).** The header is `COMMANDS RUN RECENTLY` (not `RECENT`: §1.15 already uses that word
   for the repo list, and the palette opens over it). A row is the action row as §1.6's M8 paragraph draws it (key,
   label, config id): the last 8 distinct actions run from either palette mode this session, newest first, a rerun
   moved to the front. `⏎` runs it as its key would (§2.12) and asks for its argument again.

## Why
- UX §1.4 (order by likelihood): on an empty box the likely want is what he just ran. J31 drops from 3 keys to 2.
- H8 (minimal): find rows for an empty query are noise. H6 (recognition): the placeholder says what the box finds.
- §2.12's `Enter` runs "as its key would"; a stored argument would make a rerun a different action from the key's,
  with its own preview and words. One action, one path.
- H4 (consistency): one word, one meaning — `RECENT` stays the repo list.

## Trade-off
- The empty box no longer *shows* that refs and files are findable; the placeholder *says* it.
- `⌃P ⏎` reruns the newest command, preview and all (a write still asks its `y`).
- Rebase onto `main` again is `⌃P ⏎`, then pick `main` — not one step.
- Recent runs live for this session only (persisting them is out of scope, spec 12).

## Rejected
- **§2.12's order on an empty box** (actions, find groups for the empty query, recent last): J31 = 3; noise.
- **Store the argument** (`git rebase main` rows): a rerun that is not the key's action; a stale argument (a deleted
  branch) needs its own failure words.
- **Header `RECENT`**: collides with §1.15's repo list.
