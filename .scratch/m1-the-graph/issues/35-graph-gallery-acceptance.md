# m1-the-graph/35 — Graph gallery: M1's acceptance ticket

Status: ready
Build with: **Fable** (high) — the deliverable is a judgment about what good looks like; M1's promise is visual (user decision, 2026-09-13)
Blocked by: 27d, 29, 30, 34
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
- [ ] `cargo run --example gallery` (or the chosen name) steps through every repo with a key; footer shows name, position and expected finding (test on the footer text; pty capture of two steps).
- [ ] Heavy repos: present → in the cycle; absent → `not available — build with: …` (test).
- [ ] Not in the shipped binary (state how verified, e.g. release build symbol / `cargo tree -e normal`).
- [ ] TESTING.md "Graph gallery" section: one concrete entry per repo, per-terminal block; no entry reads "looks correct" / "renders fine".
- [ ] Coverage matrix printed and committed at `docs/shots/35-coverage-matrix.txt`; gaps in KNOWN_GAPS.md with owners.
- [ ] Captures under `docs/shots/35-*`.
- [ ] Exact command for the user written into this ticket.
- [ ] **By eye (user):** walks the gallery in Ghostty and the VS Code terminal against TESTING.md. Unticked until the user does.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only). Never generate large fixtures here.

## Size
One context, under ~80k tokens. Natural seam if it grows: (a) gallery command + matrix, (b) TESTING.md entries + captures. Stop at the seam with a handoff note rather than compacting.
