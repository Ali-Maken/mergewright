# Scout 27 — rename names sweep (HEAD 8076b8d3)

## Binary / app name
- No `[[bin]]` in `crates/graft/Cargo.toml`; binary name comes from package `name = "graft"` (:2). Only `[[bin]]` in the workspace is `graft-capabilities/Cargo.toml:22` (gen-capabilities). Adding `[[bin]] name="trib"` turns `CARGO_BIN_EXE_graft` into `CARGO_BIN_EXE_trib`: **18 uses in 15 files**.
- `APP_NAME = "graft"` at `graft-ui/src/graph_screen/header.rs:20`; used `empty_state/draw.rs:15,26`; test `header.rs:223`.
- Usage string `cli/flags.rs:123` (`usage: graft [<path>]`). `repository = ".../graft"` `crates/graft/Cargo.toml:12`; `include = ["../../docs/man/graft.1"]` :123.
- Man page: `cli/man.rs` :20 (`.TH GRAFT`), :22, :45, :53, :60, :65, :71, :90, :97, :101–105, :109, :111, :126, :199–200; env names :85, :89. Committed `docs/man/graft.1`, pinned by `cli/docs_tests.rs:17,60,66` (include_str path) and MAP.md:935. Config paths at :97/:101/:105/:199 are **wrong today** (26 moved them to `tributary/`).
- `docs/release/plan.txt`: `graft` at :5, 8, 9, 11–17 (archives, formula, `[bin] graft`, `graft.1`). Dist config `Cargo.toml:94–106` `[workspace.metadata.dist]`.
- Rough count of string literals containing word `graft` in `crates/*/src` (excl. `graft_`/`graft-`): 64, mixed test/non-test — the builder must triage.

## Crash dir / author identity (26 hand-off)
- `crash.rs:164` `state_home.join("graft")`; doc :159; user text :146, :149, :151, :206 (`graft {version} panicked`); tests :383, :405. Callers of `crash::` (`startup.rs:113`, `examples/terminal_probe.rs:44`, `jobs/pool.rs:305`) install/catch only — **no reader of the dir found** by grep.
- Backup identity `snapshot/plumbing.rs:297,299`; duplicated in `graft-ops/examples/snapshot_cost.rs:114,116` (not in ticket). No reader of author name `graft` found (grep `"graft"`).

## Env vars
- 45 distinct `GRAFT_*` names, 109 files under crates+scripts. Production constants: `cli.rs:50` ASKPASS_SOCKET_VAR, `graft-git/src/run.rs:356` (literal, not the const), `trace.rs:48` PERF_TRACE, `session.rs:121` NOW, `places/practice.rs:55` PRACTICE_EPOCH, `graft-fixtures/src/cache.rs:31` FIXTURES_DIR, `graft-ops/src/init.rs:119` INIT_TARGET, `graft-ops/src/bin/askpass_test_helper.rs:14`. Most-used test var `GRAFT_UPDATE_SHOTS` (66). MAP.md/ARCHITECTURE.md: 15 `GRAFT_` lines.

## Scripts / docs / captures
- `scripts/`: 45 files, 175 word-`graft` lines (incl. `graft_sandbox.py` filename); only 2 hits of `target/*/graft` literally — the rest need reading (crate `-p graft`, env vars).
- README.md 16, TESTING.md 174 word-`graft` lines.
- `docs/shots/`: 456 files contain `graft` (header shows APP_NAME), 10 contain `GRAFT_`. Re-shooting all header captures is the big cost.

## Contradictions with the ticket
- Ticket says set `[[bin]] name` — none exists; `[lib]` name must stay `graft` (`main.rs:16–20` `use graft::…`).
- Ticket "Touches" omits: `flags.rs:123` usage, `docs_tests.rs` man path, `Cargo.toml:12,123`, `snapshot_cost.rs:114`, `run.rs:356` duplicate literal, `graft_sandbox.py`.
- Could not confirm how many of the 456 shots are regenerated vs hand-kept.

## Size
Not one ~50k context. Split: (a) code+man+env consts+bin+CARGO_BIN_EXE+plan.txt; (b) scripts+README+TESTING; (c) capture re-shoot (`GRAFT_UPDATE_SHOTS`). Build with: Sonnet for (a),(b); (c) after (a).
