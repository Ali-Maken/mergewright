# Known gaps

Everything unfinished, stubbed or deliberately deferred. Nothing here may be
described as done, and nothing in the README, the first-run tour or a milestone
summary may imply any of it exists.

Every entry names an **expiry**: the milestone that closes it, or "not planned".

**Broken and unverified are not the same thing** (ADR-0029). *Broken* is known
wrong. *Unverified* shipped with nothing in this repo able to prove it either
way — so if it looks off in real use, it was already suspect. Entries say which.

---


**Closed rows moved to [`docs/gaps/closed.md`](docs/gaps/closed.md)**
(housekeeping/01, 2026-09-18): nothing deleted, only split by
open/closed status. See also that file's own back-link.



## Deferred stretch goals (agreed at step 1, 2026-09-11)

These were in the original scope list and were cut before any code was written.
Each is either something Sublime Merge does not do well either, or something the
primary user does not use. Don't build them without asking.

| Gap | Why deferred | Expiry |
|---|---|---|
| Visual bisect (mark good/bad in the graph) | Rare workflow for this user; the graph work it needs is better spent on daily views | Not planned for v1 |
| Worktree switcher | Rare. Opening a worktree directly still works: it looks like a normal repo | Not planned for v1 |
| LFS awareness (pointer files, locks) | Writes go through the user's own `git`, so their LFS filters already work. Graft adds no LFS UI | Not planned for v1 |
| Sparse-checkout awareness | Rare for this user; correctness risk is high and the payoff is low | Not planned for v1 |
| Several repos open at once in tabs | A window-management feature, not a Git feature. One repo per process | Not planned for v1 |
| Submodule operations (update, sync, init) | Submodules **display** in the sidebar and can be opened; operating on them is deferred | Not planned for v1 |

## Platform

| Gap | Why | Expiry |
|---|---|---|
| Windows | Explicit non-goal for v1 (ADR-0005). No Windows CI, no Windows terminals in the manual checklist | Not planned for v1 |

## Design gaps (step 2 review, 2026-09-11)

Named in `docs/reviews/step-2-design.md`; deliberately left out of DESIGN.md
until the milestone that needs them.

| Gap | What's needed | Expiry |
|---|---|---|
| Rebase with a dirty tree (review N3) | DESIGN.md never says whether Graft surfaces git's refusal or offers `--autostash`. Decide in M5's design; if autostash, the popped stash needs DESIGN.md §6.3's untracked-safe snapshot handling, not a bare `git stash` | M5 |
| 50k+ refs in the sidebar and ref badges (review N4) | PROMPT.md §6.3 requires handling repos with 50k+ refs; branch-tree filtering, badge lookup and ranking at that scale are undesigned. Validate in M1's performance pass, not assumed free. m1/30 part 1: badge lookup 179 ns, `badges()` on 50k refs 1.35 s (debug, off-thread); frames with 50k refs pass the 16 ms release check like frames without them (m1-the-graph/30f, 7 of 7; idle-wake keys aside, see "Idle-wake keys can miss 16 ms"). No sidebar exists in M1, so branch-tree filtering/ranking is unmeasured | Frame part: ticket 30's remainder. Sidebar filtering/ranking: the milestone that builds the sidebar |

## Design gaps (step 2 review round 2, 2026-09-12)

Named in `docs/reviews/step-2-design-round-2.md`. (R2-2, backup-ref naming,
is already fixed in DESIGN.md §6 and ADR-0010 — not a gap. R2-1, post-promotion
checkpoint state on the no-commit-graph path, was closed by m1-the-graph/06:
DESIGN.md §3.1/§3.2/§3.6 say what a checkpoint holds on each path, and the
size is measured — 1.6 KiB, the same on both.)

| Gap | What's needed | Expiry |
|---|---|---|
| **Snapshot cost on high-frequency index ops (R2-3), measured.** `docs/perf/m3-snapshot.txt`, git 2.54.0, 20k tracked + 2k untracked: cold **868–884 ms**, warm **184–205 ms**. Reusing the real index (vs. a fresh one, 882–897 ms) wins ~4.5x, confirming the design. Cost to user: ~190 ms off-thread per stage/unstage keystroke on a repo that size (journal entry trails the keypress, fast repeated staging queues); single-digit ms on a normal repo. **Not broken; unjudged** | `m2-details-and-diffs/22`, measured by `m3/03` | M3 checkpoint (user judges feel). `snapshot::scope` narrowing to an index-tree snapshot is a one-function fix if needed — becomes an ADR, not a silent flip |

## Prototype gaps (step 3 spike, 2026-09-12; revised at the step-3 fix)

The graph spike (`prototypes/graph-spike/`, ADR-0012 + its Correction). The
original claim "proved Kitty + iTerm2 alignment and scrolling" was wrong —
that was pty-capture evidence, which proves emission, not rendering. After
the fix, the Kitty path is verified to the strongest level available without
human eyes: a real Ghostty 1.3.1 answered the probe, accepted the transmit
and placement at `q=1`, and reported zero errors. The iTerm2 tier is no
longer a gap — the user ran it in a real iTerm2 on 2026-09-12 and the graph
drew correctly (ADR-0012). It is throwaway code; these are the open
questions that remain.

| Gap | What's needed | Expiry |
|---|---|---|
| Resize frame busts the frame budget | A terminal resize clears and retransmits the whole strip: ~32–41 ms in the Ghostty bench, against a 16 ms budget. One frame per resize, so the spike leaves it. M1's fixed-height strips with ids (DESIGN.md §3.8) must not inherit a whole-viewport retransmit on every SIGWINCH | M1 tickets 25, 30 |
| Is ED-frees-image-data Ghostty-specific? | Only Ghostty 1.3.1 was tested. Unknown whether kitty-the-terminal (or other kitty-protocol terminals) also free image data on `ESC[2J`. M1 must not assume uploads survive a clear anywhere | M1 checkpoint (TESTING.md's Kitty and WezTerm pixel-tier rows exercise this by eye) |
| Sixel and plain-text rendering tiers unproven | Spike exercises only Kitty and iTerm2. Sixel alignment/scroll strategy and the text-cell fallback graph need their own validation | M1 tickets 08, 19 (text). Sixel-only fallback closed by 29 (ASCII, see "No Sixel encoder") |
| tmux passthrough and SSH unseen | m1-the-graph/29 built the wrapping: `graft_term::Transport` wraps each Kitty command (each upload chunk) and each iTerm2 band in its own `ESC P tmux;` passthrough; detection already fails closed to Unicode. It also fixed the probe wrapper, which ended `ESC ESC \` and so never terminated inside a real tmux. **Never run in a real tmux or over real SSH**: tmux is not installed on the build machine. Unknown: whether a Kitty/iTerm2 image lands in the right cell in a *split* pane (the outer terminal places at its own cursor), and whether tmux forwards the `q=1` error replies. `TESTING.md` "Must degrade, not break" has the exact commands | M1 checkpoint (the user runs the four rows) |
| Probe/input conflicts and resize | ~~Typeahead lost during probes~~ closed by m1-the-graph/20 (input mux). Still open: cell size is never re-probed on SIGWINCH | m1-the-graph/21 |

## Rendering tiers: order and the Unicode tier (settled 2026-09-12, ADR-0016)

**Settled.** The order is **Kitty → Unicode → iTerm2 → Sixel → ASCII**
(`docs/adr/0016-rendering-tier-order-kitty-then-unicode.md`). DESIGN.md §4.2
is rewritten around it, `detect::tier` implements it, and Unicode is the
default tier rather than a fallback. What is left is evidence, not decisions.

| Gap | What's needed | Expiry |
|---|---|---|
| **Sixel in VS Code unverified** | VS Code has `terminal.integrated.enableImages`; whether it gives a usable Sixel tier there is untested, and only the user can answer it. A yes would mean one of the two daily terminals is not image-less, which is the premise ADR-0016 rests on. `TESTING.md`, "Does VS Code give a usable Sixel tier?" carries the three questions and what each answer implies. Nothing is built on the answer either way — VS Code is treated as a Unicode terminal until the rows are filled in | The M1 manual checklist |
| **"Demonstrably beats Unicode" has never been measured for any protocol** | ADR-0016 puts iTerm2 and Sixel below Unicode because no side-by-side comparison exists — the Unicode tier does not exist yet. `TESTING.md`, "Does a pixel tier beat Unicode here?" is the procedure, and every row says **not run**. A yes for iTerm2 would supersede ADR-0016 with a new ADR, not edit it | M1 tickets 08 and 14, then the M1 checkpoint |

## M0 gaps

**What M0 has been seen doing** (swept at m0/12, revised at the M0 checkpoint). Everything was first exercised under a *bare pty* (no emulator) plus unit/snapshot/property tests — proves the bytes Graft writes, not a single pixel.

**The user then ran M0 in a real terminal at the checkpoint and approved it.** Confirmed by eye: repo, HEAD on `main`, **pixel** tier (Kitty protocol), cell size 17x37, **dark** theme from the terminal's own background, **truecolor**; probe answered in 367 µs (budget 200 ms); `q` exited clean. Detection, wording, theme hint and colour depth are confirmed against a real emulator. Still unseen: any other emulator, tmux, SSH, 16-colour/ASCII fallback. Six DESIGN.md divergences **confirmed by the user**: keep the code, amend the doc (ADR-0014).

| Gap | What's needed | Expiry |
|---|---|---|
| **No real terminal has been seen answering the capability probe.** Evidence: 46 unit tests on scripted replies + one bare-pty run (proves query order/timeout only). Unverified: any real emulator's replies/tier, `TIOCGWINSZ` cell size, `OSC 11` theme, tmux passthrough, SSH. `cargo run -p graft --example capability_probe` prints the report | Manual runs in Ghostty, iTerm2, Kitty, WezTerm, Terminal.app, Zed/VS Code, tmux ≥3.3 (passthrough on), tmux 3.2, SSH | **M0 review** |
| **Sanitising doesn't cover look-alikes, width, or non-`Cf` invisibles.** `term::sanitize` handles controls/bidi/`Cf`; width is handled separately at draw time (m1/08). Still passes: homoglyphs, East Asian ambiguous-width chars, variation selectors (hidden-payload channel) and Hangul fillers — none can drive the terminal, and removing variation selectors would corrupt real emoji/CJK. `Cf` ranges are literals pinned to Unicode 16.0 (no dependency answers property questions; a Unicode revision adding a format char goes unnoticed) | Whether to flag confusables/variation-selector runs is a policy call with real false positives — user decision | M1 checkpoint — open question |
| **The lane palette is argued, not seen.** Twelve hues chosen numerically (max-min OKLab distance + Machado 2009 CVD simulation, floors 0.18/0.055); checked only as flat swatches, never as thin graph lines, never by anyone with a colour-vision deficiency | m1/13 prepared the commands; only the user can judge criss-cross/foxtrot in Zed/VS Code at truecolor, 16-colour, `NO_COLOR` | M1 checkpoint (user, by eye) |
| **Non-UTF-8 paths typed for, not tested** — `RepoHandle` is `Path`/`PathBuf` end to end, but APFS/HFS+ reject such filenames so the fixture can't be built here | A heavy-tier Linux test (ext4/tmpfs) opening a non-UTF-8 path, round-tripping byte for byte; same for a non-UTF-8 branch name (`RefName` already stores bytes) | M1 heavy tier |
| **M0's screen only seen in a synthetic pty** (m0/11-12 drove the real binary at 100x30: `graft .`, `graft /tmp`, `--help`, `SIGINT`, `NO_COLOR=1` all behave correctly). Nothing answered the probe in either pty, so only the silent-terminal fallback tier has been seen rendered | Run it in Ghostty, iTerm2, Zed/VS Code and look | **M0 review** |
| **A panicking job's crash report unproven on the real hook.** Suppression covered from both ends (`crash`/`jobs` unit tests), but no test installs Graft's real process-wide panic hook — its early return (write report, don't restore terminal) reviewed, never executed | Panic a job by hand in a real terminal; check the screen survives and the status line names a real report file | **M0 review** |
| **No `criterion` benchmarks exist; no CI.** Heavy tier, 100k fixture, `getrusage`, 1,000-case `proptest` exist (m1/02); budget *assertions* added later (tickets 27/30/61 — see closed.md). ~~`cargo-fuzz` targets~~ closed by `m2-details-and-diffs/02`. No CI (PROMPT.md §7 item 6, no GitHub repo) | — | Not planned (CI only) |
| **Sanitising strips the emoji joiner (U+200D)** — multi-part emoji (family, profession) break into separate glyphs. Direct consequence of taking all of Unicode `Cf` (ADR-0014, user-confirmed: telling the joiner apart from a hostile format char needs special-casing emoji sequences) | User accepted at M0 checkpoint: cosmetic, safety trade is the right way round | Not planned; revisit only if it annoys him in daily use |


## M1 gaps

**Fixture corpus** (m1-the-graph/01, 2026-09-12).

| Gap | What is missing | Closed by |
|---|---|---|
| **Shapes PROMPT.md §7.2 names that the corpus does not hold.** Conflicting rebases, submodules, worktrees and LFS pointers are all absent. Ticket 01 put them out of scope on purpose: nothing reads them yet, and a fixture nobody opens is a fixture nobody notices going stale | One shape each, added by the milestone that first reads it, with the test that reads it | Conflicting rebases → M6; submodules and worktrees → M5; LFS pointers → M7 |
| **The non-UTF-8 shape does not exist on this machine.** `Shape::NonUtf8RefName` is skipped and the reason is recorded: `apfs at /System/Volumes/Data rejects file names that are not valid UTF-8 (Illegal byte sequence (os error 92))`. `graft_fixtures::try_borrow` returns that reason and the test for that shape prints the skip; it never passes quietly. The path half is covered anyway — `Shape::NonUtf8Path` keeps the same bytes inside a tree object and builds here — so what is untested is the byte-oriented *ref-name* handling in `core` and `git` | A heavy-tier run on Linux (ext4/tmpfs accept arbitrary bytes), where the same generator builds the shape and the same test opens it. Pairs with the existing "Non-UTF-8 paths are typed for, not tested" row above | M1 heavy tier |
| **Two fixture builders now exist.** `graft-git/src/fixture.rs` is a `#[cfg(test)]`-only helper that makes small repositories with plain `git` commands, written in M0 before the corpus existed. Its unit tests still use it. The corpus is the one every *new* test must use (m1-the-graph/01) | Move `graft-git`'s unit tests onto `graft_fixtures` where a named shape fits, and delete what is left of `fixture.rs`. Not done in ticket 01: it would rewrite tests the ticket has no reason to touch | M1, after the rev-walk tickets stop changing those tests |
| **Determinism is proven on one machine, not across machines.** The test builds the corpus twice in two temporary directories and asserts every object id matches. That covers "the generator is deterministic"; it does not cover a different `git` version producing different objects, which is why `git --version` is folded into the cache key rather than asserted | Nothing scheduled. The key makes a version change rebuild instead of silently reusing; a cross-version guarantee would need CI, which does not exist yet | Not planned |

**Heavy tier and `TESTING.md`** (m1-the-graph/02, 2026-09-12).

| Gap | What is missing | Closed by |
|---|---|---|
| **The heavy tier is a harness with almost nothing in it.** Four tests, all about the generator itself: it builds the history it promises, it caches, the commit-graph variant is separate, and the 100k fixture is 100k commits. Nothing in it measures Graft, because in M1/02 there is nothing to measure. The 300 MB gate in particular is *not* asserted anywhere — `graft_fixtures::measure` is unit-tested against a known allocation and then unused | Tickets 27 and 30 add the budget and memory assertions on top of it | M1 tickets 27, 30 |
| **The manual terminal checklist has never been run.** `TESTING.md` has the table, the steps and the pass conditions written out, and every row says **not run**. M0's checkpoint proved Kitty alone answers the probe; no tier has been looked at, because no graph is drawn yet | Tickets 08 (Unicode), 24–25 (pixel) and 29 (tmux, SSH) fill in their own rows against a real graph; 28 completes the matrix | M1 tickets 08, 24, 25, 29, 28 |
| **git/git has less memory headroom than the numbers suggest.** Gate is `getrusage` peak RSS (ADR-0021). git/git peaks at 219–226 MiB against the 300 MB budget (m1/27d); the 100k synthetic fixtures (157–179 MiB) don't share its commit/ref/tree shape, and nothing in the heavy tier measures git/git itself — a regression costing it 75 MiB could pass every test | A heavy-tier guard on a git/git-shaped fixture, or a review-time git/git RSS run | M7: real-repo-shaped guard before ship |
| **The heavy tier cannot see a git/git-sized memory regression.** `Scale::with_realistic_commits()` (m1/27d) sizes commit objects after git/git, but reverting fixes C+D moves the 100k fixtures only 23–27 MiB vs. 70 MiB on git/git — message size isn't the multiplier. Suspect lane width: git/git's `seen` merges fan to ~28 lanes (`Row.lanes` was 27b's largest holder) vs. ~3 in the fixture | A fixture shape with wide lane fan-out; decide whether the guard should be a per-fixture ceiling nearer the measured peak | M7: pairs with the row above, same fixture-shape fix |
| **1M-commit budget: gate passes with a commit-graph, reports (ungated) without one.** m1/56d/59b, release, `docs/shots/m1-memory-curve.txt`: **with cg, peak RSS 174–184 MiB — under the 300 MB gate.** **Without cg (ADR-0026, reported not gated): peak RSS 445–509 MiB, footprint 155–158 MiB** (footprint slope ~140 B/commit, live heap 76 B/commit). RSS−footprint (290–351 MiB) is the pack on disk, not a Graft structure; the same repo after writing the commit-graph drops to 173 MiB, gate pass | Offer is built (m1/54); M1 review file carries the table (ADR-0026 condition 1) | Stands while the user declines the offer on a repository; closes per-repository once its commit-graph is written |

**Reading history** (m1-the-graph/04, 2026-09-12). This ticket closed three M0
rows, now removed from the table above: the repository handle not outliving its
job (`app::repos::RepoStore`), `gix` being compiled with almost no features
(measured, see `graft-git/Cargo.toml`), and nothing ever cancelling a job
(`Esc`, with a test that presses the key).

| Gap | What is missing | Closed by |
|---|---|---|
| **The walk has never been run against a big repository.** Only the 20k-commit fixture and corpus measured; git/git (85,649 commits) cloned but unopened, no budget asserted | Ticket 27's budgets, ticket 09's first real graph | M1 tickets 09, 27 |
| **Node glyphs ignore `ui.icons`.** Deliberate: Nerd Font glyphs are private-use code points many fonts draw wider than one cell, breaking lane alignment; plain `● ◉ ◆ ◇ ■` reads the same in any font. Needs user OK to drop from scope | User decision at M1 checkpoint | M8: icon-set consistency, UI polish sweep |
| **Ahead/behind trusts commit dates**, not commit-graph generation numbers. Badly skewed clocks can miscount by the skewed commits | Generation-number ordering when a commit-graph exists | M1 ticket 30 |
| **The ref-states fixture's `HEAD` isn't where its builder says** (builder does `checkout --orphan fresh`; built fixture has `HEAD`→`main`). Nothing depends on it now, but an unborn-`HEAD` test would silently test the wrong thing | Check the fixture build's final steps/cache key | M8: fixture-build fix |
| **A history walk holds a job worker for as long as it's on screen** (pool is 4 workers max; cancel/`Ctrl+R`/shutdown free it) | Accept, or a dedicated history thread if M6 needs all four | Revisit M6 (concurrency work stresses the pool) |
| **Layout advances only as far as the screen asks** (DESIGN.md §3.6 wants background advance between frames; on-demand instead, `G` cold until it's run once — first paint 13–113 ms, `G` on git/git 0.52–1.3 s as a cancellable job). 30c removed chunk-edge spikes without building this; frontier sits at the last request's end, so fast keys can stall at the last laid-out row. One unexplained 17–18 ms release key (not a chunk edge) | Background advance while idle, and find the 17–18 ms key's cause | **M7** (user, 2026-09-17); design note must carry both |

**Layout engine** (m1-the-graph/05, 2026-09-12).

| Gap | What is missing | Closed by |
|---|---|---|
| **Neighbouring lanes can share a hue** (one-in-twelve hash collision, `BranchIdentity::slot(12)`; seen twice in the corpus). Shape/position separate them; colour doesn't. **Partly closed by m1/13:** 16-colour/`NO_COLOR` dashed odd columns separate them; truecolor/256 still rely on position only | `dashes_odd_columns` at every depth is a one-predicate fix if the user's by-eye pass wants it | M1 checkpoint (user, by eye) |
| **A folded merge looks like a plain commit** — only the header's "N merges folded" tells | A fold glyph/suffix (DESIGN.md §3.6 names none) | M8: visual polish |
| **Switching first-parent view keeps the selection only when it's near the top.** m1/16 re-finds it in the first two screens the other view sends; deeper or side-branch-only commits land on row 1 | Resolve the anchor via `StreamingLayout::row_of` (or nearest first-parent ancestor), laying out only as far as needed | M1, after ticket 16 |
| **`Esc` stops a topological sort on screen, not on the worker** — `full_topological`'s constructor doesn't poll stop, so the job thread keeps reading until construction ends even though the header already says "provisional" | Poll stop inside the read (ticket 27 makes it cheaper anyway) | M1 ticket 27 |
| **A long repository path truncates the order notice off the header** (truncates right; notice sits after path/count/filters) | Truncation priority (shorten path first) | M8: header layout-polish |
| **`Esc` during a sort unseen in a pty** — corpus sort finishes before a key can arrive; covered only by an `update` unit test | Capture on a history big enough for the sort to take seconds | M1 ticket 27 |
| **The walk answers nothing while a promotion lays out to the anchor** — a deep or off-view selection pays a long advance while scrolling waits | Bound the advance, or answer from the provisional layout until the promoted one catches up | M7: promotion responsiveness |
| **Unicode focus/ancestry highlight is bold/dim only** — no low-saturation lane colours (DESIGN.md §5.2); terminals ignoring SGR 2 show no fade at all. (Pixel tier's half closed in m1/42, alpha only, ADR-0024) | Muted per-lane colour tokens, judged by eye | M8: visual polish |
| **Fold/first-parent/stash keys (`Space`/`1`/`H`) aren't in the 80-col footer**, nor are M2's added keys; no `? help` shown | The `?` help overlay (DESIGN.md §1.16), built with M8's keymap table | **M8** (re-owned 2026-09-18) |
| **Compaction has no corpus shape and no snapshot** — only a synthetic `wide_history` test support and one scripted capture | A `ManyLanes` corpus shape, snapshots at 256/16/NO_COLOR | M8: fixture sweep |
| **A window whose lane-0-and-noded columns alone exceed `max_lanes` still draws wider than the limit** (ADR-0025 residual: `Fold::plan` never folds lane 0 or a node's own column) | A rail-node design (draw a merged-in node on the rail itself) | Not planned |
| **Clicking the rail does not expand lanes; `+` does** (DESIGN.md §3.5 named a click; ADR-0022 moved the count to the header, kept only the key) | — | Not planned |
| **Expanding (`+`) lifts the cap entirely** — a wide merge train can draw past the terminal's width; deliberate, but nothing wraps/scrolls | Horizontal scroll or re-cap while expanded, if by-eye pass finds it needed | Not planned |
| **An inclusion change re-reads history from the top of the walk to the changed commit** (batches of 1,024; `Included` keeps a second history-sized `Oid` list). Unmeasured on git/git — folding a merge 500k rows down re-reads 500k commits off-thread | Measure with ticket 27; checkpoint the frontier if slow | M1 ticket 27 |
| **A chunk cut back by `truncate` loses its eviction fingerprint** — a later rebuild of that chunk goes unchecked once | Accept, or re-fingerprint the kept prefix | M7: cache-correctness fix |
| **Elbows/junctions have no dashed forms** (Unicode has none) — a dashed lane's ends read solid | Accept unless by-eye pass finds it ambiguous | M1 checkpoint (user, by eye) |
| **HEAD on a merge draws as HEAD only** (`◆` wins over `◉`) — visible in criss-cross-merge, foxtrot-merge | A combined glyph, or leave to row text/badges | M1 ticket 11 |
| **An octopus merge subject isn't parsed** — `Merge branches 'a', 'b' and 'c'` matches none of DESIGN.md §3.4's shapes; extra-parent lanes fall back to short-SHA identity. Names are right there, in parent order | Extend `recover_branch_name` to return several names | Not planned — rare in daily work |
| **Column 0 stays empty with integration branches but no default branch** (`Seeds` reserves 1.. only). Not reachable today — `Branches::resolve` always picks a default when any local branch exists | Leave it | Not planned |
| **Snapshots are in commit-date order, not a commit-graph topological walk** — corpus has no commit-graph file, and date order is a valid topological order for it, so no snapshot shows a parent before its child. Already-drawn-parent path covered only by a directed unit test | A corpus shape with a commit-graph file and out-of-order dates | M8: corpus-coverage sweep |

**Streaming layout** (m1-the-graph/06, 2026-09-12). This ticket closed
KNOWN_GAPS R2-1 (above) and the engine half of two m1/04 rows (rewritten in
place).

| Gap | What is missing | Closed by |
|---|---|---|
| **`rows()` clones every row it returns** (a viewport's `Row`s copied out of the cache each call). Cheap at 50 rows/frame; not free | Borrowed slices/iterator once tickets 08/14 know renderer needs | M1 ticket 08 |
| **`Walk::full_topological` reads the whole range before emitting** (design, ADR-0008 — a background job; 1.2M object reads per open on git/git until the commit-graph offer is accepted, nothing caches order between runs) | Offer is built (m1/54); residual is the user's own decline | Not planned (m2-details-and-diffs/22) |

**History streams into the layout** (m1-the-graph/07, 2026-09-13).

| Gap | What would close it | Owner |
|---|---|---|
| **`AppState` keeps every row it's sent** — `History::chunks` is append-only `Arc<[Row]>`, growing with history (DESIGN.md §3.6 says the row cache must never do this). Fine on the corpus, not on git/git | The graph screen queries a long-lived layout for its viewport instead (tickets 08/14), measured in 27 | M1 tickets 08, 27 |
| **Rows laid out after `Esc` inside the interrupted chunk are dropped, not shown** (by design; count freezes at the last whole chunk) | Nothing needed unless resume-after-cancel is wanted | — |

## Scheduled interim states

These are *temporary*. Each one is a real gap between the milestone that creates
it and the milestone that closes it, and each must be visible in the UI while it
exists.

| Gap | Created by | Expiry |
|---|---|---|
| **Hunks inside an untracked file cannot be staged** — needs `add -N` first; `Space` on an untracked row stages the whole file | M3 design note, scope cut (ADR-0034, user-agreed) | M7 |
| **No `i` to add a path to `.gitignore`** from the changes pane | M3 design note, scope cut (ADR-0034, user-agreed) | M7 |
| **No commit message history** — the editor starts empty every time; no recall of recent messages | M3 design note, deferral | M7 |
| **Cannot stash only selected files** — `S` stashes everything | M3 design note, deferral | M7 |
| **Cannot create a branch from a stash** | M3 design note, deferral | M5 |
| **No file watcher** — the working tree refreshes on focus-gain and on demand, not when a file changes underneath. An external edit is invisible until you come back to the window | M3 design note, deferral (`notify` not wired) | M4 |
| **No action to clear undo backups** — the journal's backup refs accumulate; nothing in the UI prunes them | M3 design note, deferral | M8 |
| **UNVERIFIED: `Ctrl+Enter` to commit is indistinguishable from `Enter` outside the kitty keyboard protocol** — hence `Tab` to `[ Commit ]` as the real path (ADR-0034). The kitty path itself is untested in a real terminal | `m3-working-tree-undo-and-stash/05`, ADR-0034 | Standing — only a real terminal can close it |
| **`gallery --matrix` overwrites `docs/shots/35-coverage-matrix.txt` without the working-tree table** — run `m3_walk --matrix` after it, or the table is silently dropped. Fix = teach `coverage_matrix` the third table | `m3-working-tree-undo-and-stash/07` (finding 3) | M4 |
| **UNVERIFIED: the pinned uncommitted row with `HEAD` below the top row** — `k` from the top reaches it in 17 of 22 walk scenarios, all with `HEAD` topmost. The scrolled case is untested | `m2/06b`, narrowed by `m3-working-tree-undo-and-stash/07` (finding 5) | M4 |
| **Stash internals are still drawn in the graph** — `index on main:` and `untracked files on main:` rows reach the walk through `refs/stash` parents, one level below the `refs/graft/**` fix. UNVERIFIED whether M2's design intended stash commits to be visible | `m3-working-tree-undo-and-stash/08` fix round | M4 — decide intent first |
| **Capture headers carry `/var/folders/…` scratch paths** — machine-specific noise in committed captures. The obvious fix breaks TESTING.md's "what pyte saw" contract, so it needs a decision, not a patch | `m3-working-tree-undo-and-stash/07` review (Minor 7) | M4 |
| **Two acceptance tests run over 10 s each** (13.2 s, 12.1 s) against PROMPT §7.1's 10 s line. Both are real acceptance coverage; moving them to the heavy tier trades coverage for tier time. Fast tier is 40 s overall, inside budget | `m3-working-tree-undo-and-stash/07` review (M4) | M4 — user's call |
| **The release perf p99 measurement is too noisy to trust** — Pixel/50k refs `max_us` read 15772 and 9185 on consecutive runs of identical code, against a 16 ms budget. Best-of-three would cost 92 s → ~275 s | `m3-working-tree-undo-and-stash/07` review (M6) | M6 |
| **`crates/graft-git/src/repo/refs.rs` is ~2,000 lines** and wants splitting; deliberately not done in the same pass as the C1 fix, because a move in the same commit as a change hides the change | `m3-working-tree-undo-and-stash/08` fix round (M7) | M5 |
| **UNVERIFIED: the fake-remote servers' listener threads are not proven to exit** — tests prove each port closes after drop, and lsof showed no leftover listeners, but no test asserts the threads themselves end. A leaked thread would outlive its test silently | `m4-remotes/01` (acceptance criterion left unticked by its builder) | M4 review |
| **A forced undo is not pinned to the entry the drift box named** — `ops::undo` takes no aim, so between the box being drawn and `U` being pressed the journal tip can move and the undo runs against a different entry. Needs `undo(ctx, force, aim)`, `UndoError::Aimed`, and `EntryId` on `Undone::Drift` | `m3-working-tree-undo-and-stash/06` review (M4) | M4 — signature change, not a patch |
| **The id-rewind pattern survives in 13 more places** — group 06 fixed the two symptoms and added the guard (`no_local_job_id_is_ever_minted_twice`, proved red), but the minting pattern that causes it is unchanged at 13 other sites. Guarded, not removed | `m3-working-tree-undo-and-stash/06` review (M5) | M4 — structural refactor |
| **`Esc` on a journal read that has not landed cancels it, and `settle` immediately re-requests** — churn, not loss: the read repeats rather than the data going missing | `m3-working-tree-undo-and-stash/06` fix round | M5 |
| **The journal browser has no status line while it's open** — same defect `StashView::notice` fixed for the stash overlay; a failure while browsing has nowhere to draw. `u` is the key the milestone rests on | `m3-working-tree-undo-and-stash/06` | M5 — the walk has no journal-browser-failure scenario, so group 07 did not close it |
| **UNVERIFIED: no capture in this repo proves any colour, anywhere in M3** (pty pipeline records pyte grids with no colour, ADR-0029). Every colour decision in the changes pane, commit editor, stash overlay, journal browser ships judged by the user's eye only | M3, all visual groups | Standing — only the user, at the checkpoint |
| **UNVERIFIED, not closable from here: ~2% of cancels landing inside git's own index-write window can still strand a lock.** SIGKILL case is fixed/measured (below); this remainder is a race inside git itself (`create_tempfile` opens the lock before marking it active) — the same race a user's own `Ctrl+C` on git hits. Not a Graft defect, not Graft's to close | `m3-working-tree-undo-and-stash/05` review round | Not planned; revisit only on a field-note report |
| **BROKEN: `strip_comments` hardcodes `#`, ignores `core.commentChar`** — a user with a different setting gets real message lines stripped or comment lines kept | `m3-working-tree-undo-and-stash/05` review (Minor 8) | M4, sooner on a field note |
| **`GIT_EDITOR` resolved on the UI thread at editor open, not as a job** — a slow `git var GIT_EDITOR` blocks the UI briefly. Proper fix needs a new Command/Product pair + AppState field + two-step for `e` | `m3-working-tree-undo-and-stash/05` review (Minor 9) | M4 or M5, next job-runner touch |
| **UNVERIFIED: a real `$EDITOR` on a real terminal.** Suspend/resume proved by transcript order, panic guard, mux test — not the thing itself. Wrong restore breaks the user's terminal (PROMPT.md principle 8). Found: spec's suggested `InputMux::hold()` does **not** stop the reader thread (would race `$EDITOR` for keys); a self-pipe `Gate` replaces it | `m3-working-tree-undo-and-stash/05` | M4 or first field note — group 07 explicitly did not cover a real $EDITOR or a real terminal |
| **UNVERIFIED: GPG/SSH commit signing while the editor holds the terminal** — a pinentry program wanting the tty during a signed commit untried | `m3-working-tree-undo-and-stash/05` | M4 or first field note |
| **README has no M3 section — `[keys] suspend_on_ctrl_z` (ADR-0002) undocumented** (works, just unmentioned) | `m3-working-tree-undo-and-stash/05` | M8 docs pass, or acceptance group |
| **The untracked-file confirm box asks "Delete notes.md?" over a button labelled `Discard`** — question and button disagree on the one box meant to be unambiguous | `m3-working-tree-undo-and-stash/04` review | M4 — group 07's `discard-untracked-binary` opens this box but asserts repo state, not the wording |
| **UNVERIFIED: busy/done status, per-class discard wording, kept cursor — proved only by unit tests on scripted state, none key-to-repo.** Four acceptance lines ticked without real proof | `m3-working-tree-undo-and-stash/04` review | M4 — narrowed: group 07 proves the three discard classes key-to-repo (`discard-tracked-unstaged`, `-untracked-binary`, `-staged-and-unstaged`); busy/done status and kept cursor remain unproven |
| **UNVERIFIED: hunk *unstage*/*discard* have no key-to-repo test** (stage direction proved end to end; `PatchDirection::Reverse` is exactly where group 01's review found the original bug) | `m3-working-tree-undo-and-stash/04` | M4 — group 07 has `stage-hunk` and `stage-line` but no unstage-hunk or discard-hunk scenario, so this is untouched |
| **UNVERIFIED: the confirm box's git command string is hand-written, not from `Operation::plan`** — can drift from what actually runs | `m3-working-tree-undo-and-stash/04` | M5 (Ghost Preview must source planned commands from `plan`) |
| **Redo of an undone conflicted `stash pop` restores markers with a clean index** (content intact; git's "mid-conflict" flag lost, `git status` reads ordinary modified) | `m3-working-tree-undo-and-stash/03` | M6, conflict resolver owns unmerged-index state |
| **UNVERIFIED: two Graft processes open on the same worktree** — journal is a file, nothing arbitrates between instances; not tested/designed against | `m3-working-tree-undo-and-stash/03` | M4 or first field note |
| **UNVERIFIED: every M3 git-behaviour claim proved on git 2.54 only; stated minimum is 2.38.** `restore --source=HEAD` on a staged rename, `stash store -m`, `clean -f` with a pathspec, `update-ref` CAS most likely to have moved | `m3-working-tree-undo-and-stash/01`, `/02` | M3 review: run against 2.38, or raise the documented minimum to match what's tested |
| **A stash restored by undo re-lists at `stash@{0}`, not where it was** — git has no insert-at-index for the stash reflog. Content/commit unchanged, order wrong | `m3-working-tree-undo-and-stash/02` | M7 if real use finds it confusing |
| **A multi-line stash message survives undo as its first line only** (reflog keeps one line; content untouched, description shortens) | `m3-working-tree-undo-and-stash/02` | Not planned |
| **Single-line stage/discard refused on a hunk ending a file with no trailing newline** (`PatchError::NoNewlineBoundary`; whole-hunk stage/discard still works, byte-proved). Deliberate refusal — `git apply --cached` alternative silently fuses lines with no error. **Not broken — narrowed feature, visible message** | `m3-working-tree-undo-and-stash/01` review fix (`b47ae51`) | M7 if real use hits it |
| **No in-app conflict resolver.** M5 can produce conflicts (merge/rebase/cherry-pick) before M6 builds the resolver; M5 must detect conflict state, list files, offer configured `git mergetool`/`$EDITOR`, keep the operation banner (Continue/Skip/Abort). Must not claim resolution works | M5 | **M6** |

---

*Builders and reviewers: add rows here the moment something is stubbed, naming
the milestone and ticket it came from. An entry here is not a failure; a missing
entry is (CLAUDE.md).*

## Input mux (m1-the-graph/20)

| Gap | What would close it | When |
|---|---|---|
| **A reply split by more than 50 ms is misread as keys** (`SEQUENCE_TIMEOUT`; over slow SSH a late-arriving reply flushes as `Alt+[` + chars). Not observed locally. **Ticket 21 adds exposure:** resize/focus re-queries share the decoder mid-session, so a split reply could type into the graph while the user works, not just at startup | Hold an unfinished reply-shaped sequence to the open waiter's deadline instead of 50 ms, measured over real SSH | M1 heavy tier |
| **A re-query reply arriving after its deadline can be taken for the next round's** (`detect::redetect` discards waiting bytes per round, but a late reply mid-round is parsed as its answer). Needs a reply later than the 200/500 ms budget; not observed | Tag rounds (e.g. distinct `DECRQM` per round) if seen | when seen |
| **Key order under a pty proven by unit test only** — pty check proves arrival, not on-screen order (selection detector can't see cursor row); order covered by socket-pair unit tests | A pty assertion on the cursor row | M8: pty coverage sweep |
| **Decoder covers xterm/SS3/kitty `CSI u`/SGR/X10 mouse, not urxvt `1015`** (which crossterm's `EnableMouseCapture` also turns on); SGR-preferring terminals never send it | Decode `1015` if seen | when seen |

## m1-the-graph/22 — rasteriser
- Node style defined twice (`graft_ui::graph_column::NodeKind` Unicode glyphs, `graft_graph::raster::Ring` pixels, same 6 variants) — could unify; left for ticket 24.
- `CELLS_PER_LANE` (=2) declared in both `graft-ui` and `graft-graph::raster` — must agree or pixels drift from text.
- Strip cache (m1/23) has no flush owner yet on resize/theme/re-detection events; wiring lands with ticket 24. Cache key alone already prevents stale hits.
- Raster tests copy the 12 lane hues from `graft-ui` (`graph` can't see `ui`) — doesn't affect assertions beyond snapshot slot letters.
- Pixel shots rendered at 17×37 cell only; other cell sizes (e.g. 8×16) unjudged.
- Render time: fast tier only, 32 rows × 5 lanes @17×37, M5, median 1.38 ms release / 13.4 ms debug. Wide strips unmeasured (ticket 30 owns it).

## m1-the-graph/24c — Kitty draw and replies
- **Pixel column width comes from the text tier's fold, strips rasterised through one fold across every strip** — 24d places each strip on its own lane columns so narrower strips aren't stretched; a strip *wider* than its column is still squeezed (no column crop).
- **Replies drained only on a wake** — a `q=1` error with no following key/resize/job result waits for the next wake to show/retransmit; mux has no wake for replies.
- **A panic leaves uploads in the terminal** — `free_all` runs on every loop exit except the panic hook.
- `STRIPS_KEPT` (16) fixed for both strip cache and uploads, not sized from viewport.

## m1-the-graph/24d — flush, F3, Ghostty check
- ~~Retransmit after a clear uses new ids, old ones never freed~~ — moot since 25 (`forget_uploads` no longer called; resize keeps uploads). Still true if a future caller does. Ghostty frees image data on clear itself (ADR-0012); a Kitty-protocol terminal that keeps data across `ESC[2J` holds old uploads until exit.
- **The real-Ghostty check drives the encoder with synthetic strips** (`examples/kitty_check.rs`), not the app — proves Ghostty accepts transmit/crop/clear/hide/free at `q=1` with zero errors. Whether the app's real strips look right is the user's by-eye check.

## m1-the-graph/25 — iTerm2, resize frames
- **iTerm2 tier not seen by eye** — byte stream pinned by tests/pty capture only. `doNotMoveCursor=1` is iTerm2 ≥3.5; older iTerm2 may move the cursor after a band.
- **Resize frame measured without a terminal**: release in-memory, Kitty 0.30 ms / iTerm2 0.25 ms (first frames 8.8/4.2 ms) — excludes pty back-pressure and terminal's own work. 25-era Ghostty resize bench (32–41 ms) is the only real-terminal number.
- **Real Ghostty window resize not driven** — row-erase path checked, not an actual window resize. If Ghostty drops uploads on resize, `q=1` ENOENT self-heals on next wake.
- **iTerm2 erases graph cells every frame** (`ECH` then re-send, inside synchronized update — no flicker expected, unverified).
- **`Flush::Resize` now unused by the app** (kept in `graft-graph` API); DESIGN.md §3.8 still lists resize as a wholesale flush.

## m1-the-graph/26 — search
- **Only the current hit is marked**; other hits' rows aren't highlighted.
- **A hit hidden by the view** (first-parent/folded) is counted but not selectable — "(not in this view)".
- **Search walks history again on its own job**, not the rows the history job already read. 100k no-match scan measured under a pty.
- **`n` before the first hit arrives does nothing** (no queued step).

## m1-the-graph/32 — wheel scroll
- **The wheel scrolls until only the oldest commit is on screen** (`top` stops at the last row; ticket 17's clamp). Open: should it stop when the last row reaches the bottom instead.

## m1-the-graph/30 — frame budgets and scale (part 1)
- **Idle-wake keys can miss 16 ms at 50k refs (release)** — printed, not asserted (ADR-0021/30c): 14 of 140 idle-wake keys over 16 ms across 7 runs, all at 50k refs, max 19.1 ms (100k fixture stayed under). Cause named, not removed: CPU's slow power state after idle. Scrolled keys pass 7/7. Open: user's by-eye check at the gallery re-walk.
- **1M+ commits extrapolated, never observed** — PROMPT.md §1.2's "60 fps on 1M+" unmeasured; no 1M repo available, heavy fixture is 100k.
- **Frames measured without a terminal** — in-memory backend excludes pty back-pressure/terminal paint; 25-era Ghostty resize bench (32–41 ms) is the only real-terminal number.
- **Pixel strips rasterised again after eviction (30b)**: cause 2 fixed (strips wait for cursor row); still 43/556 strip misses in the release 100k run re-rasterise unchanged-input strips evicted by too-small cache/unplaced capacity, ~434 KB retransmit each, not tuned. Also: blank graph for one frame on a page key whose cursor row isn't on hand — not judged by eye.

## m1-the-graph/35 — graph gallery (part a: command + matrix)

`docs/shots/35-coverage-matrix.txt` (regen: `cargo run -p graft --example gallery -- --matrix`) shows what's *named*, not file contents.

| Gap | What is missing | Owner |
|---|---|---|
| **Light theme: nothing named `light` for any repo** (`10-row-columns.txt` holds light captures unnamed as such; no `.snap` renders light) | A light-theme snapshot per shape, or naming fix | M8: gallery-naming fix |
| **Pixel evidence for 3 shapes only** (foxtrot, criss-cross, octopus); 17 shapes + every real repo have none | Raster snapshots for shapes whose picture differs | M8: gallery-coverage sweep |
| **ASCII evidence for 3 shapes only**, none for real repos | ASCII snapshots for the same set, or a note that ASCII shares Unicode layout | M8: gallery-coverage sweep |
| **Sizes not in snapshot names** (only `feature-branches` names its size) | Put size in every screen snapshot's name | M8: gallery-coverage sweep |
| **`graft-self`/`synthetic-100k`/`git-git`: captures only, no snapshot** (rule: no test touches a heavy repo) | Nothing planned; captures are the evidence, judged by eye | — |
| **`non-utf8-ref-name`: no graph** (APFS skip, ticket 01) — matrix counts two captures, both the "not available" page | A Linux heavy-tier run | M1 heavy tier (existing row) |
| **The gallery restarts the run at every step** (terminal restored/re-probed, each `]` blinks). Key-loss bug this row first found is fixed in 35c (one stdin reader per process, `mux.hold()` on loop end) | Split `run` into terminal setup + per-repo loop | Not planned; gallery is a test harness |
| **Idle prefetch (30h) costs CPU after every pixel frame with idle time, mostly not rasterising**: release p50/p99/max 100k refs 0.8/4.6/6.7 ms, 50k refs 3.3/5.1/5.6 ms. `update`+`pixel_rows` run twice even when fully cached. 50k-ref clone cost fixed by m1-69 (`Arc<RefBadges>`): p50 now 0.6 ms both, p99 4.9–6.6 ms. What-ifs themselves remain the cost | Skip what-ifs when window/cursor/cache unchanged since last prefetch | 30f seam 4 or M7: hardening pass |
| **The "not available" page understands only `]`/`[`/`q`**; an escape sequence is skipped, a lone `Esc` swallows the next key | Reuse the input mux instead of raw byte reads | Not planned |

## m1-the-graph/45 — `y` copies the SHA
- **Whether the clipboard got it is unknown to Graft** — OSC 52 has no reply. Terminals with it off (some VS Code/Zed, tmux without passthrough/set-clipboard) silently drop it. Paste-in-terminal by-eye check not done.
- **Inside tmux the request is wrapped in DCS passthrough, like images** — tmux's own unwrapped `set-clipboard on` path unused; open whether to send unwrapped inside tmux.
- **`y` while a failure is on the status line** still sends the SHA but shows no notice — failure is never replaced.

## m1-the-graph/49 — commit handles and the id table
- `graft::history::WalkSource::links` reads each commit whole for its parents; nothing on a hot path calls it yet — the hydrate-on-demand ticket should take parents from the walk instead, as `Walk::next_links` does.
- Heavy-tier tests for the table and `next_links` (5k pair; 100k overflow ≤60 B/commit) written, run in the milestone review only.

## No-commit-graph footprint slope at 1M (m1-the-graph/59b)

- **Verdict (56d): no-cg footprint slope 138–140 B/commit** (`docs/shots/m1-memory-curve.txt`: 1M footprint 154.8–155.8 MiB, 100k 35.5–36.2 MiB). **Live heap floor 76 B/commit** (interner 58 + order 8 + promoted 4 + rest); the other ~62 B/commit is freed transient memory macOS malloc keeps in the footprint. **Bar (m1-63, user decision):** live heap ≤100 B/commit gated (76, passes); footprint at 1M reported only, floor named (~140, not gated; ADR-0026 amended).
- Retained-and-why: interner `by_prefix` map doubling (~17 MiB at 1M, paging needs a new structure); `Vec`→`Arc<[CommitHandle]>` copy in `topological_order` (~4 MiB, needs `unsafe` in a `forbid(unsafe_code)` crate); swapped-out order at promotion (~9 MiB, ADR-0008 by design); ~19 MiB in the date walk untraced. None built.
- Expires: when the live heap slope regresses above 100 B/commit (gated at reviews), or the user funds cutting the ~62 B/commit retained above the floor.

## m1-the-graph/56d — memory-curve verdict: what stays open (2026-09-14)

Every budget in 56's checklist passes (`docs/shots/m1-memory-curve.txt`). Two things reported, not proven:

- **Cold first paint at 1M ~0.7–1.0 s (ADR-0027).** Run 1: no-cg 997–1,032 ms, cg 710–790 ms; runs 2–3, 88–112 ms (under 150). Budget judged warm by user decision (ADR-0027). Evidence the cost is page cache, not Graft: a just-written 1M fixture painted in 124–134 ms on its first run. Not proven: only `sudo purge` empties macOS's cache. **Expiry: user runs one gated first-paint straight after `sudo purge` at the M1 checkpoint, or accepts the warm convention as final.** User.
- **`examples/budgets`'s counters (harness only) hold 4× the app path** (lay history out again via `direct_layout`: 100k cg 124 MiB vs 33 MiB gated). `--gate` excludes them from every real gate/curve cell. **Not planned** — harness isn't the app.

## m2-details-and-diffs/01 — git spawns outside the runner (2026-09-17)

- `contained_in` on a repository with no commits: spec asked for `Ok`+empty; with no commits git exits 129, returned as `Exited` not hidden. Tested on `UnbornBranch` instead: `Ok`. Callers only ask for an existing selected commit. Expiry: whichever detail-model ticket calling `contained_in` needs otherwise.

## m2-details-and-diffs/04 — Mistake Radar rule limits (2026-09-17)

- **A pushed pull-merge on `main` reads as accidental-pull (info), not foxtrot (warning), in a clone without a remote-tracking reflog** — the only fact that would show it (`RepoFacts::previous_default_tips`) is absent from a fresh clone. Expiry: `radar-context-and-markers` fills it from the reflog; one-rule addition.
- **Rule A (non-first parent already on default chain) unreachable through `git merge`** — only hand-built commits (`commit-tree`) produce it. Not a bug; noted so nobody looks for a fixture.
- **`UnpushedOnDefault` has no fixture** (`NoUpstream` covered only via foxtrot-merge's `feature`). Unit tests cover both paths. Expiry: M2 acceptance ticket, if it wants `main` ahead of `origin/main`.

## m2-details-and-diffs/06 — detail model seams (2026-09-17)

- **No footer hint for `Enter` yet** — a `⏎ details` hint would advertise a pane that doesn't draw until `detail-pane-frame`. Expiry: `detail-pane-frame`.

## m2-details-and-diffs/06b — working-tree row reach (2026-09-17)

- **The uncommitted row appears only while `HEAD` is the top visible history row** — with a newer branch above `HEAD` the user scrolls to reach it. Owner: M3 working tree decides whether to pin the row wherever `HEAD` sits.

## m2-details-and-diffs/10b — detail pane resize: pointer input and hints (2026-09-18)

- **No `?` help screen — deferred to M8 by decision** (planner-fable, 2026-09-18). No handler/overlay exists anywhere; 10b's mention of it was a slip, never in M2's design note. DESIGN.md §1.16 defines it as M8's keymap table filtered to the focused pane. Cost while open: `Space 1 H x w W v [ ] <> L M O` learned only from README/TESTING.md, `?` unbound. For M8: overlay geometry/widget must live in `graft-ui` (the `update/` purity guard rejects ratatui/`graft_ui` mentions there). Pairs with the M1 footer-keys row. Expiry: **M8**.
- **Map-view no-op guard for resize not added** — `Screen` has no `Map` variant until ticket 19; closes automatically once `Map` exists and gets the guard explicitly.
- **`docs/shots/m2-detail-keys-resize.txt` capture not regenerated** after the `<> resize` footer hint changed what's on screen. Expiry: next detail-pane ticket or a capture-refresh pass.

## m2-details-and-diffs/11 — compare two commits (2026-09-18)

- **A compare's pane header never shows file/line counts (`· N files +A −B`).** `compare_header` builds `Subject::Compare` with base/head SHAs only, by design from an earlier ticket. Wiring `DetailState::files`' counts in is what's left. **The gap row that named a `compare-mode` ticket to own this was dangling — no such ticket exists in M2's list; real owner is the "M2 docs-sweep" row below.**

## m2-details-and-diffs/13b — radar wiring (2026-09-18)

- **`RefBadges::branches()` added this ticket, not in 13b's file list** — needed so `view::radar::branch_facts` can resolve a branch's tip purely from `AppState.refs`.
- **`Task` lost its `Copy` derive** (`Task::RadarContext` carries a non-`Copy` `RefName`) — `jobs/tests.rs`'s `next_started` now takes `&Task`; eight call sites updated.

## M2 docs-sweep — new rows (m2-details-and-diffs/22, 2026-09-18)

Every M2 feature ticket is done (17, 19, 20, 21 and the rest); these are gaps
the design note's §8 coverage map marked "uncovered by design" or that this
sweep found while closing the rows above. Each is real product behaviour
today, not a stub.

| Gap | What's needed | Expiry |
|---|---|---|
| **PROMPT §4 image diffs in the pixel tier are not built.** M2 shows `binary · <old> → <new>` for images (the same binary-file line as any other binary), never a rendered before/after image in the Kitty/iTerm2 tier | A pixel-tier image-diff renderer; PROMPT §4's own scope | M8 polish |
| **Map view has no mouse interaction and no pixel-tier density curve.** `map_screen` draws text glyphs (`▁▂▃▄▅▆▇█`/ASCII ramp) at every tier, including Kitty (design note's Proposed changes); clicking a row or bucket does nothing — only `↑↓`/`Enter`/`Esc` | Mouse hit-testing on `MapScreen`'s rows/buckets; a pixel density curve for the pixel tier | M8 polish |
| **Overview strip defaults to off; no `[ui]` config key starts it on** (`O` toggles per-run only) | An `[ui]` startup key, seeded like `[diff]`/`[radar]` | M8 polish |
| **Map view shows local branches only; a remote doesn't collapse into its upstream's row (DESIGN §5.1)** — `build_map` walks one range per local branch | Fold upstream into its row, or list untracked remotes separately | M8 polish |
| **Map view's merge elbow marks only merges into the default branch's first-parent chain** — a feature-into-feature merge draws no elbow | A cross-branch merge search (design note weighed and declined this cost for M2) | Not planned unless asked |
| **The compare pane header never shows `· N files +A −B`** (`compare_header` builds base/head SHAs only). Note: 11's gap row named a `compare-mode` ticket that doesn't exist in M2 — dangling reference, corrected here | Wire `DetailState::files`'s totals into `compare_header` | M8 polish |

## M2 acceptance walk (m2-details-and-diffs/23, 2026-09-18)

Gallery findings under pty (`docs/shots/m2-*`); owner **M2 review** unless named.

| Gap | What's needed | Expiry |
|---|---|---|
| **No corpus commit changes two files** — stop 2 (`Enter j`) shows `j` inert, no fixture exercises the file cursor | A `two-file-commit` shape | M2 review |
| **Force-push probe (ticket 15) has no gallery stop**, never run against a real rewritten remote (capture is TestBackend) | A fixture with a rewritten-branch remote, and a stop | M2 review |
| **Gallery banner hidden at <100 cols when `Detail` tab is up; Map view draws no banner at any size** | Draw the banner/title line in tab-strip and map layouts | M2 review |
| **Coverage matrix gaps for details rows**: no pixel-tier/light-theme/ASCII except stops 1,6,7,15,16 at 80x24; no 100x30 except those five | Pixel captures need a Kitty-protocol pty harness | M2 review |

- **Heap grows ~76 B/commit against a 50 B/commit bar** (M2 review fix 1a). `heavy::slopes_per_commit_with_a_commit_graph` still fails (heavy tier exits 101, completes in 91 s). Not caused by the C1 fix (69→76 B/commit). Peak RSS 214 B/commit against 200. Needs its own ticket: find what holds per-commit state beyond the layout window.

### The Mistake Radar's force-push probe is no longer exhaustive
**Expiry: M7.** Behaviour change (M2 review fix 1a, user-registered 2026-09-18). Force-push verdicts computed only for branches seen in the visible window, capped at 32/session — a branch never scrolled to goes **unflagged**. Price of fixing C1 (unbounded probing timed the heavy tier out at 1275 s, now 91 s). M7 decides: fund an exhaustive background sweep, or keep 32-per-session and say so in the UI instead of silence reading as "no force-push found".

## M2 review round 2: Minor findings left open (2026-09-19)

16 open Minors from round 2 (`docs/reviews/m2-details-and-diffs.md`). Round 1e fixed the three Majors and (as a side effect) Minor 13. Not duplicated with the "Unverified" colour rows or housekeeping/02's two rows below.

### Mistake Radar: evil-merge probe and its text

| Finding | Broken/unverified | What a user notices | Expiry |
|---|---|---|---|
| Pane header reads `evil merge: clean (N files hand-resolved)` for unchecked files — `clean` means "nothing wrong found", not "not checked" (`detail_pane/header.rs:55`) | Broken | Reassuring reading when it shouldn't be | M8 polish |
| Both-sides-touched blind spot (hand-resolved file both sides edited can hide a real evil edit) disclosed only in the Explain paragraph, not TESTING.md | Broken (docs) | Must open Explain on the exact merge to learn this | M8 polish |
| `merge_base` takes only the first line of `git merge-base`; criss-cross history can have several bases, judged against one the real merge didn't use | Unverified — not reproduced | Rare topology: merge could be wrongly cleared/flagged | M7 |
| Merge-dense history: `j`/`k` spawns one full `git show --remerge-diff` per row, killed ≤50 ms later; 16-entry probe cache thrashes | Broken | Brief hitches holding `j`/`k` through e.g. git/git-like history | M7 |
| `force_push_candidates` allocates a `Vec` every `update` (incl. key repeat) even once the 32-probe cap is full | Broken | Small extra per-keystroke cost after cap is maxed | M7 |
| `RadarContext`'s `Eq` compares tables by `Arc::ptr_eq` — identical contents, different instances compare unequal | Broken | Not directly visible; extra radar recompute risk | M7 |
| `scripts/m2-fix-1c-evil-audit.sh` reimplements the evil-merge rule in shell, already drifted once | Broken | None; audit numbers can go stale | not planned |

### Overview strip

| Finding | Broken/unverified | What a user notices | Expiry |
|---|---|---|---|
| `view/strip.rs`'s search-hit `HashSet` rebuilt every frame the strip is shown (M2 fix moved the cost, didn't remove it) | Broken | Noticeable per-frame cost when a search matches most of history | M7 |

### Code health: visibility, file boundaries, docs (owned by M8's architecture pass)

Not user-facing.

- `ramp_glyph` went `pub`→`pub(crate)` but `graft-ui/MAP.md:151-152` still lists it public and disagrees with itself. Broken.
- Round 1d's splits widened visibility instead of narrowing: `detail_key`/`detail_event` + ~46 items across `detail_pane/*`, `map/{walk,rows}.rs`, `unified/primitives.rs`, `update/detail/state.rs` are `pub(crate)` when `pub(super)` would do. Broken.
- `update/detail/{state,ops}.rs` splits struct from impl, not along a real boundary — `ops.rs` (395 lines) mixes unrelated concerns; `detail_pane/line_render.rs` same shape. Broken.
- `graft-git/src/diff/unified.rs` is 417 non-test lines; round 1d claimed every split file under 400. Broken (contradicts its own record).
- `side_by_side.rs`'s `diff_body_side_by_side` still takes two bare bools instead of the `DiffFocus` struct introduced to replace them. Broken (inconsistency).
- `update/` purity guard (`tests.rs:1480-1557`): file-count floor `>=7` vs 20 real files (a broken walk could pass silently); substring check misses `use std::{time, thread}`/`core::time`; claims "every other file pure end to end" — false for six files with inline `#[cfg(test)]`. Broken, real holes (the recursive-scan fix itself is sound).
- `ARCHITECTURE.md` doesn't document `git::run`, `git::contained`, `LOCAL_JOB_BIT`, or the `update/` purity guard as a stated rule. Broken (docs debt).

### Test suite: watch, not yet a failure
- Fast tier grew 21→24.3 s, slowest test 6.42→7.76 s. Still under budget (reviewer). Unverified trend — watch it doesn't cross the laptop-safe budget as M3 adds tests. Expiry: M3.

---

## Unverified (shipped, nothing here can prove it)

### Two M2 colour fixes were never seen by anything
**Expiry: M3** (first capture pipeline that can carry colour, or the user's eye). ADR-0029: pty pipeline records character grids with **no colour attributes** — colour-only changes are invisible to every check this repo can run.

- **`W` (word tint)** — review Major M1, fixed round 1b. Unit-tested; never rendered.
- **Diff body all `text-muted` unless the Diff region is focused** — review Major M2, fixed round 1b. Same.

Both pass tests, neither seen. If word-level diff tinting or the unfocused diff body looks wrong in a real terminal, report it in `docs/field-notes.md` rather than assuming it's new. **Closes with:** a capture path carrying SGR attributes, or the user looking once and saying.

### `map.rs` `build_default_chain` still holds `HashSet<Oid>`
**Expiry: M7.** Broken (mild). Found by housekeeping/02, out of that ticket's scope. Same ADR-0026 breach the radar's default chain had (33-byte Oids where a `HandleSet` bit would do). Transient, Map-view only — never showed in the steady-state heap-slope guard; costs on a large repo once Map view opens. **Fix:** same shape as `92c35ac` (`HandleSet` + commit table).

### The frame gate asserts on the max of 420 keys, so one outlier fails it
**Expiry: M7.** Unverified-adjacent — passes today but fragile by construction. Found by housekeeping/02. A single slow key fails the whole gate; six interleaved A/B runs showed baseline and patched both spike over 9 ms under load — machine noise, not the code under test. Asserting on the worst sample trains re-running until green, hiding real regressions. **Fix:** assert on p99, record the max. Pairs with M7's 17–18 ms release-key ticket — same measurement path.
| M4/02 | Askpass socket has no peer-credential check; the 0700 dir is the only guard | unverified | M8 |
| M4/02 | SSH askpass and SSH/forge auth-failure strings not exercised; full git→helper→app path is group 06 (G5b) | unverified | M4 |
| M4/01 review | HungRemote wake-up race — closed by teardown fix (accepted-after-stop is dropped unanswered) | closed | M4 |
| M4/02 review | Askpass write_reply has no write timeout; close()'s join could hang on it | unverified | M4 |
| M4/02 review | Remote-controlled text (prompts, failure reasons) is unsanitised in graft-git; group 06 must pass it through term::sanitize | unverified | M4 |
| M4/02 review | RunError::Spawn mislabels a poisoned-sink error; WouldOverwrite multi-path parsing tested on one file only | broken (label) / unverified | M5 |
| M4/03 | Main red at 0ba9203..(g03 fix): app snapshot every_graph_renderer_config_value_draws_as_it_says fails on footer hint placement | broken | M4 |
| M4/04 review | 9 Minors in docs/reviews/m4-04-ops-remote-operations.md not yet triaged (fix builder takes what shares its files) | unverified | M4 |
| M4/03 | Remote footer hints appear only at ≥156 cols; at 80/120 the user never sees f/p/P in the footer | unverified (visual, user's eye) | M4 |
| M4/02 | Askpass `serve` waits on its handler with a 20 ms poll (pre-existing from group 02; review missed it). Rule 2 bans poll loops without the user's OK — replace with a channel/condvar, or the user OKs it | broken (rule) | M4 |
| M4/fix | Thread-teardown fix unverified on Linux; HungRemote failed-wake path untested | unverified | M8 |
