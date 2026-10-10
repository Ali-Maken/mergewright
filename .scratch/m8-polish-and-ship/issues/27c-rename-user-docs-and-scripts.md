# G-rename 2c — README, TESTING.md and scripts say `trib` and Tributary

Status: ready (split from 27, spec-only pass 2026-10-10)
Blocked by: 27a, 27b
Build with: **Sonnet** — line triage by a written rule, held by a grep test and a listed remainder.
Needs: 27a (`graft::cli::COMMAND`, `docs/man/trib.1`), 27b (final `TRIB_*` names). Can run beside 27d: disjoint
files (27d owns `docs/shots/`; this ticket runs cargo only for its test and the gates).

## Read first
- 27's `## Decisions` (history-docs bullet); 27a and 27b `## Decisions`
- `scouts/27-rename-sweep.md` `## Scripts / docs / captures`
- README.md and TESTING.md: only the lines `rg -n -i '\bgraft\b|GRAFT_' README.md TESTING.md` returns, with ±3 lines

## Decisions (spec-only pass)
- **The rule, per line:** change it if a person following it today would type it or read it as the app's name —
  commands, paths to the binary, env names, the app's name in instructions. Keep it if it records the past — dated
  entries, "Times I left" rows, quoted outputs of a past run, citations like `crates/graft/...:line`. Package and
  crate names stay (`-p graft`, `crates/graft`, `graft-ui`): they are code names (27's KNOWN_GAPS row).
- **README**: the command, `TRIB_*` names, and one line: `GRAFT_*` variables are no longer read.
- **TESTING.md** (174 lines): instructions and checklists change; records stay. Every kept line is listed with its
  class in a file, so the reviewer checks the remainder instead of re-reading 174 lines.
- **Scripts' prose** (echo/usage text, comments naming the app) → Tributary/`trib`; binary paths and env names were
  27a/27b. `scripts/graft_sandbox.py` → `scripts/trib_sandbox.py` (`git mv`) and every caller (grep).
- CLAUDE.md, PROMPT.md, `.claude/**`: not edited. NOTES lists the lines that would change (with 27b's env list).

## Scope
In: README.md, TESTING.md (non-record lines), `scripts/**` words and the sandbox script's name, `docs/man/` prose if
any outside the generated page (none expected). Out: history docs, CLAUDE.md, PROMPT.md, `.claude/**`, captures,
crate code.

## Signatures
None (docs and scripts). Test only:
`crates/graft/tests/it/rename_names.rs::user_docs_and_scripts_name_the_command`.

## Acceptance (each box: the test named at tick time)
- [ ] `rename_names::user_docs_and_scripts_name_the_command` — README.md and `scripts/**`: no `(?i)\bgraft\b`
      outside the allow-list (`-p graft`, `crates/graft`, `graft-[a-z]+` crate names, `graft_[a-z]+` Rust paths),
      each allow-list pattern with its reason.
- [ ] TESTING.md remainder: `.scratch/m8-polish-and-ship/27c-testing-kept.txt`, one line per kept match
      `line · class (record | crate path | package)`; count in NOTES. No kept line is an instruction (reviewer
      samples ≥10).
- [ ] `scripts/trib_sandbox.py` exists, `graft_sandbox.py` doesn't; callers grep quoted (none left on the old name).
- [ ] Every script under `scripts/` that runs the app: `bash -n` / `python3 -m py_compile` clean (output to file,
      summary quoted). Scripts are not run end to end here; 27d runs the capture ones.
- [ ] NOTES: CLAUDE.md / PROMPT.md / `.claude/**` lines that would change, `file:line → new text`.

## Touches
README.md; TESTING.md (commands that `scripts/m7-accept.sh` and the walks print may be quoted there — check both
ways); `scripts/**`; anything invoking `graft_sandbox.py` (grep across repo, `.claude/` listed not edited);
`rename_names.rs`. Each checked by grep or test, named in the report.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` —
workspace-wide.

## Size
One context, ~35k tokens.
