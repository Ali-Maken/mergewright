# 30l: small code fixes from review 10..07c (Minors 2, 8, 9)

Status: ready
Build with: Sonnet. Three local changes, each named with its line in the review.
Blocked by: 08-a (holds graft `update/operate.rs`, `update/preview.rs`).
Scout: none. The review names every line (`docs/reviews/m7-10-to-07c.md`, Minors 2, 8, 9).
Files: graft `src/update/operate.rs` (only `stopped`, ~:1008-1020, and its callers' argument), `src/update/preview.rs`
(:976 only), `src/event_loop/running.rs` (comments only); graft-ops `src/preview.rs`, a new child for `shown_aside`; tests beside each; MAP.md per crate.

## Fixes (decided)
1. **Minor 2:** `stopped(stop: Operated, …)` returns `Vec::new()` on a non-`Conflicted` value. Pass the `Conflicted`
   fields (or a struct) so the type carries the invariant. No silent `else`.
2. **Minor 8:** the unborn-HEAD take words (`update/preview.rs:976`) → `check_pick`'s words, as ticket 11 Decided. If
   unreachable, say why in a comment, with the line of the earlier `not_a_commit`.
3. **Minor 3 (source part):** stale comments in graft `src/event_loop/running.rs:210` ("These three"), `:232` ("one of
   the three"), `:284` ("exactly these four") — the match at `:212-217` has six. Word them from the code.
4. **Minor 9:** move `shown_aside` (graft-ops `preview.rs:404-424`) beside `SetAside` in a child; `preview.rs` ≤ ~400.

## Acceptance
- [ ] Minor 2: compiles with no fallback; existing stop tests green (name them).
- [ ] Minor 8: test naming the words, or the comment with its line.
- [ ] Minor 3: comments match the six arms (grep shown).
- [ ] Minor 9: `wc -l` before/after; no test changed except `use` paths.
- [ ] MAP.md; gates workspace-wide.

## Touches (check each, say how)
Callers of `stopped`; every preview title test; `shown_aside` callers.

## NOTES (builder)
