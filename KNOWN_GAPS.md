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
| Probe/input conflicts and resize | Spike loses typeahead during capability probes (two readers on stdin) and never re-probes cell size on SIGWINCH. M1 needs the input mux and resize handling named in ADR-0012 | M1 |

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
| **Colour depth is inferred from environment strings, not terminfo.** `detect` reads `COLORTERM`, `TERM` (`*-direct`, `*256*`) and a short list of known-truecolor `TERM_PROGRAM`s. A 256-colour terminal with an unusual `TERM`, or a truecolor one absent from the list, is under-reported — and DESIGN.md §4.2 makes truecolor the Unicode/ASCII boundary, so an under-report costs a tier | Either a terminfo lookup (`max_colors`, `RGB`) or a deliberate decision that the env strings are enough, once M1 knows what the renderers need from colour | M1 |
| **Sanitising does not cover look-alikes, width, or the invisibles that are not `Cf`.** `term::sanitize` neutralises controls, bidi and Unicode's whole format category. Still passing through: homoglyphs (Cyrillic `а` for Latin `a`), combining-mark stacks ("Zalgo"), ambiguous- and fullwidth characters, and the invisible characters that are *not* `Cf` — the variation selectors U+FE00–U+FE0F and U+E0100–U+E01EF (the current hidden-payload channel) and the Hangul fillers. None of them can drive the terminal; replacing the variation selectors would corrupt ordinary emoji and CJK text, which is why they are kept. The `Cf` ranges are also spelled out as literals, pinned to Unicode 16.0, because nothing in the dependency set answers character-property questions: a Unicode revision that adds a format character is not picked up until someone refreshes the list, and no test would notice | M1's renderer decides how a cell clamps over-wide and combining-heavy text. Whether Graft ever flags confusables or a variation-selector run in a ref name is a policy call with real false positives — needs the user's decision, not a builder's | M1 |
| **Twelve lanes collapse to six at 16 colours.** Measured from the resolved-value snapshot: the lane indices are 6,9,5,7,7,6,3,9,7,8,7,3 — six distinct. Colliding pairs: lanes 1/6 (cyan), 2/8 (bright red), 7/12 (yellow), and 4/5/9/11 all land on white. Perceptual mapping cannot fix this: sixteen fixed hues cannot carry twelve distinguishable ones. The ticket (m0/06) puts this out of scope on purpose | Lane identity at 16 colours must come from shape, not hue — dash pattern, glyph or a repeating 6-hue cycle with a second channel. M1's Unicode/ASCII tier decides and must be judged by eye, not by distance numbers | M1 |
| **Dark chrome flattens at 16 colours.** `bg`, `surface` and `surface-raised` all map to index 0 and `border`/`text-muted` both to 8, so a 16-colour dark terminal has no surface separation at all (verified in the same snapshot and by eye on a rendered swatch grid). Light theme flattens the same way at the top: `surface` and `surface-raised` both 15 | M1's widgets need a non-colour separator (a rule, a box-drawing edge) whenever `ColorDepth::Palette16` is in force, rather than trusting a background step that isn't there | M1 |
| **The lane palette is argued, not seen.** The twelve hues were selected numerically — max-min worst-case OKLab distance under normal vision plus Machado 2009 protan/deutan/tritan simulation, floors 0.18 adjacent / 0.055 any pair — and checked by eye only as flat swatches in an image, never as thin graph lines in a terminal, and never by anyone with a colour-vision deficiency | M1's visual pass: render real lanes in Ghostty and in the Zed/VS Code terminals and judge adjacency there. Thin antialiased lines are a harder case than a swatch | M1 |
| **The probe eats typeahead.** Keys pressed during the probe window (up to 200 ms, 500 ms over SSH; twice that inside tmux) are read by the reply reader and dropped. Already recorded as a spike gap; it is now in the product path | M1's stdin input mux, which owns the fd and hands replies to detection instead of detection reading the fd itself | M1 |
| **`ui.icons` has no consumer.** m0/11 wired the rest of the config in: `config::load()` runs before the terminal is touched, a `ConfigError` prints and exits 1, unknown-key warnings show as notes on the status screen, and `graph.renderer` / `ui.theme` decide the tier and theme. `ui.icons` still changes nothing — nothing draws icons until M1 | M1's widgets read `ui.icons` | M1 |
| **Non-UTF-8 paths are typed for, not tested.** `RepoHandle`'s API is `Path`/`PathBuf` end to end and never converts to `String`, so a repository at a path that is not valid UTF-8 should open — but no test proves it. APFS and HFS+ reject such filenames, so the fixture cannot be created on the machine m0/08 was built on (PROMPT.md §6.3) | A heavy-tier test on Linux (ext4/tmpfs accept arbitrary bytes) that opens a repository under a non-UTF-8 path and checks the reported workdir round-trips byte for byte. Same for a branch name that is not UTF-8, which `RefName` already stores as bytes | M1 heavy tier |
| **The not-a-repo path flashes the alternate screen.** `graft /tmp` enters the session, draws one "opening" frame, then leaves and prints `graft: /tmp is not inside a git repository` on a clean terminal, exit 1 — verified in a pty. The flash lasts about as long as the capability probe (~200 ms), because the repository is only opened after the session is up | Either open the repository before entering the session, or make this the first-run Init/Clone screen's job (ticket 11 puts that screen out of scope) | M1 |
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
| **The heavy tier takes two commands, not the one PROMPT.md §7.1 names.** `-- --ignored` runs *only* ignored tests, so the documented command would never reach a property test and the "1,000 cases" half of the tier would silently not happen. `TESTING.md` documents both commands and why. PROMPT.md still says one | Either amend PROMPT.md §7.1 to name both commands, or drop `--ignored` and rely on the feature alone to gate the heavy tests. Needs the user's call, not a builder's | The M1 review |
| ~~**`PROPTEST_CASES` in the environment silently overrides both tiers.**~~ **Closed 2026-09-12 (ADR-0017).** The count is now set in code — 64 fast, 1,000 heavy — and `graft_fixtures::proptest_cases` does not read the environment. The original entry blamed the user's shell; the variable was in this repository's own `.claude/settings.json`, and has been removed | Nothing. Superseded by the row below | Closed |
| **The heavy code is not in the standard clippy gate.** `cargo clippy --all-targets -- -D warnings` does not enable `heavy-tests`, so it says nothing about `synthetic.rs` or `tests/it/heavy.rs`. Both were linted by hand with `--features heavy-tests` and are clean; nothing makes that happen again | A second clippy line in whatever runs the gates — CI when it exists, or the review checklist until then. `TESTING.md` names the command | M1 ticket 28 (review checklist) / CI |
| **The manual terminal checklist has never been run.** `TESTING.md` has the table, the steps and the pass conditions written out, and every row says **not run**. M0's checkpoint proved Kitty alone answers the probe; no tier has been looked at, because no graph is drawn yet | Tickets 08 (Unicode), 24–25 (pixel) and 29 (tmux, SSH) fill in their own rows against a real graph; 28 completes the matrix | M1 tickets 08, 24, 25, 29, 28 |
| **`git/git` is 85,649 commits and nothing reads it yet.** `scripts/clone-git-git.sh` was run once by hand (42 MB, 15 s) and the clone works, but it is only usable through `cargo run -- <path>`, which in M1/02 still shows the M0 status screen. Whether Graft copes with its 1,017 refs is unknown | 09 (first graph on screen) opens it; 27 measures it; 28's checkpoint script hands it to the user | M1 tickets 09, 27, 28 |

**Reading history** (m1-the-graph/04, 2026-09-12). This ticket closed three M0
rows, now removed from the table above: the repository handle not outliving its
job (`app::repos::RepoStore`), `gix` being compiled with almost no features
(measured, see `graft-git/Cargo.toml`), and nothing ever cancelling a job
(`Esc`, with a test that presses the key).

| Gap | What is missing | Closed by |
|---|---|---|
| **History arrives in one lump, not a stream.** The walk itself is chunked and resumable — `Walk::next_chunk` hands back 500 commits at a time and checks for cancellation between chunks — but `app::history::read` drains every chunk and the job reports once, because a job reports exactly once by design. So the first row still waits for the last one, and PROMPT.md's 150 ms first paint is not achievable through this path as it stands | The consumer now exists: `StreamingLayout::advance(budget, stop)` takes commits a chunk at a time and lays them out (m1-06). What is missing is the job that feeds it and reports more than once, or a shared buffer the UI reads while the walk fills it | M1 ticket 10 |
| **ADR-0008's promotion is built in `graph`, not in the app.** The engine half exists (m1-06): `Walk::full_topological` runs the whole-range topological walk without a commit-graph file, `StreamingLayout::from_order` lays its output out by re-reading commits rather than walking, and `reanchor` finds the selected commit again by id — proved on the corpus in `graft-graph/tests/it/streaming.rs` and on skewed dates in the unit tests. What does not exist: the app job that runs the background walk and swaps the layouts behind a frame, and the screen that says the order is provisional | Ticket 10 | M1 ticket 10 |
| **`graph.default_branch` is not a config key.** ADR-0001 rule 1 is an override the user sets; `RepoStore::new` takes it and `Branches::resolve` honours it, but nothing parses it out of the config file, so `app` passes `None` and rule 1 can never fire. `config::Graph` is `Copy` today and a `String` field would end that | A config change — a new key parsed into `config::Graph`, alongside `graph.integration_branches`, which has the same hole. Wanted by whichever ticket first makes a wrong lane 0 visible, which is ticket 10 at the latest | M1 ticket 10 |
| **The corpus cannot tell date order and topological order apart.** `graft-fixtures` writes strictly increasing commit timestamps, so every child is later than its parents and date order is *always* a valid topological order on every shape in it. The corpus tests therefore prove both walks agree with `git rev-list` — they do not prove the topological walk is doing anything the date walk isn't. That distinction is made only by unit tests over a hand-built skewed DAG (`MapNodes`), where a merge is older than one of its parents | A corpus shape with deliberately skewed committer dates — a rebase or a cherry-pick that back-dates — read by the same tests that read the others | M1, whichever ticket next touches the corpus |
| **The walk has never been run against a big repository.** Everything measured so far is the synthetic 20,000-commit fixture and the corpus. `git/git` (85,649 commits, 1,017 refs) is cloned and still unopened, and no budget is asserted anywhere | Ticket 27's budget assertions, and ticket 09's first real graph on screen | M1 tickets 09, 27 |
| **Nothing reads `AppState::history`.** The walk runs at startup, the result lands in the state, `Esc` cancels it — and no widget draws a single commit. Verified by tests, not by eye | Tickets 08 and 10: the Unicode renderer and the screen that puts it up | M1 tickets 08, 10 |

**Layout engine** (m1-the-graph/05, 2026-09-12).

| Gap | What is missing | Closed by |
|---|---|---|
| **A `Row` carries a lane index and no identity.** DESIGN.md §3.1's data model puts the identity on the `Lane`, and `LayoutCursor::active_lanes()` reports only the *current* occupant of each column. A renderer that wants a row's colour must read `active_lanes()` when the row is produced; anything that colours rows after the fact (a cache, scrollback) has to capture it then. A commit that is both a root and a tip opens and closes its lane inside one row, so its identity is never observable from outside at all — the corpus snapshot test prints `?` for such a row (none in the corpus today) | The renderer tickets (08, 10) decide whether `Row` grows an identity field or the renderer captures it from the cursor; either is a small change, but it is theirs to make | M1 tickets 08 / 10 |
| **An octopus merge subject is not parsed.** `Merge branches 'a', 'b' and 'c'` matches none of DESIGN.md §3.4's four shapes, so each extra parent's lane falls back to a short-SHA identity (see the `octopus-merge` snapshot). The names are right there, in parent order | Extend `recover_branch_name` to return several names and let `open_merge_lane` take the n-th; not scheduled — octopus merges are rare in daily work | Not planned |
| **Column 0 stays empty when there are integration branches but no default branch.** `Seeds { default_branch: None, integration_branches: [..] }` reserves columns 1.. and never allocates 0, so the graph is drawn one column to the right. A repository with `develop` but no `main`/`master` and no `graph.default_branch` override would hit it. Not a correctness problem — every invariant holds — and `Branches::resolve` picks a default whenever any local branch exists, so the app cannot produce these seeds today | Leave it; revisit if a seed builder ever emits integration branches without a default | Not planned |
| **The app's seed builder does not exist yet, and its choices matter for colour.** The corpus snapshot test builds `Seeds` itself: default and integration branches from `Branches::resolve`, `known_branches` from every `refs/heads/*` *and* `refs/remotes/*` tip. The remote half is deliberate — in the `ref-states` shape the remote is ahead of the local `behind` branch, so the lane opens at the remote's tip and would be a short SHA without it | The ticket that wires the graph screen to the walk builds the real seeds; it should make the same choice (or a better one) and say why | M1 tickets 08 / 10 |
| **The snapshots are in commit-date order, not a commit-graph topological walk.** The header of every snapshot records `requested Topological, in force CommitDate (provisional)`; the corpus has no commit-graph file, and for it date order is a valid topological order (see m1-the-graph/04's row above), so no snapshot shows a parent arriving before its child. The already-drawn-parent path is covered by a directed unit test only | A corpus shape with a commit-graph file and out-of-order dates, whenever m1-the-graph/04's row is closed | M1 review |

**Streaming layout** (m1-the-graph/06, 2026-09-12). This ticket closed
KNOWN_GAPS R2-1 (above) and the engine half of two m1/04 rows (rewritten in
place).

| Gap | What is missing | Closed by |
|---|---|---|
| **`CommitSource` has no implementation in the product.** The trait is the seam between `graph` and `git` (ARCHITECTURE.md); the only implementation is the test one in `graft-graph/tests/it/streaming.rs`, a `Walk` plus `read_commit`. Nothing in `app` builds a `StreamingLayout`, so no screen streams yet | Ticket 10: a job that calls `advance` in budgets, reports rows as they land, runs the promotion walk and swaps the layouts | M1 ticket 10 |
| **The emitted order grows with history, at about 90 bytes per commit.** `EmittedOrder` keeps every emitted id twice — a `Vec<Oid>` at 33 bytes each and an `Oid → row` map — 9,027,656 bytes at 100k rows, so roughly 110 MB at git/git's 1.2M commits if all of it is ever laid out. Inside the 300 MB gate at 100k and the dominant term there; a compact index (20-byte SHA-1 storage, `u32` rows, a set of indices instead of a map of copies) would roughly halve it. The evicted-chunk fingerprint map also grows with history, at 24 bytes per chunk — negligible | Ticket 27 measures it on the real fixture with `getrusage` and decides whether the compact index is needed | M1 ticket 27 |
| **Nothing has been measured on the 100k fixture or in wall-clock time.** Every number in DESIGN.md §3.6 comes from an in-memory 100k-commit history in a unit test — fast tier, no repository, no `getrusage`: 16 chunks / 8,000 rows resident, 49 checkpoints = 90,158 bytes, 1.6 KiB per checkpoint. The 300 MB gate, the 150 ms first paint and the right size for the first `advance` budget are unmeasured; the corpus run (`tests/it/streaming.rs`, 200-commit repositories) proves the plumbing, not the budgets | Ticket 27 | M1 ticket 27 |
| **Least-recently-used is the wrong policy once checkpoints pass `max_checkpoints`.** 64 checkpoints at 2,000 rows cover 128k rows. Past that, LRU keeps whichever were touched last, so a deep history can be left with a long stretch that has no checkpoint at all, and a jump back into it replays from the nearest surviving one — correct (checkpoints are re-recorded on the way) but potentially hundreds of thousands of re-reads. Thinning (drop every other one, keep the spacing even) would bound the replay | Measure on git/git in ticket 27; change the policy if the replay shows | M1 ticket 27 |
| **`rows()` clones every row it returns.** A viewport's worth of `Row`s — each carrying a `Commit` with its message and identities — is copied out of the cache on every call. Cheap at 50 rows a frame; not free | Borrowed slices per chunk, or an iterator, once tickets 08 and 14 know what the renderers need | M1 ticket 08 |
| **Promotion hands the app a swap to get right.** `from_order` builds a second `StreamingLayout`; the provisional one's rows and checkpoints go with it. The app must advance the promoted layout to the anchor *before* swapping (the corpus test does) or the user watches the graph empty and refill, and a scroll down right after promotion pays a cold `advance` because the provisional layout's checkpoints past the anchor are gone | Ticket 10 | M1 ticket 10 |
| **`Walk::full_topological` reads the whole range before it emits.** Measured: 200 reads before the first commit on the 200-commit linear fixture. That is the design (ADR-0008 — a background job), but on git/git it is 1.2M object reads per open until the commit-graph offer (R2-4) is accepted, and nothing caches the order between runs | The commit-graph offer (R2-4) | M2 |

## Scheduled interim states

These are *temporary*. Each one is a real gap between the milestone that creates
it and the milestone that closes it, and each must be visible in the UI while it
exists.

| Gap | Created by | Expiry |
|---|---|---|
| **No in-app conflict resolver.** M5 can produce conflicts through merge, rebase and cherry-pick before M6 builds the three-pane resolver. M5 must detect the conflicted state, list the conflicting files, offer the user's configured `git mergetool` or `$EDITOR`, and keep the operation banner with Continue / Skip / Abort available. M5 must not claim conflict resolution works | M5 | **M6** |

---

*Builders and reviewers: add rows here the moment something is stubbed, naming
the milestone and ticket it came from. An entry here is not a failure; a missing
entry is (CLAUDE.md).*
