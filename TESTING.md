# Testing Graft

Three tiers. You only need the first one day to day.

| Tier | Command | When | Budget (PROMPT.md §7.1) | Measured |
|---|---|---|---|---|
| **Fast** | `cargo test` | After every change. The gate for every ticket | under 3 min, ~2 GB | 307 tests, **4.3 s** warm |
| **Heavy** | two commands, [below](#the-heavy-tier) | Milestone reviews only | under 10 min, ~3 GB | **1.7 s** with fixtures cached, 6 s when it has to build them |
| **Manual** | [the checklist](#the-manual-terminal-checklist) | Only when you choose to. The M1 and M8 checkpoints | — | about 20 min per terminal pass |

Measured on the machine this was written on (macOS, Apple silicon, `git`
2.51). They are there to make a regression obvious, not as a promise about
your laptop.

## The three gates

Code is not done until all three pass. This is the whole of a builder's
obligation:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Times I left

Every time you leave Graft for the shell, one line here. **This is the only
measurement of the promise** — the fast tier, the heavy tier and the acceptance
walk all test paths we already thought of; a reach for the shell marks the thing
nobody specified. See [ADR-0033](docs/adr/0033-times-i-left.md).

One line: date · what you wanted · what you ran instead. Half a line beats a
forgotten one. Don't triage as you write — the conductor groups repeats at each
checkpoint, and a reason appearing three times is a design gap, not a feature
request.

Entries stay after they're fixed, with the commit that closed them, so the trend
stays visible.

| Date | What I wanted | What I ran instead | Closed by |
|---|---|---|---|
| | | | |

_(empty until M3 lands — M0–M2 are read-only and there was nothing to leave for)_

## The fast tier

```sh
cargo test
```

Everything that runs by default. Small fixtures only (500 commits or
fewer), `proptest` at 64 cases, no network, no fuzzing, no benchmarks. It
builds the [fixture corpus](#fixtures) the first time and finds it cached
afterwards.

**It never runs a heavy test.** Heavy tests are behind the `heavy-tests`
feature, so a default build does not even compile them. To prove it:

```sh
cargo test --test it -p graft-fixtures -- --list | grep -c '^heavy::'   # 0
```

A fast-tier test that takes more than 10 seconds is a bug, not a slow test
(PROMPT.md §7.1).

## The heavy tier

One command (ADR-0018):

```sh
# The 100k-commit repository, the budgets from tickets 27 and 30, and every
# property test at 1,000 cases instead of 64.
cargo test --features heavy-tests -- --test-threads=2
```

The feature alone gates heavy tests; none is `#[ignore]`d. The two 1M
`frames::` tests (m1-the-graph/64) compile only behind the separate
`heavy-1m` feature (ADR-0018 amendment, m1-the-graph/80), so the command
above never reaches them (see [Frames at a
million](#frames-at-a-million)).

**Then the release perf run** (ADR-0021), alone, after the command above:

```sh
# Time budgets fail only here: input to frame (frames::, four tests) and
# first paint (heavy::budgets_hold_*, the 50k-refs one included).
cargo test --release -p graft --features heavy-tests --test it -- frames:: heavy::budgets_hold --test-threads=1 --nocapture
```

In the debug run above those same tests measure and print their numbers
(`debug build, budget recorded not asserted`) without failing; so does
`frames::`'s rows-on-hand check (`rows on hand recorded not asserted: N of M
keys`), since whether layout keeps ahead of a held `PageDown` depends on
build speed too. Peak RSS fails in both builds. Every milestone review runs both and reports the
release numbers.

**Slope guards measure each fixture in a child process** (m1-the-graph/57).
`getrusage`'s peak and `graft_fixtures::footprint`'s peak are both lifetime
maxima of the *process*, never of the work just done — fine for a ceiling
test (`budgets_hold_* < 300 MB` can only over-report), wrong for a slope
(100k peak minus 10k peak): measure both fixtures in the same test binary
and whichever ran second's peak is reported for both, so a real regression
can read as a 0 B/commit false green. `heavy::slopes_per_commit_with_a_commit_graph`
re-execs this test binary once per fixture (`budget_run::gated_in_child`,
`heavy::child_gated_run`) so each peak is that child's own. Confirmed by
running the guard alone and then straight after
`heavy::budgets_hold_on_the_100k_fixture_without_a_commit_graph` in one
invocation: 237 B/commit alone, 235 B/commit adversarial — same within 10%,
and within 10% of `scripts/memory-curve.sh`'s 10k-cg/100k-cg cells (237
B/commit, `.scratch/m1-the-graph/issues/56-memory-curve-acceptance.md`).

**The guard asserts two bars, not one** (m1-the-graph/61, user decision
2026-09-14, ADR-0026 amended): **heap (footprint) slope ≤ 50 B/commit** —
what Graft itself allocates on this path (order, bitsets, promoted, malloc
slack) — and **peak RSS slope ≤ 200 B/commit**, whose floor is named: commit-
graph pages (~56 B/commit, the walk and the id table's own file) plus pack
`.idx` pages (~68 B/commit on these fixtures — every hydration lookup goes
through gix's reader, ADR-0011) plus heap (~35), for a floor of ≈ 160
(measured 166 loose / 156 packed with refs). The single 150 B/commit bar
conflated these: it counted graph pages and heap but not the pack index every
object read needs, so it sat on its own floor and flapped without catching
anything. The RSS bar still catches a real per-commit object read (58's 237
B/commit fails it); the heap bar is the tight one, since 121 of the ~156-166
B/commit are clean file-backed pages the kernel reclaims first under
pressure.

**Frame numbers depend on the CPU's power state** (m1/30c part b). On the
build machine (Apple Silicon), a scroll paced at 60 Hz is light load, and
the CPU stays in a slow state until a heavier burst: the first key that
asks for rows past those on hand (key 37 of the script: a `j` that costs
its own frame, a progress frame and the rows' frame back to back) measured
12–25 ms in release before m1/30c–30h cut the per-key cost (after: 7 of 7
release runs pass, worst key 14.6 ms), and so did the first key after
≥ 0.5 s idle (still 14 of 140 over 16 ms at 50k refs, max 19.1 ms; printed
as `idle_wake_us`, not asserted). With one
core kept busy by an unrelated spinning thread the same key took
6.5–6.8 ms and p99 fell from 12–15 ms to 7 ms. Close other work before the
release run; a real user's first keypress after reading the screen pays
the same ramp.

Also lint the heavy code, which the standard gate does not see — a feature
that is off is not compiled, so `clippy --all-targets` says nothing about
it:

```sh
cargo clippy --all-targets --features heavy-tests -- -D warnings
```

**The case count is fixed in code: 64 fast, 1,000 heavy** (ADR-0017).
`PROPTEST_CASES` is not read, so there is nothing to unset and no way for
your shell to weaken a review:

```sh
cargo test --features heavy-tests -- --test-threads=2
```

To hammer one property while chasing a failure, edit its config for the
run — `#![proptest_config(ProptestConfig::with_cases(10_000))]` — and
revert it afterwards.

`--test-threads=2` is the heavy tier's own cap (the fast tier's 4 comes
from `.cargo/config.toml`). Do not raise it, and do not run two cargo
commands at once.

### What is in the heavy tier today

- The synthetic repository at 100,000 commits, and at 5,000 — the size
  that proves the generator is *correct*, since it builds the same trunk,
  merges, open lanes and tags, only fewer of them.
- The same history with a commit-graph, as a separate fixture: ADR-0008
  makes first-paint order depend on whether one exists, so the two are two
  measurements, not one.
- Property tests at 1,000 cases.

### Memory curve to a million commits (m1/46, re-run m1-the-graph/56c)

`scripts/memory-curve.sh [sizes…]` — release, one process at a time, 3 runs per cell, raw in `docs/shots/m1-memory-curve.txt`. Builds 1M fixtures on first run (~30 s and ~300–360 MiB each). Not part of `cargo test`: run by hand at reviews. Bar: peak RSS < 300 MB with a commit-graph (ADR-0026); without one at 1M: reported, footprint < 300 MB.

Full re-run, 2026-09-14, release, 3 runs/cell (peak RSS = process max RSS; footprint = `time -l` peak memory footprint, anonymous pages):

| Commits | small, no cg (peak RSS / footprint) | small, cg (peak RSS) | realistic, no cg (peak RSS / footprint) | realistic, cg (peak RSS) | Bar |
|---|---|---|---|---|---|
| 10k | 17.0/15.8/15.7 MiB / 10.5/9.4/9.3 MiB | 12.1/12.0/12.1 MiB | 24.6/24.5/24.6 MiB / 17.5/17.4/17.5 MiB | 15.9/16.0/16.0 MiB | < 300 MB — pass |
| 100k | 68.1/67.6/68.0 MiB / 36.0/35.5/36.0 MiB | 26.8/26.6/26.8 MiB | 75.6/75.8/76.1 MiB / 37.4/37.6/37.9 MiB | 30.5/30.5/30.3 MiB | < 300 MB — pass |
| 1M | **445.5/445.2/444.7 MiB / 155.6/155.3/154.8 MiB** | 174.3/173.5/174.3 MiB | **507.5/508.5/508.8 MiB / 156.7/157.6/157.9 MiB** | 183.2/182.6/183.7 MiB | cg < 300 MB — pass; no-cg — see below |

Pack on disk: 10k 2–3 MiB, 100k 29–35 MiB, 1M 297–358 MiB. Checkpoint stride (m1-the-graph/60): 2,000 at 10k/100k, 16,000 at 1M, every variant. `overflow_len`: commit count without a commit-graph, 0 with one, every cell.

**Without a commit-graph the 1M cell asserts two bars, not one** (m1-the-graph/59/59b/63, user decision 2026-09-14, ADR-0026 amended): **live heap slope ≤ 100 B/commit, gated at reviews** — read from `prototypes/heap-profile-59` at 100k and 1M (manual, not part of the curve script; raw `target/59b/prof-{100k,1M}.txt`); measured **76 B/commit** (100k 27.0 MiB → 1M 92.5 MiB live after `g`; interner 58 + one order 8 + promoted 4 + rest) — and **footprint at 1M: reported, floor named** — read from this table's `no cg` footprint column; measured **~138–140 B/commit** (76 live + ~62 allocator-retained memory, freed but kept in the process's footprint), against the existing "footprint under 300 MB at 1M" (154.8–157.9 MiB, well under). The floor is not gated further: see KNOWN_GAPS.

The commit-graph offer, end to end, on a copy of the 1M small no-cg fixture (`docs/shots/m1-memory-curve-offer.txt`): before `B`, reported cell as above; `B` writes the graph in 5.3 s wall; after `B`, the same repository re-measured as a cg cell — peak RSS 173.4/172.8/173.9 MiB, footprint 36.3–37.5 MiB, far jump to row 500,000 14.9 ms (4,500 rows, stride 16,000) — gate pass.

**Far jumps after a cold `G`, both 1M fixtures** (m1-the-graph/56, 60 — checkpoints thin to an even grid, never LRU): row 500,000 and row 900,000 both replay 4,500 rows in 13–16 ms at stride 16,000, every 1M variant (small/realistic × cg/no-cg). Before m1-the-graph/60's grid-thinning: row 500,000 replayed 500,500 rows in 1.39–1.46 s (LRU held only the last 128k rows' checkpoints).

First paint per cell (m1-the-graph/55, `target/release/examples/budgets <fixture>`, release, with jobs; run 1 is cold, runs 2–3 warm — page cache not purged between runs). Judged 500 ms warm, reported against 150 ms (ADR-0026); cold run 1 reported beside it:

| Commits | small, no cg (run1/2/3) | small, cg (run1/2/3) | realistic, no cg (run1/2/3) | realistic, cg (run1/2/3) |
|---|---|---|---|---|
| 10k | 23.3/2.5/2.4 ms | 11.0/2.2/2.1 ms | 13.5/2.5/2.5 ms | 11.2/2.1/2.2 ms |
| 100k | 95.9/11.7/11.9 ms | 16.4/12.5/10.8 ms | 108.5/12.2/13.8 ms | 79.8/10.1/10.0 ms |
| 1M | **997/112/104 ms** | **710/91/90 ms** | **1,032/108/111 ms** | **790/88/91 ms** |

Every 1M cell's run 1 is cold (~0.7–1.0 s, over 500 ms); warm runs 2–3 are 88–112 ms, well under 150 ms. Cause suspected (page cache: the pack `.idx` and loose refs not resident after a build or another 1M cell's I/O), not proven — needs `sudo purge` between runs (KNOWN_GAPS).

**Cold first paint, manual (ADR-0027).** Only `sudo purge` can empty macOS's
page cache; no test can drive it. Run once, at the M1 checkpoint:

```sh
sudo purge
scripts/memory-curve.sh 1000000
```

Read run 1's `first paint` line for the `1000000 small cg` cell (the
`docs/shots/m1-memory-curve.txt` it writes) and compare it to the ~1.0 s cold
number above; runs 2–3 in the same invocation should fall back to the warm
88–112 ms range, confirming the difference is the page cache and not Graft.

| Check | Time | Matches the ~1.0 s cold number? | Notes |
|---|---|---|---|
| First paint, 1M commit-graph fixture, straight after `sudo purge` | not run | | |

Not there yet: `cargo-fuzz` targets, land in M2.

### Frames at a million (m1-the-graph/64)

The `frames::` scroll harness (input-to-frame budget, `INPUT_TO_FRAME =
16 ms`) at 1M commits, deferred by ADR-0026. Manual like the memory curve
(the fixture is ~300 MiB) — but a `#[test]` fn, not a script, so it is
gated behind the `heavy-1m` cargo feature (ADR-0018 amendment,
m1-the-graph/80) instead: both standing commands above build `heavy-tests`
alone, so the two 1M tests are never even compiled into them.

```sh
cargo test --release -p graft --features heavy-1m --test it -- scrolling_the_1m_commit_graph_fixture --test-threads=1 --nocapture
```

What right looks like: p99 ≤ 16 ms at both tiers on the 1M small
commit-graph fixture, same as the 100k cells above. Release only (ADR-0021);
debug records and prints without failing.

3 runs each, release, alone, 2026-09-14:

| Tier | Run | p50 (µs) | p99 (µs) | max (µs) | rows on hand | kitty bytes |
|---|---|---|---|---|---|---|
| Unicode | 1 | 4,468 | 6,745 | 10,219 | 0 waited | 0 |
| Unicode | 2 | 4,725 | 6,374 | 6,688 | 0 waited | 0 |
| Unicode | 3 | 4,115 | 6,622 | 8,023 | 0 waited | 0 |
| Pixel | 1 | 4,182 | 8,005 | 14,346 | 0 waited | 192,996,821 |
| Pixel | 2 | 4,284 | 6,885 | 10,831 | 0 waited | 192,996,821 |
| Pixel | 3 | 4,071 | 7,182 | 8,188 | 0 waited | 192,996,821 |

Both tiers pass on every run: p99 well under the 16 ms bar, no key drawn
before its rows arrived. Raw output: `target/64/{unicode,pixel}-run{1,2,3}.txt`.

## Fixtures

### Where they are

`target/fixtures/`. Two kinds live there:

| Directory | What it is | Built by | Size |
|---|---|---|---|
| `<16 hex digits>/` | The named corpus: one small repository per history or ref shape | The fast tier, automatically | 3.1 MiB |
| `synthetic-5000-no-commit-graph-<hex>/` | The generator's correctness fixture | The heavy tier | 1.8 MiB, 0.25 s |
| `synthetic-5000-commit-graph-<hex>/` | The same history, with a commit-graph | The heavy tier | 2.1 MiB |
| `synthetic-100000-no-commit-graph-<hex>/` | PROMPT.md §7.1's 100k-commit repository | The heavy tier | 35 MiB, 2.1 s |
| `synthetic-5000-realistic-no-commit-graph-<hex>/` | The realistic-commit variant's correctness fixture | The heavy tier | about 2 MiB |
| `synthetic-100000-realistic-{no-,}commit-graph-<hex>/` | The 100k history with commit objects sized after git/git (m1/27d), for the memory budget | The heavy tier | 40 / 46 MiB |

The hex is a digest of the generator's own source. Change the generator
and the next run builds into a directory that does not exist yet, so
nothing is ever stale — and old digests stay behind until you delete
them. That is the one maintenance job:

```sh
rm -rf target/fixtures        # or `cargo clean`; both are safe, they rebuild
```

Set `GRAFT_FIXTURES_DIR` to build them somewhere else — a different disk,
say.

### Building them by hand

Neither is required: a test that needs a fixture builds it. But to build
the corpus on its own, or to see what it cost:

```sh
cargo run -p graft-fixtures
```

The synthetic repository is only reachable from the heavy tier. To build
it without running anything else:

```sh
cargo test --features heavy-tests --test it -- --nocapture \
  the_generator_builds_exactly_the_history_it_promises     # 5,000 commits
cargo test --features heavy-tests --test it -- --nocapture \
  the_heavy_fixture_is_the_size_prompt_md_asks_for         # 100,000 commits
```

Every fixture is built by `git fast-import` from a script, with a fixed
identity and clock and no global or system config, so two builds on two
machines produce the same object ids. They are published read-only; a test
that needs to write copies one into a temporary directory.

### One shape may be missing

`non-utf8-ref-name` needs a filesystem that will accept a file name that
is not valid UTF-8, because a loose ref is a file. APFS will not. On macOS
that fixture is skipped with a recorded reason and the test for it reports
the skip rather than passing quietly. Everything else builds everywhere.

## Getting `git/git`

PROMPT.md §7.2 wants the graph tried against a real, messy, merge-heavy
history that a person can eyeball. That is `git/git`: about 85,000 commits
and a thousand refs.

It is **not a fixture**. It comes off the network, its contents change, and
no test may ever touch it.

```sh
scripts/clone-git-git.sh          # clone, or fetch what is new
scripts/clone-git-git.sh --where  # print the path and exit
```

| | |
|---|---|
| Where | `${XDG_CACHE_HOME:-$HOME/.cache}/graft/git-git.git`, or `$GRAFT_GIT_GIT_DIR` |
| Disk | **42 MB** — a full clone is about 250 MB; `--filter=tree:0` fetches commits and leaves trees and blobs on the server |
| Time | **15 s** to clone, 2 s to re-run |
| Measured | 85,649 commits, 1,017 refs |
| Delete it | `rm -rf "$(scripts/clone-git-git.sh --where)"` |

It is outside the repository and outside `target/`, so `cargo clean` will
not throw away a clone that took minutes, and `git status` will not see
it.

Open it with:

```sh
cargo run -p graft -- "$(scripts/clone-git-git.sh --where)"
```

**The Linux kernel** is manual-tier only and stays that way (PROMPT.md
§7.1). If you want it, the same commands work with a different URL and a
much larger answer — roughly 1.4 million commits, and several GB even with
`--filter=tree:0`:

```sh
git clone --bare --filter=tree:0 \
  https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git \
  ~/.cache/graft/linux.git
```

## The manual terminal checklist

PROMPT.md §7.2 item 7. Run it before each release, and at the M1 and M8
checkpoints. Nothing here is automated: the whole point is that a person
looks at a screen.

**This checklist is a skeleton.** The pass conditions below are final, but
the rows are not filled in until the tiers exist — M1 tickets 08 (Unicode),
11 (pixel) and 14 (tmux and SSH) each fill in their own. A row that has
never been run says **not run**, never "pass".

### What to do in each terminal

1. `cargo run -p graft -- "$(scripts/clone-git-git.sh --where)"` — a real,
   merge-heavy history at a size where mistakes show.
2. Let the first screen paint. Scroll with `j`/`k` for a few screens, then
   to the end.
3. Resize the window — narrower, wider, shorter — while it is running.
4. Quit with `q`.

### Pass conditions

**The graph.** Lanes are continuous: no gap where a line should carry on,
no line that ends in mid-air, no two lanes drawn on top of each other.
Merges join where they happened. Nothing shifts by a pixel or a cell when
the window is resized and put back.

**The terminal afterwards.** Your prompt comes back, colours unchanged,
cursor visible, no leftover escape sequence printed, exit status 0. Same
after `Ctrl-C`.

**Never, in any terminal.** Stray escape sequences on screen. A frame that
paints half-drawn and stays that way. A crash. A hang.

### Pixel tier — must pass

The graph must render as an image, not as characters.

| Terminal | Version | Result | Notes |
|---|---|---|---|
| Ghostty | | not run | |
| Kitty | | not run | |
| WezTerm | | not run | |
| iTerm2 | | not run | |

Pass: the graph is a real image, aligned to the text rows beside it, with
no gap or overlap at the row boundaries, and it survives a resize.

Ghostty, git/git, pixel tier, cursor on the top row: lane 0 is the same
thickness as every other lane and brighter than them; nothing in the graph
is wider than a node ring's stroke.

Ghostty, git/git, pixel tier (ticket 40): each merge node has a short tail
that fades out to its right; the rightmost graph column is a faint dotted
line; no number is drawn in the graph; no curve crosses other lanes to reach
the dotted line.

**Two verdicts per terminal** (user, 2026-09-13). *Pass* is the condition
above — correct graph, or a clean fallback, never a broken screen. *Reads
well* is separate and is the user's alone: is the shape of history obvious
at a glance, the M1 promise (`docs/design-notes/m1-the-graph.md` if present,
else PROMPT.md §7.2/§9). Leave every *Reads well* cell **blank** — a builder
or reviewer never fills it in.

| Terminal | Version | Pass | Reads well | Notes |
|---|---|---|---|---|
| Ghostty | | not run | | |
| Kitty | | not run | | |
| WezTerm | | not run | | |
| iTerm2 | | not run | | |

### Unicode tier — must look deliberate

**This is the default tier** (ADR-0016: Kitty → Unicode → iTerm2 → Sixel →
ASCII). Every terminal here except a Kitty-protocol one draws it.

**Zed and VS Code first — the user's daily terminals.** He lives in these
two day to day; every other row in this table matters for coverage, but
these two are the ones that decide whether M1 shipped something usable.

| Terminal | Version | Pass | Reads well | Notes |
|---|---|---|---|---|
| Zed terminal (daily) | | not run | | |
| VS Code terminal (daily) | | not run | | |
| Terminal.app | | not run | | |
| Alacritty | | not run | | |

*Pass*: box-drawing characters line up into continuous lanes, nothing is a
replacement glyph or a tofu box, and no character is double-width where a
single-width one was assumed. "Looks deliberate" is the bar for *pass*: a
reasonable person seeing only this tier should think it was designed, not
that something failed. *Reads well* (blank for the user, as above): is the
shape of history obvious at a glance. This is the tier most people will
actually live in.

### Does a pixel tier beat Unicode here?

This is the only thing that moves iTerm2 or Sixel above Unicode in ADR-0016's
order. Run it per terminal, answer yes or no, and give the reason.

1. Same repository, same window, same font size, no resizing between runs.
2. `graph.renderer = "unicode"` in the config, `cargo run -p graft -- <repo>`. Look at
   it. Scroll a few screens.
3. Change to `graph.renderer = "pixel"`, run again, look at the same screens.
4. Answer: **is the pixel version better to look at, and does it lose
   nothing?** "Loses nothing" means alignment holds at every row boundary, a
   resize leaves it correct, no stray escape sequences, and the scrollback is
   undamaged after `q`. Any one of those failing is a no whatever it looks
   like.

| Terminal | Protocol | Pixel beats Unicode? | Why |
|---|---|---|---|
| iTerm2 | iTerm2 inline | not run | |
| Ghostty | Kitty | not run | |
| Kitty | Kitty | not run | |
| WezTerm | Kitty | not run | |

A yes here is evidence for a **new** ADR superseding ADR-0016, not an edit to
it. A no is just as much of a result: record it, and the order stands.

### Does VS Code give a usable Sixel tier?

**Open question, and only the user can answer it.** VS Code has a setting,
`terminal.integrated.enableImages`. If turning it on produces a usable Sixel
graph, then one of the two daily terminals is not image-less and ADR-0016's
premise moves. Until this row is filled in, VS Code is treated as a Unicode
terminal.

1. In VS Code settings, set `terminal.integrated.enableImages` to `true` and
   restart the terminal (the setting does not apply to open ones).
2. `cargo run -p graft --example capability_probe` in that terminal. Record
   the `sixel` line and the `protocol` line verbatim.
3. `graph.renderer = "pixel"` in the config, then `cargo run -p graft -- <repo>`.
   Scroll a few screens, resize the window, quit with `q`.

| Question | Answer | Evidence |
|---|---|---|
| Does `DA1` report Sixel with the setting on? | not run | paste the `sixel` and `protocol` lines |
| Does the forced pixel graph draw correctly? | not run | aligned / garbled / nothing drawn; and what the scrollback looks like after `q` |
| Does it beat the Unicode tier there? | not run | the comparison above, run in VS Code |

A yes to all three reopens ADR-0016. A yes to the first two and a no to the
third changes nothing: Sixel stays below Unicode, which is where the order
already puts it.

### Must degrade, not break

| Environment | Version | Result | Notes |
|---|---|---|---|
| tmux (inside a pixel terminal) | | not run | |
| tmux (inside a Unicode terminal) | | not run | |
| SSH (to a Linux host) | | not run | |
| SSH inside tmux | | not run | |

Exact commands (m1-the-graph/29). Every row: first run the probe report and
paste its `tier`, `probe`, `budget`, `tmux` and `ssh` lines into Notes, then run
Graft and walk "What to do in each terminal".

```sh
# Setup, once: tmux >= 3.3, and a git/git clone.
brew install tmux && tmux -V
REPO="$(scripts/clone-git-git.sh --where)"

# tmux (inside a pixel terminal): run from Ghostty or Kitty.
# a) passthrough on: expect `tier Pixel`, `tmux Passthrough { version: "3.x" }`,
#    a pixel graph.
tmux -L graft -f /dev/null new-session \; set -g allow-passthrough on
cargo run -p graft --example capability_probe
cargo run -p graft -- "$REPO"
# b) still inside, split and run in the right-hand pane: pixels must land in
#    that pane, not at the window's left edge. Ctrl-b %, then:
cargo run -p graft -- "$REPO"
# c) passthrough off (tmux's default): expect `tier Unicode`,
#    `tmux PassthroughSilent { .. }`, a clean Unicode graph, no stray bytes.
tmux kill-server -L graft; tmux -L graft -f /dev/null new-session
cargo run -p graft -- "$REPO"

# tmux (inside a Unicode terminal): Zed or VS Code terminal, passthrough on.
# Expect `tier Unicode` and a Unicode graph — never ASCII.
tmux -L graft -f /dev/null new-session \; set -g allow-passthrough on
cargo run -p graft -- "$REPO"

# SSH: needs a host with Rust and a Graft checkout (ssh localhost works:
# System Settings > General > Sharing > Remote Login). Expect `ssh true`,
# `budget 500ms`, and the tier the local terminal would get.
ssh -t HOST 'cd ~/path/to/graft && cargo run -p graft --example capability_probe && cargo run -p graft -- "$(scripts/clone-git-git.sh --where)"'

# SSH inside tmux: start tmux (passthrough on) locally, then the SSH line
# inside it. Expect `tier Pixel` only if the report says `tmux Passthrough`.
```

When done: `tmux kill-server -L graft`. A row passes or fails on the pass
conditions below, never on the probe report alone.

**tmux passes on *either* of two outcomes**: a correct pixel graph, *or* a
clean fall back to the Unicode tier. Which one you get depends on the tmux
version, on `allow-passthrough`, and on the outer terminal, so both are
correct answers. What is *not* a pass: a broken screen, stray escape
sequences, or a graph drawn half in pixels and half in characters. Graft
must detect tmux and choose a tier deliberately rather than hoping
passthrough works.

**SSH passes** when the tier chosen is one the link can carry and the
screen is correct at that tier. A slower probe (500 ms instead of 200 ms)
is expected; a probe that never answers must fall back, not hang.

### Hover and peek reveal (ticket 41, ADR-0023)

There is no popup any more: a truncated subject (the graph column cut it
with `…`) is read from the footer instead, never a box over the graph.

| Check | What right looks like |
|---|---|
| Hover a row whose subject is truncated | Footer swaps its hint bar for that row's full subject, no `…` |
| Hover a row whose subject was never truncated | Footer keeps the ordinary hints; nothing to reveal |
| Move the pointer off the graph, or to a row without truncation | Footer reverts to hints once the mouse moves off the truncated row |
| Press `Enter` on a truncated cursor row (no mouse) | Same reveal as hovering; a second `Enter` closes it |
| Cursor row is truncated, nothing hovered or peeking | Hint bar includes `⏎ full subject`; drops first under width pressure with the other conditional hints |
| Cursor row is not truncated | No `⏎ full subject` hint |
| `Esc` while peeking | Closes the peek before anything else Esc does (jump-withdraw, focus-leave, walk-cancel) |
| Any other key while peeking | Clears the peek (peek is one-shot, not sticky) |
| Revealed subject contains raw escape bytes (sanitized upstream) | Renders inert — no cursor jump, no color leak, same as everywhere else text reaches the screen |

Captures (80x24, release binary), written straight to files:
`docs/shots/41-hover-reveal.txt` (hovering a truncated row),
`docs/shots/41-peek.txt` (`Enter` peek on the cursor row),
`docs/shots/41-no-reveal.txt` (a short subject: hints stay put). Judged by
eye, per the project rule — green tests only prove the reveal logic
agrees with itself.

### `y` copies the SHA (ticket 45, ADR-0023)

| Check | What right looks like |
|---|---|
| Select a commit, press `y` | The bottom line reads `<7 hex>… → clipboard (osc 52)`; paste in another window gives the 40-character SHA. Press `j`: the line is gone. |
| Hint bar at 120 columns | Includes `y sha` after `H stash`; dropped from the right with the others when narrow |
| Inside tmux (`allow-passthrough on`) | Same notice; paste outside tmux gives the full SHA |
| Terminal with OSC 52 disabled | Same notice (Graft cannot tell); paste gives whatever was there before — not a Graft bug |

Automated: `python3 .scratch/m1-the-graph/pty-copy-check.py <release graft> <repo>`
asserts the byte stream holds `ESC ] 52 ; c ;` + base64 of HEAD's SHA and
writes `docs/shots/45-y-copies-sha.txt`. The paste itself is judged by hand
per terminal (Ghostty, Zed, VS Code, tmux) — unticked until tried.

## Graph gallery

M1's acceptance ticket (35). M1 promises that **the shape of history is
obvious at a glance**; the gallery is how a person checks that promise, one
repository at a time, with a stated finding per stop.

```sh
cargo run -p graft --example gallery                      # first stop
cargo run -p graft --example gallery -- --start foxtrot-merge
cargo run -p graft --example gallery -- --list            # the 24 stops
cargo run -p graft --example gallery -- --matrix          # coverage matrix
```

`]` next stop, `[` previous, `q` quit. Every graph key works inside it:
`j`/`k`, `Space` fold merges, `1` first-parent, `H` hide stashes, `F` Focus,
`+` unfold lanes, `y` copy the SHA, `F3` cycle the tier (Pixel → Unicode →
ASCII → back).

A three-line banner sits under the header, above the rows, from the first
frame (ticket 38: the footer alone did not tell a first-time viewer where
they were):

```
 gallery 1/24 trivial · one lane · ] next [ back q quit · unicode
 shows: 3 commits in one straight lane: ◆ HEAD on top, ◇ root at the bottom
 differs: the smallest stop, so only 3 rows show: that is all of it, a baseline
```

Line 1: the stop's number and name, its group, the keys, the tier in
force. Line 2, `shows:`, is what right looks like; line 3, `differs:`, is
what sets the stop apart from its neighbours. Every line fits 80 columns
(a test holds the widths). The stops are ordered as a progression — one
lane, merges, quirks, refs and badges, text, real repos — so `]` changes
one thing at a time; `--list` prints the same lines, grouped. The
banner costs three graph rows. Each stop restarts the run, so expect a
blink per step. The first stop is `trivial`: three commits is the whole
repository, and the banner says so.

Captures of every stop, Unicode and ASCII at 80x24, are under
`docs/shots/35-<stop>-{unicode,ascii}.txt` (from before the banner: the
graph is the same, the footer line is the old orientation);
`docs/shots/38-gallery-first-frame-and-two-steps.txt` shows the first
frame and two `]` presses with the banner, `docs/shots/38-trivial-ascii.txt`
the first frame at the ASCII tier, `docs/shots/38-list.txt` what `--list`
prints. They are what `.scratch/m1-the-graph/pty-gallery.py` and
`pty-gallery-38.py` saw, not what a person saw: the user judges by eye
(ticket 35's last box).

**Stale, knowingly (M2 review round 2, Major 3):** `35-evil-merge-*` predates
the Mistake Radar, so it shows no `⚠` where stop 8 above now says one
appears; and `pty-gallery.py` no longer walks the gallery (its footer never
arrives), so no M1 capture can be retaken until that script is repaired.
Judge stop 8 by running the walk, not by that file.

### Reading the entries

The fixture facts come from `crates/graft-fixtures/src/shapes/`: subjects,
branch names, parent order. "Row" counts from the top of the graph, after
any `Uncommitted changes` row (the fixtures' trees are clean, so none has
one; `graft-self` might).

**Node glyphs, Unicode tier:** `●` commit, `◉` merge, `◇` root (no
parent), `◆` HEAD's commit, `■` the newest stash, `◌` uncommitted changes.
HEAD wins over the others: a merge commit that HEAD points at draws `◆`,
not `◉`. **ASCII tier:** every node is `*`, lines are `| - / \ +`; the shape
distinction survives only in the badges. **Pixel tier:** filled disc, ringed
disc (merge), hollow circle (root), a larger disc (HEAD), a square (stash).

**Badges, plain icons (the default):** `[◆ main]` the current branch,
`[◆ main ⇄ origin]` in sync with its upstream, `[ahead ↑1]`, `[behind ↓1]`,
`[origin/main]` remote, `[tag: v0.1]`, `[HEAD]` when detached, `[stash]`.
With `ui.icons = "nerd"` the brackets become icons.

**Colour:** each lane's colour is a hash of its branch name, not its
column; the second-parent curve into a merge is drawn in the merged
branch's colour, and the branch's badge shares it. So "in feature's colour"
means: the same colour as the `[feature]` badge.

**Straight lanes:** the default branch (`main` here; `master` in git/git) is
lane 0, the leftmost column, and its first-parent chain never leaves it.

**Across tiers (`F3`):** the rows, badges, subjects and the column each
lane occupies are identical in all three tiers; only the glyphs change.
Press `F3` at any stop: the text right of the graph must not move.

### The stops

1. **trivial** — 3 rows in one column: `◆ Correct the note` with `[◆ main]`,
   `● Add the first note`, `◇ Add a readme` at the bottom. No tag, no curve,
   no second column. `Space`, `1`, `H` change nothing.
2. **linear** — 200 rows, `Commit 200` (`◆`, `[◆ main]`) down to
   `Commit 001` (`◇`). One column the whole way: press `G` then `g`, and
   `PageDown` through it; the node column never shifts left or right and no
   row shows a second lane.
3. **feature-branches** — 9 commits. `main` is lane 0. Two merge nodes on
   main: `Merge branch 'feature/two'` (top, `◆`, `[◆ main]`) and
   `Merge branch 'feature/one'` (`◉`). Into each, one curve comes in from
   the right in that feature's colour; each curve reaches back to
   `one: finish` / `two: finish` (`[feature/one]`, `[feature/two]`). `three: in
   progress` (`[feature/three]`) sits in its own lane with no curve into main:
   its line runs down to `Add b.txt` and ends there, open at the top.
   `Space`: both merged branches fold, `feature/three` stays. `1`: only the
   main column remains (5 rows: two merges, Add b, Add a, Add a readme).
4. **octopus-merge** — 5 commits. The top row `Merge branches 'topic-1',
   'topic-2' and 'topic-3'` (`◆`) has **three** curves coming in, one per
   topic, each in its own colour matching `[topic-1]`, `[topic-2]`,
   `[topic-3]`. The three topic rows each carry one node and one badge.
   `◇ Add the base` at the bottom is where all four lines join. `1`: two
   rows only (the merge and the base).
5. **criss-cross-merge** — 7 commits. `alpha: merge beta` and `beta: merge
   alpha` are both `◉`, one in alpha's lane, one in beta's, each with a curve
   from the other's lane: the two curves **cross**. Above them `Merge branch
   'alpha'` (`◆`, `[◆ main]`) in lane 0 takes one curve from alpha; `beta:
   carry on` (`[beta]`) continues beta's lane above the crossing. Select
   `alpha: merge beta` and press `F`: beta's `carry on` row fades, both
   `Add the base` and `beta: work` stay lit (they are ancestors).
6. **foxtrot-merge** — 4 commits, the shape to compare with 7. The merge
   `Merge branch 'main' into feature` is main's tip (`◆`, `[◆ main]`) but its
   **first** parent is `feature: work`, so the merge node sits in feature's
   column, not lane 0, and main's own line (`Advance main`, `Add the base`)
   is what curves **into** it. Press `1`: the first-parent walk leaves lane 0
   — `Advance main` disappears and `feature: work` stays. That is the
   foxtrot; a normal merge (stop 3, stop 8) keeps the merge node in lane 0
   and `1` keeps every main commit.
7. **backward-merge** — the same four subjects as 6, but the merge is
   `feature`'s tip, not main's. `[feature]` sits on the `◉` merge row in
   feature's column; `[◆ main]` sits on `Advance main` (`◆`) in lane 0,
   straight down to `Add the base`. Main's line curves into the merge; lane 0
   itself never bends. `1`: main shows both its rows, unchanged.
8. **evil-merge** — 4 commits, a normal merge: `Merge branch 'alpha'` (`◆`,
   `[◆ main]`) in lane 0, one curve in from `alpha: work` (`[alpha]`) in
   alpha's colour, `Advance main` below in lane 0. The cursor starts on the
   merge and the Mistake Radar probes the selected row with the pane closed,
   so a `⚠` appears beside the merge's `◆` a moment after the stop paints:
   the merge invented `evil.txt`, which neither side touched. No other row
   carries a marker, and nothing else about the merge draws differently —
   the shape is stop 3's. Stop 10 of the **details** walk opens the same
   finding in the pane.

   *Until round 1e the text here said nothing marks the merge, which was
   true only before the probe followed the selection (M2 review round 1a).*
9. **accidental-pull-merge** — 4 commits. The top row is a merge (`◆`,
   `[◆ main]`) whose subject starts `Merge branch 'main' of
   https://example.invalid/graft.git into main`; its curve comes from `Their
   commit`, which carries `[origin/main]` and no local branch. `My commit`
   sits below in lane 0.
10. **orphan-branch** — 4 commits, two roots. `main`'s two rows (`◆ Add the
    code`, `◇ Add a readme`) in lane 0. `docs: second page` (`[docs]`) and
    `docs: first page` in a second column; `docs: first page` is a `◇` with
    **no line below it** and no line ever joins the two columns.
11. **cherry-pick-duplicates** — 4 commits. `Fix the greeting` (`◆`,
    `[◆ main]`) in lane 0 and `Fix the greeting (cherry picked)` in topic's
    column below `topic: unrelated work` (`[topic]`). No curve joins the two
    fix rows; both lines run down to `◇ Add the greeting`.
12. **skewed-dates** — 15 commits. `Commit from a fast clock` is dated a
    day ahead of everything above it. Topological order wins: it sits
    **below** `Main 01` (directly above `◇ Add the base`), never above
    `Main 12`. `feature: build on the fast commit` (`[feature]`) branches off
    it in a second column.
13. **relative-dates** — one straight lane, 7 commits, oldest at the bottom.
    The When column reads `now`, `2m`, `2h`, `2d`, `2w`, `2mo`, `1y` top to
    bottom — one commit sitting right at each unit's own threshold (5
    seconds; 90 seconds; 90 minutes; 36 hours; 14, 70, 400 days ago), so
    every unit the column can show appears exactly once, in order. Viewed
    at `graft_fixtures::CORPUS_NOW`, a fixed instant (m1-the-graph/53), not
    the wall clock: the same seven units, any day, any machine — the
    capture never goes stale.
14. **tags** — 4 rows, one tag each, reading down: `[◆ main] [tag: v2.0]` on
    `Add c.txt`, `[tag: v1.0]` on `Add b.txt`, `[tag: v0.2]` on `Add a.txt`,
    `[tag: v0.1]` on `◇ Add a readme`. `v1.0` and `v2.0` are annotated: their
    badges sit on the commit rows (the tag object is peeled), and read the
    same as the lightweight ones.
15. **ref-states** — 8 commits. `Advance main again` carries
    `[◆ main ⇄ origin]` (one merged badge, no counts). `ahead: not pushed
    yet` carries `[ahead ↑1]` with `[origin/ahead]` one row below it.
    `behind: theirs` carries `[origin/behind]` and, one row below, `behind:
    shared` carries `[behind ↓1]`. `Archived work` carries `[tag: archived]`
    and no branch badge, in its own column off `◇ Add the base`.
16. **detached-head** — 4 rows, one column. `[main]` (no `◆`: it is not
    the current branch) on `Commit 4`, then `Commit 3`, then `◆ Commit 2`
    with `[HEAD]`, two rows below main's tip; `◇ Commit 1` at the bottom.
17. **unborn-branch** — 3 commits. HEAD is the unborn branch `fresh`: no
    row is `◆`, and no `Uncommitted changes` row appears at the top. `Add
    a.txt` (`[main]`) and `other: work` (`[other]`) in two columns above
    `◇ Add a readme`.
18. **stash** — top row is the newest stash: `■` with `[stash]`, subject
    `On main: wip: with an untracked file`, one row **above** `◆ Add a.txt`
    (`[◆ main]`). Press `H`: the stash row disappears, `Add a.txt` becomes
    the top row, nothing else moves. `H` again brings it back.
19. **unicode-names** — 5 commits, author `Ünïcödé Fixture`. Three branch
    rows off `日本語のコミットメッセージ`: `[fonctionnalité]`, `[機能]`,
    `[ветка]`, each in its own column. The `☕` and `🚀` subjects and the
    CJK badge take two cells per glyph, yet the author, date and SHA columns
    on those rows start in the same screen column as on every other row.
    `[tag: версия-1.0]` sits on the `日本語` row next to `[◆ main]`.
20. **non-utf8-path** — 2 columns: `Add a Latin-1 path` (`[latin1]`) and
    `◆ Add a readme` (`[◆ main]`). The bad byte lives in a tree path, so no
    `\u{fffd}` or stray byte appears anywhere on screen.
21. **non-utf8-ref-name** — builds only where the filesystem takes a Latin-1
    file name (not APFS): on a Mac the stop reads `not available — build
    with: …`. Where it builds: `Work under a Latin-1 branch name` carries a
    badge reading `caf�` (one replacement character), `Add a readme` carries
    `[◆ main]`, and nothing else on the row is disturbed.
22. **graft-self** — this checkout, ~100 commits at M1's end. One column the
    whole way, every node `●` except `◆` at HEAD and `◇` at the bottom; no
    curve anywhere. An `Uncommitted changes` row appears at the top only if
    the tree is dirty.
23. **synthetic-100k** — present only after the heavy tier has built it
    (`--matrix` and the stop itself say `not available — build with: …`
    otherwise). With it: the first frame paints without a visible wait; more
    than eight parallel lanes fold behind a `⋮` rail, the header counting them
    (`+N lanes`; `+` unfolds them);
    `j` held down and `G` never stall; `1` collapses to the default branch's
    single column.
24. **git-git** — `scripts/clone-git-git.sh` first. `master` is lane 0 and
    stays in the leftmost column for the whole history (`G` to the bottom
    and back). Merge rows are `◉` with a curve coming in; release commits
    carry `[tag: v2.x.y]` badges. `1` leaves a single column of merges and
    direct commits; `Space` folds each merge's side commits behind it.
    **At 80×24 (ticket 39, 47):** on the first frame the header ends `+N lanes`, N ≥ 10 (17 on the
    2026-09-13 clone); each merge
    row whose second parent is folded shows `◉▾` (stub right of its node); the
    last graph column is `⋮` on every row; no `+` or digits in any row's graph
    column; no curve reaches the `⋮` column. `+` expands: rail and count gone.
    After `j j` (ticket 47, ADR-0025 — the cursor's move onto a merge train no
    longer pins its whole thread against folding): the header still ends
    `+N lanes` with N ≥ 10, and the graph column and rail are not cut to half
    the terminal's width.
    **The cap follows width (ticket 44):** git/git at 80 columns: 7 lanes and
    the rail; at 160: 11 and the rail. `graph.max_lanes = 6` in config: 5 and
    the rail at both widths. `graph.max_lanes = 30` at 80 columns: 19 and the
    rail, nothing cut (the screen's own bound, not the configured number,
    wins).
    **many-lanes (ticket 39/47's synthetic corpus, fourteen topics merged one
    after another):** at 80×24 the first frame reads `+6 lanes` (9 node
    columns and the rail drawn: lane 0 and topics 14–7 hold nodes in the
    window; topics 6–1 fold); at 120×40 nothing folds and no rail (every
    column holds a node in the 38-row window — correct, not a gap).

    **Pins are priorities, not vetoes (ticket 47, ADR-0025):** the merge
    train's own thread column, the reserved integration lanes and the
    cursor's own column are budget priorities — they fold too once the
    budget can't hold them. `docs/shots/47-git-git-80x24-after-jj.txt`: the
    header still ends `+N lanes` (N ≥ 10) after `j j` lands on a merge, and
    the rail sits well inside the terminal's width, not cut at half of it.

### Per terminal: walk the gallery

Procedure, in each terminal of the checklist above: `cargo run -p graft
--example gallery`, then `]` through all 24 stops, checking each against its
entry; at stops 3, 5, 6 and 18 also press `Space`, `1`, `F` (`Esc` to leave)
and `H` as the entry says; at stop 24 press `F3` three times and scroll.
The pass conditions are the ones above:

- **Pixel tier, must pass** (Ghostty, Kitty, WezTerm, iTerm2): the 23
  entries hold with the pixel glyphs; strips line up with their text rows at
  every stop, and the octopus (4), criss-cross (5) and foxtrot (6) curves are
  smooth, not stepped.
- **Unicode tier, must look deliberate** (Terminal.app, Alacritty, the VS
  Code and Zed terminals): the 24 entries hold with the glyphs listed under
  "Reading the entries"; at stop 19 nothing is a tofu box.
- **Must degrade, not break** (tmux, SSH): either a correct pixel gallery or
  a clean Unicode one, per the tmux/SSH block above; every stop's footer
  line reads in full.

Record the result in the tier tables above, one line per terminal, naming
the stop that failed if one did.

### Out of scope

Windows terminals (PROMPT.md, Mission; ADR-0005). Not tested, not
supported in v1.

## Details and diffs

M2's acceptance ticket (m2-details-and-diffs/23). M2 promises that **what a
commit changed is readable without leaving the graph, and a bad merge is
flagged and explained**; the `details` gallery group is how a person checks
that, one stop at a time, each stop a repository plus the keys the harness
presses before its first frame.

```sh
cargo run -p graft --example gallery -- --group details                 # stop 1
cargo run -p graft --example gallery -- --group details --start foxtrot-merge
cargo run -p graft --example gallery -- --list                          # both groups
cargo run -p graft --example gallery -- --stems                         # capture stems
cargo run -p graft --example gallery -- --matrix                        # both tables
```

`]` next stop, `[` previous, `q` quit; the stop's keys are already pressed
when its frame appears, and every Graft key works after that. The banner's
first line names the keys where M1's names the group:
`gallery 3/19 feature-branches · Enter Tab v · ] next [ back q quit · unicode`.
Each stop restarts the run, so the prelude is pressed once per visit.

Captures: `docs/shots/m2-<nn>-<stop>-<keys>-<WxH>-<tier>.txt`, every stop at
160x40 and 80x24 Unicode, stops 1, 6, 7, 15, 16 also at 100x30 Unicode and
80x24 ASCII (`.scratch/m2-details-and-diffs/pty-gallery-m2.py`). They are
what pyte saw, not what a person saw: **the user judges by eye**.
`m2-10-evil-merge-enter-*` predates the probe following the selection (round
1a): at 160x40 it already shows the `⚠` on the merge row, but with the pane
**open**, which is the only case that ever worked; the pane-closed marker
stop 8 now describes has no capture in either group. Retake both with the
script above when the walk is next run. Two
capture-side facts to know: at 80x24 the pane is a tab strip and the
`Detail` tab hides the banner, so those files name their stop only in their
first line; and the Map view (stop 15) draws no banner at any size — its
`MAP · main · last 1 day` header and `M/Esc back` footer are the stop's
identity.

### What the walk proves and what it does not

- **The evil-merge probe ran against live git here.** The gallery runs the
  interactive path with the real job runner; stop 10's `· evil merge` came
  from `git` reading the merge's trees, not from an injected verdict (ticket
  14's own capture had one injected). This walk is the first live exercise
  of that arm.
- **The force-push probe (ticket 15) has no stop and was not exercised.**
  No fixture has a rewritten remote; `docs/shots/m2-force-push.txt` is still
  the only evidence, and it is a TestBackend capture. Nothing in this walk
  implies otherwise.
- **No corpus commit changes two files**, so the file-cursor stop (2) shows
  `j` doing nothing rather than the second file's diff; stop 18 (git/git)
  is where a multi-file tree is seen.
- A compare's header carries no `· N files +A −B` yet (KNOWN_GAPS).

### Reading the entries

Fixture facts come from `crates/graft-fixtures/src/shapes/history.rs` and
the M1 entries above; the fixture SHAs are stable (`0683985` is `three: in
progress` in every build). "Row" counts from the top of the graph.

**Marker column:** one cell between the graph and the badges, present only
when some row on screen has a finding: `⚠` (ASCII `!`) a warning, `ⓘ`
(ASCII `i`) information. The pane's subject row appends the finding's name
after `·` (`Merge branch 'alpha' · evil merge`); a ref finding sits on the
`Contained in:` line's row (`no upstream (feature/three)`). Branch tips
without an upstream carry `ⓘ` from the first stop on: the marker column is
not exclusive to merges.

**Pane header, four lines at 160 columns:** `COMMIT <sha>`; author `<name>
<email> · <date> · <age>`; `parents <sha>[, <sha>] · diff vs first parent`;
`Contained in: <branches>`. A message with a body adds a `✉ message (N
lines)` row (stop 18); a one-line message has no `✉` row (stop 17).

**Files tree:** one row per file, `A`/`M`/`D`/`R`/`C`/`T` then the path,
`+N −N` right-aligned (`bin` for binary); directories fold with `▾`. The
highlighted row is the file whose diff is below.

**Diff body:** gutter line number, sign (`+`/`-`/space), text; no `@@`
heading for a single-hunk file. Side-by-side (`v`): two halves, `│` between,
each with its own gutter; an unpaired side is blank. Word tint is colour
only, on by default: at 256 colours the changed words carry a stronger
background inside a changed line; at 16 colours the whole line carries the
tint and words do not differ. `W` toggles it off (line colour — added,
removed, context — is unaffected either way); pressing it again turns it
back on. A capture cannot show either state; judge in the terminal.

**Compare:** `x` marks the base (`▏` on its row, footer `compare base <sha>
— move, x again clears`); `x` on another row opens the pane with
`COMPARE <base> → <head>` in place of `COMMIT`.

**Strip (`O`):** one column at the right edge, one cell per bucket of rows,
by priority `⚠` > `ⓘ` > `◆` search hit > `●` HEAD > `◉` merge > `█` rows on
screen > `┆` rows off screen with nothing to mark; blank = not walked yet.
ASCII: `! i + * o # :`.

### The stops

1. **feature-branches · Enter** — row 0 is `three: in progress` (feature/three's
   tip sits above `Merge branch 'feature/two'`), so the pane opens on
   `COMMIT 0683985`: `Graft Fixture <fixture@example.invalid> · 2020-01-01
   00:09 +0000`, `parents de95041 · diff vs first parent`, `Contained in:
   feature/three`, then `no upstream (feature/three)`. Files: one row,
   `A three.txt +1 −0`, highlighted. Body: `1 + a`. Row 0 carries `ⓘ`; so do
   `two: finish` and `one: finish` (no upstream). Footer has `. explain`.
2. **feature-branches · Enter j** — the same pane as 1, byte for byte: one
   file, so `j` inside Files has nowhere to go. Header, tree and body do not
   move; the graph row stays on `three: in progress`.
3. **feature-branches · Enter Tab v** — the body splits: `│` down the middle,
   the left half blank (no old side for an added file), the right half
   `1 + a` with its own gutter. Header and tree as in 1.
4. **feature-branches · Enter W** — same text as 1; in the terminal `W`
   turns the word tint off: the `a` on the `+` line loses its stronger
   background and reads like the rest of the line (256 colours). The
   line's own added/removed colour is unaffected; nothing else changes.
5. **feature-branches · x j j x** — `x` on row 0, two `j`, `x` on `two: finish`:
   the pane header reads `COMPARE 0683985 → e30e870` where `COMMIT` would
   be; no author or parents lines; `▏` on row 0. No file count in the
   header (KNOWN_GAPS).
6. **foxtrot-merge** — the marker column shows `⚠` on `Merge branch 'main'
   into feature` (row 0, `[◆ main]`) and `ⓘ` on `feature: work` (`[feature]`,
   no upstream); `Advance main` and `Add the base` unmarked. No pane. Footer
   has `. explain`.
7. **foxtrot-merge · .** — a four-item menu over the graph, `Explain`
   leading: `Explain`, `Copy SHA`, `Compare with this`, `Open details`.
   The cursor row has a finding (stop 6's `⚠`), so `open_for_history_row`
   asks `view::radar::findings_for` and lists `Explain` first
   (m2-details-and-diffs/24). `Esc` closes the menu.
8. **backward-merge** — `ⓘ` on `Merge branch 'main' into feature` (`◉`,
   `[feature]`); `Advance main` (`[◆ main]`) and `Add the base` unmarked.
   The severity is info, not warning: the marker is `ⓘ`, not `⚠`.
9. **accidental-pull-merge** — `ⓘ` on `Merge branch 'main' of
   https://example.invalid/graft.git into main` (row 0, `[◆ main]`); `Their
   commit` and `My commit` unmarked.
10. **evil-merge · Enter** — `⚠` on `Merge branch 'alpha'` (row 0); the pane's
    subject row reads `Merge branch 'alpha' · evil merge`, `parents 0631550,
    bd20b78`, `Contained in: main`; files `A alpha.txt +1 −0` and
    `A evil.txt +1 −0`. In the terminal the subject row passes through
    `unchecked` and `checking…` first; the capture holds the last state.
    `alpha: work` carries `ⓘ` (no upstream).
11. **cherry-pick-duplicates** — `ⓘ` on both `Fix the greeting` (`[◆ main]`)
    and `Fix the greeting (cherry picked)`; `topic: unrelated work` also
    carries `ⓘ`, for no upstream, not for the duplicate. `Add the greeting`
    unmarked.
12. **ref-states** — no marker column at all: every branch has an upstream
    (`main ⇄ origin`, `ahead`→`origin/ahead`, `behind`→`origin/behind`) and
    `ahead: not pushed yet` is not on the default branch, which is the only
    ahead state the radar flags. A negative control.
13. **unicode-names · Enter** — the pane opens on `ветка: первый коммит`
    (row 0); files `A документ.txt +1 −0` with `+1 −0` in the same column
    as every other stop's counts; body `1 + содержимое`. The CJK badge rows
    keep their SHA column aligned.
14. **non-utf8-path · Enter** — `COMMIT` on `Add a Latin-1 path`, `Contained
    in: latin1`; the tree reads `A caf�.txt +1 −0`, exactly one `�`; the body
    loads (`1 + latin-1 path`).
15. **feature-branches · M** — no graph, no banner: `MAP · main · last 1 day`,
    a `BRANCH … NOW` header, then `▸ main` with `█` and `●` at the right,
    `feature/three` with `▁`, `feature/two` and `feature/one` each `●╮`
    (the elbow: merged into main). Footer `↑↓ select ⏎ zoom to commits
    M/Esc back`. ASCII: `@` for `█`, `.` for `▁`, `o\` for `●╮`.
16. **feature-branches · O** — a one-cell column at the right edge, rows 4
    to 40: `█` over the rows on screen, `◉` twice (the two merges), `●` for
    HEAD near the top, `ⓘ` where a no-upstream tip sits. The graph is
    unchanged but one column narrower. `O` again removes it.
17. **graft-self · Enter** — this checkout's HEAD: `COMMIT <sha>`, your name
    and email, `parents <sha> · diff vs first parent`, `Contained in: main`
    (more branches if any contain it); no `✉` row unless HEAD's message has
    a body. Files are real paths with `M`; the body carries syntax colours
    (Markdown or Rust roles) — colour only, judge in the terminal.
18. **git-git · /v2.41.0-rc0⏎ Enter** — `scripts/clone-git-git.sh` first. The
    search lands on `Merge tag 'v2.41.0-rc0'` (`1 match (1/1)` in the
    footer), `Enter` opens it: `parents f66ad35, 0df2c18`, `Contained in:
    bisect, jch, maint, master, next, seen, …`, `✉ message (24 lines)`, files
    from `M .mailmap +1 −0` and a folded `▾ Documentation/`. Under a pty the
    last repaint came 3.3 s after the first frame (`docs/perf/
    acceptance-details-and-radar.txt`, debug build). Then `j` down to a long
    file and `L`: the `… N more lines · L load all` row disappears and the
    rest loads. (`L` on `.mailmap` does nothing: one line.)
19. **git-git · O** — the strip on 85k rows: `◉` and `ⓘ` in the walked
    buckets at the top, `┆` where rows exist without a mark, and blank
    cells below the walked range that fill as the header's `N rows so far`
    grows. Never all at once.

**Working-tree row** (no stop: the fixtures are clean) — in a dirty checkout
of graft itself, `k` from the top row lands on `Uncommitted changes (…)`,
its highlight moves and HEAD's row un-highlights; `Enter` opens
`UNCOMMITTED · N staged N unstaged N untracked` / `diff vs HEAD <sha>`, staged
and unstaged files listed, an untracked file's body reads `untracked · no
diff yet (M3)`; `j` returns to HEAD's commit and the pane follows. Captures:
`docs/shots/m2-working-tree-row.txt` (graph),
`docs/shots/m2-detail-pane-breakpoints.txt` (the `UNCOMMITTED` pane).

### Per breakpoint

Stop 1 at three sizes (`m2-01-feature-branches-enter-*`):

- **160+ columns:** the pane is a right column from row 3 down; the graph
  keeps its badges and SHAs; the banner stays on the left.
- **100–159:** the pane sits **below** the graph: graph rows to about the
  middle, then the header folded to one line (`COMMIT 0683985 · Graft
  Fixture …`), tree and body under it. Banner still on top.
- **< 100:** a tab strip `Graph  [Detail]` replaces both; the `Detail` tab
  is the whole screen (`COMMIT`, author, `three: in progress · no upstream
  (feature/three)`, tree, body) and the banner is not visible. `Tab` back
  to `Graph` shows the banner and the marker column.

### Per terminal: walk the details group

Two lines added to the M1 procedure above: `cargo run -p graft --example
gallery -- --group details`, then `]` through the 19 stops at ≥ 160 columns,
checking each against its entry; at stop 10 wait for the subject row to
settle on `· evil merge`; in a dirty checkout of graft itself press `k`,
`Enter` at the top and check the "Working-tree row" entry. Record the result
in the tier tables, naming the stop that failed if one did.
