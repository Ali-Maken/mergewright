# 08r — every done line leads with its tick

Status: absorbed by 08s (planner, 2026-10-18; `08s-commit-fixes.md` Decided "M4 + 08r", Acceptance "08r"). Was: ready (conductor, 2026-10-18, from 08's one open box)
Blocked by: 08-commit-screen (merged)
Build with: Sonnet — one renderer, captures follow.
Why: the accepted commit mock draws `✓ Committed 3 files to wallet-release.  u undo · P push` (`prototypes/ease-mocks/mergewright-commit-screen.html:178`). The shared renderer for `Status::Done` puts the tick after `what` today (08 NOTES, box `:84`). Build to the mock (memory `build-to-the-mocks`). Every done line goes through the same renderer, so they all change; one look for one meaning.

## Scope
- The shared `Status::Done` renderer draws `✓ <what>  <keys>`. Find it from 08's NOTES. The `gfx ok` / status-state rule (CLAUDE.md) is unchanged.
- Re-shoot every capture whose only diff is the tick's place (D3-style: list them in NOTES with their writer test). Any other diff → stop and report.
- Tick 08's box `:84` in `08-commit-screen.md`, naming its test.

## Touches
Every `Status::Done` producer (grep); captures under docs/shots; pty waiters matching done words; man page / capabilities if they quote a done line.

## Acceptance
- [ ] A state + render test: a commit's done line renders `✓ Committed 3 files to main.` with the keys after it.
- [ ] Every other `Done` producer renders tick-first (one table test over the producers, or the renderer test plus a grep proving one renderer).
- [ ] Re-shot captures listed in NOTES; look unjudged (rule 6), colour unverified (rule 5).
