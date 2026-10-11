# M8 pass 3 (2026-10-11): Mergewright, and usability Q1/Q2

Part of `docs/design-notes/m8-polish-and-ship.md`. Planner, after the user closed naming (field notes :96-99) and
decided Q1/Q2 (:97). Decisions are in ADR-0057's Amendment (2026-10-11) §1–§12; this file holds what the pipeline asks
for. Specs: `.scratch/m8-polish-and-ship/issues/{mergewright-*,usability-q*}.md`.

## Promise
**Your repositories open under Mergewright with nothing lost.** Every backup, journal entry and set-aside edit made
under Graft or Tributary is still his, and `u` still takes back his last real operation. Proved by
`mergewright-accept`.

## Need
- Him, daily, in his team repos: he launches the tool and gets on with work. The rename must cost him nothing: no
  step, no lost backup, no "where did my set-aside go". Field notes :96-99: he chose the name to read as a git utility
  at once, so the name has to be the same everywhere he sees it: command, man page, brew, J.
- Q1 (`m8-usability.md:155`): after every rebase, J says `✗ lost … no branch holds it` while `u` would bring it back,
  so it reads as damage. His words: "yes say replaced".
- Q2 (`m8-usability.md:159`): `⏎` on his own branch keeps the picker open with "you're on main". His words: "close the
  picker".

## Prior art
| Aspect | Best answer found | Ours | T/R/I |
|---|---|---|---|
| Moving a user's data under a new name | Copy, read back, delete; old readable forever (ADR-0057 2026-10-09, user-approved) | Same, generalised to any older generation | **T** (own prior decision) |
| Package vs crate names | dist names artifacts by package (ticket 24 B+2, read in dist's source) | Rename the shipping package only | **I**: no override exists |
| Homebrew formula | dist's template (compiled in; man page in `pkgshare`, 3 `brew style` offences) | Own 30-line template filled from dist's manifest | **R** dist's, **I** ours |
| Naming what an op left behind | `docs/research/reflog.md` (J's study): tools show reflog loss without saying if it is recoverable | `⇢ replaced`/`deleted` when undo brings it back; `✗ lost` otherwise | **I**: no tool found says it; UX H2, H9 |
| `⏎` on the current item | Switch rule: `.` Checkout says `already on <b>` (keymap :174) | Close, land on HEAD, say nothing | **I**: UX §3.1 (land where he asked), H8 |

No new prior-art study for Q1/Q2: they are word and landing fixes on screens whose studies exist (`reflog.md`,
`palette-find.md` / M7's picker). The user decided both. Judged against `docs/research/ux-principles.md`.

## Journeys (steps = keys + decisions, UX §1.3)
| id | Task | Steps | Before | Best tool |
|---|---|---|---|---|
| `open-older-names-repo` | Open a repo last used by Graft/Tributary | `mwr` = 1 | 1 | silent migrations (VS Code, Homebrew): 1 |
| `undo-after-rename` | Undo his last real op after the move | `u` = 1 | 1 | GitUp 1 |
| `journal-replaced` | See that a rebase's old tip is safe | `J` = 1, read | 1, read as damage | not studied |
| `picker-enter-current` | Pick his own branch | `C` name `⏎` = 3 | 4 (`+ Esc`) | J32's tools 3–4 |

Install and man stay J16/J17 (`brew install <owner>/tap/mergewright`, `man mwr`, person steps). The acceptance ticket
adds the four ids to the main note's `journey-counts` fence with their walks.

**Lands:**
| Event | Lands |
|---|---|
| Move done | graph, cursor on HEAD, no status line; J has one `● Mergewright … rename to Mergewright (N refs, M files)` |
| Move can't run (another window) | graph on the old names; `rename: still using the old names · another Mergewright window is open on this repository` |
| Move fails before the switch | old names in force; `rename: still using the old names · <reason>` (26r, unchanged) |
| `⏎` on the current branch | picker closed, graph focused, cursor on HEAD, nothing written |
| `⏎`/`b` on a `⇢` row | as on `✗` (keymap §2.11) |

## Interfaces and data shapes
In the specs; the load-bearing one is `graft_core::on_disk::Generation` (ticket 1, Fable signature). The others:
`graft::cli::COMMAND = Generation::CURRENT.command()`, `APP_NAME = …title()`, `on_disk::message_tag()`,
`journal_list::{Fate, fate_of}`, `journal_screen::RowKind::{Replaced, Deleted}`.

## Hard-to-reverse choices
1. **Generation table vs a second New/Old pair.** **Table**: a third rename is one row; a pair strands Graft repos.
2. **Rename the shipping package vs keep `graft`.** **Rename** (`mergewright`, lib `graft`): dist ships the package
   name, and `graft-installer.sh` is another product's name. Cost: every live `-p graft` moves once.
3. **Own the formula vs patch dist's.** **Own it**: fixes the cause; patching output is a workaround (rule 2).

## Where I'd differ from DESIGN.md
- 01-view-mockups §1.10: J's `$` rows say `outside`, not `outside Graft` (ADR §10). Small; follows the name.
- 27d's "re-shoot all 244": only M8's captures and the ones live docs show (ADR §12). The rest record what the app
  drew then. Trade-off: old milestones' sheets show the old name, labelled by date.

## New UI
- J row `└ ⇢ replaced  <sha> "<subject>" · was <b>'s tip · undo brings it back` (`deleted` for a branch delete),
  muted; legend clause `⇢ undo brings it back`, added only when it fits whole; no ASCII form (J has none). Eye lands on the `●` row above it (unchanged).
  `n`/`N` skip it. Empty and error states unchanged.
- Picker: the current row's bar clause is `⏎ show it on the graph`, `⇥`'s words for the same act (was `⏎ close`, beside `esc close`; amended 2026-10-12, review m8 g1 minor 1, conductor's call); the "you're on" message is gone.
- J column: `●  Mergewright` / `$  outside`. No other new screen.

## Genuinely risky tickets
- `mergewright-names-table-and-migration` Part A: **Fable (signature)**. Every later ticket calls the table.
  Part B is Opus: the move's order is unchanged and injected-failure tests judge it.
- Risky, not Fable: `mergewright-git-messages-and-journal-column` (a missed reader means a silent put-back miss, but
  git-truth tests judge it); `usability-q1-*` (undo semantics read, judged by journal + git tests).
- **Fable count: 1** (+ acceptance).

## Groups (max two cargo builders; every-crate tickets alone)
| # | Group | Model | Beside |
|---|---|---|---|
| 1 | `usability-q1-replaced-when-undoable` ∥ `usability-q2-picker-enter-on-current-closes` | Opus ∥ Sonnet (after scouts) | — |
| 2 | `mergewright-names-table-and-migration` A → B | Fable (sig) → Opus | `mergewright-protected-lines` (no cargo; waits for his yes) |
| 3 | `mergewright-package-and-command` | Sonnet | — |
| 4 | `mergewright-app-words` | Sonnet | — |
| 5 | `mergewright-env-names` | Sonnet | — |
| 6 | `mergewright-git-messages-and-journal-column` ∥ `mergewright-release-and-formula` | Opus (after scout) ∥ Opus | — |
| 7 | `mergewright-docs-sweep` ∥ `mergewright-recaptures` | Sonnet ∥ Sonnet (after scout) | — |
| 8 | `mergewright-accept` | Fable | — |

Then the whole-app usability walk (reviewer). Nothing is published, registered, tagged or pushed (ADR-0057 §1).
