# G-hygiene 1 — askpass peer check, argument limits, config isolation, doc warnings

Status: ready
Build with: Opus (conductor, after scout `scouts/21-hygiene-code.md`: askpass loop is in graft-git, which has no libc/nix — check crate rules before adding)
Blocked by: none
Needs: none. Crates graft-git, graft-ops (snapshot/restore raw only), graft (askpass server, if it lives there).

## Read first
- `crates/graft-git/MAP.md` (askpass, `snapshot_reads`, submodule parse); `crates/graft-ops/MAP.md` `## snapshot`,
  `## restore`; `crates/graft/MAP.md` `jobs` (askpass)
- PROMPT.md §7.1 (laptop-safe tests) — one paragraph

## Items (from the open rows)
1. **Askpass socket has no peer-credential check**; the 0700 dir is the only guard. Add: the server accepts a
   connection only when the peer's uid equals ours (`getpeereid` on macOS, `SO_PEERCRED` on Linux); a refused peer
   gets no answer and one log line, never the secret.
2. **`raw_tree` passes every kept path on one `hash-object` command line** (`snapshot/raw.rs:198-205`); a large kept
   set can pass ARG_MAX and every operation refuses. Use `--stdin-paths`, as `restore/raw.rs:103` does.
3. **`write_his_bytes` temp name appends `.graft-raw-<pid>`** (`restore/raw.rs:177-178`); a file name within 18 bytes
   of NAME_MAX makes `u` fail every time. Use a short temp name in the same directory (`.~<6 hex>`), retry on clash.
4. **`autocrlf_converts` (graft-git `snapshot_reads`) reading global/system `core.autocrlf` is untested**: the test
   process doesn't isolate global config. Isolate (`GIT_CONFIG_GLOBAL`, `GIT_CONFIG_NOSYSTEM`) and test both values.
5. **Question: does `autocrlf_converts` (via gix) see `GIT_CONFIG_COUNT`/`GIT_CONFIG_PARAMETERS`**, which the git
   spawns honour? Test it; if not, read them or say in MAP.md which config sources it reads.
6. **No fuzz/proptest for `submodule::parse`** (`graft-ops/src/submodule.rs`) → a proptest: never panics; round-trips
   what it accepts.
7. **rustdoc warns: public docs link private items** (`restart_source`, `merge_msg`, `is_operation` in graft-ops) →
   `cargo doc --workspace --no-deps` with `-D warnings` clean; add it to the gates script if the conductor agrees
   (NOTES proposes, does not edit CLAUDE.md).

## Acceptance
- [ ] 1: a test connects from a process with a different uid where possible (Linux CI) or fakes the peer check's
      input (unit) and asserts refusal; same-uid path unchanged (M4 walk's askpass scenarios green).
- [ ] 2: a kept set over 200k bytes of paths snapshots and restores (test, temp dir, fast tier).
- [ ] 3: a 250-byte file name: `u` restores it (test).
- [ ] 4, 5: tests named; 5's answer in NOTES and MAP.md.
- [ ] 6: proptest named, 256 cases in the fast tier.
- [ ] 7: `cargo doc` command output line quoted (0 warnings).

## Touches
- Askpass server and the M4 credential walk; snapshot/restore raw paths and the M7 recover walks (`recover.rs`);
  `TESTING.md` if askpass wording changes (it must not); MAP.md graft-git, graft-ops, graft.

## Scout questions
- Where is the askpass socket's accept loop (crate, file:line)?
- Is `nix` or `libc` already a dependency of that crate (no new dependency without the crate rules)?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~50k tokens.
