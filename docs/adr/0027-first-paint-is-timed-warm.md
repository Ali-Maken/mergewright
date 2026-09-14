# ADR-0027: First paint is timed warm; one cold check by the user

Status: Accepted (user, 2026-09-14)

## Context
PROMPT.md §6.3 sets a first-paint budget. At 1M commits (ticket 55) warm first paint is 92–112 ms, but the first run after the files leave the OS page cache takes ~1.0 s on both commit-graph paths (ticket 56). The suspected cause is disk reads of the pack and ref files. Only `sudo purge` can empty the page cache on macOS, so no test can prove it without the user's password.

## Decision
- §6.3's first-paint budget is judged on a **warm** run (repository files already in the page cache). Budget runs discard the first, cold run.
- At the M1 checkpoint the user runs one first paint straight after `sudo purge` on the 1M fixture, to confirm the cold cost is the page cache. Recorded in TESTING.md's manual checklist (ticket 28).
- The cold number stays a named row in KNOWN_GAPS until that check is done.

## Consequences
- The budget measures Graft's work, not the disk.
- A cold first open of a very large repository can take about a second. Nothing hides this: it's in KNOWN_GAPS and the M1 review.
