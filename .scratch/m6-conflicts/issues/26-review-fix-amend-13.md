# review-fix-amend-13: a file-list click under `d`'s box closes the resolver; `c` refused on a stale status; two over-long files

Status: ready
Blocked by: none (13 merged). Runs **before** 14 and 25 (same crates, ADR-0036).
Was: filed by the conductor 2026-09-24 from `docs/reviews/m6-amend-13.md` Major 1, Major 2, Minor 1. Minor 1 promoted: the app says `every file done ✓ · c finishes the merge` and then refuses `c` — the words contradict the key.
Build with: **Opus** (medium).
Needs: none.
Crates: graft, graft-ui.

## Read first
- `docs/reviews/m6-amend-13.md`: "The builder's question", Major 1, Major 2, Minor 1 — each names file and line.
- `crates/graft/MAP.md` (`update::resolver`, `banner`, `jobs`), `crates/graft-ui/MAP.md` (`conflict_resolver::files`).
- Ticket 12's `ReadRefs(JobId)` guard (`.scratch/m6-conflicts/issues/12-landing-after-a-stop.md` NOTES) — the model the review suggests.
- `ARCHITECTURE.md`; `codebase-design` for the splits.

## Scope
In:
1. **Major 1.** A click on a file-list row while `d`'s confirm box is up closes the resolver and leaves the box (`update/resolver/files.rs:190-202` has no box check; `update/resolver.rs:447-453` removes the resolver before the refusal). Failing test first. Fix at the cause: a click on the list under a modal box does what every other key under that box does.
2. **Minor 1 — stale `c`.** After the last Stage, `answers.rs:281-298` says `every file done ✓ · c finishes the merge`, but `c`'s refusal counts the cached status (`banner.rs:200-206`, via `:77-86`) until the status refresh the Stage queued (`operate.rs:966-974`) lands. Failing test first (the review's: Stage last file, press `c` before the read lands). Fix at the cause, no sleep/retry/special case (rule 2). The review offers: give `ReadWorkingTree` a `JobId` (as 12 did for refs) and hold the `every file done` words until that read lands; or `continue_refusal` says `checking the files…` while the read is out. Choose, say why in NOTES. The words and the key must never disagree.
3. **Major 2.** Split, pure moves, ~400 lines per file where a seam exists: graft-ui `conflict_resolver/files.rs` (612); graft `update/resolver.rs` (476). Update MAP.md. No seam → say so with the count.

Out: Minor 2 (split computed twice) — ticket 14 moves the width source to `resolver_split`; every other Minor → KNOWN_GAPS.

## Acceptance (each tick names its test, at tick time)
- [ ] Major 1: failing test first, red on unchanged code — test:
- [ ] Major 1: a list click under `d`'s box leaves the resolver and the box as they were — test:
- [ ] Minor 1: failing test first, red on unchanged code — test:
- [ ] Minor 1: after the last Stage, the words and `c` agree at every moment before and after the status read lands — test:
- [ ] Major 2: splits done, line counts in NOTES, MAP.md updated — (gates green; list files)
- [ ] Gates workspace-wide: fmt, clippy `-D warnings`, nextest, doctests

## NOTES
