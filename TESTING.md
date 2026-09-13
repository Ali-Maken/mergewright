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

The feature alone gates heavy tests; none is `#[ignore]`d.

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

Not there yet: the performance budgets and the 300 MB peak-memory gate
(M1 tickets 27 and 30). The measurement they will use —
`graft_fixtures::measure` — is here and unit-tested; the assertions are
not. `cargo-fuzz` targets land in M2.

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
cargo run -- "$(scripts/clone-git-git.sh --where)"
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

1. `cargo run -- "$(scripts/clone-git-git.sh --where)"` — a real,
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

### Unicode tier — must look deliberate

**This is the default tier** (ADR-0016: Kitty → Unicode → iTerm2 → Sixel →
ASCII). Every terminal here except a Kitty-protocol one draws it, including
the two the graph is read in most days.

| Terminal | Version | Result | Notes |
|---|---|---|---|
| Zed terminal | | not run | |
| VS Code terminal | | not run | |
| Terminal.app | | not run | |
| Alacritty | | not run | |

Pass: box-drawing characters line up into continuous lanes, nothing is a
replacement glyph or a tofu box, and no character is double-width where a
single-width one was assumed. "Looks deliberate" is the bar: a reasonable
person seeing only this tier should think it was designed, not that
something failed. This is the tier most people will actually live in.

### Does a pixel tier beat Unicode here?

This is the only thing that moves iTerm2 or Sixel above Unicode in ADR-0016's
order. Run it per terminal, answer yes or no, and give the reason.

1. Same repository, same window, same font size, no resizing between runs.
2. `graph.renderer = "unicode"` in the config, `cargo run -- <repo>`. Look at
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
3. `graph.renderer = "pixel"` in the config, then `cargo run -- <repo>`.
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

### Out of scope

Windows terminals (PROMPT.md, Mission; ADR-0005). Not tested, not
supported in v1.
