# Scout — 21 hygiene-code (2026-10-02, read-only, at dc711840)

## Scout questions
- **Askpass accept loop: `crates/graft-git/src/run/askpass.rs:286`** (`listen`, `listener.incoming()`), bind at `:141`. NOT in `graft` — `crates/graft/src/jobs/askpass.rs` is only the `AskPass` handler (channel to `update`). Ticket's "graft (askpass server, if it lives there)" is wrong crate.
- **`nix`/`libc` in graft-git: neither.** graft-git deps: gix, graft-core, thiserror, sha1, tempfile (`crates/graft-git/Cargo.toml:56-75`). `libc = "0.2"` exists only in graft-fixtures (`Cargo.toml:23`) and graft-term (`Cargo.toml:14`). Item 1 needs a new dep in graft-git (crate-rules question) — std `UnixStream::peer_cred` is unstable (not checked against toolchain: could not confirm).

## Items — still present?
1. Peer check: absent. `listen` serves every accepted stream (`askpass.rs:286-307`); errors dropped with comments. **Open.**
2. `raw_tree` one command line: `crates/graft-ops/src/snapshot/raw.rs:191-205` (loop pushes each `GitArg::Path`). **Open.** `--stdin-paths` precedent at `restore/raw.rs:103` confirmed.
3. Temp name `.graft-raw-<pid>`: `restore/raw.rs:177-178`. **Open.**
4. `autocrlf_converts` is `crates/graft-git/src/snapshot_reads.rs:139` (gix `config_snapshot().try_boolean("core.autocrlf")` `:145`). Its test file already sets `GIT_CONFIG_NOSYSTEM`/`GIT_CONFIG_GLOBAL=/dev/null` on git spawns (`graft-git/tests/it/snapshot_reads.rs:26-27`) — but whether the in-process gix read is isolated (process env) could not confirm. Related test also at `graft-ops/src/snapshot/raw/tests.rs:25`.
5. Open question; no test for `GIT_CONFIG_COUNT` found in graft-git (grep). Only use of COUNT is `graft-ops/src/apply/set_aside/put_back.rs:61-67`.
6. `submodule::parse` = private `fn parse` at `crates/graft-ops/src/submodule.rs:79`. **No proptest in graft-ops** (no `proptest` in `graft-ops/Cargo.toml`, dev-deps at `:27`) → new dev-dep. **Contradiction:** ticket asks 256 fast-tier cases; graft-git/graft-term/graft-graph Cargo comments state PROMPT §7.1 caps fast tier at 64 (`graft-git/Cargo.toml:113-116`).
7. Intra-doc links to private items present: `restart_source` `apply/restart.rs:218,320`; `merge_msg` `conflict/set.rs:62`; `is_operation` `undo.rs:569`. **Open** (cargo doc not run).

## File/crate set
- graft-git: `src/run/askpass.rs`, `src/run/askpass/tests.rs`, `src/snapshot_reads.rs`, `tests/it/snapshot_reads.rs`, `Cargo.toml` (if libc), MAP.md.
- graft-ops: `src/snapshot/raw.rs`, `src/restore/raw.rs`, `src/submodule.rs`, `src/apply/restart.rs`, `src/conflict/set.rs`, `src/undo.rs`, `Cargo.toml` (proptest dev-dep), tests, MAP.md.
- graft: none needed for code (ticket's assumption wrong); M4 walk askpass tests in `crates/graft/tests/it/{askpass_mode,credential_by_key}.rs`, `m4_walk/` are regression coverage only.
