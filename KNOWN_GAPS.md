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
| Unconditional snapshot cost on high-frequency index ops (R2-3) | Every stage/unstage fires the full §6.3 snapshot (status walk, untracked hashing, commit object, backup ref, two journal lines), and §6.8's manual-only retention lets backup refs pile up at keystroke rate. Off-thread, so no UI block — but measure on the heavy fixture; if staging lags on untracked-heavy repos, propose scoping (e.g. skip untracked hashing for ops that provably can't touch untracked files) as an ADR, never silently | M3 (m2-details-and-diffs/22: no staging exists in M2 to measure) |

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

**What M0 has been seen doing** (swept at m0/12, revised at the M0 checkpoint,
2026-09-12). Everything M0 built was first exercised under a *bare pty* — a
`pty.fork` or `script -q /dev/null` with no emulator behind it — and by unit,
snapshot and property tests. That proved the bytes Graft writes and the
decisions it makes from scripted replies, but not a single pixel.

**That is no longer the whole story: the user ran M0 in a real terminal at the
M0 checkpoint and approved it.** Observed by eye, not inferred: the status
screen showed the repo, HEAD on `main`, the **pixel** tier via the Kitty
protocol, cell size **17x37**, a **dark** theme taken from the terminal's own
background, and **truecolor**. The capability probe was answered in **367 µs**
— three orders of magnitude inside the 200 ms budget. `q` exited clean. So
detection, the `detected` wording, the theme hint and the colour depth are all
now confirmed against a real emulator, and the two rows that used to sit here
saying otherwise are gone.

Still unseen: any emulator other than the user's daily one, tmux, SSH, and the
16-colour and ASCII fallback paths — nobody has watched those degrade. The
six DESIGN.md divergences were **confirmed by the user at the checkpoint**:
keep the code, amend the document, as recorded in ADR-0014.

| Gap | What's needed | Expiry |
|---|---|---|
| **No real terminal has been seen *answering* the capability probe.** m0/04's evidence is: 46 unit tests against scripted replies, plus one run in a bare pty (`script -q /dev/null`) which proved the six queries go out in the right order with `DA1` last, and that a silent terminal times out at 201 ms against the 200 ms budget and falls back per query. What is **not** verified: any real emulator's replies, the tier they produce, `TIOCGWINSZ`-derived cell size, `OSC 11` theme, tmux passthrough, and SSH. `cargo run -p graft --example capability_probe` prints the full report; run it in Ghostty, iTerm2, Kitty, WezTerm, Terminal.app, the Zed and VS Code terminals, inside tmux >= 3.3 with `allow-passthrough on`, inside tmux 3.2, and over SSH | The manual runs above, at the M0 checkpoint. Any terminal that answers differently from the fixtures is a bug in the parser or the tier table, not a gap to live with | **M0 review** |
| **Sanitising does not cover look-alikes, width, or the invisibles that are not `Cf`.** `term::sanitize` neutralises controls, bidi and Unicode's whole format category. Width is handled at draw time since m1-the-graph/08: `ui::fit::fit` lays text into an exact cell count, never splits a wide character, and keeps at most two combining marks per base. Still passing through `sanitize`: homoglyphs (Cyrillic `а` for Latin `a`), East Asian *ambiguous*-width characters (which `fit` counts as one cell; a CJK-locale terminal may draw two), and the invisible characters that are *not* `Cf` — the variation selectors U+FE00–U+FE0F and U+E0100–U+E01EF (the current hidden-payload channel) and the Hangul fillers. None of them can drive the terminal; replacing the variation selectors would corrupt ordinary emoji and CJK text, which is why they are kept. The `Cf` ranges are also spelled out as literals, pinned to Unicode 16.0, because nothing in the dependency set answers character-property questions: a Unicode revision that adds a format character is not picked up until someone refreshes the list, and no test would notice | Whether Graft ever flags confusables or a variation-selector run in a ref name is a policy call with real false positives — needs the user's decision, not a builder's | M1 checkpoint — open question, see m1-the-graph/28's report; not decided by this ticket |
| **The lane palette is argued, not seen.** The twelve hues were selected numerically — max-min worst-case OKLab distance under normal vision plus Machado 2009 protan/deutan/tritan simulation, floors 0.18 adjacent / 0.055 any pair — and checked by eye only as flat swatches in an image, never as thin graph lines in a terminal, and never by anyone with a colour-vision deficiency | Re-owned: m1/13 prepared the commands (ticket 13, Notes) but only the user can look. Judge criss-cross and foxtrot in Zed and VS Code at truecolor, 16 colours and `NO_COLOR` | M1 checkpoint (user, by eye) |
| **Non-UTF-8 paths are typed for, not tested.** `RepoHandle`'s API is `Path`/`PathBuf` end to end and never converts to `String`, so a repository at a path that is not valid UTF-8 should open — but no test proves it. APFS and HFS+ reject such filenames, so the fixture cannot be created on the machine m0/08 was built on (PROMPT.md §6.3) | A heavy-tier test on Linux (ext4/tmpfs accept arbitrary bytes) that opens a repository under a non-UTF-8 path and checks the reported workdir round-trips byte for byte. Same for a branch name that is not UTF-8, which `RefName` already stores as bytes | M1 heavy tier |
| **M0's screen has only been seen in a synthetic pty.** m0/11 drove the real binary through a `pty.fork` at 100x30, and m0/12 re-walked the README's demo steps the same way: `graft .` shows the workdir, `HEAD main`, the tier and theme lines and exits 0 on `q` with the alternate screen left and every mode popped; `graft /tmp` exits 1 with the line above; `graft --help` prints usage and exits 0; `SIGINT` kills it by signal with the terminal restored; under `NO_COLOR=1` the only SGR left is the reset (`ESC[m`, `ESC[0m`) with no colour parameters at all, and the theme line reads `no colour (NO_COLOR)`. Nothing answered the probe in either pty, so the tier shown was the silent-terminal fallback and no real emulator's rendering of this screen has been seen | The M0 checkpoint: run it in Ghostty, iTerm2 and the Zed/VS Code terminals and look at it | **M0 review** |
| **A panicking job's crash report is unproven on the real hook.** The suppression path is covered from both ends — `catching` marks the thread (`crash` unit tests) and a panicking job becomes `Event::JobFailed(_, Panicked { .. })` while the pool keeps working (`jobs` tests) — but no test installs Graft's real panic hook, so the hook's early return (write the report, do *not* restore the terminal, hand the path back) has been reviewed and never executed. Installing it would set a process-wide hook and write into `$XDG_STATE_HOME` for every other test in the binary | The M0 review: panic a job by hand in a real terminal, check the screen survives and the status line names a report file that exists | **M0 review** |
| **`graft-ops` is empty.** A crate doc comment and nothing else: no types, no functions, no tests. It exists so ARCHITECTURE.md's crate map and m0/02's dependency-edge test are real from the start. (`graft-graph` was in this row too until m1-the-graph/05 filled it with the layout engine; the renderers are still to come) | `ops` fills up in M3 (write operations and the undo journal). Until then, a green `cargo test` on the workspace says nothing about it | M3 |
| **No budget in the spec has ever been measured.** Partly closed by m1-the-graph/02 — this row used to read "only the fast test tier exists": the heavy tier now exists, with the 100k-commit fixture, the `getrusage` measurement and `proptest` at 1,000 cases, and `TESTING.md` documents all three tiers. What is still missing is the part that *asserts*: no performance budget and no 300 MB memory gate is checked by any test, and there are no `criterion` benchmarks. ~~`cargo-fuzz` targets~~ **closed by `m2-details-and-diffs/02` (`diff-parser`, `Status: done`):** `fuzz/fuzz_targets/parse_unified.rs` and `parse_file_list.rs` fuzz `graft-git`'s `parse_unified`/`parse_file_list`. There is also no CI: PROMPT.md §7 item 6 makes that conditional on the project having a GitHub repo, which it does not | The budget assertions are M1 tickets 27 (memory, first paint) and 30 (frame budgets, benches) | M1 (budgets) / not planned (CI) |
| **Sanitising strips the emoji joiner (U+200D), so a multi-part emoji breaks into its pieces** in a commit subject or branch name. A family or a profession emoji renders as two or three separate glyphs. Direct consequence of taking all of Unicode `Cf` (ADR-0014), which the user confirmed — the joiner is a format character, and telling it apart from a hostile one means special-casing emoji sequences | Raised and accepted by the user at the M0 checkpoint, 2026-09-12: cosmetic, and the safety trade is the right way round. **Revisit only if it annoys him in daily use** — not scheduled work | Not planned; revisit on user's say-so |


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
| **git/git has less memory headroom than the numbers suggest.** The gate stays `getrusage` peak RSS (user, 2026-09-13, ADR-0021). git/git peaks at 219–226 MiB RSS against the 300 MB budget (m1/27d), and the 100k synthetic fixtures (157–179 MiB) don't show what git/git does: its commit objects, ref layout and tree sizes differ. Nothing in the heavy tier measures git/git itself (see 27d's guard gap), so a regression that costs git/git 75 MiB could pass every test | A heavy-tier guard on a git/git-shaped fixture, or a review-time git/git RSS run with its number reported | M7 (perf hardening): git/git headroom needs a real-repo-shaped guard before ship, not another M1 pass |
| **The heavy tier cannot see a git/git-sized memory regression.** m1/27d added `Scale::with_realistic_commits()` (commit objects sized after git/git) and budget tests on it, but reverting C+D moves the 100k fixtures by only 23 MiB (small) / 27 MiB (realistic), against 70 MiB on git/git, and all stay near 130–160 MiB, far under 300. Message size is not the main multiplier. Suspect lane width: git/git's `seen` merges fan out to ~28 lanes and `Row.lanes` was the largest holder in 27b's profile, while the fixture keeps about 3 | A fixture shape with wide lane fan-out, then decide whether the guard should be a per-fixture ceiling nearer the measured peak | M7 (perf hardening): pairs with :152's guard, same fixture-shape fix |
| **verdict: 56d.** The 300 MB budget at a million commits, re-run m1/56c (2026-09-14, release, 3 runs each, `docs/shots/m1-memory-curve.txt`): **1M with a commit-graph (small) peak RSS 174.3/173.5/174.3 MiB; realistic-commit cg 183.7/182.6/183.2 MiB — both under the 300 MB gate.** **1M without a commit-graph: reported, not gated (ADR-0026) — peak RSS (small) 445.5/445.2/444.7 MiB, footprint 155.6/155.3/154.8 MiB; realistic peak RSS 507.5/508.5/508.8 MiB, footprint 156.7/157.6/157.9 MiB** (footprint slope ~138–140 B/commit, live heap slope 76 B/commit — row below, m1-the-graph/59b). Pack on disk: small 297 MiB, realistic 358 MiB. Was 881–949 MiB before ADR-0008's commit-graph path (m1/46). **Verdict (m1-the-graph/56d, 2026-09-14): with a commit-graph the gate passes (174–184 MiB against 300); without one the numbers say what ADR-0026 said they would — RSS − footprint = 290 MiB (small) / 351 MiB (realistic), i.e. the pack on disk, and the same repository after `B` is 173 MiB, gate pass. Reported, not a miss.** | The offer is built (m1/54). The M1 review file carries this row's table (ADR-0026 condition 1) | Stands while the user declines the offer on a given repository; closes for that repository when its commit-graph is written |

**Reading history** (m1-the-graph/04, 2026-09-12). This ticket closed three M0
rows, now removed from the table above: the repository handle not outliving its
job (`app::repos::RepoStore`), `gix` being compiled with almost no features
(measured, see `graft-git/Cargo.toml`), and nothing ever cancelling a job
(`Esc`, with a test that presses the key).

| Gap | What is missing | Closed by |
|---|---|---|
| **The walk has never been run against a big repository.** Everything measured so far is the synthetic 20,000-commit fixture and the corpus. `git/git` (85,649 commits, 1,017 refs) is cloned and still unopened, and no budget is asserted anywhere | Ticket 27's budget assertions, and ticket 09's first real graph on screen | M1 tickets 09, 27 |
| **Node glyphs ignore `ui.icons`.** Ticket 11 asked for node glyphs to honour it too. Not done, deliberately: Nerd Font's commit glyphs (`nf-oct-git_commit` and kin) are private-use code points that many fonts draw wider than one cell, which would break lane alignment, and the plain `● ◉ ◆ ◇ ■` shapes already read the same in any font. Recommend leaving node glyphs icon-set independent; needs the user's OK to drop from scope | User decision at the M1 checkpoint | M8 (polish and ship): icon-set consistency belongs with the UI polish sweep |
| **Only the newest stash entry has a badge.** `refs/stash` is one ref; older entries live only in its reflog, and nothing reads the reflog yet. `stash@{1}` and older draw as ordinary rows with no badge | Reading the stash reflog (sidebar STASHES) | M3 (m2-details-and-diffs/22: no stash list exists in M2) |
| **Ahead/behind trusts commit dates.** `graft_git::history::ahead_behind` walks newest-first by committer date and stops once everything queued is shared, as git does. Badly skewed clocks can miscount by the skewed commits; it never uses commit-graph generation numbers | Generation-number ordering when a commit-graph exists (ticket 30's scale work) | M1 ticket 30 |
| **The ref-states fixture's `HEAD` is not where its builder says.** `shapes/refs.rs::ref_states` ends with `checkout --orphan fresh` (an unborn `HEAD`), but the built fixture under `target/fixtures/` has `HEAD` → `refs/heads/main`. Found by m1-11's tests; nothing depends on it now, but a test for an unborn `HEAD` beside other branches would silently be testing the wrong thing | Check the fixture build's final steps / cache key | M8 (polish and ship): cheap fixture-build fix, bundle with the pre-ship test-hygiene sweep |
| **50k refs slow the graph's first paint in debug builds.** On the 100k fixture with 50k real refs, 30d's continuation fixed the main cause (`commit_tips` read every ref's commit object: now one header check per distinct target) without behaviour change: first paint 160 → 57 ms release (budget 150 met), 800 → 494 ms debug (still over). The debug remainder was three passes over 50k packed refs in unoptimised gix. **The "shared ref read" decision is made and built (m1-the-graph/55):** one `RefSnapshot` serves `Branches::resolve_from`, seeds, walk tips and `refs::read`. Debug first paint with 50k refs **not re-measured** since; release heavy tests (`many_refs::*`) pass. | Re-measure debug, or judge the budget in release only | **Won't fix (user, 2026-09-17):** release passes and only release timings gate (ADR-0021); a gap only in an unshipped build isn't one |
| **A history walk holds a job worker for as long as it is on screen.** It waits on its request channel between requests (`jobs::read_history`); the pool is at most 4 workers (`MAX_WORKERS`), so one is gone for the run. Cancel/`Ctrl+R`/shutdown drop the sender and end it | Accept, or a dedicated history thread outside the pool if a later job needs all four | Revisit at M6: conflict/concurrency work is next to stress the job pool |
| **Layout advances only as far as the screen asks.** DESIGN.md §3.6 says later `advance` calls run in the background between frames; m1/09a lays out on demand, so a `G` is cold until it has run once. m1/27 measured: first paint 13–113 ms on all four configurations, `G` on git/git 0.52 s (commit-graph) / 1.3 s (none) as a cancellable job, so left on demand | m1/30 part 1: every Unicode `PageDown` crossing a 500-row chunk waited 32–41 ms (debug). **30c part a removed the chunk-edge spikes without it**: a request now lays out only the rows it names (a real-repo chunk was 26–33 ms debug). Background advance still not built; the frontier now sits at the last request's end, so two keys faster than an answer stop the cursor at the last laid-out row, and release still shows one 17–18 ms key (key 37, a `j`, not a chunk edge; cause unknown) | **M7 ticket (user, 2026-09-17):** background advance while idle, **and find the cause of the unexplained 17–18 ms release key** (an open question, not a limitation). M7 design note must carry both |

**Layout engine** (m1-the-graph/05, 2026-09-12).

| Gap | What is missing | Closed by |
|---|---|---|
| **Neighbouring lanes can share a hue.** Lane colour is `BranchIdentity::slot(12)` — a hash, as PROMPT.md §1.2 asks — so two adjacent lanes collide one time in twelve, and nothing in a stable, per-branch rule can prevent it. The corpus shows it twice: feature-branches, where `main`, `feature/two` and `feature/three` all land on `lane-3` (checked: the slot spread over 1,200 names is uniform — this is chance), and foxtrot-merge, where the merge lane recovers the name `main` and so is the *same* identity as lane 0 beside it. Shape and position still separate them; colour does not | **Partly closed by m1/13:** at 16 colours and `NO_COLOR` the dashed odd columns separate them (foxtrot's lanes 0/1 at 16 colours are the snapshot). At truecolor and 256 lanes stay solid, so the one-in-twelve hash collision is still told apart by position only. Switching dashes on at every depth is one predicate (`dashes_odd_columns`) if the user's by-eye pass wants it | M1 checkpoint (user, by eye) |
| **A folded merge looks like a plain commit.** m1/15: once folded, a merge has one parent drawn, so its node is the ordinary `●` and nothing on its row says a branch is folded into it; only the header's "N merges folded" tells | A fold glyph or suffix on the folded row (DESIGN.md §3.6 names none) | M8 (polish and ship): fold glyph is a visual-polish addition |
| **Switching first-parent view keeps the selection only when it is near the top.** m1/15 reset `1` to the top; m1/16 re-finds the selected commit in the rows the other view sends back (the first two screens) and puts the cursor on it. A commit further down, or one the other view leaves out (a side-branch commit in first-parent view), still lands on the top row | The worker resolves the anchor with `StreamingLayout::row_of` in the other view — or its nearest first-parent ancestor — laying out only as far as it must | M1, after ticket 16 |
| **`Esc` stops a topological sort on screen, not on the worker.** `Walk::full_topological`'s constructor reads the whole range without polling stop, so after `Esc` the header says `order stays provisional` at once but the job thread keeps reading until construction ends; its order is then dropped | Poll stop inside `full_topological`'s read (ticket 27 makes it cheaper anyway) | M1 ticket 27 |
| **A long repository path truncates the order notice off the header.** Seen capturing m1/18 at 100 columns: `graft <long fixture path>` cut the header at `15 comm…`, so `date order, sorting…` never showed; through a short path it does. The header truncates from the right, and the notice sits after the path, count and filters | Give the header a truncation priority (shorten the path first), per DESIGN.md's header rules | M8 (polish and ship): header truncation priority is a layout-polish fix |
| **`Esc` during a sort has not been seen in a pty.** On the corpus the sort lands in milliseconds, before a key can arrive, so `order stays provisional (date)` is proved only by the `update` test (`esc_while_sorting_keeps_the_provisional_order_and_says_so`) | Capture it on a history big enough for the sort to take seconds (heavy tier or ticket 27's measurement) | M1 ticket 27 |
| **The walk answers nothing while a promotion lays out to the anchor.** `Views::promote` advances the new layout to the selected commit before swapping; scrolling during that waits. A selection deep in a large history pays a long advance; a selection the view does not show (a side commit in first-parent view) walks to the end | Bound the advance, or keep answering from the provisional layout until the promoted one reaches the anchor | M7 (perf hardening): promotion responsiveness fits the hardening pass |
| **The Unicode tier's focus and ancestry highlight are bold/dim only.** m1/16: no low-saturation lane colours (DESIGN.md §5.2 asks for a "low-saturation, dimmed render"); terminals that ignore SGR 2 (dim) show no fade at all. Snapshots record the modifiers, not how a terminal draws them. (The pixel tier closed its half in m1/42: emphasis there is alpha only — `OFF_THREAD_ALPHA`/`FADED_ALPHA` — never a wider stroke, ADR-0024.) | Muted per-lane colour tokens in the Unicode theme, judged by eye | M8 (polish and ship): theme/colour work belongs with the visual polish sweep |
| **The fold, first-parent and stash keys are not in the footer.** m1/15: `Space`, `1` and `H` are in DESIGN.md §2.3 but the 80-column footer has no room for three more hints. M2 adds more keys the footer can't fit (`x`, `L`, `M`, `O`, `<`/`>` at < 100 columns); until the overlay exists they are learned from README/TESTING.md, and no footer shows `? help` (design note M2, spec fix 2026-09-18) | The `?` help overlay (DESIGN.md §1.16) lists them. Built with M8's keymap table (PROMPT.md §8 M8 "keybinding help and remapping"): §1.16 is that table filtered to the focused pane, so an earlier hand-typed list would be rewritten | **M8** (re-owned 2026-09-18; was "M1, with the help overlay") |
| **Compaction has no corpus shape and no snapshot.** m1/14 tests fold over `graft-ui::test_support::wide_history` (synthetic, 12 topics) and a scripted repo (`docs/shots/14-many-lanes*.txt`); `graft-fixtures` has no many-lanes shape, so the corpus snapshot suites never see a `+N` | A `ManyLanes` corpus shape and its snapshots at 256/16/NO_COLOR | M8 (polish and ship): corpus-shape coverage fits the pre-ship fixture sweep |
| **A window whose lane-0-and-noded columns alone exceed `max_lanes` still draws wider than the limit (ADR-0025's residual).** `Fold::plan` never folds lane 0 or a column holding a node in the window — the one class that may still exceed the budget, since a row without its node is not a graph. A window packed with enough simultaneous merges can still overflow the configured cap even though every priority class beyond that is empty | A rail-node design (draw a merged-in node ON the rail itself, ADR-0022's "what would reopen this") would close it; not attempted this ticket | Not planned |
| **Clicking the rail does not expand lanes; `+` does.** DESIGN.md §3.5 names a click on the marker; ADR-0022 moved the count to the header and kept only the key | Not planned for M1 | not planned |
| **Expanding (`+`) lifts the cap entirely, so a wide merge train can draw past the terminal's width.** `expanded` plans with `max_lanes = usize::MAX`: every real column draws, whatever the screen holds. Deliberate — expand means "show me everything" — but nothing wraps or scrolls horizontally when the result is wider than the terminal | Accept for M1; a horizontal scroll or a re-cap while expanded, if the user's by-eye pass finds it needed | Not planned |
| **An inclusion change re-reads history from the top of the walk to the changed commit.** `Included::reinclude` rebuilds its frontier by re-reading every walked commit before the change (batches of 1,024), and `Included` keeps one `Oid` per walked commit — a second history-sized list beside `EmittedOrder`. Unmeasured on git/git; folding a merge 500k rows down re-reads 500k commits off-thread | Measure with ticket 27's numbers; if slow, checkpoint the frontier beside the layout's checkpoints | M1 ticket 27 |
| **A chunk cut back by `truncate` loses its eviction fingerprint.** The fingerprint covered the longer chunk and cannot check the shorter one, so a later rebuild of that one chunk goes unchecked once | Accept, or re-fingerprint the kept prefix when the chunk is held | M7 (perf hardening): cache-correctness fix fits the hardening pass |
| **Elbows and junctions have no dashed forms.** `╭ ╮ ╯ ╰ ├ ┤ ┬ ┴ ┼` stay solid on a dashed lane, so a dashed lane's first and last cells read solid. Unicode has no dashed corners | Accept, unless the user's by-eye pass finds a dashed lane ambiguous at its ends | M1 checkpoint (user, by eye) |
| **HEAD on a merge draws as HEAD only.** `NodeKind::Head` (`◆`) wins over `Merge` (`◉`), so a merge you are sitting on does not show it is one. Visible in criss-cross-merge and foxtrot-merge | A combined glyph, or leave it to ticket 10's row text/badges | M1 ticket 11 |
| **An octopus merge subject is not parsed.** `Merge branches 'a', 'b' and 'c'` matches none of DESIGN.md §3.4's four shapes, so each extra parent's lane falls back to a short-SHA identity (see the `octopus-merge` snapshot). The names are right there, in parent order | Extend `recover_branch_name` to return several names and let `open_merge_lane` take the n-th; not scheduled — octopus merges are rare in daily work | Not planned |
| **Column 0 stays empty when there are integration branches but no default branch.** `Seeds { default_branch: None, integration_branches: [..] }` reserves columns 1.. and never allocates 0, so the graph is drawn one column to the right. A repository with `develop` but no `main`/`master` and no `graph.default_branch` override would hit it. Not a correctness problem — every invariant holds — and `Branches::resolve` picks a default whenever any local branch exists, so the app cannot produce these seeds today | Leave it; revisit if a seed builder ever emits integration branches without a default | Not planned |
| **The snapshots are in commit-date order, not a commit-graph topological walk.** The header of every snapshot records `requested Topological, in force CommitDate (provisional)`; the corpus has no commit-graph file, and for it date order is a valid topological order (see m1-the-graph/04's row above), so no snapshot shows a parent arriving before its child. The already-drawn-parent path is covered by a directed unit test only | A corpus shape with a commit-graph file and out-of-order dates, whenever m1-the-graph/04's row is closed | M8 (polish and ship): pairs with :200's corpus-coverage sweep |

**Streaming layout** (m1-the-graph/06, 2026-09-12). This ticket closed
KNOWN_GAPS R2-1 (above) and the engine half of two m1/04 rows (rewritten in
place).

| Gap | What is missing | Closed by |
|---|---|---|
| **`rows()` clones every row it returns.** A viewport's worth of `Row`s — each carrying a `Commit` with its message and identities — is copied out of the cache on every call. Cheap at 50 rows a frame; not free | Borrowed slices per chunk, or an iterator, once tickets 08 and 14 know what the renderers need | M1 ticket 08 |
| **`Walk::full_topological` reads the whole range before it emits.** Measured: 200 reads before the first commit on the 200-commit linear fixture. That is the design (ADR-0008 — a background job), but on git/git it is 1.2M object reads per open until the commit-graph offer (R2-4) is accepted, and nothing caches the order between runs | The offer is built (m1/54); the residual is what remains when the user declines it (`X`) or has no git ≥ 2.38 | Not planned (m2-details-and-diffs/22): the offer is built and the residual is the user's own decline, not an unbuilt feature |

**History streams into the layout** (m1-the-graph/07, 2026-09-13).

| Gap | What would close it | Owner |
|---|---|---|
| **`AppState` keeps every row it is sent.** `History::chunks` is an append-only list of `Arc<[Row]>`, so the UI side grows with history — the thing DESIGN.md §3.6 says the row cache must never be. The layout's own cache evicts on the worker, but the copies already sent stay. Fine on the corpus; not on git/git. Same order of memory as the one-lump `Arc<[Commit]>` it replaces | The graph screen asks the layout for its viewport's rows instead of keeping all of them — a long-lived layout the UI can query between frames (tickets 08/14), measured in ticket 27 | M1 tickets 08, 27 |
| **Rows laid out after `Esc` inside the interrupted chunk are dropped, not shown.** `lay_out` stops reporting once `stop` is set, and `update` ignores any chunk after `Esc`, so the count freezes at the last whole chunk that landed — by design, but the layout on the worker has laid out a few more rows nobody sees | Nothing needed unless resume-after-cancel is wanted | — |

## Scheduled interim states

These are *temporary*. Each one is a real gap between the milestone that creates
it and the milestone that closes it, and each must be visible in the UI while it
exists.

| Gap | Created by | Expiry |
|---|---|---|
| **Single-line stage and discard are refused on a hunk that ends a file with no trailing newline.** `PatchError::NoNewlineBoundary`. The whole hunk still stages and discards, byte-proved both directions. This is a deliberate refusal, not a silent failure: the alternative found in the group 01 review was `git apply --cached` exiting 0 and fusing two lines into one, i.e. wrong content in the index with no error. **Not broken — a narrowed feature with a visible message** | `m3-working-tree-undo-and-stash/01` review fix (`b47ae51`) | M7, if real use hits it; `docs/field-notes.md` decides |
| **UNVERIFIED: the heavy tier's `many_refs::reading_tips_from_50k_real_refs_is_within_budget` fails in debug at 275 ms against a 200 ms guard.** Deterministic, predates M3 group 01 and is not its code. The release perf run is clean (13/13 in budget). Either the guard is a debug-inappropriate number or there is a real regression the release run is too fast to show — nothing has told them apart | pre-existing, surfaced by the M3 group 01 review | The M3 review's perf run: decide guard-vs-regression with numbers, never by relaxing the guard |
| **No in-app conflict resolver.** M5 can produce conflicts through merge, rebase and cherry-pick before M6 builds the three-pane resolver. M5 must detect the conflicted state, list the conflicting files, offer the user's configured `git mergetool` or `$EDITOR`, and keep the operation banner with Continue / Skip / Abort available. M5 must not claim conflict resolution works | M5 | **M6** |

---

*Builders and reviewers: add rows here the moment something is stubbed, naming
the milestone and ticket it came from. An entry here is not a failure; a missing
entry is (CLAUDE.md).*

## Input mux (m1-the-graph/20)

| Gap | What would close it | When |
|---|---|---|
| **A reply split by more than 50 ms is misread as keys.** `SEQUENCE_TIMEOUT` (50 ms) is one constant: over a slow SSH link a reply whose bytes arrive further apart than that is flushed as `Alt+[` and characters, and the probe sees no reply. Not observed; local splits land within a millisecond | Hold an unfinished reply-shaped sequence (`ESC [ ?`, `ESC ]`, `ESC _`, `ESC P`) up to the open waiter's deadline instead of 50 ms, measured over a real SSH session. **Ticket 21 did not fix this and adds exposure:** resize and focus re-queries go through the same decoder mid-session, so a split reply is delivered to `update` as keys (e.g. `Alt+[`, then `6;40;20t` typed into the graph) while the user is working, not just during startup | M1 heavy tier |
| **A re-query reply that arrives after its deadline can be taken for the next round's.** `detect::redetect` discards waiting bytes before each round, but a late reply landing *during* the next round is parsed as its answer (its fence ends the read early). Needs a reply later than the 200/500 ms budget; not observed | Tag rounds (e.g. a distinct `DECRQM` per round as the fence) if it is ever seen | when seen |
| **Key order under a pty is proven by unit test only.** The pty check proves arrival (F2 ×2 and ×3 during the probe land on the right screen) but its selection detector cannot see the cursor row, so `jjjk` during the probe was not verified on screen. Order is covered by `input::mux::tests` over a socket pair | A pty assertion on the cursor row (e.g. read the selected commit from the footer's `Enter` peek reveal) | M8 (polish and ship): pty coverage fits the pre-ship test-hygiene sweep |
| **Decoder covers xterm, SS3, kitty `CSI u`, SGR and X10 mouse.** Not covered: urxvt `1015` mouse (`CSI b;x;y M` without `<`), which crossterm's `EnableMouseCapture` also turns on; terminals that prefer SGR (`1006`) never send it | Decode `1015` if a supported terminal is found sending it | when seen |

## m1-the-graph/22 — rasteriser
- Node style is defined twice: `graft_ui::graph_column::NodeKind` (Unicode glyphs) and `graft_graph::raster::Ring` (pixels), same six variants. `ui` depends on `graph`, so `NodeKind` could become `Ring` (or map to it); left for the pixel-output wiring ticket (24) to decide.
- `CELLS_PER_LANE` (= 2) is declared in both `graft-ui` and `graft-graph::raster`; they must agree or pixels drift from text. Same fix: `ui` re-uses `graph`'s.
- Strip cache (m1/23) has no owner yet: nothing calls `StripCache::flush` on 21's resize / theme re-query / re-detection events. Wiring lands with the pixel output (24). Until then the key alone (theme colours, cell size, width in the layout hash) already keeps stale strips from being hit.
- The raster tests copy the 12 lane hues from `graft-ui/src/theme/lanes.rs` (`graph` cannot see `ui`). Palette colour does not affect any assertion except the snapshot's slot letters.
- Pixel shots are rendered at the 17×37 cell only; how they look at other cell sizes (e.g. 8×16 on a small font) is unjudged.
- Render time measured on the fast tier only: 32 rows × 5 lanes at 17×37, Apple M5, median 1.38 ms release / 13.4 ms debug. Wide strips (many lanes) unmeasured; the frame-budget ticket (30) owns it.


## m1-the-graph/24c — Kitty draw and replies
- **Pixel column width comes from the text tier's fold** (visible rows), while strips are rasterised through one fold across every strip drawn (`pixel_rows`). 24d places each strip across its own lane columns (`placed_columns`), so a narrower strip is no longer stretched; a strip *wider* than the column is still squeezed into it (`Place` has no column crop).
- **Replies are drained only on a wake.** A `q=1` error reply arriving with no key, resize or job result after it waits for the next wake to be shown and retransmitted. The mux has no wake for replies.
- **A panic leaves uploads in the terminal.** `free_all` runs on every loop exit (quit, signal, error), not from the panic hook.
- `STRIPS_KEPT` (16) is fixed for both the strip cache and uploads, not sized from the viewport (`StripCache::capacity_for`).

## m1-the-graph/24d — flush, F3, Ghostty check
- ~~Retransmit after a clear uses new ids~~ — moot since m1-the-graph/25: nothing calls `forget_uploads` any more (resize keeps uploads). Still true if a future caller does. Original note: **Retransmit after a clear uses new ids; the old ones are never freed.** `forget_uploads` drops Graft's record, so the next frame sends `i=1002…` and `free_all` on exit frees only those. Ghostty frees image data on the clear itself (ADR-0012), so nothing leaks there; a Kitty-protocol terminal that keeps data across `ESC[2J` holds the old uploads until it exits. Seen in `docs/shots/24d-kitty-check.esc.txt`.
- **The real-Ghostty check drives the encoder with synthetic strips** (`examples/kitty_check.rs`), not the app: it proves Ghostty accepts Graft's transmit/crop/clear/hide/free commands at `q=1` with zero errors. Whether the app's history strips look right is the user's by-eye check.

## m1-the-graph/25 — iTerm2, resize frames
- **iTerm2 tier not seen by eye.** Byte stream pinned by tests and a pty capture (`docs/shots/25-iterm2-capture.*`); no one has looked at Graft's iTerm2 output in a real iTerm2 yet. `doNotMoveCursor=1` is iTerm2 ≥3.5; older iTerm2 may move the cursor after a band (band on the last row could scroll).
- **Resize frame measured without the terminal.** Release, in-memory backend: Kitty resize frame 0.30 ms, iTerm2 0.25 ms (first frames 8.8 / 4.2 ms). Write back-pressure and the terminal's own work excluded; iTerm2 still writes every band's bytes per frame (~tens of KB). Real-terminal bench: M1 performance pass (30).
- **Real Ghostty window resize not driven.** The row-erase path was checked in Ghostty (`docs/shots/25-kitty-resize-check-report.txt`); an actual window resize was not. If Ghostty drops uploads on a real resize, `q=1` ENOENT replies retransmit on the next wake (self-healing, shows `gfx:` once).
- **iTerm2 erases the graph cells every frame** (`ECH` over last frame's rects, then re-send). Inside the synchronized update, so no flicker expected; unverified in iTerm2.
- **`Flush::Resize` is now unused by the app** (kept in `graft-graph` API). DESIGN.md §3.8 still lists resize as a wholesale flush; the key already covers it. Proposed change in 25's report.


## m1-the-graph/26 — search
- **Only the current hit is marked** (the selection moves to it); other hits' rows are not highlighted.
- **A hit hidden by the view** (first-parent or folded history) is counted but not selectable; the line says "(not in this view)".
- **Search walks history again on its own job** (a second walk alongside the history walk), not the rows the history job already read. 100k no-match scan measured under a pty; see `docs/shots/26-search-typing.txt`.
- **`n` before the first hit arrives does nothing** (no queued step).

## m1-the-graph/32 — wheel scroll
- **The wheel scrolls until only the oldest commit is left on screen** (`top` stops at the last row), where `G` fills a page. This was ticket 17's clamp and 32 keeps it. It's open whether the wheel should stop when the last row reaches the bottom.

## m1-the-graph/30 — frame budgets and scale (part 1)
- **Idle-wake keys can miss 16 ms at 50k refs (release).** The first key after 600 ms idle is printed, not asserted (ADR-0021, user's decision in m1-the-graph/30c): over the 7 counted release runs of m1-the-graph/30f, 14 of 140 idle-wake keys took over 16 ms, all at 50k refs, max 19.1 ms; the 100k-fixture ones stayed under. Cause named but not removed: the CPU's slow power state on the first burst after idle (ADR-0021 caveat). The scrolled keys pass 7 of 7 (worst 14.6 ms). Open: the user's by-eye check of the first key after a pause, at the gallery re-walk.
- **1M+ commits is extrapolated, never observed.** PROMPT.md §1.2's "60 fps on 1M+" has no measurement: no 1M repository was available and the heavy fixture is 100k. Nothing may report a 1M number until one is run by hand.
- **Frames measured without a terminal.** The in-memory backend leaves out pty write back-pressure and the terminal's own parse/paint; the 25-era Ghostty resize bench (32–41 ms) is the only real-terminal number.

- **Pixel strips rasterised again after eviction (m1-the-graph/30b):** cause 2 fixed (strips wait for the cursor's row). Still, 43 of 556 strip misses in the release 100k pixel frames run re-rasterise a strip whose inputs didn't change, because it was evicted (StripCache or Kitty unplaced capacity too small for a PageUp return). Each costs a ~434 KB retransmit. Not tuned. Also: on a page key whose cursor row isn't on hand, the graph shows blank for one frame. Not yet judged by eye.

## m1-the-graph/35 — graph gallery (part a: command + matrix)

`docs/shots/35-coverage-matrix.txt` (regenerate: `cargo run -p graft --example gallery -- --matrix`) reads file names only, so it shows what is *named*, not what a file happens to contain. Gaps it shows, with owners:

| Gap | What is missing | Owner |
|---|---|---|
| **Light theme: nothing named `light` for any repo.** `10-row-columns.txt` holds light captures, but its name does not say so, and no `.snap` renders the graph in light (`graph_column` asserts dark == light bytes and snapshots dark) | A light-theme snapshot per shape, or a name that says light on the captures that have it | M8 (polish and ship): gallery-naming fix fits the pre-ship sweep |
| **Pixel evidence for 3 shapes only** (foxtrot, criss-cross, octopus raster buffers and PNGs); 17 shapes and every real repo have none | Raster snapshots for the shapes whose picture differs (orphan root, cherry-pick twins, tags, unicode names) | M8 (polish and ship): pairs with :315/:317/:318's gallery-coverage sweep |
| **ASCII evidence for 3 shapes only**, none for the real repos | ASCII snapshots for the same set, or a statement that ASCII shares the Unicode layout so the 3 suffice | M8 (polish and ship): pairs with :316/:318's gallery-coverage sweep |
| **Sizes are not in snapshot names** (`?` in nearly every cell); only `graph_screen__feature-branches-{80x24,120x40,200x60}` names its size, so §7.2 item 3's three sizes are provably covered for one shape | Put the size in every screen snapshot's name, or add the three sizes for the merge shapes | M8 (polish and ship): pairs with :316/:317's gallery-coverage sweep |
| **`graft-self`, `synthetic-100k`, `git-git`: captures only** (`docs/shots/35-<repo>-{unicode,ascii}.txt`, 80x24, from 35c), no snapshot (by rule: no test may touch a heavy repo) | Nothing more planned; the captures are the evidence, judged by eye | — |
| **`non-utf8-ref-name`: no graph**, the APFS skip (ticket 01). The matrix counts its two 35c captures, but both show the gallery's `not available` page, not a graph | A Linux heavy-tier run | M1 heavy tier (existing row) |
| **The gallery restarts the run at every step**: terminal restored, alternate screen re-entered, capabilities re-probed, so each `]` blinks. (The key loss this row first reported — one stdin reader thread per run, ~every other `]` lost from stop 3 on — is fixed in 35c: `InputMux::on_stdin` is one reader per process, each run gets a fresh `Inputs`, and `run` calls `mux.hold()` when its loop ends so a key typed during the blink reaches the next stop.) | Split `run` into terminal setup and a per-repo loop | Not planned for M1; the gallery is a test harness |
| **Idle prefetch (30h) costs CPU after every pixel frame with idle time, mostly not rasterising**: release, per frame p50/p99/max: 100k fixture refs 0.8/4.6/6.7 ms; 50k refs 3.3/5.1/5.6 ms (429 of 697 frames over 1 ms; `target/30h/release-frames.txt`). Each look runs `update` twice and `pixel_rows` twice even when every strip is cached. The 50k-ref clone (~1 ms each) is fixed by m1-the-graph/69 (`Arc<RefBadges>`): per frame p50 now 0.6 ms at both ref counts, p99 4.9–6.6 ms, max 7.4–9.1 ms, about 303 of 697 frames over 1 ms (`target/m1-69/after-*.txt`). What remains is the what-ifs themselves | Skip the what-ifs when the window, cursor and cache are unchanged since the last prefetch | 30f seam 4 or M7 (perf hardening): idle-CPU cost fits the hardening pass |
| **The "not available" page understands only `]`, `[`, `q`**; an escape sequence is skipped, a lone `Esc` swallows the next key | Reuse the input mux instead of raw byte reads | Not planned |

## m1-the-graph/45 — `y` copies the SHA
- **Whether the clipboard got it is unknown to Graft.** OSC 52 has no reply; the notice says what was sent. Terminals with OSC 52 off (some VS Code/Zed settings, tmux without `allow-passthrough` or `set-clipboard`) silently drop it. Paste in Ghostty/Zed/VS Code/tmux is a user by-eye check, not yet done.
- **Inside tmux the request is wrapped in DCS passthrough, like images (ticket spec).** tmux's own `set-clipboard on` path (unwrapped OSC 52) is not used; with passthrough off the copy is dropped. Open whether to send unwrapped inside tmux instead.
- **`y` while a failure is on the status line** still sends the SHA but shows no notice: the failure is never replaced by a notice.

## m1-the-graph/49 — commit handles and the id table
- `graft::history::WalkSource::links` reads each commit whole to get its parents (`read_commit`). Nothing calls it on a hot path yet (`pull`/`read` still hand out whole commits); the hydrate-on-demand ticket should take parents from the walk, as `Walk::next_links` does.
- Heavy-tier tests for the table and `next_links` (5k pair; 100k overflow ≤ 60 B/commit) are written, not run by the builder; they run in the milestone review.

## No-commit-graph footprint slope at 1M (m1-the-graph/59b)

- **verdict: 56d.** After 59b's three fixes the no-cg footprint slope is **138–140 B/commit** (`budgets --gate`, 3 fresh runs: 1M 155.1–155.8 MiB, 100k 36.0–36.2 MiB; was ~176; m1-the-graph/56c's full curve re-run, `docs/shots/m1-memory-curve.txt`, 2026-09-14, confirms within rounding: 1M small footprint 154.8–155.6 MiB, 100k small 35.5–36.0 MiB). Live heap slope is the named floor, **76 B/commit** (`prototypes/heap-profile-59`, `target/59b/prof-{100k,1M}.txt`: 1M live-after-`g` 92.5 MiB, 100k 27.0 MiB; interner 58 + one order 8 + promoted 4 + rest); the other ~62 B/commit is freed transient memory macOS malloc keeps in the footprint.
- **Verdict (m1-the-graph/56d, 2026-09-14): live heap 76 B/commit ≤ 100 — pass; footprint 155–158 MiB at 1M reported beside its ~140 B/commit floor, under the 300 MB footprint line. The design holds: the one unavoidable table (the interner, 58 of the 76) is what ADR-0026 said would grow; the retained ~62 is allocator slack, not a second structure.** Open only as the four retained parts below; expiry as the last line says.
- **Bar re-derived (m1-the-graph/63, user decision 2026-09-14):** the single ≤ 100 B/commit bar is split — **live heap ≤ 100 B/commit, gated** (measured 76, under bar) and **footprint at 1M: reported, floor named** (~140, not gated further; ADR-0026 amended). This row's expiry tracks the live bar, not the footprint number against 100.
- Left, and why:
  - Interner `by_prefix` map doubling (~17 MiB chain at 1M): paging it means several prefix maps searched in turn — a new structure, not ADR-0026's one prefix map. Not built.
  - `Vec` → `Arc<[CommitHandle]>` copy in `graft::history::topological_order` (~4 MiB at 1M): building the `Arc` in place needs `unsafe` (`assume_init`) in a `forbid(unsafe_code)` crate, or `Views.promoted` changing shape across crates. Not built.
  - Swapped-out order at promotion (~9 MiB): ADR-0008 by design.
  - ~19 MiB retained in the date walk not attributed to a holder after the interner pages landed (prof: footprint − live 36 MiB after the walk, before and after 59b). Untraced.
- Expires: when the live heap slope regresses above 100 B/commit (gated at reviews), or the user decides to fund a structure change to cut the ~62 B/commit retained above the floor.

## m1-the-graph/56d — memory-curve verdict: what stays open (2026-09-14)

Every budget in 56's checklist passes on 56c's numbers (`docs/shots/m1-memory-curve.txt`, `target/56c/`). Three things are reported, not proven, and each names whose call it is.

- **Cold first paint at 1M is ~0.7–1.0 s (ADR-0027).** Run 1 of every 1M cell in 56c's re-run: no-cg 997–1,032 ms, cg 710–790 ms; runs 2–3 are 88–112 ms on every path, under 150. The budget is judged warm (ADR-0027, user decision). Evidence the cost is the page cache and not Graft: the offer copy, written seconds earlier, painted in 124 ms on its run 1; 55's just-built fixture painted in 134 ms. Not proven: only `sudo purge` empties the cache on macOS. **Expiry: the user runs one `budgets --gate` first paint straight after `sudo purge` on the 1M fixture at the M1 checkpoint (TESTING.md manual checklist), or accepts the warm convention as final.** User.
- **`examples/budgets`'s counters hold 4× the app path (harness only).** `order_bytes`, `cold_jump` and `far_jump` lay history out again through `direct_layout`: 100k cg 124 MiB process max RSS vs 33 MiB for the gated path, 1M cg 997 vs 240 (56 pass 1). `--gate` keeps them out of every gate and out of `scripts/memory-curve.sh`'s RSS/footprint cells; the counters run is a separate process. Related: `ru_maxrss` is a lifetime maximum, so any guard that measured two fixtures in one process read a false green in the wrong order — fixed by m1-the-graph/57 (guards measure each fixture in a fresh child). **Expiry: not planned** — the harness is not the app, and the gated path is what every number in this file reports.

## m1-the-graph/28 — bare `cargo run --` is ambiguous at the workspace root (2026-09-14)

`graft-fixtures` has carried its own `src/main.rs` (a corpus-report binary) since early M1, so `cargo run --` at the workspace root fails with "could not determine which binary to run" — it must be `cargo run -p graft --`. TESTING.md and `scripts/clone-git-git.sh`'s own printed "Open it with" line still show the bare form; `scripts/m1-checkpoint.sh` (this ticket) was written straight to `-p graft` and documents why in its header. Not fixed here: rewriting every existing doc/script mention, or adding `[workspace] default-members` to `Cargo.toml` to make the bare form work again (a workspace-wide default changes what `cargo build`/`cargo test` do everywhere they're invoked without `-p`, out of scope for a wrap ticket). Expiry: M2 (or whichever milestone next touches `clone-git-git.sh`/TESTING.md's copy-paste commands) — a small sweep fixing the remaining bare mentions, or the `default-members` decision, whichever the user prefers.

**Closed by m2-details-and-diffs/22 (docs-sweep).** Every bare `cargo run --` in `README.md` (114, 146–148), `TESTING.md` (323, 350, 438, 469, 505, 508, 512, 517, and the second invocation on the SSH line 522) and `scripts/clone-git-git.sh` (78) now reads `cargo run -p graft --`. Decision: the sweep, not `[workspace] default-members` — `default-members` would change what `cargo build`/`cargo test` do everywhere they're invoked without `-p`, and the fast tier relies on the whole workspace running. `CLAUDE.md` 176 (report-style example, not a command, and the user's file) and `PROGRESS.md` 38 (`--manifest-path`, unambiguous) were left alone; `scripts/m1-checkpoint.sh` was already `-p`.

## m2-details-and-diffs/01 — git spawns outside the runner (2026-09-17)

- ~~`graft-git::version::system_git` and `graft-git::commit_graph::write` build their own `Command::new("git")`.~~ **Closed by `m3-working-tree-undo-and-stash/01` (piece 1, `073a107`)**: both moved onto `run`, which also gained `GitInvocation::env`. Row moved to `docs/gaps/closed.md`.
- `contained_in` on a repository with no commits: the spec asked for `Ok` with empty lists, but with no commits there is no id to ask about, and an id naming no commit makes git exit 129 — returned as `Exited`, not hidden as "contained nowhere". Tested instead on `UnbornBranch` (HEAD unborn, other branches with commits): `Ok`. Callers only ask for a selected commit, which exists. Expiry: the detail-model ticket that calls `contained_in`, if it needs otherwise.

## m2-details-and-diffs/04 — Mistake Radar rule limits (2026-09-17)

- **A pushed pull-merge on `main` reads as accidental-pull (info), not foxtrot (warning), in a clone without a remote-tracking reflog.** The rule keeps `Merge branch 'main' of <url>` on the default chain as accidental-pull, because the merge was made on `main` itself and the fixture matrix needs it that way; once pushed, the *remote's* first-parent line is the rewritten one, and the only fact that shows it is `RepoFacts::previous_default_tips` (the `origin/main` reflog), which a fresh clone lacks. Expiry: `radar-context-and-markers` fills `previous_default_tips` from the reflog; a remote-default-chain fact (`refs/remotes/origin/main`'s first parents) would close the fresh-clone case and is a one-rule addition.
- **Rule A (a non-first parent already on the default chain) is unreachable through `git merge`.** Kept because the spec lists it and it is one comparison; only hand-built commits (`commit-tree`) produce it. Not a bug; noted so nobody looks for a fixture.
- **`NoUpstream` is fixture-covered only through `foxtrot-merge`'s `feature`; `UnpushedOnDefault` has no fixture at all.** `ref-states` gives every branch an upstream and its `ahead` branch is not the default, so `classify_refs` returns `[]` for it. Unit tests cover both paths. Expiry: the M2 acceptance ticket, if it wants a shape with `main` ahead of `origin/main`.

## m2-details-and-diffs/06 — detail model seams (2026-09-17)

- **No footer hint for `Enter` yet.** The peek hint (`⏎ full subject`) was removed with the peek; a `⏎ details` hint would advertise a pane that does not draw until `detail-pane-frame`. Expiry: `detail-pane-frame`.

## m2-details-and-diffs/06b — working-tree row reach (2026-09-17)

- **The uncommitted row appears only while `HEAD` is the top visible history row (m1-the-graph/12b); with a newer branch above `HEAD` the user scrolls `HEAD` to the top to reach it.** Owner: M3 working tree — it decides whether to pin the row above `HEAD` wherever `HEAD` sits.

## m2-details-and-diffs/10b — detail pane resize: pointer input and hints (2026-09-18)

- **No `?` help screen — deferred to M8 by decision (planner-fable, 2026-09-18; design note M2 "Spec fix (2026-09-18, `?` help")).** Confirmed (again): no `?`-key handler or help-overlay surface exists anywhere in the codebase. It was never in M2's design note; 10b's line naming it was a slip. DESIGN.md §1.16 defines the overlay as the §2 keymap table filtered to the focused pane, and that table is what M8's "keybinding help and remapping" (PROMPT.md §8) builds — a hand-typed M2 list would be rewritten there. Cost while open: `Space` `1` `H` `x` `w W v [ ]` `<>` `L` `M` `O` are learned from README/TESTING.md; `?` stays unbound and no footer shows `? help`. For M8's builder: `crates/graft/src/update/`'s purity guard rejects any mention of ratatui/`graft_ui` types in that module, doc comments included, so the overlay's geometry and widget live in `graft-ui` and `update` keeps only a shown/hidden flag and the focused-pane id. Pairs with the M1 row "The fold, first-parent and stash keys are not in the footer". Expiry: **M8**.
- **Map-view no-op guard for resize not added.** `Screen` has no `Map` variant until ticket 19 (per 10b's own "Out" note); `<`/`>` are already no-ops in `Tab`/hidden, so this closes automatically once `Map` exists and needs the same guard added explicitly then.
- **`docs/shots/m2-detail-keys-resize.txt` capture not regenerated.** The `<> resize` footer hint changes what's on screen while the pane is shown; the ticket asked for the capture to be refreshed but this session's budget ran out before a pty capture could be taken. Expiry: next detail-pane ticket, or a dedicated capture-refresh pass.

## m2-details-and-diffs/11 — compare two commits (2026-09-18)

- **A compare's pane header never shows file/line counts (`· N files +A −B`).** `crates/graft/src/view/detail.rs::compare_header` builds `Subject::Compare` only — its own doc comment already says "header-only per this ticket's scope (`compare-mode` fills in the rest)", a decision from an earlier ticket (`detail-model-and-jobs`), not this one. Ticket 11's design-note text illustrates the fuller header, but wiring `DetailState::files`' counts into `compare_header` is what's left. `docs/shots/m2-compare.txt`'s capture reflects the header as it actually renders today (`COMPARE <base> → <head>`, no counts). **No ticket named `compare-mode` exists in M2's ticket list — the reference is dangling; see the "M2 docs-sweep" gap row below for the real owner.**
- **Fixed in this ticket, not a gap, noted for the session log:** `crates/graft/src/view/detail.rs::detail_view` was reading `AppState::selected_spec()` for the pane header, which never names `Compare` — a pinned compare's header silently redrew as the cursor's plain commit every frame. Changed to read `AppState::detail.spec` (falling back to `selected_spec()` before anything has opened), since that field is what `DetailState::open`/`follow` actually set and is what the pane is showing. Pre-existing bug, invisible before this ticket because `detail.spec` and `selected_spec()` never disagreed until `Compare` could diverge from the cursor.

## m2-details-and-diffs/13b — radar wiring (2026-09-18)

- **Resolved in `m2-details-and-diffs/13c`.** `event_loop::startup::seed_config` now sets `AppState.radar.policy = radar.policy()` from `[radar]`'s startup values, the same place `[diff]` seeds the pane's diff options; `terminal_and_state` takes the `Radar` config and passes it through. Tested by `startup::tests::a_non_default_radar_setting_reaches_app_state_policy`: `[radar] foxtrot = off` measurably changes `AppState.radar.policy` (`severity(FoxtrotMerge)` becomes `None`, and the policy differs from the all-default one). `run_headless` still does not seed either `[diff]` or `[radar]` — pre-existing asymmetry, left alone rather than introducing new behaviour there.
- **`RefBadges::branches()`, `crates/graft/src/refs/mod.rs`, added this ticket though not in 13b's file list.** Needed so `view::radar::branch_facts` can resolve a branch's tip purely from `AppState.refs` — nothing else exposed a name→tip iterator. Not a scope creep in effect (the ticket's own item 3 needs `branch_facts`), just a file the spec didn't name.
- **`Task` lost its `Copy` derive** (`crates/graft/src/update/events.rs`) because `Task::RadarContext` carries a `RefName` (not `Copy`). `jobs/tests.rs`'s `next_started` helper now takes `&Task` instead of `Task` to keep clippy quiet about the now-more-expensive copy; the eight call sites were updated to pass `&Task::...`.

## M2 docs-sweep — new rows (m2-details-and-diffs/22, 2026-09-18)

Every M2 feature ticket is done (17, 19, 20, 21 and the rest); these are gaps
the design note's §8 coverage map marked "uncovered by design" or that this
sweep found while closing the rows above. Each is real product behaviour
today, not a stub.

| Gap | What's needed | Expiry |
|---|---|---|
| **PROMPT §4 image diffs in the pixel tier are not built.** M2 shows `binary · <old> → <new>` for images (the same binary-file line as any other binary), never a rendered before/after image in the Kitty/iTerm2 tier | A pixel-tier image-diff renderer; PROMPT §4's own scope | M8 polish |
| **Map view has no mouse interaction and no pixel-tier density curve.** `map_screen` draws text glyphs (`▁▂▃▄▅▆▇█`/ASCII ramp) at every tier, including Kitty (design note's Proposed changes); clicking a row or bucket does nothing — only `↑↓`/`Enter`/`Esc` | Mouse hit-testing on `MapScreen`'s rows/buckets; a pixel density curve for the pixel tier | M8 polish |
| **Overview strip defaults to off; no `[ui]` config key starts it on.** `AppState::strip_shown` starts `false` every session; `O` toggles it for that run only, and there is no `[ui].overview_strip` (or similar) key to make it start shown | An `[ui]` startup key read the same way `[diff]`/`[radar]` seed their state at startup | M8 polish |
| **Map view shows local branches only; a remote does not collapse into its upstream's row (DESIGN §5.1).** `build_map` walks one date range per local branch (`MapRequest`/`build_map`); a branch's remote-tracking ref never merges into its local row or appears on its own | Fold a branch's upstream into its row, or list untracked remotes separately, per DESIGN §5.1 "Shows" | M8 polish |
| **Map view's merge elbow marks only merges into the default branch, at the tip's own bucket.** `MapBranch::merged_into` is computed by `build_map` only against the default branch's first-parent chain; a merge of one feature branch into another draws no elbow | A cross-branch merge search (DESIGN §5.1's fuller "Shows"), a one walk-per-local-branch cost the design note weighed against and declined for M2 | Not planned unless asked |
| **The compare pane header never shows `· N files +A −B` (m2-details-and-diffs/11's gap row named a `compare-mode` ticket that does not exist in M2's list — dangling reference, corrected here).** `crates/graft/src/view/detail.rs::compare_header` builds `Subject::Compare` with base/head SHAs only; `DetailState::files`' counts are never read into it | Wire `DetailState::files`'s `FileListView` totals into `compare_header` | M8 polish |

## M2 acceptance walk (m2-details-and-diffs/23, 2026-09-18)

What the details gallery revealed, verified under a pty (captures
`docs/shots/m2-*`); owner **M2 review** unless a ticket is named.

| Gap | What's needed | Expiry |
|---|---|---|
| **No corpus commit changes two files**, so stop 2 (`Enter j`) shows `j` inert and no fixture exercises the file cursor. | A `two-file-commit` shape (or a second file on `feature-branches`' merges), then re-point stop 2. | M2 review |
| **The force-push probe (ticket 15) has no gallery stop and was never run against a real rewritten remote.** `docs/shots/m2-force-push.txt` is a TestBackend capture. | A fixture with a remote whose branch was rewritten (needs a bare remote in the corpus), and a stop. | M2 review |
| **The gallery banner is hidden at < 100 columns when the `Detail` tab is up**, and the Map view draws no banner at any size. The 80x24 captures name their stop only in their first line. | Draw the banner (or its title line) in the tab-strip and map layouts. | M2 review |
| **Coverage matrix gaps for the details rows:** no pixel-tier, no light-theme and no ASCII captures except stops 1, 6, 7, 15, 16 at 80x24; no 100x30 except those five. The pane's pixel/light rendering is judged only by eye. | Pixel captures need a Kitty-protocol pty harness (M1's raster shots came from the widget, not a pty). | M2 review |

- **Heap grows ~76 B per commit, against a 50 B/commit bar** (M2 review fix
  1a, 2026-09-18). `heavy::slopes_per_commit_with_a_commit_graph` fails, so
  the heavy tier still exits 101 even though it now completes in 91 s. Not
  caused by the C1 fix: 69 B/commit at 16f3ed1, 76 B/commit after. Peak RSS
  is 214 B/commit against 200. Numbers:
  `docs/perf/m2-review-fix-1a.txt`, `docs/perf/m2-review-fix-1a-baseline-slope.txt`.
  Needs its own ticket: find what holds per-commit state beyond the layout
  window.

### The Mistake Radar's force-push probe is no longer exhaustive

**Expiry: M7.** Behaviour change, user-registered 2026-09-18 (M2 review fix
round 1a, `32845d3`/`6437790`).

Force-push verdicts are computed only for branches seen in the visible window
this session, and at most 32 of them. A force-push on a branch you never
scrolled to goes **unflagged** — the Radar no longer covers every tracked
branch. This was the price of C1: unbounded per-branch probing made the heavy
tier time out at 1275 s (now 91 s) and stalled the release perf run.

The user asked for this to be visible here, not only in a commit message. M7
should decide whether an exhaustive background sweep earns its cost, or whether
32-per-session is the honest permanent answer — and, if it stays, whether the
UI should say so rather than letting silence read as "no force-push found".

## M2 review round 2: Minor findings left open (2026-09-19)

17 Minors from the round-2 review (`docs/reviews/m2-details-and-diffs.md`,
`## Round 2` → `### Minor`). Round 1e fixed the round's three Majors only.
Minor 13 (false "never writes/takes the index lock" doc comments on
`run.rs`/`diff/read.rs`) was fixed as a side effect of round 1e's Major-1 fix
— not listed below. The other 16 are still open. None of these duplicate the
two rows already filed under "Unverified" (the two colour fixes) or
housekeeping/02's two rows (`map.rs` `HashSet<Oid>`, the frame-gate max) —
cross-referenced below where they touch the same code.

### Mistake Radar: evil-merge probe and its text

| Finding | Broken/unverified | What a user notices | Expiry |
|---|---|---|---|
| Pane header reads `evil merge: clean (N files hand-resolved)` for a merge with unchecked files — `clean` is `EvilVerdict::Clean`'s word ("nothing wrong found"), not "not checked". `crates/graft-ui/src/detail_pane/header.rs:55` | Broken | A hand-resolved merge with real files skipped reads as reassuring, not "we didn't check this" | M8 polish |
| The both-sides-touched blind spot (a file both sides edited, resolved by hand, can hide a real evil edit inside the resolution) is disclosed only in the Explain paragraph and a doc comment, not in TESTING.md's radar section | Broken (docs) | Has to open Explain on the exact merge to learn the Radar can miss this kind of evil merge | M8 polish |
| `merge_base` (`crates/graft-ops/src/mistake_radar/probes.rs`) takes only the first line of `git merge-base`; criss-cross history can have several bases, so "changed by one side" is judged against a base the real (recursive/ort) merge didn't use | Unverified — not reproduced | On unusual branch topology only, a merge could be wrongly cleared or wrongly flagged evil; no case has shown this yet | M7 |
| Selecting the cursor's way through a merge-dense history spawns one full `git show --remerge-diff` per merge row, killed ≤50 ms later when the cursor moves on again; the 16-entry probe cache thrashes | Broken | Holding `j`/`k` down through a history like git/git's (a merge every few rows) causes brief hitches from throwaway git spawns | M7 |
| `update/radar.rs::force_push_candidates` walks the resident window and allocates a `Vec` on every `update`, including key repeat, even once the 32-probe session cap is already full | Broken | Small extra per-keystroke cost that keeps paying after force-push probing has already maxed out for the session | M7 |
| `RadarContext`'s `PartialEq`/`Eq` compares its tables by `Arc::ptr_eq`; two contexts with identical members but different table instances compare unequal, though `Eq` promises they'd be treated the same | Broken | Not directly visible; risks extra radar recompute when two logically-identical contexts are treated as different | M7 |
| `scripts/m2-fix-1c-evil-audit.sh` reimplements the evil-merge rule in shell; already drifted once (round 1c's rule vs round 1e's rename-aware fix) and nothing keeps the two in step | Broken | No user impact; the audit's numbers can go silently stale the next time the probe's rule changes | not planned |

### Overview strip

| Finding | Broken/unverified | What a user notices | Expiry |
|---|---|---|---|
| `view/strip.rs`'s search-hit `HashSet` is rebuilt every frame the strip is shown — M10's round-1a fix moved the per-row cost of the old scan to a per-frame allocation, it didn't remove it | Broken | On a repo where a search matches most of history, the overview strip can still cost noticeably every frame it's shown | M7 |

### Code health: visibility, file boundaries, docs (M8's architecture pass)

None of these are user-facing; `planner-fable`'s M8 pass already reads every
`MAP.md`/`ARCHITECTURE.md`, so it's the natural owner.

- `ramp_glyph` (`crates/graft-ui/src/map_screen/render.rs`) went `pub` → `pub(crate)` in round 1d's split, but `crates/graft-ui/MAP.md:151-152` still lists it as public and separately notes it's now `pub(crate)` — the two lines disagree. Broken.
- Round 1d's splits widened visibility instead of narrowing it: `detail_key`/`detail_event` and roughly 46 further items across `detail_pane/*` (17 in `line_render.rs`), `map/{walk,rows}.rs`, `unified/primitives.rs` and `update/detail/state.rs` are `pub(crate)` when only their parent module needs them (`pub(super)`, as `run/exec.rs` and `diff/read/raw.rs` already do). Broken.
- `update/detail/{state,ops}.rs`'s split cuts "the struct" from "its `impl`", not along a real boundary — `ops.rs` (395 lines) mixes file loading, the evil probe, focus/options and pane resizing. `detail_pane/line_render.rs` has the same shape (17 unrelated items in one file). Broken.
- `crates/graft-git/src/diff/unified.rs` is 417 non-test lines; round 1d's own note claimed every split file was under 400. Broken (contradicts the record that closed it).
- `detail_pane/side_by_side.rs`'s `diff_body_side_by_side` still takes two bare bools (`focused`, `word_tint`) instead of the `DiffFocus` struct that was introduced specifically so `diff_body` wouldn't need them. Broken (inconsistency).
- The `update/` purity guard (`crates/graft/src/update/tests.rs:1480-1557`): its file-count floor is `>= 7` against 20 real files (a broken directory walk could pass silently), its substring check misses `use std::{time, thread}` and `core::time`, and its own comment claiming "every other file is pure end to end" is false for six files that carry inline `#[cfg(test)]` modules. Broken — the safety net has real holes, though round 1d's own fix (making the scan recursive) is confirmed sound.
- `ARCHITECTURE.md` still doesn't document `git::run` (the typed runner every M2 git spawn goes through), `git::contained`, the `LOCAL_JOB_BIT` job-id split, or the `update/` purity guard as a stated rule. Broken (docs debt).

### Test suite: watch, not yet a failure

- Fast tier grew 21 s → 24.3 s across the fix rounds, and its single slowest test (`diff_fixture::parse_unified_matches_numstat_for_every_shape`) grew 6.42 s → 7.76 s. The reviewer called it still under budget. Unverified (a trend, not yet a breach) — watch it doesn't cross the laptop-safe budget as M3 adds its own tests. Expiry: M3.

---

## Unverified (shipped, nothing here can prove it)

### Two M2 colour fixes were never seen by anything
**Expiry: M3** (first milestone with a capture pipeline that could carry colour,
or the user's eye, whichever comes first). Registered 2026-09-18, ADR-0029.

The pty capture pipeline (`pty-gallery-m2.py`, pyte) records **character grids
with no colour attributes**. A colour-only change is therefore invisible to every
check this repo can run — captures included. Two M2 review fixes are in that
state:

- **`W` (word tint)** — review Major M1, fixed round 1b. Unit-tested; nothing
  rendered it.
- **The diff body drawn all `text-muted` unless the Diff region is focused** —
  review Major M2, fixed round 1b. Same.

Both pass their tests. Neither has been seen. If word-level diff tinting or the
unfocused diff body looks wrong in a real terminal, this is why — report it in
[`docs/field-notes.md`](docs/field-notes.md) rather than assuming it's new.

**Closing it needs one of:** a capture path that carries SGR attributes, or the
user looking at it once and saying.

---

### `map.rs` `build_default_chain` still holds `HashSet<Oid>`
**Expiry: M7.** Broken (mild). Found by housekeeping/02, 2026-09-18, not fixed —
out of that ticket's scope.

Same ADR-0026 breach the radar's default chain had: 33-byte Oids where a
`HandleSet` bit would do. **Transient and Map-view only**, so it never showed in
the heap-slope guard, which measures the steady state. It will cost on a large
repo the moment the Map view is opened.

**Fix:** the same shape as `92c35ac` — `HandleSet` plus the commit table.

### The frame gate asserts on the max of 420 keys, so one outlier fails it
**Expiry: M7.** Unverified-adjacent: the gate passes today, but it is fragile by
construction. Found by housekeeping/02, 2026-09-18.

A single slow key out of 420 fails the whole gate. Under load **both** a baseline
and a patched build spike over 9 ms — six interleaved A/B runs showed the tail is
machine noise, not the code under test. A gate that fails on the worst sample
will eventually fail for reasons nothing in the diff caused, which trains us to
re-run it until it passes — the exact habit that hides a real regression.

**Fix:** assert on a percentile (p99) with the max recorded, not asserted.
Pairs with M7's existing ticket for the unexplained 17–18 ms release key
(PROGRESS.md, user 2026-09-17) — same measurement path, same investigation.
