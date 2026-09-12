# m1-the-graph/15 — Search over history

Status: ready
Build with: Opus
Blocked by: 06, 10

Why: PROMPT.md §8 lists search in M1, and on a million-commit repo search is the only way to reach a commit that is not on screen. It is the first feature that has to admit "I have not read all the history yet" without lying about it.

## Read first
- DESIGN.md §2.3 (the graph keymap: `/`, `n`, `N`), §3.6 (checkpoints and deep jumps)
- PROMPT.md §1.4 (input-to-frame budget) and §7.1 (cancellable jobs)
- CONTEXT.md — "job"

## Goal
`/` finds commits by subject, author, sha prefix or full message, over history that may still be streaming, and never blocks the UI thread.

## Scope
In:
- Search runs as a cancellable job (M0's runner), incremental: results appear as they are found, the first match is jumped to as soon as it exists.
- Fields: subject, author name/email, sha prefix, full message body. Case-insensitive by default; a literal substring match is enough for M1 — no regex, no query language.
- `n` / `N` walk hits forward and backward; jumping to a hit far from the viewport uses ticket 06's checkpoints, not a re-walk from HEAD.
- Honest state while history is still streaming: the count must read as partial (e.g. `12 matches so far`), never as a final total, and it must settle visibly when the walk completes.
- Zero matches, search cancelled, and search-failed are three distinct visible states. No silent empty result.
- Search matches against the **raw** repo text, but anything rendered goes through `SanitizedText` — a match must never be the route by which an escape reaches the screen.
- `Esc` cancels the search job and restores the prior selection.

Out:
- Searching diffs or file contents (that is a later milestone's read).
- Filtering the graph to matches only — highlight and jump, don't re-layout.

## Acceptance
- [ ] Tests over a fixture: match in each field, no match, match beyond the first chunk, `n`/`N` wrap behaviour, cancel mid-search.
- [ ] A search started while the walk is still streaming yields a count that is labelled partial and later settles — asserted, not assumed.
- [ ] Deep jump to a far hit is measured, not guessed: report the time from `Enter` to the row on screen on the 100k fixture (heavy tier).
- [ ] Input stays responsive during a search over the big fixture — say how you verified it.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
