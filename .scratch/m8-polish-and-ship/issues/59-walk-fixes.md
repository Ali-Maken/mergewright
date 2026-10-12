# 59 — usability walk fixes (Esc during a write in J; blank row after undoing a switch)

Status: ready
Build with: Opus (git writes, journal, keys)
Blocked by: none
From: docs/reviews/m8-mergewright-usability.md (lines verified by the conductor)

## Major A — J's bar says `J/esc close` while `Esc` cancels a running write
`crates/graft/src/update/keys.rs:262-269` stops any write on `Esc` (design note, choice 4) before J sees the key;
`crates/graft-ui/src/journal_screen.rs:251-252` still offers `J/esc close`. The walk saw it cancel a push.
The behaviour is the design's; the bar is wrong. Read `docs/design/02-keymap.md` first; while a write runs, the bar
must say what `Esc` does (and `J` alone closes). Check every other screen/overlay whose bar offers `esc` while a write
can run — same key, same meaning.

## Major B — `u`/`U` on a switch leaves a blank `$ outside … git: ""` row in J
`crates/graft-ops/src/restore/steps.rs:131-135` runs `symbolic-ref HEAD <name>` with no `-m`, so the reflog line has an
empty message, and `crates/graft-ops/src/journal_list/owners.rs:51-58` can't claim it. Fix the cause (give the
restore's HEAD move a message the owners can match, from the names table / `MESSAGE_TAG` as other writers do), and
check every other restore/undo step that moves HEAD or a ref without `-m`.

## Small, same files (review minors)
- m3: own cancel reads `failed: cancelled` — a status must not read as an error when nothing is wrong (CLAUDE.md).
- m6: `?` in J labels `⏎` "Revert / put back (preview)" but on `← u`/`← U` rows `⏎` runs at once — make the label true.
- m7: `docs/man/mwr.1:3` NAME line lacks the product name (regenerate via its writer if generated).
- m10: `b makes restored at d7943ab` — quote the branch name so the sentence reads.

## Touches (check each, name how)
Keymap doc; every bar that offers `esc`; J help (`?`); captures of J/push/status lines (re-shoot any whose text
changes, list for the user's eye); journal ownership tests; undo/redo walks; man page writer.

## Acceptance
- [ ] Major A: test, red first — while a write runs, J's bar names `Esc`'s real action (name it, quote red)
- [ ] Major B: test, red first — `u` on a switch leaves no blank `$ outside` row (real repo)
- [ ] m3, m6, m7, m10 each a named test or grep
- [ ] Gates workspace-wide green after last edit
