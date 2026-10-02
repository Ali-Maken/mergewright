# G-hygiene 1 — askpass peer check, argument limits, config isolation, doc warnings

Status: done (item 7 partial: graft-git and graft-ops clean; other crates are outside this builder's set)
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
- [x] 1: different uid without root: the server is told its owner is `our uid + 1` and the test process
      connects — `run::askpass::tests::a_peer_with_another_uid_is_refused_unanswered_and_never_asked` (the helper
      gets EOF, no bytes, the handler is never asked). Faked input: `admit_takes_only_our_own_uid` (another uid,
      root, or an unreadable uid → refused). Real kernel read: `peer_uid_of_a_real_connection_is_ours`. Same-uid
      path: 26 askpass tests green, including `graft`'s `askpass_mode`, `credential_by_key` and `m4_walk` (nextest
      filter `askpass`). A real different-uid peer (root or Linux CI) was not run: this laptop has no root.
- [x] 2: `undo_bytes::u_brings_back_a_kept_set_of_over_200k_path_bytes` (1000 files, 209 000 bytes of paths, a
      stash and `u`). NB: this test also passed **before** the fix, because macOS's ARG_MAX is 1 MiB. The fix's
      evidence is `snapshot_raw::a_large_kept_set_is_hashed_from_stdin_not_the_command_line`: red before (a
      command line over 1 KiB), green after.
- [x] 3: `undo_bytes::u_brings_back_a_file_with_a_250_byte_name`. Units: `restore::raw::tests::a_250_byte_name_is_written_through_a_short_temporary`,
      `a_taken_temporary_name_is_skipped_never_written_over`, `every_temporary_name_taken_fails_and_writes_nothing`,
      `temporary_names_are_short_and_fixed_per_process`.
- [x] 4: `snapshot_reads::autocrlf_from_the_global_config_is_read_isolated` and
      `autocrlf_from_the_system_config_is_read_isolated` (graft-git `tests/it`). Each case runs in a re-exec'd
      child with HOME, XDG, the global file and the system file controlled, for both values, and compares Graft's
      answer with git's.
- [x] 5: `snapshot_reads::autocrlf_from_environment_config_agrees_with_git`, plus 4 parser unit tests in
      `snapshot_reads::tests`. The answer is in NOTES and MAP.md.
- [x] 6: `submodule::tests::parse_never_panics`, `what_parse_accepts_round_trips`, `a_printed_log_parses_back`.
      **64** cases in the fast tier, not 256: the conductor ruled to follow `graft_fixtures::proptest_cases()`
      (64 fast, 1000 heavy).
- [~] 7: `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p graft-git -p graft-ops` → "Finished `dev` profile …
      Generated …/graft_git/index.html and 2 other files", 0 warnings. **Not clean workspace-wide.** Still failing,
      all outside my crate set: graft 99, graft-ui 53, graft-fixtures 5, graft-graph 1, graft-term 1 (counted as
      `-->` lines from `cargo doc --workspace --no-deps --keep-going`). Adding the check to the gates script is
      proposed in NOTES, not done.

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

## NOTES (builder, 2026-10-02)
- Scout used: `scouts/21-hygiene-code.md`. Its correction held: the askpass server is in graft-git, not graft.
  No `graft` edits.
- **Dependency:** added `nix 0.29` to graft-git (features `socket` and `user`, no defaults). Rule check:
  ARCHITECTURE.md's table and the `cargo metadata` edge test govern **internal** crate edges only. External
  dependencies follow the convention of one reason comment per dependency in Cargo.toml. `#![forbid(unsafe_code)]`
  stays: nix's `getpeereid` and `getsockopt(PeerCredentials)` are safe wrappers. std's `UnixStream::peer_cred` is
  still unstable on 1.98.1 (checked: E0658). nix's own dependencies (libc, bitflags, cfg-if) were already in the
  lockfile.
- Refusal log: `eprintln!("graft: askpass: refused …")`. Graft has no log file; the precedent is the scratch
  cleanup in `graft-ops/src/snapshot/raw.rs`. While the TUI is up this line prints over the terminal. It fires
  only for a peer that got past the 0700 dir (root), so I judged it acceptable. Say if not.
- **Item 5 answer:** gix reads `GIT_CONFIG_COUNT`/`_KEY_n`/`_VALUE_n` (that test was green before any change). gix
  does **not** read `GIT_CONFIG_PARAMETERS`: before the fix it read `(graft false, git true)`. Fixed:
  `autocrlf_converts` now parses that variable the way git's `parse_config_env_list` does and passes
  `core.autocrlf` as a `-c` override. PARAMETERS beats COUNT, as in git (tested). A malformed value gives
  `ReadError::Config`, since git refuses to run then too. This fixes the cause.
- Item 3 design: the temporary name is `.~<6 hex>` of `pid×16+attempt`, with 16 attempts. It is deterministic per
  process, not random, so the three "raw write cannot land" tests (`undo_bytes` ×2, `recover_bytes` ×1) can still
  block every candidate. They now call a new `TempRepo::block_raw_writes_in(dir)`, which duplicates the formula
  (they duplicated `.graft-raw-<pid>` before).
- `c_quoted` moved from `restore/raw.rs` to `snapshot/plumbing.rs`, so both `--stdin-paths` callers share it.
- Item 7 also fixed 28 graft-git links the ticket did not list: links to private items became plain code;
  unresolved links got full paths.
- **Proposed (gates):** add `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` once graft, graft-ui,
  graft-fixtures, graft-graph and graft-term are clean. That needs a follow-up ticket for whoever holds those
  crates.
- No compaction.
