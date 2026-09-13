# Known gaps

Everything unfinished, stubbed or deliberately deferred. Nothing here may be
described as done, and nothing in the README, the first-run tour or a milestone
summary may imply any of it exists.

Every entry names an **expiry**: the milestone that closes it, or "not planned".

---

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
| 50k+ refs in the sidebar and ref badges (review N4) | PROMPT.md §6.3 requires handling repos with 50k+ refs; branch-tree filtering, badge lookup and ranking at that scale are undesigned. Validate in M1's performance pass, not assumed free | M1 ticket 30 |

## Design gaps (step 2 review round 2, 2026-09-12)

Named in `docs/reviews/step-2-design-round-2.md`. (R2-2, backup-ref naming,
is already fixed in DESIGN.md §6 and ADR-0010 — not a gap. R2-1, post-promotion
checkpoint state on the no-commit-graph path, was closed by m1-the-graph/06:
DESIGN.md §3.1/§3.2/§3.6 say what a checkpoint holds on each path, and the
size is measured — 1.6 KiB, the same on both.)

| Gap | What's needed | Expiry |
|---|---|---|
| Unconditional snapshot cost on high-frequency index ops (R2-3) | Every stage/unstage fires the full §6.3 snapshot (status walk, untracked hashing, commit object, backup ref, two journal lines), and §6.8's manual-only retention lets backup refs pile up at keystroke rate. Off-thread, so no UI block — but measure on the heavy fixture in M2; if staging lags on untracked-heavy repos, propose scoping (e.g. skip untracked hashing for ops that provably can't touch untracked files) as an ADR, never silently | M2 |
| Commit-graph offer plumbing unspecified (R2-4) | §3.2's "declined, remembered" flag has no stated home — put it in a file under `git rev-parse --git-path graft/…`, per §6.1's rule. And `git commit-graph write --reachable` is a repo write, which §6 says only `ops` may run: route the offer through `ops`, journaled as an ordinary non-undoable entry or explicitly exempted | M2 |

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
| Is ED-frees-image-data Ghostty-specific? | Only Ghostty 1.3.1 was tested. Unknown whether kitty-the-terminal (or other kitty-protocol terminals) also free image data on `ESC[2J`. M1 must not assume uploads survive a clear anywhere | M1 |
| Sixel and plain-text rendering tiers unproven | Spike exercises only Kitty and iTerm2. Sixel alignment/scroll strategy and the text-cell fallback graph need their own validation | M1 tickets 08, 19 (text), 29 (Sixel-only fallback) |
| tmux passthrough and SSH untested | No `tmux` escape wrapping attempted; detection over SSH (env vars lie) not exercised | M1 ticket 29 |
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
| **No Sixel encoder is planned** | ADR-0016 does not keep Sixel, so tickets 24–25 build Kitty and iTerm2 only. A terminal whose *only* image protocol is Sixel and which has no truecolor is selected for the pixel tier by `detect::tier` (Sixel beats ASCII) and will find no encoder behind it. Ticket 29 must either build one or make that combination fall to Unicode/ASCII explicitly — not leave a tier with nothing behind it | M1 ticket 29 |

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
| **Colour depth is inferred from environment strings, not terminfo.** `detect` reads `COLORTERM`, `TERM` (`*-direct`, `*256*`) and a short list of known-truecolor `TERM_PROGRAM`s. A 256-colour terminal with an unusual `TERM`, or a truecolor one absent from the list, is under-reported — and DESIGN.md §4.2 makes truecolor the Unicode/ASCII boundary, so an under-report costs a tier | **Closed by ADR-0019:** environment strings are enough; terminfo's `RGB` adds nothing `TERM=*-direct` doesn't, and `TERM` lies over SSH/tmux. Under-report fix is `COLORTERM` or `graph.renderer` | Closed |
| **Sanitising does not cover look-alikes, width, or the invisibles that are not `Cf`.** `term::sanitize` neutralises controls, bidi and Unicode's whole format category. Width is handled at draw time since m1-the-graph/08: `ui::fit::fit` lays text into an exact cell count, never splits a wide character, and keeps at most two combining marks per base. Still passing through `sanitize`: homoglyphs (Cyrillic `а` for Latin `a`), East Asian *ambiguous*-width characters (which `fit` counts as one cell; a CJK-locale terminal may draw two), and the invisible characters that are *not* `Cf` — the variation selectors U+FE00–U+FE0F and U+E0100–U+E01EF (the current hidden-payload channel) and the Hangul fillers. None of them can drive the terminal; replacing the variation selectors would corrupt ordinary emoji and CJK text, which is why they are kept. The `Cf` ranges are also spelled out as literals, pinned to Unicode 16.0, because nothing in the dependency set answers character-property questions: a Unicode revision that adds a format character is not picked up until someone refreshes the list, and no test would notice | Whether Graft ever flags confusables or a variation-selector run in a ref name is a policy call with real false positives — needs the user's decision, not a builder's | M1 |
| **Twelve lanes collapse to six at 16 colours.** Measured from the resolved-value snapshot: the lane indices are 6,9,5,7,7,6,3,9,7,8,7,3 — six distinct. Colliding pairs: lanes 1/6 (cyan), 2/8 (bright red), 7/12 (yellow), and 4/5/9/11 all land on white. Perceptual mapping cannot fix this: sixteen fixed hues cannot carry twelve distinguishable ones. The ticket (m0/06) puts this out of scope on purpose | **Closed by m1/13.** At 16 colours and `NO_COLOR`, lanes in odd columns draw dashed (`╎`, `╌`), so neighbours differ in shape whatever their hue. Snapshots: `crates/graft-ui/tests/snapshots/lanes_without_hue__*`; a test asserts no same-coloured neighbours share a glyph over the whole corpus. By-eye check still the user's (ticket 13) | Closed |
| ~~**Dark chrome flattens at 16 colours.**~~ **Closed m1-the-graph/12c.** `bg`, `surface` and `surface-raised` all map to index 0, so the highlighted row's raised background painted nothing distinguishable. `Theme::color_depth()` (stored in `Theme::new`) now lets `graph_screen::draw_row` skip that background step at `ColorDepth::Palette16` and draw an accent-coloured left-edge rule (`▏`) in the marker slot instead; truecolor/256 unchanged. pty capture: `docs/shots/12c-palette16-chrome.txt` (raw ANSI, forced to 16 colours) | — | Closed m1-12c |
| **The lane palette is argued, not seen.** The twelve hues were selected numerically — max-min worst-case OKLab distance under normal vision plus Machado 2009 protan/deutan/tritan simulation, floors 0.18 adjacent / 0.055 any pair — and checked by eye only as flat swatches in an image, never as thin graph lines in a terminal, and never by anyone with a colour-vision deficiency | Re-owned: m1/13 prepared the commands (ticket 13, Notes) but only the user can look. Judge criss-cross and foxtrot in Zed and VS Code at truecolor, 16 colours and `NO_COLOR` | M1 checkpoint (user, by eye) |
| **~~The probe eats typeahead.~~ Closed by m1-the-graph/20**: `graft_term::input::InputMux` owns stdin from before the probe to exit; detection reads replies through a `ReplyWaiter`. pty check `.scratch/m1-the-graph/pty-typeahead-check.py`, capture `docs/shots/20-typeahead-f2-during-probe.txt` | — | — |
| **~~`ui.icons` has no consumer.~~ Closed for badges by m1-the-graph/11.** `Session::icons` carries it to the graph screen; badges draw bracketed plain Unicode by default and Nerd Font icons under `icons = "nerd"` (snapshot pair `badges-icons-plain`/`-nerd`). Node glyphs do **not** change: see the next row | — | Closed m1-11 |
| **Non-UTF-8 paths are typed for, not tested.** `RepoHandle`'s API is `Path`/`PathBuf` end to end and never converts to `String`, so a repository at a path that is not valid UTF-8 should open — but no test proves it. APFS and HFS+ reject such filenames, so the fixture cannot be created on the machine m0/08 was built on (PROMPT.md §6.3) | A heavy-tier test on Linux (ext4/tmpfs accept arbitrary bytes) that opens a repository under a non-UTF-8 path and checks the reported workdir round-trips byte for byte. Same for a branch name that is not UTF-8, which `RefName` already stores as bytes | M1 heavy tier |
| **~~The not-a-repo path flashes the alternate screen.~~ Closed by m1-the-graph/18**: `event_loop::run` opens the repository before the session; `tests/it/not_a_repo.rs` checks no `ESC[?1049h` is written, capture `docs/shots/18-not-a-repo.txt` | — | — |
| **M0's screen has only been seen in a synthetic pty.** m0/11 drove the real binary through a `pty.fork` at 100x30, and m0/12 re-walked the README's demo steps the same way: `graft .` shows the workdir, `HEAD main`, the tier and theme lines and exits 0 on `q` with the alternate screen left and every mode popped; `graft /tmp` exits 1 with the line above; `graft --help` prints usage and exits 0; `SIGINT` kills it by signal with the terminal restored; under `NO_COLOR=1` the only SGR left is the reset (`ESC[m`, `ESC[0m`) with no colour parameters at all, and the theme line reads `no colour (NO_COLOR)`. Nothing answered the probe in either pty, so the tier shown was the silent-terminal fallback and no real emulator's rendering of this screen has been seen | The M0 checkpoint: run it in Ghostty, iTerm2 and the Zed/VS Code terminals and look at it | **M0 review** |
| **A panicking job's crash report is unproven on the real hook.** The suppression path is covered from both ends — `catching` marks the thread (`crash` unit tests) and a panicking job becomes `Event::JobFailed(_, Panicked { .. })` while the pool keeps working (`jobs` tests) — but no test installs Graft's real panic hook, so the hook's early return (write the report, do *not* restore the terminal, hand the path back) has been reviewed and never executed. Installing it would set a process-wide hook and write into `$XDG_STATE_HOME` for every other test in the binary | The M0 review: panic a job by hand in a real terminal, check the screen survives and the status line names a report file that exists | **M0 review** |
| **`graft-ops` is empty.** A crate doc comment and nothing else: no types, no functions, no tests. It exists so ARCHITECTURE.md's crate map and m0/02's dependency-edge test are real from the start. (`graft-graph` was in this row too until m1-the-graph/05 filled it with the layout engine; the renderers are still to come) | `ops` fills up in M3 (write operations and the undo journal). Until then, a green `cargo test` on the workspace says nothing about it | M3 |
| **No budget in the spec has ever been measured.** Partly closed by m1-the-graph/02 — this row used to read "only the fast test tier exists": the heavy tier now exists, with the 100k-commit fixture, the `getrusage` measurement and `proptest` at 1,000 cases, and `TESTING.md` documents all three tiers. What is still missing is the part that *asserts*: no performance budget and no 300 MB memory gate is checked by any test, and there are no `criterion` benchmarks or `cargo-fuzz` targets. There is also no CI: PROMPT.md §7 item 6 makes that conditional on the project having a GitHub repo, which it does not | The budget assertions are M1 tickets 27 (memory, first paint) and 30 (frame budgets, benches). `cargo-fuzz` targets are M2 | M1 (budgets) / M2 (fuzzing) / not planned (CI) |
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
| ~~**The heavy tier takes two commands, not the one PROMPT.md §7.1 names.**~~ **Closed 2026-09-13 (ADR-0018, M1 ticket 31):** dropped `#[ignore]`, gate on the `heavy-tests` feature alone; PROMPT.md §7.1, `TESTING.md` and the four generator tests in `tests/it/heavy.rs` updated | Nothing | Closed |
| ~~**`PROPTEST_CASES` in the environment silently overrides both tiers.**~~ **Closed 2026-09-12 (ADR-0017).** The count is now set in code — 64 fast, 1,000 heavy — and `graft_fixtures::proptest_cases` does not read the environment. The original entry blamed the user's shell; the variable was in this repository's own `.claude/settings.json`, and has been removed | Nothing. Superseded by the row below | Closed |
| **The heavy code is not in the standard clippy gate.** `cargo clippy --all-targets -- -D warnings` does not enable `heavy-tests`, so it says nothing about `synthetic.rs` or `tests/it/heavy.rs`. Both were linted by hand with `--features heavy-tests` and are clean; nothing makes that happen again | A second clippy line in whatever runs the gates — CI when it exists, or the review checklist until then. `TESTING.md` names the command | M1 ticket 28 (review checklist) / CI |
| **The manual terminal checklist has never been run.** `TESTING.md` has the table, the steps and the pass conditions written out, and every row says **not run**. M0's checkpoint proved Kitty alone answers the probe; no tier has been looked at, because no graph is drawn yet | Tickets 08 (Unicode), 24–25 (pixel) and 29 (tmux, SSH) fill in their own rows against a real graph; 28 completes the matrix | M1 tickets 08, 24, 25, 29, 28 |
| **`git/git` has been opened once, and only looked at.** m1/09b opened it under a pty: first graph frame 567 ms (debug build, 120×40, one run), 500 rows laid out. Whether 1,017 refs cost anything, and any memory figure, are still unmeasured | 27 measures it; 28's checkpoint script hands it to the user | M1 tickets 27, 28 |
| **~~git/git's `seen` merges fan out to ~28 lanes and fill the graph column.~~ Closed by m1-the-graph/14.** `graph::fold` folds the least-recently-touched lanes behind `+N` (`graph.max_lanes`, default 8): on git/git at 80×24 seven lanes plus `+17` draw and the subjects start at column 20 (`docs/shots/14-git-git-80x24.txt`, `14-git-git-120x40.txt`) | — | closed |

**Reading history** (m1-the-graph/04, 2026-09-12). This ticket closed three M0
rows, now removed from the table above: the repository handle not outliving its
job (`app::repos::RepoStore`), `gix` being compiled with almost no features
(measured, see `graft-git/Cargo.toml`), and nothing ever cancelling a job
(`Esc`, with a test that presses the key).

| Gap | What is missing | Closed by |
|---|---|---|
| **~~History arrives in one lump, not a stream.~~ Closed by m1-the-graph/07.** `app::history::stream` drives a `StreamingLayout` over the product `WalkSource` and sends each chunk's rows to the UI thread as `AppEvent::Progress` the moment it is laid out; a job may now report progress any number of times before its one ending event | — | Closed m1-07 |
| **~~ADR-0008's promotion is built in `graph`, not in the app.~~ Closed by m1-the-graph/18**: `Command::PromoteHistory` sorts off-thread, `History::promote` hands the order to the walk, the header says `date order, sorting…` / `order stays provisional (date)` | — | — |
| **~~`graph.default_branch` is not a config key.~~ Closed by m1-the-graph/07** for `default_branch`: parsed into `config::Graph` (a name git refuses is a parse error with file and line) and passed to `RepoStore::new`, so `Branches::resolve` honours ADR-0001 rule 1. **Still open: `graph.integration_branches`** — integration branches are still only detected by name (`develop`, `release/*`), never configured | A `graph.integration_branches` key and a `Branches::resolve` parameter for it | M1 ticket 10 |
| **~~The corpus cannot tell date order and topological order apart.~~ Closed by m1-the-graph/18**: `Shape::SkewedDates` (a commit a day ahead) puts the fast commit at row 1 by date and row 13 topologically (`graft-git` corpus test) | — | — |
| **The walk has never been run against a big repository.** Everything measured so far is the synthetic 20,000-commit fixture and the corpus. `git/git` (85,649 commits, 1,017 refs) is cloned and still unopened, and no budget is asserted anywhere | Ticket 27's budget assertions, and ticket 09's first real graph on screen | M1 tickets 09, 27 |
| **~~The graph screen marks no `HEAD` node.~~ Closed by m1-the-graph/11.** The refs job (`refs::read`) resolves `HEAD` to a commit id; `view::row_entry` draws that row's node as `◆` (and the newest stash entry's as `■`) | — | Closed m1-11 |
| **Node glyphs ignore `ui.icons`.** Ticket 11 asked for node glyphs to honour it too. Not done, deliberately: Nerd Font's commit glyphs (`nf-oct-git_commit` and kin) are private-use code points that many fonts draw wider than one cell, which would break lane alignment, and the plain `● ◉ ◆ ◇ ■` shapes already read the same in any font. Recommend leaving node glyphs icon-set independent; needs the user's OK to drop from scope | User decision at the M1 checkpoint | M1 review |
| **Only the newest stash entry has a badge.** `refs/stash` is one ref; older entries live only in its reflog, and nothing reads the reflog yet. `stash@{1}` and older draw as ordinary rows with no badge | Reading the stash reflog (sidebar STASHES, M2) | M2 |
| **Ahead/behind trusts commit dates.** `graft_git::history::ahead_behind` walks newest-first by committer date and stops once everything queued is shared, as git does. Badly skewed clocks can miscount by the skewed commits; it never uses commit-graph generation numbers | Generation-number ordering when a commit-graph exists (ticket 30's scale work) | M1 ticket 30 |
| **The ref-states fixture's `HEAD` is not where its builder says.** `shapes/refs.rs::ref_states` ends with `checkout --orphan fresh` (an unborn `HEAD`), but the built fixture under `target/fixtures/` has `HEAD` → `refs/heads/main`. Found by m1-11's tests; nothing depends on it now, but a test for an unborn `HEAD` beside other branches would silently be testing the wrong thing | Check the fixture build's final steps / cache key | M1 review |
| **Badges are not counted against the 50k-ref scale.** One `RepoHandle::upstream` config lookup and one cache probe per local branch, on every refresh; unmeasured beyond the corpus | Ticket 30 (50k refs) | M1 ticket 30 |
| **A history walk holds a job worker for as long as it is on screen.** It waits on its request channel between requests (`jobs::read_history`); the pool is at most 4 workers (`MAX_WORKERS`), so one is gone for the run. Cancel/`Ctrl+R`/shutdown drop the sender and end it | Accept, or a dedicated history thread outside the pool if a later job needs all four | Revisit at M1 review |
| **Layout advances only as far as the screen asks.** Deliberately left by m1/09b: its scope is viewport-driven requests, and a background walk on git/git needs ticket 27's numbers to size it. DESIGN.md §3.6 says later `advance` calls run in the background between frames; m1/09a lays out on demand (a screen of slack past the viewport), so a `G` is always "cold" until it has run once, and the status screen's row count means "laid out so far", not "read so far" | Background advance while idle, if first `G` on big repos proves too slow (ticket 27 measures) | M1 ticket 27 |

**Layout engine** (m1-the-graph/05, 2026-09-12).

| Gap | What is missing | Closed by |
|---|---|---|
| **Neighbouring lanes can share a hue.** Lane colour is `BranchIdentity::slot(12)` — a hash, as PROMPT.md §1.2 asks — so two adjacent lanes collide one time in twelve, and nothing in a stable, per-branch rule can prevent it. The corpus shows it twice: feature-branches, where `main`, `feature/two` and `feature/three` all land on `lane-3` (checked: the slot spread over 1,200 names is uniform — this is chance), and foxtrot-merge, where the merge lane recovers the name `main` and so is the *same* identity as lane 0 beside it. Shape and position still separate them; colour does not | **Partly closed by m1/13:** at 16 colours and `NO_COLOR` the dashed odd columns separate them (foxtrot's lanes 0/1 at 16 colours are the snapshot). At truecolor and 256 lanes stay solid, so the one-in-twelve hash collision is still told apart by position only. Switching dashes on at every depth is one predicate (`dashes_odd_columns`) if the user's by-eye pass wants it | M1 checkpoint (user, by eye) |
| **~~Dashes follow the column, so compaction must keep it.~~ Closed by m1-the-graph/14.** `GraphColumn` keys the dash pattern on `Fold::real(drawn)`, the lane's pre-fold column, so a lane folded and expanded never flips solid↔dashed (`graph_column::tests::dashes_follow_the_real_column_through_a_fold`, `docs/shots/14-many-lanes-16.txt`) | — | closed |
| **Layout can name an `incoming` column that is not in that row's `lanes`.** Seen in the m1/14 property tests: a lane closing into a root commit gives `lane: 0, incoming: [1], lanes: [Some(main)]`. `Fold::plan` reads the touched columns as well as `lanes` so it copes; `GraphColumn` draws the edge into column 1 with no lane above it (a dangling `─` with no `╯`). Whether `Row` promises `lanes` covers every edge is unstated | Decide the `Row` invariant in `graph::row` and make layout keep it, or make the widget draw a closing edge without a lane cell | M1 review |
| **Tied commit dates put a parent above its child.** Seen capturing m1/15: a repo whose commits all share one timestamp, with no commit-graph file (so the ADR-0008 commit-date fallback), walks `main: set up` and `initial` above `main: work before merge 1`, their child. `git log --date-order` on the same repo keeps every child above its parents. The layout then draws lanes that close upwards. Real repos made by scripts, rebases or `git am` often share timestamps | Make the fallback walk break date ties without ever emitting a parent before a child (as `git log --date-order` does), with a corpus shape of tied dates | M1 review |
| **A folded merge looks like a plain commit.** m1/15: once folded, a merge has one parent drawn, so its node is the ordinary `●` and nothing on its row says a branch is folded into it; only the header's "N merges folded" tells | A fold glyph or suffix on the folded row (DESIGN.md §3.6 names none) | M1 review |
| **Switching first-parent view keeps the selection only when it is near the top.** m1/15 reset `1` to the top; m1/16 re-finds the selected commit in the rows the other view sends back (the first two screens) and puts the cursor on it. A commit further down, or one the other view leaves out (a side-branch commit in first-parent view), still lands on the top row | The worker resolves the anchor with `StreamingLayout::row_of` in the other view — or its nearest first-parent ancestor — laying out only as far as it must | M1, after ticket 16 |
| **`Esc` stops a topological sort on screen, not on the worker.** `Walk::full_topological`'s constructor reads the whole range without polling stop, so after `Esc` the header says `order stays provisional` at once but the job thread keeps reading until construction ends; its order is then dropped | Poll stop inside `full_topological`'s read (ticket 27 makes it cheaper anyway) | M1 ticket 27 |
| **A long repository path truncates the order notice off the header.** Seen capturing m1/18 at 100 columns: `graft <long fixture path>` cut the header at `15 comm…`, so `date order, sorting…` never showed; through a short path it does. The header truncates from the right, and the notice sits after the path, count and filters | Give the header a truncation priority (shorten the path first), per DESIGN.md's header rules | M1 review |
| **`Esc` during a sort has not been seen in a pty.** On the corpus the sort lands in milliseconds, before a key can arrive, so `order stays provisional (date)` is proved only by the `update` test (`esc_while_sorting_keeps_the_provisional_order_and_says_so`) | Capture it on a history big enough for the sort to take seconds (heavy tier or ticket 27's measurement) | M1 ticket 27 |
| **The walk answers nothing while a promotion lays out to the anchor.** `Views::promote` advances the new layout to the selected commit before swapping; scrolling during that waits. A selection deep in a large history pays a long advance; a selection the view does not show (a side commit in first-parent view) walks to the end | Bound the advance, or keep answering from the provisional layout until the promoted one reaches the anchor | M1 review |
| **The selection's thread is worked out over the window on hand, not the whole history.** m1/16: `Ancestry::of` reads the three screens the walk last sent. Which rows are kin is exact inside it; a descendant edge from a child above the window is lit up its column by rule, not by knowing the child; nothing off the window is known, so the detail pane's "Contained in" (DESIGN.md §1.1) cannot reuse it | A reachability index (commit-graph generation numbers) when the detail pane needs whole-history answers | M2 detail pane |
| **Focus and the ancestry highlight are bold/dim only.** m1/16: no low-saturation lane colours (DESIGN.md §5.2 asks for a "low-saturation, dimmed render"); terminals that ignore SGR 2 (dim) show no fade at all. Snapshots record the modifiers, not how a terminal draws them | Muted per-lane colour tokens in the theme, judged by eye | M1 review |
| **The fold, first-parent and stash keys are not in the footer.** m1/15: `Space`, `1` and `H` are in DESIGN.md §2.3 but the 80-column footer has no room for three more hints | The `?` help overlay (DESIGN.md §1.16) lists them | M1, with the help overlay |
| **Compaction has no corpus shape and no snapshot.** m1/14 tests fold over `graft-ui::test_support::wide_history` (synthetic, 12 topics) and a scripted repo (`docs/shots/14-many-lanes*.txt`); `graft-fixtures` has no many-lanes shape, so the corpus snapshot suites never see a `+N` | A `ManyLanes` corpus shape and its snapshots at 256/16/NO_COLOR | M1 review |
| **An inclusion change re-reads history from the top of the walk to the changed commit.** `Included::reinclude` rebuilds its frontier by re-reading every walked commit before the change (batches of 1,024), and `Included` keeps one `Oid` per walked commit — a second history-sized list beside `EmittedOrder`. Unmeasured on git/git; folding a merge 500k rows down re-reads 500k commits off-thread | Measure with ticket 27's numbers; if slow, checkpoint the frontier beside the layout's checkpoints | M1 ticket 27 |
| **A chunk cut back by `truncate` loses its eviction fingerprint.** The fingerprint covered the longer chunk and cannot check the shorter one, so a later rebuild of that one chunk goes unchecked once | Accept, or re-fingerprint the kept prefix when the chunk is held | M1 review |
| **Elbows and junctions have no dashed forms.** `╭ ╮ ╯ ╰ ├ ┤ ┬ ┴ ┼` stay solid on a dashed lane, so a dashed lane's first and last cells read solid. Unicode has no dashed corners | Accept, unless the user's by-eye pass finds a dashed lane ambiguous at its ends | M1 checkpoint (user, by eye) |
| **HEAD on a merge draws as HEAD only.** `NodeKind::Head` (`◆`) wins over `Merge` (`◉`), so a merge you are sitting on does not show it is one. Visible in criss-cross-merge and foxtrot-merge | A combined glyph, or leave it to ticket 10's row text/badges | M1 ticket 11 |
| **An octopus merge subject is not parsed.** `Merge branches 'a', 'b' and 'c'` matches none of DESIGN.md §3.4's four shapes, so each extra parent's lane falls back to a short-SHA identity (see the `octopus-merge` snapshot). The names are right there, in parent order | Extend `recover_branch_name` to return several names and let `open_merge_lane` take the n-th; not scheduled — octopus merges are rare in daily work | Not planned |
| **Column 0 stays empty when there are integration branches but no default branch.** `Seeds { default_branch: None, integration_branches: [..] }` reserves columns 1.. and never allocates 0, so the graph is drawn one column to the right. A repository with `develop` but no `main`/`master` and no `graph.default_branch` override would hit it. Not a correctness problem — every invariant holds — and `Branches::resolve` picks a default whenever any local branch exists, so the app cannot produce these seeds today | Leave it; revisit if a seed builder ever emits integration branches without a default | Not planned |
| **~~The app's seed builder does not exist yet.~~ Closed by m1-the-graph/07.** `app::history::seeds_for` makes the corpus snapshots' choice — default and integration branches from `Branches::resolve`, known tips from every `refs/heads/*` and `refs/remotes/*` — and says why in its doc. The corpus test still builds its own copy (`graft-graph` cannot depend on `app`); `app::history::tests` checks the product rows equal the whole layout on every shape | — | Closed m1-07 |
| **The snapshots are in commit-date order, not a commit-graph topological walk.** The header of every snapshot records `requested Topological, in force CommitDate (provisional)`; the corpus has no commit-graph file, and for it date order is a valid topological order (see m1-the-graph/04's row above), so no snapshot shows a parent arriving before its child. The already-drawn-parent path is covered by a directed unit test only | A corpus shape with a commit-graph file and out-of-order dates, whenever m1-the-graph/04's row is closed | M1 review |

**Streaming layout** (m1-the-graph/06, 2026-09-12). This ticket closed
KNOWN_GAPS R2-1 (above) and the engine half of two m1/04 rows (rewritten in
place).

| Gap | What is missing | Closed by |
|---|---|---|
| **~~`CommitSource` has no implementation in the product.~~ Closed by m1-the-graph/07** (`app::history::WalkSource`); promotion closed by m1-the-graph/18 | — | — |
| **The emitted order grows with history, at about 90 bytes per commit.** `EmittedOrder` keeps every emitted id twice — a `Vec<Oid>` at 33 bytes each and an `Oid → row` map — 9,027,656 bytes at 100k rows, so roughly 110 MB at git/git's 1.2M commits if all of it is ever laid out. Inside the 300 MB gate at 100k and the dominant term there; a compact index (20-byte SHA-1 storage, `u32` rows, a set of indices instead of a map of copies) would roughly halve it. The evicted-chunk fingerprint map also grows with history, at 24 bytes per chunk — negligible | Ticket 27 measures it on the real fixture with `getrusage` and decides whether the compact index is needed | M1 ticket 27 |
| **Nothing has been measured on the 100k fixture or in wall-clock time.** Every number in DESIGN.md §3.6 comes from an in-memory 100k-commit history in a unit test — fast tier, no repository, no `getrusage`: 16 chunks / 8,000 rows resident, 49 checkpoints = 90,158 bytes, 1.6 KiB per checkpoint. The 300 MB gate, the 150 ms first paint and the right size for the first `advance` budget are unmeasured; the corpus run (`tests/it/streaming.rs`, 200-commit repositories) proves the plumbing, not the budgets | Ticket 27 | M1 ticket 27 |
| **Least-recently-used is the wrong policy once checkpoints pass `max_checkpoints`.** 64 checkpoints at 2,000 rows cover 128k rows. Past that, LRU keeps whichever were touched last, so a deep history can be left with a long stretch that has no checkpoint at all, and a jump back into it replays from the nearest surviving one — correct (checkpoints are re-recorded on the way) but potentially hundreds of thousands of re-reads. Thinning (drop every other one, keep the spacing even) would bound the replay | Measure on git/git in ticket 27; change the policy if the replay shows | M1 ticket 27 |
| **`rows()` clones every row it returns.** A viewport's worth of `Row`s — each carrying a `Commit` with its message and identities — is copied out of the cache on every call. Cheap at 50 rows a frame; not free | Borrowed slices per chunk, or an iterator, once tickets 08 and 14 know what the renderers need | M1 ticket 08 |
| **~~Promotion hands the app a swap to get right.~~ Closed by m1-the-graph/18**: `Views::promote` lays the promoted layout out to the anchor before swapping; uses `WalkSource::in_order` under `Included`, not `from_order`, so every view keeps its filtering. Checkpoints past the anchor are still cold after the swap | — | — |
| **`Walk::full_topological` reads the whole range before it emits.** Measured: 200 reads before the first commit on the 200-commit linear fixture. That is the design (ADR-0008 — a background job), but on git/git it is 1.2M object reads per open until the commit-graph offer (R2-4) is accepted, and nothing caches the order between runs | The commit-graph offer (R2-4) | M2 |

**History streams into the layout** (m1-the-graph/07, 2026-09-13).

| Gap | What would close it | Owner |
|---|---|---|
| **`AppState` keeps every row it is sent.** `History::chunks` is an append-only list of `Arc<[Row]>`, so the UI side grows with history — the thing DESIGN.md §3.6 says the row cache must never be. The layout's own cache evicts on the worker, but the copies already sent stay. Fine on the corpus; not on git/git. Same order of memory as the one-lump `Arc<[Commit]>` it replaces | The graph screen asks the layout for its viewport's rows instead of keeping all of them — a long-lived layout the UI can query between frames (tickets 08/14), measured in ticket 27 | M1 tickets 08, 27 |
| **The first `advance` is a full 500-row chunk, not sized to the 150 ms first paint.** `lay_out` advances `chunk_rows` at a time from the start | Ticket 27's measurement picks the first budget | M1 ticket 27 |
| **Rows laid out after `Esc` inside the interrupted chunk are dropped, not shown.** `lay_out` stops reporting once `stop` is set, and `update` ignores any chunk after `Esc`, so the count freezes at the last whole chunk that landed — by design, but the layout on the worker has laid out a few more rows nobody sees | Nothing needed unless resume-after-cancel is wanted | — |

## Scheduled interim states

These are *temporary*. Each one is a real gap between the milestone that creates
it and the milestone that closes it, and each must be visible in the UI while it
exists.

| Gap | Created by | Expiry |
|---|---|---|
| **No in-app conflict resolver.** M5 can produce conflicts through merge, rebase and cherry-pick before M6 builds the three-pane resolver. M5 must detect the conflicted state, list the conflicting files, offer the user's configured `git mergetool` or `$EDITOR`, and keep the operation banner with Continue / Skip / Abort available. M5 must not claim conflict resolution works | M5 | **M6** |
| **The commit tooltip has no "contained in" line.** m1-the-graph/17 built the tooltip (hover and `Enter`, identical content): subject, full body, parent shas. Which branches/tags a commit is contained in (PROMPT.md §1.3) needs the branch-tip ancestry lookup the sidebar work hasn't built yet, so it is left out rather than faked | m1-the-graph/17 | M2 |

---

*Builders and reviewers: add rows here the moment something is stubbed, naming
the milestone and ticket it came from. An entry here is not a failure; a missing
entry is (CLAUDE.md).*

## Input mux (m1-the-graph/20)

| Gap | What would close it | When |
|---|---|---|
| **A reply split by more than 50 ms is misread as keys.** `SEQUENCE_TIMEOUT` (50 ms) is one constant: over a slow SSH link a reply whose bytes arrive further apart than that is flushed as `Alt+[` and characters, and the probe sees no reply. Not observed; local splits land within a millisecond | Hold an unfinished reply-shaped sequence (`ESC [ ?`, `ESC ]`, `ESC _`, `ESC P`) up to the open waiter's deadline instead of 50 ms, measured over a real SSH session. **Ticket 21 did not fix this and adds exposure:** resize and focus re-queries go through the same decoder mid-session, so a split reply is delivered to `update` as keys (e.g. `Alt+[`, then `6;40;20t` typed into the graph) while the user is working, not just during startup | M1 heavy tier |
| **A re-query reply that arrives after its deadline can be taken for the next round's.** `detect::redetect` discards waiting bytes before each round, but a late reply landing *during* the next round is parsed as its answer (its fence ends the read early). Needs a reply later than the 200/500 ms budget; not observed | Tag rounds (e.g. a distinct `DECRQM` per round as the fence) if it is ever seen | when seen |
| **Key order under a pty is proven by unit test only.** The pty check proves arrival (F2 ×2 and ×3 during the probe land on the right screen) but its selection detector cannot see the cursor row, so `jjjk` during the probe was not verified on screen. Order is covered by `input::mux::tests` over a socket pair | A pty assertion on the cursor row (e.g. read the selected commit from the `Enter` tooltip) | M1 review |
| **Decoder covers xterm, SS3, kitty `CSI u`, SGR and X10 mouse.** Not covered: urxvt `1015` mouse (`CSI b;x;y M` without `<`), which crossterm's `EnableMouseCapture` also turns on; terminals that prefer SGR (`1006`) never send it | Decode `1015` if a supported terminal is found sending it | when seen |

## m1-the-graph/22 — rasteriser
- Node style is defined twice: `graft_ui::graph_column::NodeKind` (Unicode glyphs) and `graft_graph::raster::Ring` (pixels), same six variants. `ui` depends on `graph`, so `NodeKind` could become `Ring` (or map to it); left for the pixel-output wiring ticket (24) to decide.
- `CELLS_PER_LANE` (= 2) is declared in both `graft-ui` and `graft-graph::raster`; they must agree or pixels drift from text. Same fix: `ui` re-uses `graph`'s.
- Strip cache (m1/23) has no owner yet: nothing calls `StripCache::flush` on 21's resize / theme re-query / re-detection events. Wiring lands with the pixel output (24). Until then the key alone (theme colours, cell size, width in the layout hash) already keeps stale strips from being hit.
- The raster tests copy the 12 lane hues from `graft-ui/src/theme/lanes.rs` (`graph` cannot see `ui`). Palette colour does not affect any assertion except the snapshot's slot letters.
- Pixel shots are rendered at the 17×37 cell only; how they look at other cell sizes (e.g. 8×16 on a small font) is unjudged.
- Render time measured on the fast tier only: 32 rows × 5 lanes at 17×37, Apple M5, median 1.38 ms release / 13.4 ms debug. Wide strips (many lanes) unmeasured; the frame-budget ticket (30) owns it.

- **m1/24 pixel tier not wired (2026-09-13).** `graft-term::kitty` encoder exists and is tested; nothing in `app` calls it yet, so Ghostty still draws the Unicode graph. Remaining: 24b aligned-strip rows, 24c draw + reply draining + `gfx` status, 24d flush/F3/real-Ghostty check. See `.scratch/m1-the-graph/issues/24-kitty-output.md` Handoff.
