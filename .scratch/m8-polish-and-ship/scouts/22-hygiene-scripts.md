# Scout — 22 hygiene-scripts-and-fixtures (2026-10-02, read-only, at dc711840)

## Scout question
- **Which capture writers derive layout from scratch path length?** The pty walks open the real scratch dir; redaction keeps same width: `crates/graft-fixtures/src/capture/machine.rs:19-33` (`redacted` `:125`) — comment itself says the fix is "a fixed scratch location". Rust-drawn frames use `captures::CAPTURE_REPO_PATH` (machine.rs:29). The line in each pty-walk/headless prefix that creates the scratch repo: **could not confirm** (only `config_home = tempfile.mkdtemp` found, e.g. `pty-walk-m6.py:62`, `-m5.py:88`, `-m4.py:66`, `-m3.py:54`).

## Items — still present?
1. Ref-states: builder ends `checkout --orphan fresh` + `rm -r` at `crates/graft-fixtures/src/shapes/refs.rs:184-185`. Built fixture's HEAD / cache key: **could not confirm** (not built).
2. `ManyLanes`: no match in crates. **Open.**
3. Drain on end: m5 fixed (`pty-walk-m5.py:216-240`, `self.pump(0.05)`). **m6 still sleeps** (`pty-walk-m6.py:170-186`, `time.sleep(0.05)`); **m3 `:157-166` and m4 `:169-178` block in `os.waitpid(pid,0)`** with no read. Location wrong in ticket: scripts are `.scratch/<m>/pty-walk-*.py`, not `scripts/pty-walk-m6.py`.
4. **No fuzz runner script exists**: no `cargo fuzz`/`max_total_time` in `scripts/` (grep hits only ticket and `.scratch/m7-power-tools/gap-rows-draft.md`). Targets: `fuzz/fuzz_targets/{blame_porcelain,conflict_parse,follow_log,line_log,parse_file_list,parse_porcelain_v2,parse_unified}.rs`. Builder must create one.
5. Threshold: `crates/graft/src/update/commit_graph.rs:23-25` (`offer_after_sort`, `SLOW_SORT`) confirmed. Walk m6-06 location not checked.
6. `write_shots` is `crates/graft-graph/tests/it/raster.rs:706`. Not reproduced.
7. `scripts/fix-scroll-emphasis-cost.sh` names test `emphasis_layer::emphasis_cost` — **exists** (`crates/graft-graph/tests/it/emphasis_layer.rs:517`). Raster path at `:11` not confirmed stale; may be **already fine** — builder should check `:11` contents.
8. shellcheck not run; `scripts/check-scripts.sh` does not exist.
9. Open (see scout question).
10. Committed: `git ls-files 'docs/shots/*.esc'` = 3 files. Sizes: 24d 132 KB, **25-iterm2 32 KB** (not ~132), 33 132 KB. **References:** `.scratch/m1-the-graph/issues/{24d,25,33}-*.md`, `.scratch/m1-the-graph/iterm2_capture.py`, **`docs/log/m5.md`** — grep finds **none** in `docs/capabilities/m1-the-graph.md` or `docs/log/m1.md` (ticket names wrong files).

## File set
`.scratch/m{3,4,5,6}-*/pty-walk-m*.py`, `scripts/` (new check-scripts.sh, fuzz runner, perf scripts incl. `m4-review-perf.sh`, `47-debug-idle.sh`, `fix-scroll-emphasis-cost.sh`), `crates/graft-fixtures` (shapes/refs.rs, corpus shape, capture/), `crates/graft-graph/tests/it/raster.rs` (item 2 snapshots, 6), `fuzz/`, `docs/shots/*.esc`→`.txt`, `docs/log/m5.md`, `.scratch/m1-the-graph/...`. Item 5 touches the m6 walk (pty-walk-m6.py or graft tests: could not confirm which). Note `.claude/worktrees/agent-a029.../` holds stale copies of the pty walks.
