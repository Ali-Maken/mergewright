# Mergewright 9: the lines in CLAUDE.md, PROMPT.md and `.claude/**` that the name changes — for the user

Status: ready (user approved the protected-line edits 2026-10-11)
Blocked by: 49
He said yes to "about 20 lines naming the product in CLAUDE.md, PROMPT.md and the reviewer's instructions" before
seeing the table; a builder applies exactly the table's rows, nothing beyond it.
Build with: **Sonnet**, after his yes only. Runs no cargo, so it can go beside anything.
Needs: none to list. To apply: `mergewright-package-and-command` merged (`-p mergewright` exists).

## Read first
- ADR-0057 Amendment (2026-10-11) §9 (cost: `reviewer.md`), §12 (codename vs product name)
- `.scratch/m8-polish-and-ship/27c-protected-lines.txt` (the Tributary-era list this replaces)

## The rule (§12)
- **Product lines change.** PROMPT.md is the product spec: where it names the product, its command, its config path or
  its refs, it says Mergewright / `mwr` / `mergewright`.
- **Codename lines stay.** CLAUDE.md's title and `.claude/**` descriptions ("Builds one Graft ticket") name the
  project. They stay `Graft`.
- **One functional line changes:** `.claude/agents/reviewer.md:20` runs `-p graft`, which stops working when the
  package is renamed.

## The list (line numbers at `1a0d37c2`; re-check by grep before applying)

| # | File:line | Today | Proposed |
|---|---|---|---|
| 1 | CLAUDE.md:5 | `"Graft" is a **codename**. The name is taken by another Git GUI; the real one is chosen in M8. Never publish, register or tag anything public under it.` | `"Graft" is the project's **codename** (another Git GUI's name): never publish, register or tag anything under it. The product is **Mergewright**, command `mwr` (ADR-0057); nothing goes public under that either until the trademark search is done.` |
| 2 | PROMPT.md:1 | `# Build Prompt: "Graft", a graphical terminal Git client` | `# Build Prompt: "Mergewright", a graphical terminal Git client` |
| 3 | PROMPT.md:3 | `… "Graft" is a codename; the real name is chosen in M8.` | `… The product is Mergewright (command mwr); "Graft" remains the project's codename (ADR-0057).` |
| 4 | PROMPT.md:13 | `Build **Graft**, …` | `Build **Mergewright**, …` (word swap only) |
| 5 | PROMPT.md:23 | `… Graft must make those mistakes obvious …` | word swap `Graft` → `Mergewright` |
| 6 | PROMPT.md:27 | `… Give Graft its own identity.` | word swap |
| 7 | PROMPT.md:60 | `` `~/.config/graft/config.toml` `` | `` `~/.config/mergewright/config.toml` `` |
| 8 | PROMPT.md:69 | `Graft detects which tier …` | word swap |
| 9 | PROMPT.md:163 | `… the main way Graft makes history editing safe …` | word swap |
| 10 | PROMPT.md:170 | `╭─ ◆ graft ─ ~/code/api ─ …` (header mock) | The graph header draws no product word (graft-ui MAP: `APP_NAME` is drawn by the empty state only). Proposed: drop `◆ graft ─` and keep the rest of the line, so the mock shows what the app draws. Builder confirms against a committed M8 graph capture's top line and quotes it. |
| 11 | PROMPT.md:216 | `… copy text out of Graft.` | word swap |
| 12 | PROMPT.md:242 | `This is Graft's headline safety feature … Graft's answer is …` | word swap (both) |
| 13 | PROMPT.md:244 | `Graft captures it for undo by default …` | word swap |
| 14 | PROMPT.md:246 | `Undo reverses the last Graft operation …` | word swap |
| 15 | PROMPT.md:247 | `Before each operation, Graft records ref states …` | word swap |
| 16 | PROMPT.md:248 | `` `refs/graft/backup/<timestamp>` `` | `` `refs/mergewright/backup/<timestamp>` `` |
| 17 | PROMPT.md:273 | `… where Graft is meant to be better.` | word swap |
| 18 | PROMPT.md:277 | `` `graft [path]` `` | `` `mwr [path]` `` |
| 19 | PROMPT.md:608 | `Killing Graft with kill -9 … on relaunch Graft offers …` | word swap (both) |
| 20 | .claude/agents/reviewer.md:20 | `cargo test --release -p graft --features heavy-tests …` | `-p graft` → `-p mergewright` (else the release perf run fails to start) |

**Kept, with the reason** (§12, codename): CLAUDE.md:1 (`# Graft: rules for Claude`); PROMPT.md:504
(`graft_fixtures::proptest_cases`, a Rust path); `.claude/skills/conduct/SKILL.md:3`; `.claude/agents/*.md`
descriptions and "for Graft" lines (builder-fable :3 :8, builder-opus :3 :8, builder-opus-xhigh :8, builder-sonnet
:3 :8, debugger :3, planner-fable :9, researcher :3 :9, reviewer :3, scout :3 :9). 27c's list had these as Tributary
swaps; under §12 they name the project, and renaming them would make every agent file disagree with CLAUDE.md:1.

## Acceptance (after his yes; each box with its evidence)
- [ ] Each approved row applied verbatim; `git diff --stat` touches only CLAUDE.md, PROMPT.md and
      `.claude/agents/reviewer.md` (quoted).
- [ ] `rg -n '\bGraft\b|refs/graft|~/.config/graft|\bgraft \[path\]' PROMPT.md` returns only row-kept lines (quoted).
- [ ] The user's words of approval quoted in NOTES with the date (from his message, as relayed by the conductor).

## Touches
- PROMPT.md is cited by section in design notes and reviews: section numbers don't change (lines are edited in
  place, none added or removed). `rg 'PROMPT.md:[0-9]+'` across live docs: any citation of a changed line keeps its
  number; listed.
- Reviewer dispatches: the conductor's command text matches row 20 either way.
- **Code / screens / keymap / journal / captures:** none.

## Gates
None (no code). `rg` outputs to a file; summary quoted.

## Size
Small: ~10k tokens.
