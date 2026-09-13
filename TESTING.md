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

**Then the release perf run** (ADR-0021), alone, after the command above:

```sh
# Time budgets fail only here: input to frame (frames::, four tests) and
# first paint (heavy::budgets_hold_*, the 50k-refs one included).
cargo test --release -p graft --features heavy-tests --test it -- frames:: heavy::budgets_hold --test-threads=1 --nocapture
```

In the debug run above those same tests measure and print their numbers
(`debug build, budget recorded not asserted`) without failing; peak RSS
fails in both builds. Every milestone review runs both and reports the
release numbers.

**Frame numbers depend on the CPU's power state** (m1/30c part b). On the
build machine (Apple Silicon), a scroll paced at 60 Hz is light load, and
the CPU stays in a slow state until a heavier burst: the first key that
asks for rows past those on hand (key 37 of the script: a `j` that costs
its own frame, a progress frame and the rows' frame back to back) measured
12–25 ms in release, and so did the first key after ≥ 0.5 s idle. With one
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
cargo run -- "$REPO"
# b) still inside, split and run in the right-hand pane: pixels must land in
#    that pane, not at the window's left edge. Ctrl-b %, then:
cargo run -- "$REPO"
# c) passthrough off (tmux's default): expect `tier Unicode`,
#    `tmux PassthroughSilent { .. }`, a clean Unicode graph, no stray bytes.
tmux kill-server -L graft; tmux -L graft -f /dev/null new-session
cargo run -- "$REPO"

# tmux (inside a Unicode terminal): Zed or VS Code terminal, passthrough on.
# Expect `tier Unicode` and a Unicode graph — never ASCII.
tmux -L graft -f /dev/null new-session \; set -g allow-passthrough on
cargo run -- "$REPO"

# SSH: needs a host with Rust and a Graft checkout (ssh localhost works:
# System Settings > General > Sharing > Remote Login). Expect `ssh true`,
# `budget 500ms`, and the tier the local terminal would get.
ssh -t HOST 'cd ~/path/to/graft && cargo run -p graft --example capability_probe && cargo run -- "$(scripts/clone-git-git.sh --where)"'

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

## Graph gallery

M1's acceptance ticket (35). M1 promises that **the shape of history is
obvious at a glance**; the gallery is how a person checks that promise, one
repository at a time, with a stated finding per stop.

```sh
cargo run -p graft --example gallery                      # first stop
cargo run -p graft --example gallery -- --start foxtrot-merge
cargo run -p graft --example gallery -- --list            # the 23 stops
cargo run -p graft --example gallery -- --matrix          # coverage matrix
```

`]` next stop, `[` previous, `q` quit. Every graph key works inside it:
`j`/`k`, `Space` fold merges, `1` first-parent, `H` hide stashes, `F` Focus,
`+` unfold lanes, `F3` cycle the tier (Pixel → Unicode → ASCII → back).
The footer's last line names the stop, its `n/N` and `expect: …`, the
one-line finding. Each stop restarts the run, so expect a blink per step.

Captures of every stop, Unicode and ASCII at 80x24, are under
`docs/shots/35-<stop>-{unicode,ascii}.txt`;
`docs/shots/35-gallery-two-steps.txt` shows two `]` presses. They are what
`.scratch/m1-the-graph/pty-gallery.py` saw, not what a person saw: the
user judges by eye (ticket 35's last box).

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
   alpha's colour, `Advance main` below in lane 0. Nothing on screen marks
   the merge as evil (the extra `evil.txt` is a tree fact, Mistake Radar's
   job in M2): this stop is the control for "an ordinary merge looks
   ordinary".
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
13. **tags** — 4 rows, one tag each, reading down: `[◆ main] [tag: v2.0]` on
    `Add c.txt`, `[tag: v1.0]` on `Add b.txt`, `[tag: v0.2]` on `Add a.txt`,
    `[tag: v0.1]` on `◇ Add a readme`. `v1.0` and `v2.0` are annotated: their
    badges sit on the commit rows (the tag object is peeled), and read the
    same as the lightweight ones.
14. **unicode-names** — 5 commits, author `Ünïcödé Fixture`. Three branch
    rows off `日本語のコミットメッセージ`: `[fonctionnalité]`, `[機能]`,
    `[ветка]`, each in its own column. The `☕` and `🚀` subjects and the
    CJK badge take two cells per glyph, yet the author, date and SHA columns
    on those rows start in the same screen column as on every other row.
    `[tag: версия-1.0]` sits on the `日本語` row next to `[◆ main]`.
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
19. **non-utf8-path** — 2 columns: `Add a Latin-1 path` (`[latin1]`) and
    `◆ Add a readme` (`[◆ main]`). The bad byte lives in a tree path, so no
    `\u{fffd}` or stray byte appears anywhere on screen.
20. **non-utf8-ref-name** — builds only where the filesystem takes a Latin-1
    file name (not APFS): on a Mac the stop reads `not available — build
    with: …`. Where it builds: `Work under a Latin-1 branch name` carries a
    badge reading `caf�` (one replacement character), `Add a readme` carries
    `[◆ main]`, and nothing else on the row is disturbed.
21. **graft-self** — this checkout, ~100 commits at M1's end. One column the
    whole way, every node `●` except `◆` at HEAD and `◇` at the bottom; no
    curve anywhere. An `Uncommitted changes` row appears at the top only if
    the tree is dirty.
22. **synthetic-100k** — present only after the heavy tier has built it
    (`--matrix` and the stop itself say `not available — build with: …`
    otherwise). With it: the first frame paints without a visible wait; more
    than eight parallel lanes fold behind a `+N` marker (`+` unfolds them);
    `j` held down and `G` never stall; `1` collapses to the default branch's
    single column.
23. **git-git** — `scripts/clone-git-git.sh` first. `master` is lane 0 and
    stays in the leftmost column for the whole history (`G` to the bottom
    and back). Merge rows are `◉` with a curve coming in; release commits
    carry `[tag: v2.x.y]` badges. `1` leaves a single column of merges and
    direct commits; `Space` folds each merge's side commits behind it.

### Per terminal: walk the gallery

Procedure, in each terminal of the checklist above: `cargo run -p graft
--example gallery`, then `]` through all 23 stops, checking each against its
entry; at stops 3, 5, 6 and 18 also press `Space`, `1`, `F` (`Esc` to leave)
and `H` as the entry says; at stop 23 press `F3` three times and scroll.
The pass conditions are the ones above:

- **Pixel tier, must pass** (Ghostty, Kitty, WezTerm, iTerm2): the 23
  entries hold with the pixel glyphs; strips line up with their text rows at
  every stop, and the octopus (4), criss-cross (5) and foxtrot (6) curves are
  smooth, not stepped.
- **Unicode tier, must look deliberate** (Terminal.app, Alacritty, the VS
  Code and Zed terminals): the 23 entries hold with the glyphs listed under
  "Reading the entries"; at stop 14 nothing is a tofu box.
- **Must degrade, not break** (tmux, SSH): either a correct pixel gallery or
  a clean Unicode one, per the tmux/SSH block above; every stop's footer
  line reads in full.

Record the result in the tier tables above, one line per terminal, naming
the stop that failed if one did.

### Out of scope

Windows terminals (PROMPT.md, Mission; ADR-0005). Not tested, not
supported in v1.
