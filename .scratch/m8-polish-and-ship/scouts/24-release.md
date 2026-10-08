# Scout — 24 release-dist-and-matrix

## How tests (and the app) find git
- **PATH only. No env override exists.** Every spawn is a literal `Command::new("git")`:
  production `crates/graft-git/src/run.rs:459` (GitInvocation::spawn), `crates/graft-git/src/status/tracked.rs:115`,
  `crates/graft/src/event_loop/outside.rs:495,510`, `crates/graft/src/repository.rs:145`; fixtures
  `crates/graft-git/src/fixture.rs:105`; tests e.g. `crates/graft-ops/tests/it/absorb_oracle.rs:24`, `finish.rs:220`.
- grep for `GRAFT_GIT` / git-binary env var: none (only `GRAFT_GIT_GIT_DIR`, a repo path, `crates/graft/tests/it/gallery.rs:669`).
- Tests already override `PATH` in children: `crates/graft-ops/tests/it/set_aside_kill.rs:88`, `crates/graft/tests/it/m6_stash_put_back/set_aside.rs:1141`.
- `git_alone_stops_on_a_pick_of_an_ancestor`: `crates/graft-ops/tests/it/cherry_pick.rs:305`, uses `repo.git(...)` (fixture, PATH git).
- Rewrite-preview tests: `crates/graft-ops/tests/it/rewrite_preview.rs` (did not enumerate which compare against git).

## Floor
- `GitVersion::MINIMUM` = 2.38.0 at `crates/graft-git/src/version.rs:68-72`. **Ticket cites `version.rs:5`** — that is a doc comment, not the constant.

## Machine (checked, nothing pulled/started)
- docker: `/usr/local/bin/docker` → Docker.app; `docker info` server 29.7.2 (daemon running). podman/colima/limactl/orbctl: none.
- Build-from-source toolchain: `/usr/bin/make`, `/usr/bin/cc`, `/usr/bin/clang`, `/usr/bin/curl` present. Host git 2.54.0 (Apple Git-157). Could not confirm autoconf/openssl/curl headers for git's build.

## Existing release/profile config
- Root `Cargo.toml:84-88`: only `[profile.dev]` and `[profile.test]` (`debug = "line-tables-only"`). No `[profile.release]`, no `[profile.dist]`, no `[workspace.metadata.dist]`.
- `crates/graft/Cargo.toml:2-3,9`: name `graft`, version 0.1.0, license MIT; no explicit `[[bin]]` section (ticket Touches assumes one).
- No `.github/`, no `docs/release/`, no `rust-toolchain*`, no `dist-workspace.toml`.
