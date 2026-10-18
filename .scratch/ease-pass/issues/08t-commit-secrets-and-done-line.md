# 08t — commit fixes, part B1: masked secrets, tick-first done line

Status: ready (split by the conductor from 08s after two compactions, 2026-10-18; no new decision — scope, words and tests are 08s's)
Blocked by: 08s part A (merged)
Build with: Opus (`builder-opus-xhigh`), as 08s.

## Scope
08s's Part B, first half, in the handoff order (08s NOTES "Handoff: Part B"):
1. **Q1** — `view/commit_editor/mask.rs` (`MASK`, `mask_secret_line`), masking in `files.rs diff_arrived` for a `looks_like_secrets` choice.
2. **08r** — `done_line` tick first, one renderer; re-shoot every capture matching `grep -rn '✓ ·\| ✓ ·' docs/shots crates/*/tests`.
3. **m5, m8** — caption and `TESTING.md` / 02-keymap lines.

Everything else — signatures, words, Touches — is 08s's spec; read 08s top to bottom first, plus its Step 1 answers.

## Acceptance
Tick in **08s's file**, each naming its test: **Q1 lines**, **Q1 screen**, **Q1 captures**, **08r**, **m5**, **m8**, and the captures box's rows for `commit-secrets-120` and the `✓` re-shoots. Leave M4, M5+Q2, m4, m12, M6 to 08u.
