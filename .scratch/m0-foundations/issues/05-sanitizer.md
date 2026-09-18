# m0-foundations/05 — Escape-sequence sanitiser (`term::sanitize`)

Status: done (2026-09-12)
Build with: Sonnet
Blocked by: 01

Why: ticket 11 draws repo-derived strings (branch name, repo path); ARCHITECTURE.md makes `SanitizedText` the only way those cross into `ui`, so injection is a compile error. Small, pure, security-critical.

## Read first
- docs/design/08-architecture.md §8.2, the `term::sanitize` / `SanitizedText` paragraphs (the exact character classes to strip)
- PROMPT.md §6.3 Security
- ARCHITECTURE.md (the SanitizedText boundary rule)

## Goal
`graft-term`: `sanitize(&str) -> SanitizedText`, a newtype only this function can construct.

## Scope
In:
- Strip/replace exactly the classes docs/design/08-architecture.md §8.2 lists: C0/C1 controls except `\n`/`\t` (handled as §8.2 says), `ESC` itself, bidi override/isolate chars (U+202A–202E, U+2066–2069), zero-width/invisible formatting chars (U+200B–200F, U+2060, …). Where §8.2 leaves the replacement policy open, pick one visible policy (e.g. U+FFFD) and document it in the doc comment.
- `SanitizedText`: derefs/displays as `&str`; constructor private to the crate.
- Non-UTF-8 inputs: provide a lossy entry point for `OsStr`/bytes (paths are not always UTF-8 — PROMPT.md §6.3).

Out:
- Wiring into view models (ticket 11).
- Width/wrapping concerns.

## Acceptance
- [x] Unit tests: an ANSI-injection commit message, OSC 8 hyperlink, bidi spoof and zero-width payload all come out inert.
- [x] `proptest` (≤64 cases, PROMPT.md §7.1): output never contains a forbidden class; `sanitize` is idempotent.
- [x] Outside `graft-term` there is no way to construct `SanitizedText` (compile-fail confirmed manually, noted in a test comment).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Done

`crates/graft-term/src/sanitize/`. Four entry points, one loop behind them:

- `sanitize(&str)` — multi-line text (commit bodies, diff hunks). `\n` and
  `\t` pass through for the widget to lay out, per docs/design/08-architecture.md §8.2.
- `sanitize_line(&str)` — anything drawn on one line. Replaces `\n` and `\t`
  as well: a file name may legally contain either, and drawn raw in a status
  row a newline scrolls the screen and a tab jumps the cursor.
- `sanitize_path(&Path)` / `sanitize_path_bytes(&[u8])` — the lossy entry
  points for paths that are not valid UTF-8 (PROMPT.md §6.3). Single-line,
  because file names are drawn in rows and headers.

`SanitizedText` has a private field and this module holds the only
constructors. One hostile character in, one visible placeholder out: `␛`
(U+241B) for C0/DEL/C1, `�` (U+FFFD) for the format category and the line
separators. Character count is preserved, so nothing is silently deleted, and
the placeholders are themselves clean, so sanitising is idempotent.

**Widened past §8.2 — needs the user's ruling at the M0 checkpoint.** §8.2
lists five ranges of invisible characters; this takes the whole `Cf` category
plus U+2028/U+2029. §8.2's ranges are a subset, the extras are invisible by
the same argument (soft hyphen, U+061C, the U+E0000 tag block, which can
carry a whole hidden sentence), and a rule stated as a category does not
quietly stop being complete at the next Unicode revision. No ADR written and
DESIGN.md untouched: if the user approves, §8.2 gets one sentence and this
becomes an ADR. `sanitize_line` is the same kind of addition, forced by §8.2's
own "never passed through raw" clause having no enforcement behind it.

Verified: 22 sanitiser tests (68 in `graft-term`, 85 in the workspace) —
18 unit, 4 property at 64 cases (`PROPTEST_CASES` raises them). They cover an
ANSI-injection commit message, OSC 8 in both its ESC and C1 (0x9D) spellings,
an RLO-spoofed branch name, a zero-width author name, every C0/DEL/C1 code
point one at a time, a newline and a tab inside a file name, and a non-UTF-8
path through both path entry points. The property tests check against an
oracle written from the documented rule rather than from `classify`, so the
two can disagree and the test notices.

Checked by hand, not just asserted: adding
`graft_term::SanitizedText("raw".to_owned())` to `graft`'s integration test
fails to compile with E0423 ("cannot initialize a tuple struct which contains
private fields"); the line was reverted. And a throwaway example run in this
terminal printed a hostile string through `sanitize` — the output bytes
contained no `0x1B` and no C1 introducer, and the line drew as literal text
(`fix: ␛[31mRED␛[0m �exe.txt�␛2K`) with no colour applied. Reviewed by a
second agent against DESIGN §8.2 and the acceptance criteria; its Major
finding (raw `\n`/`\t` on single-line widgets) is what `sanitize_line` and
its tests came from.

Not covered (KNOWN_GAPS): homoglyphs, combining-mark stacks, ambiguous and
fullwidth characters, and the non-`Cf` invisibles — variation selectors
(U+FE00–U+FE0F, U+E0100–U+E01EF) and Hangul fillers. Replacing the variation
selectors would mangle ordinary emoji and CJK text to block something that
can only mislead the eye, never drive the terminal.

`proptest` added as a `graft-term` dev-dependency (`default-features = false`,
`std` + `bit-set`). Dev-dependencies are outside the dependency-edge test by
design.
