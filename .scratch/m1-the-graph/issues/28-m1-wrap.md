# m1-the-graph/28 — Wrap: manual checklist, checkpoint script, gaps sweep

Status: done (checkpoint question left for the user)
Build with: **Sonnet** (medium) — mechanical: a script, docs, a sweep
Blocked by: 26, 27d, 29, 30c, 30d, 35c, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 51b, 51c, 51d, 52, 52b, 53, 54, 55, 56, 56c, 56d, 57, 58, 58b, 59, 59b, 60, 61, 63, 64
Leaves runnable: yes — one command per checkpoint scenario.

Why: M1's checkpoint is the user opening `git/git` and the merge-mistake fixtures and judging whether the history shape is obvious at a glance. This makes that one command each and makes sure nothing was quietly left half-done. (Retired ticket 17, text in 35bec3e.)

## Read first
- KNOWN_GAPS.md — every row whose owner column says M1
- PROMPT.md §7.1 "Manual" row; §8 M1 row
- TESTING.md "The manual terminal checklist" and "Getting `git/git`"
- `docs/reviews/m0-foundations-round-2.md` "Unverified" section, for the shape of an honest report

## Scope
In:
- TESTING.md's manual matrix complete for M1: Ghostty, Kitty, WezTerm, iTerm2 (pixel); Terminal.app, Alacritty, VS Code, Zed (Unicode); tmux inside each; one SSH session. Pass condition per PROMPT.md §7.1: correct graph **or** clean fallback, never a broken screen or stray escapes. Merge the rows 08, 24, 25 and 29 filled in.
- **Two verdicts per terminal, not one** (user, 2026-09-13): (1) "correct or clean fallback" — the PROMPT.md §7.1 pass condition; (2) "reads well" — is the shape of history obvious at a glance (the M1 promise). Verdict 2 is the user's alone: leave it blank for the user, never fill it in. Put the Unicode tier in **Zed and VS Code** first and mark them as the user's priority (where the user works daily).
- `scripts/m1-checkpoint.sh <scenario>`: locates or clones `git/git`, builds the merge-mistake fixtures, prints and runs the command for each; says what to look at (default branch a straight line, where a merge went wrong, lanes distinguishable).
- KNOWN_GAPS sweep: every M1-owned row closed, or re-owned to a named later milestone with a reason. No row silently carried.
- The heavy-code clippy line (`--features heavy-tests`) added to `.claude/agents/reviewer.md`'s checks; closes that KNOWN_GAPS row.
- README and CHANGELOG say what M1 actually does.
- **Cold first paint check (ADR-0027, user 2026-09-14):** TESTING.md's manual checklist gets one row: run `sudo purge`, then first paint on the 1M commit-graph fixture; record the time and whether it matches the ~1.0 s cold number. Result slot left for the user.
- **Spec edits for accepted ADRs** (conductor, 2026-09-14): PROMPT.md §1.3 and DESIGN.md still describe the hover tooltip and pin (ADR-0023), "ancestry path never folds" in DESIGN §3.5 (ADR-0025), the fold marker/count and lane cap (ADR-0022) and stroke emphasis (ADR-0024). Edit those sections to match the ADRs, citing each; ADR text itself stays as record.

Out:
- Fixing anything the checkpoint finds.

## Signatures
Expose: `scripts/m1-checkpoint.sh <git-git|criss-cross|foxtrot|octopus|list>` — exit non-zero with a message on a missing prerequisite, never a silent skip.
Call: `scripts/clone-git-git.sh`; `cargo run -p graft-fixtures`; `cargo run -- <path>`.

## Acceptance
- [x] Every TESTING.md M1 row has a pass condition and a result slot.
- [x] Each checkpoint scenario verified to open the graph — see Notes: this builder has no real terminal (Bash tool has no TTY), so verification is a Python `pty.openpty()` harness with a real window size set (`TIOCSWINSZ`, 24x80), not a real emulator. `criss-cross` confirmed end-to-end: `cargo run -p graft-fixtures` builds the corpus, `shape_path` resolves the path, `cargo run -p graft -- <path>` renders actual graph content (lane glyphs `◆◉●◇`, curves, branch colours, commit subjects, relative dates, footer status) in the captured ANSI, and `q` exits 0 with the terminal restored. `git-git`'s no-network path confirmed separately: forcing `clone-git-git.sh` to fail (unwritable cache dir, since this sandbox may have network anyway) makes `m1-checkpoint.sh` exit 1 with the intended clear message, never a silent skip. `foxtrot`/`octopus` share `open_shape`'s code path with `criss-cross`, not independently pty-verified. None of this is by-eye verification in a real terminal — that's the user's, per TESTING.md.
- [x] KNOWN_GAPS.md has no M1-owned row undecided — see Notes.
- [x] Gates clean.
- [ ] **The checkpoint question stays unanswered until the user answers it.** (left unanswered, as required — box intentionally left unchecked)

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)
All three pass: fmt clean (after `cargo fmt` fixed one new function's formatting), clippy clean (plain and `--features heavy-tests`), `cargo test` 298+ tests across the workspace, 0 failed.

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.

## Notes (2026-09-14, builder-sonnet)

- **KNOWN_GAPS sweep:** two bare `| M1 |` rows found and re-owned:
  - "Is ED-frees-image-data Ghostty-specific?" → M1 checkpoint (resolved by the manual checklist's Kitty/WezTerm pixel rows).
  - "Sanitising does not cover look-alikes..." (confusables/variation-selector policy) → re-owned to "M1 checkpoint — open question", explicitly **not decided here**. **Question for the user:** should Graft ever flag homoglyphs or a variation-selector run in a ref name? Real false-positive risk either way; the row lays out the trade-off.
  - Added a new row for the `cargo run --` workspace-root ambiguity (`graft` vs `graft-fixtures` both define a binary) discovered while building this script — expiry M2, a small doc/script sweep or a `default-members` decision, whichever the user prefers.
  - Closed "The heavy code is not in the standard clippy gate" — `.claude/agents/reviewer.md` now runs `cargo clippy --all-targets --features heavy-tests -- -D warnings` alongside the plain gate.
  - Did not re-verify every other M1 row from scratch beyond the earlier grep pass; rows already owned to "M1 review", "M1 checkpoint", specific ticket numbers, or "Not planned" were treated as decided (sub-steps of M1, not a silent carry to a later milestone).
  - Did not find a separate "rows 08/24/25/29" table to merge — TESTING.md's pixel/Unicode/tmux-SSH tables were already single unified tables when read; this scope item looks already satisfied by those tickets' own edits, not independently re-verified against their original history.
- **Spec edits:** PROMPT.md §1.2/§1.3 and DESIGN.md §2.3, §2.15, §3.2, §3.5, §5.2, §7 updated for ADR-0022/23/24/25, each citing its ADR; ADR files themselves untouched.
- **New files:** `crates/graft-fixtures/examples/shape_path.rs` (resolves a fixture shape name to its on-disk path — the plumbing `m1-checkpoint.sh` needed since `graft-fixtures` has no CLI); `Shape::from_name` added to `crates/graft-fixtures/src/shape.rs`.
- **Not run:** the actual manual matrix (TESTING.md) and the checkpoint script in a real terminal — that's explicitly the user's, not this ticket's, to do.
