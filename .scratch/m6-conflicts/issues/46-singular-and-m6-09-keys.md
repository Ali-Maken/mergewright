# singular-and-m6-09-keys: `1 conflicts`; m6-09's capture shows a stray refusal

Status: ready
Blocked by: none (45 merged `e6d3d43b`).
Was: filed by the conductor 2026-09-24 from ticket 45 NOTES findings 2 and 3 (line checked by the conductor).
Build with: **Opus** (low).
Needs: none.
Crates: graft-ops (1), graft tests + scripts + docs (2).

## Scope
In:
1. **Singular.** `graft-ops/src/operation.rs:647-649`, `ConflictsOpen` → `"{open} conflicts still open in …"`, so the bottom row reads `1 conflicts still open in both.txt`. Failing test first (`open == 1`). Fix it, then grep the workspace for every other drawn `{n} conflicts` / `{n} files` / `{n} blocks` phrase with no singular. Fix the ones the user sees, same rule, each with a test. List them in NOTES.
2. **m6-09 keys.** The capture's keys `k ⏎ j j j j ⏎ d` carry a `⏎` that has been refused since ticket 17 (`j` moves files in the resolver), so the frame's bottom row shows a refusal, not the box's keys (`←/→ move  ⏎ choose  esc cancel`). Change the capture keys, tests only, so the journey reaches the box the way a user would today. Retake at 120x40 and 80x24, fix the captions, and re-run the m6 contact sheet (parts ≤ 200 000 B).
3. Any other capture whose words change because of 1: regenerate it by its own test and list it in NOTES.

Out: app behaviour other than the wording.

## Acceptance (each tick names its test, at tick time)
- [ ] 1: failing test first (`open == 1` → `1 conflict still open in …`) — test:
- [ ] 1: other singular sites found and fixed, each tested; the list is in NOTES — tests:
- [ ] 2: m6-09 bottom row shows the box's keys at both sizes — test: the pty walk frame check named in NOTES
- [ ] Sheet re-run: 0 uncaptioned — test: `every_scenario::contact_sheet_captions_cover_every_m6_capture`
- [ ] Gates workspace-wide: fmt, clippy `-D warnings`, nextest, doctests.

## NOTES
