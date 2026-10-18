# 08v — mask key files whole (secrets leak from 08t), plus one stale comment

Status: done (2026-10-18); was ready; was draft (planner, spec-only pass, 2026-10-18; from 08t NOTES questions a and d)
Build with: Sonnet — the rule is fully decided below and a table test judges it; one function and its tests.
Needs: from 08t (built): `view::commit_editor::mask::{MASK, mask_secret_line}`, `update::commit_editor::files::diff_arrived` (calls it). graft-ops `looks_like_secrets` (unchanged).
Blocked by: none. Shares `crates/graft/MAP.md` (one row each) with 08u: run after 08u merges, or alongside and the conductor merges the row by hand.

## Read first
- `docs/design-notes/ease-pass.md`: the paragraph "**Commit secrets and done-line follow-ups settled** (2026-10-18)" near the top.
- `.scratch/ease-pass/issues/08s-commit-fixes.md`: Decided → **Q1** bullet; Signatures → `view::commit_editor::mask`; NOTES (part B1 = 08t) → Deviations (hunk headings) and Questions a, d.
- `crates/graft/MAP.md`: the `mask` row. Then `view/commit_editor/mask.rs`, `update/commit_editor/mask_tests.rs`.

## The leak
`mask.rs:21` splits at the first `=` or `:` and keeps what's before it. `looks_like_secrets` flags `id_rsa`, `*.pem`, `*.key` (`graft-ops/src/operation/choice.rs:70,81`, per 08t NOTES). A PEM or OpenSSH key's last base64 line `…IDAQAB==` therefore draws `…IDAQAB=••••••••`: the key material, readable. Same for its hunk heading (git's funcname picks a base64 line) and for a multi-line quoted value's continuation in an `.env`.

## Scope
In: `crates/graft/src/view/commit_editor/mask.rs` (rule + doc comment + unit table); `crates/graft/src/update/commit_editor/mask_tests.rs` (one row-level test); `crates/graft/tests/it/m5_walk/resolving.rs:50` (comment only); `crates/graft/MAP.md` mask row (reword).
Out: graft-ops (`looks_like_secrets` and its file list unchanged); masking outside the commit screen (KNOWN_GAPS row stands); captures (the `.env` fixture's output is unchanged: `TOKEN=••••••••`); `tests/it/commit_screen.rs` (08u's).

## The rule (decided)
`mask_secret_line(line)` keeps `<prefix><sep>` and draws the rest as `MASK` **only when all three hold**; otherwise the whole line is `MASK`:
1. `sep` is the line's first `=` or `:`.
2. The prefix is a key: optional leading whitespace, optional `export `, then `[A-Za-z_][A-Za-z0-9_.-]*`, then optional whitespace; the name part ≤64 chars.
3. A character follows `sep` and it is not `=` (base64 padding `…AB=` / `…AB==`; an empty `KEY=` masks whole too — harmless).

Base64 has no `:` and `=` only at its end, so every line of a PEM/OpenSSH key masks whole by (2) or (3); no file-kind switch, no graft-ops change. Hunk headings keep going through the same function (08t's deviation, accepted). Signature unchanged.

## Signatures
```rust
// graft app, view::commit_editor::mask — unchanged signature; doc comment states the three conditions.
pub(crate) const MASK: &str = "••••••••";
pub(crate) fn mask_secret_line(line: &str) -> String;
```

## Touches (check each, report how)
- Callers of `mask_secret_line` (grep): `files.rs diff_arrived` (diff lines and hunk headings). No other.
- Existing tests: `mask::tests::a_secret_line_keeps_its_key_and_hides_its_value` (all six old rows still hold, unedited), `mask_tests::a_secrets_row_is_masked_in_and_out`, `…::an_ordinary_file_is_not_masked`, it `commit_screen::a_secrets_files_values_never_reach_the_screen`, `commit_screen::shots::capture_the_commit_screen`: pass unedited.
- `docs/shots/ease-pass/commit-secrets-120.txt`: unchanged (no re-shoot; say so with `git diff --stat`).
- MAP.md mask row. Keymap, journal/undo, other crates: none (grep).

## Acceptance
Each tick names its test inline. Seen red first for the new rows.
- [x] **Table:** (seen red first, now green) in `view::commit_editor::mask::tests::a_key_files_lines_are_masked_whole` — `MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC7` → `MASK`; `x5dhkAbCdEfGhIDAQAB==` → `MASK`; `x5dhkAbCdEfGhIDAQAB=` → `MASK`; `-----BEGIN RSA PRIVATE KEY-----` → `MASK`; `-----BEGIN OPENSSH PRIVATE KEY-----` → `MASK`; `b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW` → `MASK`; `KEY=` → `MASK`; `my secret=x` → `MASK`; `ssh-rsa AAAAB3NzaC1yc2E== me@host` → `MASK`.
- [x] **Keys still kept:** in `…::a_secret_line_keeps_its_key_and_hides_its_value` (rows added, old six untouched) — `export API_KEY=abc` → `export API_KEY=••••••••`; `  password: hunter2` → `  password:••••••••`; `Proc-Type: 4,ENCRYPTED` → `Proc-Type:••••••••`.
- [x] **Row:** (seen red first on the all-MASK assert; `looks_like_secrets(id_rsa)` asserted true in the test) a secrets choice for path `id_rsa` whose diff has a heading `…IDAQAB==` and added lines `-----BEGIN OPENSSH PRIVATE KEY-----`, `b3Bl…ZW`, `…IDAQAB==` → after `diff_arrived`, every line and the heading text read `••••••••`, and a `Debug` dump of the screen's view holds none of `IDAQAB`, `b3Bl`, `BEGIN`. — unit `update::commit_editor::mask_tests::a_key_file_is_masked_whole` (also proves `looks_like_secrets` flags `id_rsa`; if it doesn't, stop and report — graft-ops is out).
- [x] **d:** `tests/it/m5_walk/resolving.rs:50` comment calls the last file's answer a `Status::Done` (as `update/resolver/resolved.rs:266` builds), not a `Sticky`. — by reading; quote the new line in NOTES.
- [x] MAP.md mask row states the rule in one line. — by reading; length checked by it `map_coverage::map_lines_are_short_and_carry_no_ticket_ids`.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Small (~25k).

## NOTES (builder, 2026-10-18)
- No scout: the ticket names the lines. No compaction.
- d, new line: "`Settle`, not `Write`: the last file's answer is a `Status::Done` (`✓ every file done  c finishes the merge`, `update/resolver/resolved.rs`)." The "Settle, not Write" reason itself was not re-derived.
- Touches: `mask_secret_line` callers (grep): only `mask.rs` (`mask_heading`, `mask_line`), reached from `files.rs diff_arrived`. Old rows + existing mask/commit_screen tests pass unedited (full nextest). `git diff --stat docs/shots` empty: capture unchanged. Keymap, journal/undo, other crates untouched.
