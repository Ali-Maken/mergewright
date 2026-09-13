# m1-the-graph/35 — Graph gallery: M1's acceptance ticket

Status: done — part a here; remainder completed in 35b, 35c; by-eye box (user) unticked
Build with: **Fable** (high) — the deliverable is a judgment about what good looks like; M1's promise is visual (user decision, 2026-09-13)
Blocked by: 27d, 29, 30b, 30d, 34
Leaves runnable: yes — one dev command walks every test repo so the user can judge the graph by eye.

**M1's promise:** *the shape of history is obvious at a glance.* This ticket builds the means to judge that promise, not more tests of it. M1 isn't done until this ticket is.

Why: layers 1–4 of the graph test plan (layout invariants, named corpus, drawing snapshots, interaction) prove correctness, not that it reads well. The only repo the user has seen has no merges. Layer 3's coverage (repo × theme × tier) is assumed, not shown.

## Read first
- PROMPT.md §1.1–1.3 (tiers, layout rules, what each row shows), §7.2 items 2, 3 and 7 (named corpus, snapshots, manual terminal checklist)
- Ticket 01 "Scope" and its notes (fixture definitions); `crates/graft-fixtures/MAP.md` (`Shape::ALL`, `borrow`, `try_borrow`)
- Ticket 02 notes (heavy tier: 100k synthetic fixture, git/git with `--filter=tree:0`)
- Existing `TESTING.md`
- `crates/graft/MAP.md`, `crates/graft-ui/MAP.md`, `crates/graft-graph/MAP.md`
- Scouts (Explore, haiku/sonnet) for source and for the fixture scripts; summaries of a page or less.

## Scope
In:
1. **One command cycles the repos.** A dev-only example (e.g. `cargo run --example gallery`), **not part of the shipped binary** (no default feature, nothing in `graft`'s release build). A key steps to the next / previous repo; the footer shows the current repo's name, `n/N` position, and its **expected finding** in one line (e.g. `foxtrot-merge · expect: main's first-parent line jumps lanes at the foxtrot merge`). Not twenty separate commands. All graph keys and F3 keep working inside it.
2. **The cycle:** every `Shape::ALL` fixture, the small real linear repo, **the 100k synthetic fixture and git/git**, so the user sees a busy history, not only tidy ones. Heavy ones are used **only if already present** (built by the heavy tier / cloned per ticket 02); don't generate or clone them in this ticket. When absent, the gallery shows that repo as `not available — build with: <exact command>` rather than skipping silently.
3. **TESTING.md** gains a "Graph gallery" section: for each repo, what right looks like, **concretely** — which commit carries which node shape or badge, which lane must stay straight, where a curve should appear and in whose colour, what a foxtrot merge looks like versus a normal merge, what the fold / first-parent / Focus views should leave on screen, what must be identical across tiers. Draw on ticket 01's fixture definitions and PROMPT.md §1. **Never "looks correct".** Plus a per-terminal block matching §7.2 item 7 (pixel must pass: Ghostty, Kitty, WezTerm, iTerm2; Unicode must look deliberate: Terminal.app, Alacritty, VS Code terminal; tmux/SSH must degrade).
4. **Snapshot coverage matrix:** the command (or a flag of it) prints repo × theme (dark, light) × tier (pixel buffer, Unicode, ASCII) × size, marking each cell `snapshot` / `capture` / `—`, from what actually exists in the test suite and `docs/shots/`. Also written to `docs/shots/35-coverage-matrix.txt`. Gaps are shown, not filled in this ticket (list them in KNOWN_GAPS.md with an owner).
5. Captures: Unicode and ASCII pty captures of every available repo under `docs/shots/35-<repo>-<tier>.txt`.

Out:
- Filling the coverage gaps the matrix reveals (new tickets or the M1 review).
- Judging by eye — the user does that.

## Guard (context)
- Captures go **straight to files** under `docs/shots/35-*`. **Never print capture contents into your own context** (no `cat`, no reading them back whole). Check them with byte counts, grep for a marker, or a scout if you must.
- Report **file paths and the coverage matrix only**.

## Acceptance
- [x] `cargo run -p graft --example gallery` steps through every repo with `]`/`[`; footer shows name, position and expected finding (tests: `tests/it/gallery.rs` footer text, `view.rs` footer draw, `update.rs` hand-back). pty capture of two steps: `docs/shots/35-gallery-two-steps.txt` (35c).
- [x] Heavy repos: present → in the cycle; absent → `not available — build with: …` (test `heavy_repos_are_used_only_if_present_and_say_how_to_build_otherwise`).
- [x] Not in the shipped binary: `cargo tree -e normal -p graft --depth 1` lists no `graft-fixtures`; `strings target/release/graft` has none of the gallery's strings (`expect: `, `no finding written yet`; its two `not available` hits are libstd's). The lib seam (`Harness`) is in the binary but default-empty.
- [x] TESTING.md "Graph gallery" section: one concrete entry per repo, per-terminal block; no entry reads "looks correct" / "renders fine". (35b)
- [x] Coverage matrix printed and committed at `docs/shots/35-coverage-matrix.txt`; gaps in KNOWN_GAPS.md with owners.
- [x] Captures under `docs/shots/35-*` (35c): `35-<stop>-{unicode,ascii}.txt` for all 23 stops at 80x24, each verified (grep) to name its own stop; `35-gallery-two-steps.txt`; `non-utf8-ref-name` shows the not-available page (APFS).
- [x] Exact command for the user written into this ticket (below).
- [ ] **By eye (user):** walks the gallery in Ghostty and the VS Code terminal against TESTING.md. Unticked until the user does.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only). Never generate large fixtures here.

## Size
One context, under ~80k tokens. Natural seam if it grows: (a) gallery command + matrix, (b) TESTING.md entries + captures. Stop at the seam with a handoff note rather than compacting.

## Part a: what was built (2026-09-13, Fable)

**Command:** `cargo run -p graft --example gallery` — `]` next repo, `[` previous, `q` quit; all graph keys and `F3` work. `-- --start foxtrot-merge` starts there; `-- --list` prints the 23 stops; `-- --matrix` prints and writes the coverage matrix.

- **Lib seam** (`crates/graft`): `update::Harness { notice, hand_back }` on `Startup` and `AppState`; `Quit::HandedBack(KeyCode)` → `Exit::HandedBack(KeyCode)`; `view::footer_notice` shows the notice on the hints' line, after F3's renderer line when both apply. Default-empty in `main.rs`. A hand-back key is not consulted while the search prompt is open.
- **Cycle** (`tests/it/gallery.rs`, included by the example via `#[path]` like `budgets.rs`): `Shape::ALL` (20) + `graft-self` (this checkout: the real linear repo) + `synthetic-100k` + `git-git`. Heavy ones are located, never built: `target/fixtures/synthetic-100000-*/corpus.ok` (or `GRAFT_FIXTURES_DIR`/`CARGO_TARGET_DIR`), `~/.cache/graft/git-git.git` (or `GRAFT_GIT_GIT_DIR`/`XDG_CACHE_HOME`). Absent → a raw-mode page `<name> · n/N · not available — build with: <cmd>`.
- **Each step restarts `event_loop::run`** on the next repo (terminal restored, re-probed): a blink per step. Accepted for a dev tool; in KNOWN_GAPS.
- **Matrix:** classifies every `.snap` under `crates/` and every file in `docs/shots/` by name (tier from `raster/kitty/iterm2/.png`, `ascii`, graft-graph's corpus snaps as tier-less `layout`; theme from the word `light`; size from a `WxH` token, else `?`). Names only, stated in its header.
- On this machine: `synthetic-100k` absent (only 5k variants built), `git-git` present (85k commits), `non-utf8-ref-name` unavailable (APFS).
- Not verified by eye: the gallery has only been run headlessly (`--matrix`, `--list`) and through its tests; nobody has yet walked it in a terminal.

## Handoff: part b (TESTING.md entries + captures)

Not started. Exact next steps:

1. **TESTING.md "Graph gallery" section** (after "The manual terminal checklist", before "Out of scope"): one entry per stop, in `--list` order. Long form of each `expect` line in `tests/it/gallery.rs::expectation` — use the fixture facts from ticket 01's `src/shapes/` (subjects, branch names, parent order) to say which row carries which node shape/badge, which lane stays straight, where the curve is and in whose colour, foxtrot vs normal merge, what fold (`Space`), first-parent (`1`) and Focus (`F`) leave on screen, what is identical across tiers (`F3`). Then the per-terminal block (§7.2 item 7): pixel must pass Ghostty/Kitty/WezTerm/iTerm2; Unicode must look deliberate in Terminal.app/Alacritty/VS Code; tmux/SSH degrade — pass conditions already exist above it; add "walk the gallery" as the procedure. Never "looks correct".
2. **Captures:** write `.scratch/m1-the-graph/pty-gallery.py` modelled on `pty-wheel-check.py` (pyte, 80x24). `cargo build -p graft --example gallery`, exec `target/debug/examples/gallery` under the pty with `XDG_CONFIG_HOME=<tmp>` where `<tmp>/graft/config.toml` holds `[graph]\nrenderer = "unicode"` (then a second run with `"ascii"`) — **verify the config file name in `crates/graft/src/config/mod.rs::path_in_environment` first**. For each of the 23 stops: wait for the frame (`\x1b[?2026h` marker, then settle ≥ 1 s: each step re-probes; give git-git 3 s), write `docs/shots/35-<name>-<tier>.txt`, send `]`. The first three frames of the Unicode run also go to `docs/shots/35-gallery-two-steps.txt`. Gotchas: stops 20 (non-utf8-ref-name) and 22 (synthetic-100k) show the "not available" page (raw text, still capture it); a `[` byte in pyte input is fine (no ESC before it). **Never cat a capture**: check with `wc -c` and `grep -c "expect:"`.
3. Tick the remaining acceptance boxes (TESTING.md, captures, the two-step capture) and re-run `--matrix` so the 35-* captures appear in the matrix; commit `m1-the-graph-35: part b …`.
