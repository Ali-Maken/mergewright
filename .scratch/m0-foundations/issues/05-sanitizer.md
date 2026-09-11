# m0-foundations/05 — Escape-sequence sanitiser (`term::sanitize`)

Status: todo
Build with: Sonnet
Blocked by: 01

Why: ticket 11 draws repo-derived strings (branch name, repo path); ARCHITECTURE.md makes `SanitizedText` the only way those cross into `ui`, so injection is a compile error. Small, pure, security-critical.

## Read first
- DESIGN.md §8.2, the `term::sanitize` / `SanitizedText` paragraphs (the exact character classes to strip)
- PROMPT.md §6.3 Security
- ARCHITECTURE.md (the SanitizedText boundary rule)

## Goal
`graft-term`: `sanitize(&str) -> SanitizedText`, a newtype only this function can construct.

## Scope
In:
- Strip/replace exactly the classes DESIGN.md §8.2 lists: C0/C1 controls except `\n`/`\t` (handled as §8.2 says), `ESC` itself, bidi override/isolate chars (U+202A–202E, U+2066–2069), zero-width/invisible formatting chars (U+200B–200F, U+2060, …). Where §8.2 leaves the replacement policy open, pick one visible policy (e.g. U+FFFD) and document it in the doc comment.
- `SanitizedText`: derefs/displays as `&str`; constructor private to the crate.
- Non-UTF-8 inputs: provide a lossy entry point for `OsStr`/bytes (paths are not always UTF-8 — PROMPT.md §6.3).

Out:
- Wiring into view models (ticket 11).
- Width/wrapping concerns.

## Acceptance
- [ ] Unit tests: an ANSI-injection commit message, OSC 8 hyperlink, bidi spoof and zero-width payload all come out inert.
- [ ] `proptest` (≤64 cases, PROMPT.md §7.1): output never contains a forbidden class; `sanitize` is idempotent.
- [ ] Outside `graft-term` there is no way to construct `SanitizedText` (compile-fail confirmed manually, noted in a test comment).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
