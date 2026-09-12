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
| 50k+ refs in the sidebar and ref badges (review N4) | PROMPT.md §6.3 requires handling repos with 50k+ refs; branch-tree filtering, badge lookup and ranking at that scale are undesigned. Validate in M1's performance pass, not assumed free | M1 |

## Design gaps (step 2 review round 2, 2026-09-12)

Named in `docs/reviews/step-2-design-round-2.md`. (R2-2, backup-ref naming,
is already fixed in DESIGN.md §6 and ADR-0010 — not a gap.)

**REQUIRED M1 TICKET — R2-1: post-promotion checkpoint state on the
no-commit-graph path.** Whoever drafts M1's tickets must create a ticket for
this; it is not optional polish. DESIGN.md §3.2/§3.6 never say what
`walker_state` checkpoints contain after promotion on the fallback path. The
naive reading — snapshot the fallback topo walker's in-degree state — is
O(remaining history) per checkpoint: 100+ MB of checkpoints on the 100k
no-commit-graph fixture, busting the heavy tier's 300 MB peak-memory gate
(the exact configuration ADR-0008 orders that gate to run). Required fix, one
or two sentences in §3.2/§3.6 plus the matching implementation: after the
background walk completes, retain the emitted order as a compact commit-id
list; post-promotion `walker_state` is an index into it, so checkpoints stay
small on both paths and §3.1's "bounded by parallelism" annotation holds
unconditionally. Expiry: **M1**.

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
| Resize frame busts the frame budget | A terminal resize clears and retransmits the whole strip: ~32–41 ms in the Ghostty bench, against a 16 ms budget. One frame per resize, so the spike leaves it. M1's fixed-height strips with ids (DESIGN.md §3.8) must not inherit a whole-viewport retransmit on every SIGWINCH | M1 |
| Is ED-frees-image-data Ghostty-specific? | Only Ghostty 1.3.1 was tested. Unknown whether kitty-the-terminal (or other kitty-protocol terminals) also free image data on `ESC[2J`. M1 must not assume uploads survive a clear anywhere | M1 |
| Sixel and plain-text rendering tiers unproven | Spike exercises only Kitty and iTerm2. Sixel alignment/scroll strategy and the text-cell fallback graph need their own validation | M1 |
| tmux passthrough and SSH untested | No `tmux` escape wrapping attempted; detection over SSH (env vars lie) not exercised | M1 |
| Probe/input conflicts and resize | Spike loses typeahead during capability probes (two readers on stdin) and never re-probes cell size on SIGWINCH. M1 needs the input mux and resize handling named in ADR-0012 | M1 |

## Rendering tiers: order and the Unicode tier (recorded 2026-09-12, settle at the start of M1)

Raised by the user at the step-3 fix. **Recorded, not decided** — DESIGN.md is
deliberately untouched and no ADR is written yet. M1 opens by settling this.

| Gap | What's needed | Expiry |
|---|---|---|
| **Unicode tier is the daily view, not a fallback** | The user works in the Zed and VS Code terminals, which have no image protocol, so the Unicode graph is what he sees most days. M1 must build it as a first-class tier — designed, not degraded — and judge it by eye in those terminals to the same standard as the pixel tier in Ghostty. Nothing may describe it as a downgrade | M1 |
| **Protocol preference order conflicts with DESIGN.md §4.2** — **needs an ADR when settled** | The user's order is Kitty → **Unicode** → iTerm2 → Sixel, with Sixel used only if it demonstrably beats Unicode. DESIGN.md §4.2 says Kitty > iTerm2 > Sixel and treats Unicode as the downgrade path. These disagree. M1 must resolve it and then do both: **write an ADR recording the order and why** (this is not optional — it is a design decision that overrides an approved DESIGN.md section), and amend §4.2 to match. Neither DESIGN.md nor an ADR may be edited before the decision is made | M1 |
| Sixel in VS Code unverified | VS Code has `terminal.integrated.enableImages`; whether it gives a usable Sixel tier there is untested. Check before assuming VS Code is image-less — it changes what the order above should be | M1 |

## M0 gaps

**What M0 has been seen doing, and what it has not** (swept at m0/12,
2026-09-12). Everything M0 built has been exercised under a *bare pty* — a
`pty.fork` or `script -q /dev/null` with no emulator behind it — and by unit,
snapshot and property tests. That proves the bytes Graft writes and the
decisions it makes from scripted replies. It does not prove a single pixel.
**No real terminal emulator has ever answered a capability probe, and no human
has looked at Graft's screen.** So the tier, cell size, theme and colour depth
every pty run reports are the silent-terminal fallback, and the status screen's
layout, colours and hint row have been read as text, never seen rendered. The
four rows below marked **M0 review** are the checkpoint's job; nothing a
builder can do closes them.

| Gap | What's needed | Expiry |
|---|---|---|
| **No real terminal has been seen *answering* the capability probe.** m0/04's evidence is: 46 unit tests against scripted replies, plus one run in a bare pty (`script -q /dev/null`) which proved the six queries go out in the right order with `DA1` last, and that a silent terminal times out at 201 ms against the 200 ms budget and falls back per query. What is **not** verified: any real emulator's replies, the tier they produce, `TIOCGWINSZ`-derived cell size, `OSC 11` theme, tmux passthrough, and SSH. `cargo run -p graft --example capability_probe` prints the full report; run it in Ghostty, iTerm2, Kitty, WezTerm, Terminal.app, the Zed and VS Code terminals, inside tmux >= 3.3 with `allow-passthrough on`, inside tmux 3.2, and over SSH | The manual runs above, at the M0 checkpoint. Any terminal that answers differently from the fixtures is a bug in the parser or the tier table, not a gap to live with | **M0 review** |
| **The `Rendering` line says `detected` even when nothing was detected.** Found at m0/12 by running the demo in a pty: the probe timed out (`kitty Silent`, `sixel Silent`, `probe TimedOut in 200.5 ms`) and the screen still read `Rendering  unicode, detected, cell 8x16 (assumed)`. `RenderingView::tier_forced` only records whether the config file overrode the tier, so `status_screen::rendering` prints `detected` for "not forced" — the cell size and theme carry honest provenance (`assumed`, `terminal background unknown`) and the tier does not. A user reading that line is told the terminal answered when it did not. Not fixed here: m0/12 is a docs-and-sweep ticket and this is a code change | The tier needs the same provenance the other two fields have — three states (forced / the terminal answered / fell back), sourced from `Capabilities::probe` rather than from `tier_forced` alone. M1 owns the rendering line because M1 is when the tier starts mattering | M1 |
| **Colour depth is inferred from environment strings, not terminfo.** `detect` reads `COLORTERM`, `TERM` (`*-direct`, `*256*`) and a short list of known-truecolor `TERM_PROGRAM`s. A 256-colour terminal with an unusual `TERM`, or a truecolor one absent from the list, is under-reported — and DESIGN.md §4.2 makes truecolor the Unicode/ASCII boundary, so an under-report costs a tier | Either a terminfo lookup (`max_colors`, `RGB`) or a deliberate decision that the env strings are enough, once M1 knows what the renderers need from colour | M1 |
| **Sanitising does not cover look-alikes, width, or the invisibles that are not `Cf`.** `term::sanitize` neutralises controls, bidi and Unicode's whole format category. Still passing through: homoglyphs (Cyrillic `а` for Latin `a`), combining-mark stacks ("Zalgo"), ambiguous- and fullwidth characters, and the invisible characters that are *not* `Cf` — the variation selectors U+FE00–U+FE0F and U+E0100–U+E01EF (the current hidden-payload channel) and the Hangul fillers. None of them can drive the terminal; replacing the variation selectors would corrupt ordinary emoji and CJK text, which is why they are kept | M1's renderer decides how a cell clamps over-wide and combining-heavy text. Whether Graft ever flags confusables or a variation-selector run in a ref name is a policy call with real false positives — needs the user's decision, not a builder's | M1 |
| **Twelve lanes collapse to six at 16 colours.** Measured from the resolved-value snapshot: the lane indices are 6,9,5,7,7,6,3,9,7,8,7,3 — six distinct. Colliding pairs: lanes 1/6 (cyan), 2/8 (bright red), 7/12 (yellow), and 4/5/9/11 all land on white. Perceptual mapping cannot fix this: sixteen fixed hues cannot carry twelve distinguishable ones. The ticket (m0/06) puts this out of scope on purpose | Lane identity at 16 colours must come from shape, not hue — dash pattern, glyph or a repeating 6-hue cycle with a second channel. M1's Unicode/ASCII tier decides and must be judged by eye, not by distance numbers | M1 |
| **Dark chrome flattens at 16 colours.** `bg`, `surface` and `surface-raised` all map to index 0 and `border`/`text-muted` both to 8, so a 16-colour dark terminal has no surface separation at all (verified in the same snapshot and by eye on a rendered swatch grid). Light theme flattens the same way at the top: `surface` and `surface-raised` both 15 | M1's widgets need a non-colour separator (a rule, a box-drawing edge) whenever `ColorDepth::Palette16` is in force, rather than trusting a background step that isn't there | M1 |
| **The lane palette is argued, not seen.** The twelve hues were selected numerically — max-min worst-case OKLab distance under normal vision plus Machado 2009 protan/deutan/tritan simulation, floors 0.18 adjacent / 0.055 any pair — and checked by eye only as flat swatches in an image, never as thin graph lines in a terminal, and never by anyone with a colour-vision deficiency | M1's visual pass: render real lanes in Ghostty and in the Zed/VS Code terminals and judge adjacency there. Thin antialiased lines are a harder case than a swatch | M1 |
| **The probe eats typeahead.** Keys pressed during the probe window (up to 200 ms, 500 ms over SSH; twice that inside tmux) are read by the reply reader and dropped. Already recorded as a spike gap; it is now in the product path | M1's stdin input mux, which owns the fd and hands replies to detection instead of detection reading the fd itself | M1 |
| **`ui.icons` has no consumer.** m0/11 wired the rest of the config in: `config::load()` runs before the terminal is touched, a `ConfigError` prints and exits 1, unknown-key warnings show as notes on the status screen, and `graph.renderer` / `ui.theme` decide the tier and theme. `ui.icons` still changes nothing — nothing draws icons until M1 | M1's widgets read `ui.icons` | M1 |
| **Non-UTF-8 paths are typed for, not tested.** `RepoHandle`'s API is `Path`/`PathBuf` end to end and never converts to `String`, so a repository at a path that is not valid UTF-8 should open — but no test proves it. APFS and HFS+ reject such filenames, so the fixture cannot be created on the machine m0/08 was built on (PROMPT.md §6.3) | A heavy-tier test on Linux (ext4/tmpfs accept arbitrary bytes) that opens a repository under a non-UTF-8 path and checks the reported workdir round-trips byte for byte. Same for a branch name that is not UTF-8, which `RefName` already stores as bytes | M1 heavy tier |
| **The repository handle does not outlive the job that opened it.** m0/11 reads the workdir, `HEAD` and bareness into a plain `RepositoryFacts` and shows them; the `RepoHandle` itself is still dropped on the worker thread, so every future read would re-open the repository from scratch | M1's rev-walk needs a handle that lives as long as the app: an owner outside `AppState` (which must stay `Clone` and world-free) that jobs borrow, decided when the first long read exists | M1 |
| **The not-a-repo path flashes the alternate screen.** `graft /tmp` enters the session, draws one "opening" frame, then leaves and prints `graft: /tmp is not inside a git repository` on a clean terminal, exit 1 — verified in a pty. The flash lasts about as long as the capability probe (~200 ms), because the repository is only opened after the session is up | Either open the repository before entering the session, or make this the first-run Init/Clone screen's job (ticket 11 puts that screen out of scope) | M1 |
| **M0's screen has only been seen in a synthetic pty.** m0/11 drove the real binary through a `pty.fork` at 100x30, and m0/12 re-walked the README's demo steps the same way: `graft .` shows the workdir, `HEAD main`, the tier and theme lines and exits 0 on `q` with the alternate screen left and every mode popped; `graft /tmp` exits 1 with the line above; `graft --help` prints usage and exits 0; `SIGINT` kills it by signal with the terminal restored; under `NO_COLOR=1` the only SGR left is the reset (`ESC[m`, `ESC[0m`) with no colour parameters at all, and the theme line reads `no colour (NO_COLOR)`. Nothing answered the probe in either pty, so the tier shown was the silent-terminal fallback and no real emulator's rendering of this screen has been seen | The M0 checkpoint: run it in Ghostty, iTerm2 and the Zed/VS Code terminals and look at it | **M0 review** |
| **The code diverges from DESIGN.md in six named places and no ADR records any of them.** The sanitiser takes the whole Unicode `Cf` category plus U+2028/U+2029 where §8.2 lists five ranges, and adds `sanitize_line` (which also replaces `\n` and `\t`) where §8.2 assumed widgets would handle them (m0/05). Detection spells `Capabilities::detect(…)` as the free function `graft_term::detect::detect(…)`, because `Capabilities` lives in `core` and ADR-0007 forbids behaviour there; inside tmux it forces `iterm2_images` false unless the passthrough re-probe answered, gives a protocol-less tmux session Unicode where §4.2's table says ASCII, and spends a second full budget on the tmux re-probe where §4.4 sizes only one (m0/04). Each is defensible and each is currently an undocumented departure from an approved design | The user's ruling at the M0 checkpoint (all four are listed under "Open for the M0 checkpoint" in PROGRESS.md), then an ADR plus the matching DESIGN.md edit for whatever is approved | **M0 review** |
| **Nothing cancels a job yet.** The runner cancels — `Command::CancelJob`, the `Cancel` token, cancel-at-shutdown — but `update` never emits a `CancelJob`, because M0 has no long job to interrupt and no job-aware screen: `AppState` keeps no job ids, so a result that arrives for a screen the user has already left still writes to the status line. Cancellation is proven by the runner's own tests, never by a key press | M1's first rev-walk, which is the first job worth interrupting: `AppState` remembers the id of the job it is waiting for, `Esc` cancels it, and a result whose id is not the awaited one is discarded rather than shown | M1 |
| **A panicking job's crash report is unproven on the real hook.** The suppression path is covered from both ends — `catching` marks the thread (`crash` unit tests) and a panicking job becomes `Event::JobFailed(_, Panicked { .. })` while the pool keeps working (`jobs` tests) — but no test installs Graft's real panic hook, so the hook's early return (write the report, do *not* restore the terminal, hand the path back) has been reviewed and never executed. Installing it would set a process-wide hook and write into `$XDG_STATE_HOME` for every other test in the binary | The M0 review: panic a job by hand in a real terminal, check the screen survives and the status line names a report file that exists | **M0 review** |
| **`gix` is compiled with almost no features.** m0/08 enables only `sha1` and `sha256`. Rev-walks, status, blame and mailmap are off, as are `parallel` and the pack caches, so nothing here says anything about read performance | The ticket that needs each surface turns its feature on and measures it. M1's first rev-walk is the one that decides whether `parallel` and the object caches are on by default | M1 |
| **`graft-graph` and `graft-ops` are empty.** Both are a crate doc comment and nothing else: no types, no functions, no tests. They exist so ARCHITECTURE.md's crate map and m0/02's dependency-edge test are real from the start. Nothing in M0 draws a graph or writes to a repository, and these two crates are where that would live | `graph` fills up in M1 (layout engine and the three renderers); `ops` in M3 (write operations and the undo journal). Until then, a green `cargo test` on the workspace says nothing about either | M1 / M3 |
| **Only the fast test tier exists.** `cargo test` is 268 tests in about 0.6 s warm. PROMPT.md §7.1's heavy tier, §7's `criterion` benchmarks and `cargo-fuzz` targets, and the scripted test corpus of §7.2 have no harness, no fixtures and no feature flag yet — so "gates green" currently means the fast tier only, and no performance or memory budget in the spec has ever been measured. There is also no CI: PROMPT.md §7 item 6 makes that conditional on the project having a GitHub repo, which it does not | M1 builds the fixtures first (PROMPT.md §8) and is the milestone that first has something worth benchmarking. `TESTING.md` and its manual checklist start there too | M1 |

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
