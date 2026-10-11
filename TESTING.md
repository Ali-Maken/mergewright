# Testing Tributary

Three tiers. You only need the first one day to day.

| Tier | Command | When | Budget (PROMPT.md §7.1) | Measured |
|---|---|---|---|---|
| **Fast** | `cargo nextest run` + `cargo test --doc` | After every change. The gate for every ticket | under 3 min, ~2 GB | **34.3 s** of test time for 1822 tests (one run, 2026-09-20, after the 19 M4 walk tests, whose own test time is 14.8 s spread over the workers; 50.8 s wall including a rebuild, so the warm wall figure is not measured). Before M4: **38–40 s** warm (two runs, 2026-09-19, after the 22 M3 walk tests; 36.2 s without them the same hour, 21.7 s the morning before — the walk adds 2–4 s, the rest is not explained) |
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
cargo nextest run
cargo test --doc   # nextest does not run doctests
```

## Captures

Every file under `docs/shots/` is **asserted by every `cargo nextest run`** and
written only when you ask for it (m5-history-and-ghost-preview/40). Change what
the app draws and the run goes red, naming the file, the first differing line
and its column. There is no tolerance to relax: the captures are what the user
judges by eye (CLAUDE.md rule 6), so a capture allowed to differ is not
evidence of anything.

They were `#[ignore]`d generators until then — written unconditionally, run by
hand, and 42 of them drifted across four milestones without a test ever going
red.

When the new picture is the right one:

```sh
TRIB_UPDATE_SHOTS=1 cargo nextest run   # rewrites, and fails anyway
git diff --stat docs/shots/              # read it before you commit it
```

**A regeneration is always a red run.** The files are rewritten *and* the tests
fail, so a changed capture cannot reach a commit without someone having looked
at it. One pass rewrites everything: a generator that owns several files
collects its mismatches (`graft_fixtures::Captures`) instead of aborting at the
first. Then run the gate again, plain, and it is green.

A capture must be reproducible on any machine, so nothing the run invents may
reach the file. Two seams handle that for you: `captures::frame_at` draws every
frame at a fixed instant and at the fixed path `/code/api` (never substituted
into the text afterwards — the header fits itself to its width, so a real path
changes the truncation), and `captures::without_invented_id` zeroes an object id
a test committed itself, same width, every prefix. A capture that differs for
any other reason is a defect, and neither is a place to hide one.

**Nothing in a tracked file names the machine it was written on**
(m5-history-and-ghost-preview/41). `graft_fixtures::capture` redacts three
classes out of what the code drew, before it compares or writes it — a home
directory, a path under the OS scratch directory, and a port the OS picked —
each replaced by a placeholder of exactly the width it took, so no grid moves.
`nothing_a_commit_would_carry_names_this_machine` then checks, on every run,
**every file a commit from here would carry** — the index (tracked, and
anything already `git add`ed) *and* the working tree's un-ignored files. Both,
because a new file is not in the index until it is added, so reading the index
alone lets a builder's last green gate precede the failure by one commit. That
happened to this rule's own module. What it does not see: `.gitignore`d files,
which cannot be committed without `-f`, and index content the working tree no
longer has, since it reads files from disk. It says what to do:

```sh
cargo run -q -p graft-fixtures -- redact <path>...   # a file older than the rule
```

Write `<home>` by hand where a test or a document needs a home directory; a
test that needs one of these literally assembles it from pieces, the way
`graft_fixtures::capture::machine` spells its own roots and ports. The
pty walks do not write to `docs/shots/` themselves: they pipe each frame to
`graft-fixtures capture <path>`, which is the same mechanism on the same terms,
so a walk asserts like everything else and `TRIB_UPDATE_SHOTS=1` is how it
regenerates.

**Records, not asserted** (m8/22c). "Every file is asserted" holds only for a
file some test passes to `captures::check_shot` / `graft_fixtures::capture`:
nothing lists `docs/shots/` and fails on a file no test owns. Hand-shot files
from before m5/40 are therefore **records of their ticket**, not current
output — their chrome is stale and no run checks them. Do not hand-edit them
(the file would claim a frame the app never drew); a current picture of the
same thing gets its own generated capture. Known records:
`26-search-far.txt`, `26-search-typing.txt` (record of m1/26, 100k synthetic
repo); `m2-18-git-git-search-enter-160x40-unicode.txt` (record of M2 stop 18,
git/git); `17-mouse-select-and-tooltip.txt` (record of m1/17, pty, release
binary). The current search hint is `m8/search-hint-120x36.txt`
(`search_hint::the_search_hint_draws_his_esc_spelling`). Assembled contact
sheets (`*-contact-sheet.txt`) come from `scripts/contact-sheet.sh`, not a test.
This list is not an audit of every file under `docs/shots/`.

## Switches the binary reads for tests

Two environment variables reach the shipped binary. Both are for reproducible
pty runs and captures, not settings. Each takes whole seconds since 1970;
anything else refuses (`TRIB_NOW` at launch, exit 2; `TRIB_PRACTICE_EPOCH`
when the tour starts, which then does not start).

- **`TRIB_NOW` draws only and writes nothing** (m8/19e §R1). It pins the
  clock every age on screen is drawn against, and the practice repo's `now`.
  The journal's stamps come from the wall clock, always: `u`, J and the time
  machine order entries by stamp, so a pin left exported in a shell would
  otherwise write false, permanent history into any repository opened under
  it. The trade-off: a stray pin draws wrong ages for that session, with no
  marker on screen (a marker would change every pty capture); unset, they are
  right again and nothing persisted. The pty harness sets it to
  `graft_fixtures::PINNED_NOW` unless the test names it. So no pty test may
  assert a journal stamp's value, and no capture may draw a journal entry's
  age. Test: `clock_pin::graft_now_never_stamps_the_journal`.
- **`TRIB_PRACTICE_EPOCH` dates the practice repo**, the tour's temp
  repository (removed on every ending), so its commits and their ids are the
  same every run.

In-process tests pin stamps through `App::open_with_clock` and
`JobRunner::with_clock`, which no env var reaches.

## Times I left

Every time you leave Tributary for the shell, one line here. **This is the only
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
| 2026-09-23 · walk | Resolve a conflict with a merge tool | Until M6: `git mergetool` in a shell. M6: **0** — `M` in the resolver (`mergetool-hand-over-and-back`; a script stood in for the tool) | m6-conflicts |
| 2026-09-23 · walk | Fix a block by hand | Until M6: `$EDITOR` from a shell. M6: **0** — `e` in the resolver, re-read on return (`editor-edits-are-never-clobbered`) | m6-conflicts |
| 2026-09-23 · walk | Keep one side of a whole file | Until M6: `git checkout --ours`/`--theirs` + `git add`. M6: **0** — `o`/`t` in the resolver (`every-whole-file-shape`, `crlf-and-no-final-newline-survive`) | m6-conflicts |
| 2026-09-23 · walk | Accept a delete | Until M6: `git rm`. M6: **0** — `t` on the card (`every-whole-file-shape`) | m6-conflicts |
| 2026-09-23 · walk | Finish a merge **begun in the shell** (a `git pull` that conflicted, resolved in Graft) | M6: **0** — `c y` on the banner (P4, P12; `crates/graft/tests/it/resolver/shell_begun.rs::c_y_finishes_a_shell_begun_merge`), or `y` in the summary the last file opened (`m5_walk::every_scenario::journey_b_whole_two_files`). Was: the shell, ruling F5 (`graft-ops/src/operation.rs:581-585`). | closed (m6-conflicts, shell_begun::c_y_finishes_a_shell_begun_merge) |
| 2026-09-23 · walk | Start a file over **after `⏎`**, inside an operation | **0** in a merge Graft began — open the done file, `d y` (m6-conflicts/28; words and `y` m7-power-tools/17; walk `crates/graft/tests/it/m7_walk.rs::restart_after_done`). Still refused in a merge begun in the shell (no stages kept), in words: `git can't bring back the conflict in <file>: <reason>`. | closed (m7-power-tools/17, 26; `m7_walk::restart_after_done`) |
| 2026-09-23 · walk | A submodule conflict | Still the shell — no resolver shape for it. Expires M7 | open (M7) |
| 2026-09-23 · walk | A rename/rename conflict | Still the shell — no resolver shape for it. Expires M7 | open (M7) |
| 2026-09-23 · walk | A file with a custom `conflict-marker-size` | Still the shell — the parser reads git's default seven-character markers and refuses others in words (`graft-git/src/conflict.rs:345`). Expires M7 | open (M7) |
| 2026-09-23 · test | Get back a hand edit after a whole-file take (`t`/`d`) in a merge **begun in the shell** | M6: **0** — `u` on the banner undoes the whole-file take (`crates/graft/tests/it/resolver/shell_begun.rs::u_on_the_banner_restores_a_whole_file_take_in_a_shell_merge`), and `a y` aborts the merge with `u` to put it back (`shell_begun.rs::a_on_the_banner_aborts_a_shell_merge`). Was: `git merge --abort`, then `Ctrl+R`, `u` — both keys refused while the merge was open. Source: ticket 08 `## Review fixes, part 2`. | closed (m6-conflicts/57, shell_begun::a_on_the_banner_aborts_a_shell_merge, shell_begun::u_on_the_banner_restores_a_whole_file_take_in_a_shell_merge) |
| 2026-09-24 · review N3 | Finish a stash pop that clashed | Until the put-back pass: the clashing pop read as a git error and sent him to the shell — **11 keys** (review N3, `docs/design-notes/m6-stash-put-back.md` :15). Now **5**: `S p y o ⏎` (`y` confirms the pop preview since m6-conflicts/86), the resolver opens on the clash and the last file done drops the stash (`m5_walk::every_scenario::journey_stash_pop_clash_to_done`, `m6_stash_put_back::a_stash_pop_that_clashes_is_five_keys`) | closed (m6-conflicts/72b, 75, 86; `stash-pop-clash-to-done`) |
| 2026-09-24 · review N4 | Pull and push with edits in the tree | Until the put-back pass: the dirty rebase was refused after confirm — **16 keys** walked (review N4, `m6-stash-put-back.md` :16). Now **5**: `p r r P y`, the edits set aside and put back (`m5_walk::every_scenario::journey_rejected_push_dirty_to_pushed`, `m6_stash_put_back::set_aside::a_rejected_push_with_edits_is_five_keys`). A fast-forward pull over edits to files it brings: was refused (m6-conflicts/85 D, 7 keys through a stash); now **1**, `p` (m7-power-tools/03b, `m7_walk::pull::pull_ff_over_edits`) | closed (m6-conflicts/73, 74, 75; `rejected-push-dirty-to-pushed`) |
| M8 · daily use (to fill) | _each time M8's app sends him to the shell or another tool: what he wanted, one line, the day it happens_ | _what he ran instead_ | open (M8 daily use) |

_Rows marked `walk` are what the M6 acceptance walk says a reach used to be
and is now; they are not daily use. Rows marked `review N…` are the M6
review's reaches (`docs/design-notes/m6-stash-put-back.md` "Need"), added
closed by ticket 75: they were counted in the review, never written here.
Daily-use rows are dated plainly._

## The fast tier

```sh
cargo nextest run
cargo test --doc   # nextest does not run doctests
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

**Exit 3 from a `scripts/` release perf run means "not judged: re-run on mains"** — neither pass nor miss. Every such script sources `scripts/lib/machine-state.sh` (m7/49), writes `SUMMARY machine ac= lowpower= os=` first in its perf file, and exits 3 on macOS when on battery or in Low Power Mode (or with `ALLOW_LOW_POWER=1`, for diagnosis).

**Then the release perf run** (ADR-0021), alone, after the command above:

```sh
# Time budgets fail only here: input to frame (frames::, four tests) and
# first paint (heavy::budgets_hold_*, the 50k-refs one included).
cargo test --release -p graft --features heavy-tests --test it -- frames:: heavy::budgets_hold --test-threads=1 --nocapture
```

**Held keys** (m7-power-tools/42): `scripts/fix-scroll-held-key.sh` runs

```sh
cargo test --release --features heavy-tests --test it -- held_key:: --test-threads=1 --nocapture
```

and writes `docs/perf/fix-scroll-held-key.txt`. Four tests, held `j` (188
keys, one every 16 ms) on the team fixture shaped like his work repo
(`TeamSpec::MONOREPO`) at his 187×50, Unicode and Pixel; scenario B adds
`J` 50 ms after the last `j`. The terminal is a `DrainSink`: a 16 KiB pipe
drained at 500 KB/s (Unicode) or 40 MB/s (Pixel) that blocks the writer when
full — a stress model of a terminal busy parsing, chosen so today's build
overruns it and a fixed one has room (the real macOS pty capacity is printed
beside it). The in-memory sink above has no back-pressure, which is why the
`frames::` gate never saw this. Every time runs from the **last key's
arrival** (its send): a key release never reaches the loop, so this is at
least as strict as "after I let go". One `SUMMARY held_key` line per tier
and scenario: `frames_after_release` (frames drawn after the last `j`; in B, up to `J`'s frame),
`tail_ms` (last `j` → the terminal swallowing the last frame; in B, up to `J`'s frame),
`bytes_per_frame_held`, `strip_transmits_held` / `emphasis_transmits_held`
(Kitty transmits split by image id bit 31), `rows_scrolled`, and in B `j_ms`
(`J` → the terminal reaching the journal frame's first byte), `j_done_ms`
(printed only), `frames_before_J`; `peak_rss_mb` printed, not asserted.
Premises (≥ 12 lanes, the selection's lit columns changing) fail the test if
the fixture loses his shape. Counts fail in every build, times in release
only. **These four are red in the heavy tier until 46** (Unicode green after
43): the full heavy command above fails on them, deliberately. Right looks
like: every `VERDICT held_key` line `ok`, except the two below.

**Paced** (m8-polish-and-ship/02, ADR-0054): the `DrainSink` now answers
each fence (`CSI 5 n`) once it has drained past it, and the harness's
terminal says it answers DSR, so every `held_key` test measures the paced
loop — at most one unanswered frame in the pipe. Three more tests run held
`j` at the other drains J1 names: `held_j_stops_on_release_unicode_40mb`,
`held_j_stops_on_release_unicode_free`, `held_j_stops_on_release_pixel_free`,
`held_j_stops_on_release_pixel_deflate_free` (free = `drain_sink::FREE`,
every byte drained as written; the ticked drain moves at most a pipeful per
1 ms tick, so its "40 MB/s" is ≤ 16 MB/s in fact). `scripts/m8-pace.sh` runs them all plus
`frames::after_idle::g_after_idle` and writes `docs/perf/m8-pace.txt`:
right looks like `J1 … verdict=pass` (tail ≤ 100 ms) for Unicode and Pixel
(Kitty + deflate, as Ghostty probes) at 40 MB/s and free, `J3 g_after_idle … verdict=pass` (≤ 16 ms), and a
`ROW 1089` / `ROW 1095` verdict line each.

**The strip bound** (m7-power-tools/46c) is `strip_transmits_held ≤
ceil(rows_scrolled/32) + 2 + fold_changes_held × (ceil(screen_rows/32) + 1)`,
not 42's `ceil(rows_scrolled/32) + 2`. On a graph wider than the lane cap
(MONOREPO is) the fold keeps the selection's thread lanes (ADR-0025 Decision
2), so a `j` that moves the thread can change which lanes draw, and every
strip on screen goes again: a layout change, not a paint one. ADR-0053 keeps
the paint out of the strips; it never promised the layout. `held_key` cannot
read the drawn fold (its harness hands out events and byte offsets, not the
fold), so it prints `fold_changes_held=unread` and a
`strip_transmits_held=<n>: unjudged` line instead of judging the count. The
invariant is guarded in the fast tier, every build, by
`pixel_scroll::a_capped_graph_resends_strips_only_when_the_fold_changes`: on a
capped fixture, a backdrop goes only on a frame whose fold plan or strip
window changed, at least one `j` does change the fold, and a changed selection
paint always sends an emphasis layer.

**The strip memory bound** (m8 04): the strip cache weighs its layers in
bytes against one budget (`STRIP_BUDGET`, 96 MiB; DESIGN §3.8), the viewport's
strips kept whatever they weigh. Fast tier, every build:
`event_loop::tests::pixel_scroll::held_j_on_wide_strips_stays_within_the_byte_budget`
(budget = the viewport floor plus two strips, so the budget binds, not the
floor — the regime the 17×37 cell runs in: resident ≤ budget and the floor
never wins on every frame, evictions happen, and the pool makes no new buffer
over the second half of the hold). The floor-wins regime is the cache-level
`strip_cache::the_viewport_minimum_wins_over_a_budget_too_small`. Heavy tier:
`heavy::budgets_hold_at_pixel_on_monorepo_with_j_held_free` and
`heavy::budgets_hold_at_pixel_on_monorepo_with_j_held_40mb` run `held_key`'s
scenario A (Kitty + deflate, 187×50, 17×37 cell) in a child process of their
own and assert its `getrusage` peak < 300 MB. Right looks like: both green, and
a `SUMMARY heavy pixel_rss … peak_rss=` line each. Where the bytes go per layer,
per cell size: `scripts/scroll-pixel-memory.sh` →
`docs/perf/scroll-pixel-memory-budget.txt` (a measurement patch applied to an
exported tree, never committed to the source).

**Deflate off is red by known gap** (`KNOWN_GAPS` M7/46-kitty-no-deflate): the
raw Kitty pair (`held_j_stops_on_release_pixel`,
`j_then_journal_opens_at_once_pixel`) misses `tail_ms` / `j_ms` ≤ 100 by
construction (a raw layer is megabytes at the 40 MB/s model) and stays
asserting, red, until `scroll-kitty-without-deflate` lands.

**The frame gate is the p99** (m7-power-tools/04): each `frames::` scroll
passes when its p99 input-to-frame is ≤ 16 ms; its line prints
`p50_us= p99_us= max_us= over_budget=` every run, and every key over the
budget is listed, recorded not failing. One outlier in 420 keys no longer
fails the run.

**Where a slow key's time goes:** `scripts/m7-perf-key.sh` (~10 min) runs
the release `frames::` tests 3 times plain and once per control (A no idle
pause, B a core held busy, C no idle prefetch), with every key's phases
traced, and writes `docs/perf/m7-04-slow-key.txt`; when the file exists it
appends the run as the next `# ===== rev N` section (`REV_NOTE="…"` says
what changed), headed by `SUMMARY revN` before/after lines. Read its
`SUMMARY` lines. Right looks like: `gate(p99<=16ms)=ok` on every `spread`
line across all 3 runs, and the `frame_trace:` line's `send -> recv` under
1 ms (asserted in release). Since m7-power-tools/04b the keys come from an
input thread that sends them into the loop's wake channel, as the app's
input forwarder does, and a key's latency runs from its send: `oversleep`
is that thread's own sleep waking late (OS timer slack, before the send, not
in the latency; the `machine:` lines show it with no Tributary code at all),
`queued` is send → the loop taking it, `draw` is Tributary's frame. What it
cannot show: the real terminal (keys are made in the test, output goes to
memory), so a pty's wake and the terminal's own paint are not in these
numbers.

The script **exits 1** when any run failed or any `spread` line is not
`ok` (m7-power-tools/12d): `SUMMARY runs:` counts every run's exit, and a
spread reads `FAIL(<run> failed)` when a plain run failed, `MISSING(k of 3)`
when a scenario printed fewer than 3 cells, then `OVER` or `ok`. A failed
run never reads as a pass. `scripts/m7-perf-key-selftest.sh` checks this on
canned outputs (no cargo, seconds).

**The idle advance** (m7-power-tools/12) adds three readings to the same
runs. Right looks like:

- `frame_trace: … startup …` lines (with `TRIB_FRAME_TRACE` set): every
  wake from the first window of rows to key 1, then
  `startup burst_after_first_paint_us=` — how long a key pressed just after
  first paint could queue behind startup's job answers. Queued answers are
  drawn once, not once each.
- `SUMMARY idle_gap … idle_gap_wakes=0` for every run with idle pauses: once
  the walk has said `LaidOut`, a 600 ms pause with no input wakes nothing
  (the history worker is parked). `pauses_after_laid_out=N of 5` says how
  many pauses were counted; `0 of 5` means `LaidOut` came late and the line
  proves nothing.
- `SUMMARY g_after_idle … latency_us=` under 16 000 and `idle_gap_wakes=0`:
  `frames::after_idle::g_after_idle`, `G` pressed after history is laid out
  and the app sat idle 2 s (100k cg, 50k refs, Unicode); the latency runs
  to the frame showing the last row. Asserted in release.

**Before and after the idle advance**, `scripts/m7-12-final-perf.sh`
(~40 min) runs `g_after_idle` ×3, `scripts/heap-slope.sh` ×2 and
`examples/budgets` on git/git ×2 on both sides — "before" is this HEAD in a
throwaway worktree with `scripts/m7-12-no-idle-advance.patch`, run with
`TRIB_FRAME_NO_IDLE_ADVANCE=1` — then the heavy tier's `heavy::` and the
release `heavy::budgets_hold`. It appends a section to
`docs/perf/m7-12-idle-advance.txt`; read its `SUMMARY` lines. Every step
records `uptime` and any process over 40% CPU (`SUMMARY quiet:`); a busy
step is marked, never rerun.

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
what Tributary itself allocates on this path (order, bitsets, promoted, malloc
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
88–112 ms range, confirming the difference is the page cache and not Tributary.

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
| `team-400-<hex>/` | `team::TeamSpec::SMALL`: a team monorepo's shape — `main` advancing by merges of short feature branches, ~10 open at once (m7/41) | The fast tier, automatically | 0.20 MiB, 0.11 s |
| `team-17700-<hex>/` | `team::TeamSpec::MONOREPO`: the same shape at his scale — 17 700 commits, ~20 open, 1 330 packed refs, split commit-graph (m7/41) | The heavy tier | 20.2 MiB, 8–16 s |

The hex is a digest of the generator's own source. Change the generator
and the next run builds into a directory that does not exist yet, so
nothing is ever stale — and old digests stay behind until you delete
them. That is the one maintenance job:

```sh
rm -rf target/fixtures        # or `cargo clean`; both are safe, they rebuild
```

Set `TRIB_FIXTURES_DIR` to build them somewhere else — a different disk,
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
| Where | `${XDG_CACHE_HOME:-$HOME/.cache}/tributary/git-git.git`, or `$TRIB_GIT_GIT_DIR` |
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
  ~/.cache/tributary/linux.git
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
Tributary and walk "What to do in each terminal".

```sh
# Setup, once: tmux >= 3.3, and a git/git clone.
brew install tmux && tmux -V
REPO="$(scripts/clone-git-git.sh --where)"

# tmux (inside a pixel terminal): run from Ghostty or Kitty.
# a) passthrough on: expect `tier Pixel`, `tmux Passthrough { version: "3.x" }`,
#    a pixel graph.
tmux -L trib -f /dev/null new-session \; set -g allow-passthrough on
cargo run -p graft --example capability_probe
cargo run -p graft -- "$REPO"
# b) still inside, split and run in the right-hand pane: pixels must land in
#    that pane, not at the window's left edge. Ctrl-b %, then:
cargo run -p graft -- "$REPO"
# c) passthrough off (tmux's default): expect `tier Unicode`,
#    `tmux PassthroughSilent { .. }`, a clean Unicode graph, no stray bytes.
tmux kill-server -L trib; tmux -L trib -f /dev/null new-session
cargo run -p graft -- "$REPO"

# tmux (inside a Unicode terminal): Zed or VS Code terminal, passthrough on.
# Expect `tier Unicode` and a Unicode graph — never ASCII.
tmux -L trib -f /dev/null new-session \; set -g allow-passthrough on
cargo run -p graft -- "$REPO"

# SSH: needs a host with Rust and a Tributary checkout (ssh localhost works:
# System Settings > General > Sharing > Remote Login). Expect `ssh true`,
# `budget 500ms`, and the tier the local terminal would get.
ssh -t HOST 'cd ~/path/to/checkout && cargo run -p graft --example capability_probe && cargo run -p graft -- "$(scripts/clone-git-git.sh --where)"'

# SSH inside tmux: start tmux (passthrough on) locally, then the SSH line
# inside it. Expect `tier Pixel` only if the report says `tmux Passthrough`.
```

When done: `tmux kill-server -L trib`. A row passes or fails on the pass
conditions below, never on the probe report alone.

**tmux passes on *either* of two outcomes**: a correct pixel graph, *or* a
clean fall back to the Unicode tier. Which one you get depends on the tmux
version, on `allow-passthrough`, and on the outer terminal, so both are
correct answers. What is *not* a pass: a broken screen, stray escape
sequences, or a graph drawn half in pixels and half in characters. Tributary
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
| Terminal with OSC 52 disabled | Same notice (Tributary cannot tell); paste gives whatever was there before — not a Tributary bug |

Automated: `python3 .scratch/m1-the-graph/pty-copy-check.py <release trib> <repo>`
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
when its frame appears, and every Tributary key works after that. The banner's
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
7. **foxtrot-merge · .** — the menu over the graph opens with the fixed
   slots (m6-conflicts/71): `Checkout — already on main` (dim: `main` is
   checked out and is the row's only branch), `Cherry-pick 84cc93e`, then
   `Explain`, `Copy SHA (y)`, `Compare with this (x)`, `Open details (⏎)`,
   then the row's other history verbs and the dropped keys. The cursor row
   has a finding (stop 6's `⚠`), so `open_for_history_row` asks
   `view::radar::findings_for` and lists `Explain` right after the slots
   (m2-details-and-diffs/24). `Esc` closes the menu. The m2-07 stills
   predate the slots (their header says so).
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
    **Blind spot:** an edit to a file *both* sides touched is not checked —
    it reads `evil merge: none found · N hand-resolved files not checked`, never `clean`.
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
of Tributary itself, `k` from the top row lands on `Uncommitted changes (…)`,
its highlight moves and HEAD's row un-highlights; `Enter` opens
`UNCOMMITTED · N staged N unstaged N untracked` / `diff vs HEAD <sha>`, staged
and unstaged files listed, an untracked file's body shows its content as
added lines (`empty file` when it has none; binary/capped as any diff); `j` returns to HEAD's commit and the pane follows. Captures:
`docs/shots/m2-working-tree-row.txt` (graph),
`docs/shots/m2-detail-pane-breakpoints.txt` (the `UNCOMMITTED` pane) (historical record, drawn at M2; today's drawing: `docs/shots/m7/diff-cursor-commit-120x40.txt`).

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
settle on `· evil merge`; in a dirty checkout of Tributary itself press `k`,
`Enter` at the top and check the "Working-tree row" entry. Record the result
in the tier tables, naming the stop that failed if one did.

## Working tree, undo and stash

M3's acceptance ticket (m3-working-tree-undo-and-stash/07). M3 promises that
**you cannot lose work**: every write Tributary makes is journaled with a backup
and comes back byte for byte on `u`. The walk is how a person checks that —
one command, 22 scenarios, each a scratch copy of a corpus repository with a
scripted dirty state, the real app driven by keys over the real job pool and
real `git`, and then **the repository read back by running git**
(`for-each-ref`, `ls-files -s`, `stash list`, `status --porcelain`, file
bytes) — never by trusting the app's own state.

```sh
cargo run -p graft --example m3_walk                        # every scenario, one line each
cargo run -p graft --example m3_walk -- --only stash-drop   # one
cargo run -p graft --example m3_walk -- --list              # the names
cargo run -p graft --example m3_walk -- --stems             # capture stems, keys, expected text
cargo run -p graft --example m3_walk -- --matrix            # rewrites docs/shots/35-coverage-matrix.txt (all three tables)
```

`ok <name>` per scenario, `FAIL <name>: <what differs>` (20 lines at most),
exit 1 on any failure. The same list runs in the fast tier as one test per
scenario (`tests/it/m3_walk.rs`, `m3_walk::every_scenario::*`; slowest 1.4 s;
the 22 alone take 4.6 s under nextest, and add 2–4 s to the whole tier). Every scenario
ends with `git fsck --no-dangling` saying nothing but notices, exactly one
`refs/tributary/backup/<worktree>/<id>` ref per journal entry that took a backup,
and **no backup commit drawn in the history window** — every row's subject is
the user's, never a `graft <ulid> …` (m3-07 review, C1; the check is
`no_bookkeeping_in_the_graph`, run after every scenario).

The walk's own `git` runs under `graft_fixtures::git::run_isolated`, and each
scenario's repository overrides the settings that would otherwise decide what
a write means (`neutralise_global_config`): the 22 tests pass the same on a
machine with `commit.gpgsign`, a global `core.hooksPath` or
`status.showUntrackedFiles=no` set. Checked by running the tier with all
three on.

Captures: `docs/shots/m3-<nn>-<what>-<WxH>.txt`, nine visual results at
80x24 and 160x48, Unicode tier
(`.scratch/m3-working-tree-undo-and-stash/pty-walk-m3.py <WxH>`, after
`cargo build -p graft --example m3_walk`). The script runs
`m3_walk --capture <scenario>`, which scripts the state and opens the real
loop with the capture's keys as the harness prelude; the prelude now waits
for a write in flight, a journal or stash read and the editor's prefill
before feeding the next key, so `u` after `Space` undoes the stage rather
than racing it. **A capture is what pyte saw, not what a person saw, and
it carries no colour** — the user judges by eye.

### What the walk proves and what it does not

- **Every write and undo ran through the app against live git.** Nothing is
  injected: the keys are the user's keys (`update` fed directly, the job
  pool real), the journal, backups and restores are graft-ops's, and the
  verdict is git's.
- **Byte-for-byte is checked as bytes.** The untracked binary scenario uses
  `00 ff 0a 0d fe`; the tracked ones compare whole-file contents and blob
  ids, never a diff's text.
- **The pty run is the interactive path; the walk is not.** The walk drives
  `update` headless — no terminal, no probe, no real `$EDITOR`
  (`Ctrl+E` is untested here), no signing prompt, no hook. Its focus event is
  `Event::FocusGained` injected, not a terminal's focus report. What the
  captures show is the interactive loop, but only for the nine capture keys.
- **`Ctrl+Enter` is not the path exercised.** Commits go through `Tab` to
  `[ Commit ]` and `Enter` (ADR-0034), which is the path that works in Zed and
  VS Code.
- **Nothing here measures the snapshot cost** — that is
  `scripts/m3-snapshot-cost.sh` at review (`docs/perf/acceptance-m3.txt`,
  `docs/perf/m3-snapshot.txt`).
- **Two known gaps the walk does not cover, on purpose:** a forced undo is
  not pinned to the entry the drift box named, so a concurrent journal move
  could undo a different entry (the walk has no second writer); and `Esc` on
  a journal read that has not landed cancels it and `settle` re-requests
  (churn, not loss — the walk waits for the read).

### Reading the entries

The scratch starts from `Shape::Trivial` unless said otherwise;
`sample.txt` is committed as 16 lines `one` … `sixteen`, the "edit" changes
line 1 to `ONE` and line 16 to `SIXTEEN` (two hunks under three lines of
context). "Back" means `diff_truth` finds **nothing** different between the
truth taken after setup and the truth now: `HEAD`, every ref outside
`refs/tributary/`, every `ls-files -s` line, every file's bytes (untracked
included) and every `stash list` line.

### The scenarios

1. **stage-file** — `k` `Enter` `Space` on `sample.txt`: `ls-files -s` holds
   the edited blob for `sample.txt`. `u`: everything back.
2. **unstage-file** — the edit staged first; `Space` under `STAGED`:
   `ls-files -s` holds HEAD's blob, the file still reads edited. `u`: back.
3. **stage-hunk** — `Enter` into the diff (line cursor on row 0, the first
   `@@`), `Space`: the index holds `ONE` … `sixteen` (first hunk only); the
   file still holds both.
4. **stage-line** — line 1 becomes `ONE` + `ONE-B` (rows `@@`, `-one`,
   `+ONE`, `+ONE-B`); `j j j` `Space` on `+ONE-B`: `git show :sample.txt` is
   exactly `one` `ONE-B` `two` … `sixteen` — the `+ONE-B` line without the
   `-one`/`+ONE` pair.
5. **stage-all** — `a.txt`, `b.txt` modified, `c.txt` untracked, `Ctrl+A`:
   `diff --cached --name-only` is `a.txt b.txt c.txt`, `diff --name-only`
   empty, no untracked files left.
6. **discard-tracked-unstaged** — `d`, `→`, `Enter`, or `d` `y` (the box
   opens on the safe answer; `y` confirms it, m6-conflicts/50): the file reads as HEAD's 16 lines. `u`: back.
7. **discard-untracked-binary** — `bin.dat` = `00 ff 0a 0d fe`; `d` `→`
   `Enter`, or `d` `y` (the bar reads `y delete`, the button's word): the
   file is gone. `u`: the five bytes are back and
   `status --porcelain` shows `?? bin.dat`.
8. **discard-staged-and-unstaged** — first hunk staged, second unstaged, the
   file listed under both sections; `d` on the `UNSTAGED` row: **both** the
   index and the file read as HEAD's (a path discard restores both sides).
   `u`: index and file back.
   **Current behaviour, under review — not an acceptance line.** The design
   note has `d` discarding the same unit `Space` stages, so on an `UNSTAGED`
   row it should leave the staged half alone; the confirm box says "in the
   working tree" while the index is reset too. Nothing is lost (`u` puts it
   all back, and this scenario proves it), but which of the two is right is
   the user's call (m3-07 review, Major 2).
9. **commit** — `c`, subject `shout three`, `Tab` to `[ Commit ]`, `Enter`,
   or `⏎` in the subject (m6-conflicts/51, the same as `Ctrl+Enter`):
   HEAD moved and `log -1 --format=%s` is `shout three`. `u`: HEAD is the
   old commit and `ls-files -s` holds the edited blob again (re-staged).
   `U`: HEAD is the **same** sha the commit made, not a new one.
10. **commit-refused-nothing-staged** — clean tree, `c`, a subject, `[ Commit ]`
    `Enter`: the editor stays open with `nothing staged`, no write is
    queued, no journal file exists, nothing in the repository changed.
11. **amend** — `a` prefills `add sample.txt`; retyped `amended sample`,
    `[ Commit ]`: HEAD is a new sha whose subject is `amended sample` and
    whose tree holds the staged edit. `u`: HEAD is the old sha, subject
    `add sample.txt`, the edit staged again.
12. **commit-on-unborn-head** — `Shape::UnbornBranch`, `first.txt` staged,
    `c`: the branch exists with one commit. `u`: `rev-parse HEAD` fails
    (unborn again), the branch ref is gone, `ls-files -s` holds `first.txt`.
13. **undo-drift-guard** — commit through the editor, then `sample.txt`
    gains a line `DRIFT` out of band; `u`: a box `Undo "commit …"?` with
    `Stop` under the cursor, and under what drifted `Undo leaves your edits
    since as they are.` — the commit's undo writes no file, so it names none
    (m6-conflicts/77, the walk's `the commit drift is not named` step);
    `Enter`: nothing at all changed. `u` `→`
    `Enter`: HEAD and the index back as after a plain undo, `sample.txt`
    still holds `DRIFT` (a commit touches refs and index, not the file),
    and `rev-list --objects <undo entry's backup ref>` lists the `DRIFT`
    blob at `worktree/sample.txt` — the drifted state is kept.
14. **stash-save** — `S` `s`, `wip`, `Enter`: `stash list` is exactly
    `stash@{0}: On main: wip`, the file reads as HEAD's. `u`: back (no
    stash, edit on disk). `U`: the stash is listed again, tree clean.
15. **stash-pop** — `Shape::Stash` (two entries); `p`, `y` on its preview
    (m6-conflicts/86): one entry fewer, the
    tree dirty. `u`: back — both entries, in order, the tree clean.
16. **stash-apply** — `Enter`, `y` on its preview: both entries still listed, the tree dirty.
    `u`: back.
17. **stash-drop** — `d` `→` `Enter`, or `d` `y`: one entry fewer. Then
    `git gc --prune=now`. `u`: `stash list` is byte for byte what it was —
    the pin in the drop's journal entry kept the commit alive through gc.
18. **stash-apply-refused-dirty-touched-file** — a file the stash changes
    gets an uncommitted edit; `Enter` on the stash: the preview box over the
    list reads `your unsaved edits to … are in the way` (m6-conflicts/86;
    before it, the status line read `… uncommitted changes now …`), nothing
    changed, no journal file.
19. **interrupted-entry** — a real discard through the app, then the
    journal's outcome line deleted (a process killed mid-write). On launch
    the footer notice reads `an operation was interrupted: discard
    sample.txt · J to inspect`; `u`: the discarded edit is back on disk.
20. **linked-worktree** — `git worktree add linked`, the edit staged from
    inside it: `.git/worktrees/linked/tributary/undo-journal.jsonl` has the one
    entry, `.git/tributary/undo-journal.jsonl` does not exist, the only backup
    ref is under `refs/tributary/backup/linked/`.
21. **refresh-on-focus** — pane open (`1 unstaged, 0 untracked`), `extra.txt`
    written out of band, `FocusGained`: the status re-reads (`ReadWorkingTree`
    dispatched, `ReadHistory` not) and the counts read `1 unstaged,
    1 untracked`.
22. **no-index-write-on-read** — pane, `Esc`, `J`, `J`, `S`, `Esc`:
    `.git/index` is byte for byte what it was before launch.

### Per breakpoint

Every capture exists at 80x24 and 160x48; 120x40 was not captured.

| View | 80x24 | 120x40 | 160x48 |
|---|---|---|---|
| Changes pane (`m3-01-stage-file`, `m3-02-stage-hunk`) | tab strip, `Detail` tab is the whole screen: sections, files, diff | not run | pane right of the graph; `STAGED (1)` / `UNSTAGED (1)` section rows, the file under each, diff below |
| Confirm box (`m3-03-discard-confirm`) | a framed box `╭ Discard changes to sample.txt?`, the `git restore --worktree -- sample.txt` command, `▸[ Cancel ]   [ Discard ]`; the bottom row is the box's own keys, `y discard  ←/→ move  ⏎ choose  esc cancel` | not run | same box, graph and pane around it |
| Commit editor (`m3-04-commit-editor`, `m3-05-commit-refused`) | replaces the graph; title `COMMIT ── ⎇ main ── …`, `Subject`, `Body`, options, `[ Commit ]`; refused: `nothing staged` on the title and on its own row directly above the options row (ticket 40) | not run | same, wider; the pane stays |
| Stash (`m3-06-stash-list`, `m3-07-stash-prompt`) | `STASHES (2)` and rows `stash@{0} …`; prompt: `▸message wip`, `(•) all incl. untracked ( ) staged only` | not run | same |
| Journal (`m3-08-journal-after-unstage`) | the unstage entry with `← u` | not run | same |
| Drift box (`m3-09-undo-drift-box`) | a framed box `╭ Undo “commit "before the drift"”?`, what drifted, then `Undo leaves your edits since as they are.` (m6-conflicts/77), `undo restores the recorded state`, `▸[ Stop ]   [ Undo ]`; the bottom row `y undo  ←/→ move  ⏎ choose  esc cancel` (the undo's own `Busy` ended when its answer opened the box, m6-conflicts/69) | not run | same |
| Undo over drift, the way back (`m6/fix-undo-drift-way-back`, m6-conflicts/77) | a menu checkout, an edit, `u`: the box reads `Undo replaces your edit to fix.txt; a backup keeps it, and U brings it back.`, whole; `y`: `undo: checkout fix ✓ · U brings back your edit to fix.txt`; `U`: `redo: checkout fix ✓` and the changes row back; after `y` the graph marks `[◆ main]`, `fix` plain (m6-conflicts/79); the box is drawn over a settled graph, no `sorting…`, and frame 4, the time machine over an edit, lands on `reverted to before “checkout fix” (2 steps) ✓ · U (2 times) brings it back`, whole at 80 columns (m6-conflicts/80) | not run | same, 120x40 |
| Status under a box (`m6/fix-status-under-a-box`, m6-conflicts/78) | the bottom row, frame by frame: 1. `f` to a remote that is not there, `b`, the fetch fails under the prompt: `⏎ create  ⇥ toggle  esc cancel` (held, not drawn over the box); 2. `topic ⏎`: `fetch all remotes failed: … · esc dismiss`; 3. `esc`: `branch topic at …`, the create not cancelled; 4. `f`, `b`, `other ⏎` while that fetch runs: `wait for fetch all remotes to finish`, the prompt open, `other` kept; 5. the fetch answers: the prompt's keys again; 6. `esc`: `fetched ✓`. Colour unverified | same six rows | not run |

### Per terminal: walk the working tree group

Run `cargo run -p graft --example m3_walk` (all `ok`), then in a scratch
checkout of your own: stage a file and a hunk, commit, amend, stash and pop,
discard an edit, and press `u` after each, checking the entry above for what
right looks like. Record the result here, naming the scenario that failed if
one did.

| Terminal | Result |
|---|---|
| Ghostty | not run |
| Zed | not run |
| VS Code | not run |

### From today: the promise is measured in use

M3 is the first milestone that writes, so from today the two lists that
measure the promise start collecting: **"Times I left"** at the top of this
file (every reach for the shell, ADR-0033) and
[`docs/field-notes.md`](docs/field-notes.md) (what real use turned up,
ADR-0029/0030). Both are empty on purpose until you use Tributary for a day.

## Remotes

M4's acceptance ticket (m4-remotes/07). M4 promises that **you never leave
Tributary to fetch, pull or push — not for a password, a rejection or a hang**.
The walk is how a person checks that: one command, 21 scenarios, each a
scratch `remote.git` (bare) with a `work` clone and a `teammate` clone, the
bare served over `file://`, over loopback smart HTTP with Basic auth
(`HttpRemote`), or by a socket that accepts and never answers (`HungRemote`);
the real app driven by keys over the real job pool with the walk binary as
its own askpass helper; and then **the truth read back by running git on
both the work clone and the bare** (`for-each-ref` on each, `rev-parse
HEAD`, `ls-files -s`, file bytes, `.git/index` bytes, the journal file) —
never from `AppState`.

```sh
cargo run -p graft --example m4_walk                              # every scenario, one line each
cargo run -p graft --example m4_walk -- --only push-plain         # one
cargo run -p graft --example m4_walk -- --list                    # the names
cargo run -p graft --example m4_walk -- --stems                   # capture stems, keys, expected text
cargo run -p graft --example m4_walk -- --matrix                  # rewrites docs/shots/35-coverage-matrix.txt (all four tables)
```

`ok <name>` per scenario, `FAIL <name>: <what differs>` (20 lines at most:
the work clone's refs/files/index, the bare's refs, the journal's length),
exit 1 on any failure. The same list runs in the fast tier as one test per
scenario (`tests/it/m4_walk/`, `m4_walk::every_scenario::*`; slowest
`u_never_touches_the_remote` 1.4 s; the 19 take 14.8 s of test time, 7.8 s
wall with the M3 walk alongside, and the whole tier is at 34.3 s test time).
Every scenario ends with `git fsck --no-dangling` saying nothing but notices
in the work clone, and with a search of the journal file and every status
line the walk recorded for the walk's two passwords (`no_secret_leaked`);
under `cargo test`, `captures_are_named_keyed_and_listed_in_the_matrix` reads
all sixteen committed capture files and searches them the same way. A
password typed by the walk goes through `Step::TypeSecret`, which the walk's
trace never echoes.

The walk's own `git` runs isolated from the machine (`run_isolated`), and
each work clone has `credential.helper` set to the empty string, so a
Keychain or `gh` helper on the machine cannot answer a prompt the walk
expects to see itself.

Captures: `docs/shots/m4-<nn>-<what>-<WxH>.txt`, eight visual results at
80x24 and 160x48, Unicode tier (`.scratch/m4-remotes/pty-walk-m4.py <WxH>`
after `cargo build -p graft --example m4_walk`, `TRIB_TARGET` naming the
target directory). The script runs `m4_walk --capture <scenario>`, which
builds the remote, runs the scenario's headless prefix, then opens the real
loop on the work clone with the capture's keys as the harness prelude (an
open credential prompt counts as settled, so the typed password lands in the
masked field). It fails, and writes nothing, if a password is on screen or
the masked capture shows no `•`. **A capture is what pyte saw, not what a
person saw, and it carries no colour** — the user judges by eye.

### What the walk proves and what it does not

- **Proves:** with a bare over `file://`, over loopback HTTP with Basic
  auth, and behind a socket that never answers, every fetch, pull and push
  above ends inside Tributary — a password is typed into Tributary's own prompt, a
  rejection is a sticky line with the next key in it, a hung fetch, pull or
  push ends on `Esc` with no lock file left and nothing moved on either side
  (`hung-remote-esc-cancels`, `hung-remote-pull-esc-cancels`,
  `hung-remote-push-esc-cancels`) — and the refs on both sides are what the
  entry says. `u` after a fetch or push moves nothing on the remote, ever
  (`u-never-touches-the-remote`, `push-plain`).
- **Loopback only, no TLS, no SSH.** The HTTP server is `git http-backend`
  behind a tiny server on `127.0.0.1`; nothing here exercises certificates,
  proxies, `ssh-agent`, host keys or `SSH_ASKPASS`. The SSH askpass path is
  set up but **unverified** until a field note says it worked.
- **No real credential helper.** Every clone disables helpers, so what a
  Keychain helper does before Tributary's prompt is not covered.
- **Esc during `u`/`U` does nothing** (m4-05 review): the line says `undo
  cannot be stopped part-way — finishing…` and the undo completes. The walk
  never presses it.
- **Fetch and push are transparent to undo; an up-to-date pull is
  journalled but skipped by `u`.** The walk asserts the first (`u` after a
  fetch says `nothing to undo`) and the journal rows for the second; it has
  no up-to-date pull scenario.
- **A lease push is offered only for the branch whose upstream is HEAD's**;
  the walk's rejections are always on `main`.
- **The remote hints in the footer (`f fetch  p pull  P push`) draw only at
  156 columns or more**; the 80x24 captures have none, by design, not by
  fault.
- **The password lives in a plain `String` inside the prompt's `TextField`**
  while the prompt is open — not zeroed on close. The walk proves it never
  reaches a file or the screen, nothing more.
- **No colour, no focus, no real terminal.** pyte grids; the user's by-eye
  pass is the table at the end.

### Reading the entries

Every scenario starts on `Shape::Linear`; the bare's `main` and the work
clone's `main`/`origin/main` agree before the scenario's setup runs. "The
teammate commits" means `teammate` adds `remote-N.txt` and pushes, so the
bare's `main` is one ahead; "ours locally" means a commit `ours` in the work
clone. `<tip>` is whatever `rev-parse` says that run — SHAs differ per run
(the fixtures are timestamped), so the entries name the relation, and the
walk asserts it. Status lines, box titles and the box lines quoted below are
asserted by the walk as substrings (`sticky_containing`, `done_containing`,
`box_title_starts`, `box_message_contains`); a box's button row
(`[ Cancel ]     Push`) and the exact `git push` command are from the
captures only. A `sticky` line stays until the next key, a `done` line goes
at the next refresh.

### The scenarios

1. **fetch-brings-new-commits** — the teammate commits; `f`: status
   `fetched …`; `git -C work rev-parse refs/remotes/origin/main` equals
   `git -C remote.git rev-parse main`, `HEAD` unmoved; the journal's last
   entry is kind `Fetch`, status `Committed`. `u`: status `nothing to undo`,
   every ref in `for-each-ref` identical to before `u`, the bare untouched.
2. **fetch-prunes-deleted-branch** — the bare had `gone`, the clone fetched
   it, the bare deleted it; `f`: `refs/remotes/origin/gone` is no longer in
   the clone's `for-each-ref`.
3. **pull-fast-forward** — the teammate commits; `p`: status `pulled
   origin/…`; `HEAD` equals the bare's `main`, and the teammate's file reads
   byte for byte what `git -C remote.git show <tip>:<file>` gives. `u`:
   `HEAD` back to the old tip, every file back, `origin/main` **stays** at
   the bare's tip, the bare unchanged. `U`: `HEAD` forward to the bare's tip
   again.
4. **pull-diverged-refused** — the teammate commits, ours locally; `p`:
   sticky `pull stopped: … diverged … m merge it · r rebase onto it`;
   `HEAD` unchanged, `origin/main` advanced to the bare's tip (the fetch half
   happened), the bare unchanged.
5. **pull-dirty-file-sets-it-aside** (was pull-dirty-file-refused, before
   m7-power-tools/03b) — the teammate rewrites a tracked file's one line and
   pushes; the same line is edited in the clone; `p`: the edit is set aside,
   the pull fast-forwards, the put-back clashes and the resolver opens on
   the file; `HEAD` at the teammate's commit, the file holds both his line
   and theirs.
6. **pull-no-upstream-refused** — `branch --unset-upstream`; `p`: the line
   contains `has no upstream`, nothing was queued (`operating` empty), no
   ref moved on either side.
7. **push-plain** — ours locally; `P`: a box titled `Push main →
   origin/main`, message `1 commit · can't be undone from here — origin
   keeps what you…`, command `git push --progress origin
   refs/heads/main:refs/heads/main`, `[ Cancel ]     Push`. `→` `⏎`, or `y` (the bar reads `y push`): status
   `pushed …`; `git -C remote.git rev-parse main` equals `git -C work
   rev-parse HEAD`; the journal's `Push` entry has `refs_before
   {refs/remotes/origin/main: <old tip>}` and `refs_after {…: <new tip>}`.
   `u`: `HEAD` back to the old tip, status ends `origin/main still has it`,
   **the bare unchanged**. `U`: `HEAD` forward again.
8. **push-sets-upstream** — on a new `feature` branch; `P`: the box's
   message contains `sets upstream`; `⏎`: the bare has `refs/heads/feature`
   at the local tip, `branch.feature.remote` is `origin`,
   `branch.feature.merge` is `refs/heads/feature`.
9. **push-detached-refused** — `HEAD` detached; `P`: no box, the line
   contains `detached`, the bare unchanged.
10. **push-rejected-then-lease** — the tip is amended locally (a commit
    origin already has, rewritten; the bare has not moved, so `origin/main`
    is current); `P` `→` `⏎` (or `P` `y`): sticky `push rejected: origin/main has commits you don't
    · p pull, then m or r · P force with lease`, the bare unchanged. `P` again: a box titled `Force-push main →
    origin/main?`, `origin/main <old> → <amended> · refused if origin moved
    since…`, `rewrites history for everyone who has <old>`, command `git
    push --progress --force-with-lease=refs/heads/main:<old sha>…`,
    `[ Cancel ]     Force push`. `⏎`: `git -C remote.git rev-parse main`
    is the amended tip.
11. **push-lease-refused-stale** — the teammate commits (the bare moved,
    the clone has not fetched, so `origin/main` is stale), ours locally;
    `P` `→` `⏎`: sticky `push rejected …`; `P`, `⏎` on the lease box:
    sticky `push refused: … stale lease … f to fetch`; the bare's refs
    identical to before the scenario — the lease protected the teammate's
    commit.
11a. **push-rejected-rebased-then-plain** — as 11 (the teammate pushed, ours locally); `P` `→` `⏎`: sticky `push
    rejected …`; `p`: `main and origin/main have diverged · m merge it  r rebase onto it`; `r` `r`: rebased; `P`:
    a box titled `Push main → origin/main` (not `Force-push`), its command without `--force-with-lease`; `⏎`: the
    bare's `main` is the local tip, the teammate's commit its ancestor.
12. **wrong-password-then-right** — `HttpRemote` with `tester` /
    `correct-…`; ours locally; `P` `⏎`: a masked prompt `Password for
    'http://tester@127.0.0.1:<port>':`; the wrong password: sticky
    `authentication failed for http://127.0.0.1:<port>/repo.git/ · P to
    try again` (the full URL, not `host:port` as the design pictured — the
    ticket's NOTES), the bare unchanged. `P` `⏎`, the right password:
    the bare's `main` equals `HEAD`; the server counted at least one
    authorised and one refused request; neither password is in the journal
    file, any status line, or any capture.
13. **password-prompt-esc-cancels** — the prompt is up; `Esc`: the prompt
    is gone, the journal's last entry is `Push` with status `Aborted
    ("cancelled")`, the bare unchanged, **zero** authorised requests reached
    the server.
14. **username-then-password** — the URL carries no user; `P` `⏎`: an
    unmasked prompt `Username for 'http://127.0.0.1:<port>':`, showing
    `▏tester` as typed; `⏎`: a masked `Password for …`; the password, `⏎`:
    the bare's `main` equals `HEAD`.
15. **hung-remote-esc-cancels** — the remote is a socket that accepts and
    never answers; `f`: `operating` is set; once the socket has one
    connection, `Esc`: the line ends `cancelling`, `operating` clears in
    under 5 s, the line then reads `fetch all remotes cancelled`; the
    journal's last entry is `Fetch` `Aborted("cancelled")`, no `*.lock`
    under `.git`, the socket saw exactly one connection, the bare's refs and
    the work clone's `HEAD`, refs, index (bytes too) and files are what they
    were before the key.
16. **hung-remote-pull-esc-cancels** — the same socket; `p`: `operating` is
    set; once the socket has the connection, `Esc`: `cancelling`, clear in
    under 5 s, then `pull origin/main cancelled`; the last entry is `Pull`
    `Aborted("cancelled")`, no `*.lock`, one connection, and — this is the
    operation that would write them — `HEAD`, `.git/index` (byte for byte)
    and every file are unchanged, the bare too.
17. **hung-remote-push-esc-cancels** — the same socket, ours locally; `P`
    `→` `⏎`: the `Push ` box, then `operating` is set; once the socket has
    the connection, `Esc`: `cancelling`, clear in under 5 s, then `push main
    → origin/main cancelled`; the last entry is `Push` `Aborted("cancelled")`,
    no `*.lock`, one connection, the bare's refs unchanged, `HEAD` still at
    `ours`.
18. **progress-reaches-the-footer** — the bare gains 400 files of 4 KiB;
    `f`: some recorded status line starts `fetch all remotes · ` and
    contains `objects` (`fetch all remotes · enumerating objects · esc
    cancels` in the capture).
19. **u-never-touches-the-remote** — fetch, pull, commit, push; then `u`
    until `nothing to undo`: `git -C remote.git for-each-ref` is identical
    to right after the push; `HEAD` is back where the scenario started;
    `origin/main` is at the pushed tip (the tracking ref is not undone).
20. **journal-rows** — fetch, pull, commit (`Ctrl+R` after), push; `J`: the
    journal holds exactly `Fetch, Pull, Push` in that order; the push row's
    summary contains `not undoable — origin has it`, the fetch row's `not
    undoable`, the pull row is the next undo (`← u`).
21. **no-index-write-on-fetch** — `f`: `.git/index` is byte for byte what
    it was.

### Per breakpoint

What each capture shows; 120x40 is `not run` (the script takes two sizes).

| View | 80x24 | 120x40 | 160x48 |
|---|---|---|---|
| Push box (`m4-01-push-box-plain`) | `Push main → origin/main`, the `1 commit · …` line, the `git push --progress …` command, `[ Cancel ]     Push`; **no frame is drawn around the box**, and the graph's `Comm` shows left of it — judge whether that reads as a box | not run | same lines, the graph and pane around it |
| Lease box (`m4-02-push-box-lease`) | `Force-push main → origin/main?`, `origin/main <a> → <b> · refused if origin moved since…`, `rewrites history for everyone who has <a>`, the `--force-with-lease=` command, `[ Cancel ]     Force push` | not run | same |
| Credential prompt, masked (`m4-03-credential-masked`) | `Credentials — 127.0.0.1:<port>`, `Password for 'http://tester@127.0.0.1:<port>':`, a field of `•` only (28 of them), `typed here only · never stored by Tributary`, `⏎ send  ⌃U clear  esc cancel push main → origin/main` in the border (m6-conflicts/78; `esc cancel the push` only when that does not fit), `⏎ send  esc cancel` on the bottom row (m6-conflicts/69); pty capture re-taken 2026-10-01 (m7-power-tools/30u) | not run | same |
| Credential prompt, username (`m4-04-credential-username`) | `Username for 'http://127.0.0.1:<port>':`, `▏tester` in clear | not run | same |
| Progress footer (`m4-05-progress-footer`) | `fetch all remotes · enumerating objects · esc cancels` on the status line | not run | same; taken the instant it appeared, the frame is not settled |
| Rejected sticky (`m4-06-rejected-sticky`) | `push rejected: origin/main has commits you don't · p pull, then m or r · P force with lease` — both keys on it; at 80 columns the reason is cut with `…` and both keys stay (`m4-06-rejected-sticky-80x24.txt:26`) | not run | the same line |
| Journal (`m4-07-journal-remote-rows`) | three rows; the push row reads `not undoable — origin has it · <a> → …` (the verdict first, the ref pair is what 80 columns cuts); the pull row `1 ref · the index · the…  ← u`; summaries in one column, the push row's description cut to `push main →…` so its verdict fits (24-b) | not run | push `not undoable — origin has it · <a> → <b>`; pull `1 ref · the index · the working tree · backup saved  ← u`; fetch `origin/main moved · not undoable` (the refs it moved, m8/21c; `nothing new` when none moved, per 21c's NOTES, not seen here). The committed capture still reads `remote-tracking refs`: its re-shoot draws every Tributary row's age as `future` (frame pinned, journal rows stamped by the wall clock; m8/19d-c NOTES), so it is not re-taken yet |
| `u` after a push (`m4-08-undo-after-push`) | `undo: commit "ours" · origin/main still has it ✓` | not run | same; the footer ends `S stash  H hide stash  f fetch  p pull  P push` (the remote hints need 156 columns) |

### Per terminal: walk the remotes group

Run `cargo run -p graft --example m4_walk` (all `ok`), then in a clone of
your own with a real remote: `f`, `p`, `P` on a branch that is ahead, `P`
after a teammate pushed (expect the rejection, then the lease box), and a
push to an HTTPS remote with no credential helper (expect Tributary's own
prompt: **the password field shows `•` only**, and after `Esc` nothing was
sent). Press `Esc` during a fetch that is slow. Record the result here,
naming the scenario that failed if one did. Colour, focus and the box's
frame are what only this pass can judge.

| Terminal | Result |
|---|---|
| Ghostty | not run |
| Zed | not run |
| VS Code | not run |

### The review's runs

The reviewer's, not the builder's (ADR-0021): figures `not run` here.

```sh
cargo test --features heavy-tests -- --test-threads=2                # the heavy tier — not run
cargo test --release -p graft --features heavy-tests --test it -- frames:: heavy::budgets_hold --test-threads=1 --nocapture   # release perf — not run
cargo run --release -p graft --example m4_walk                       # the walk under release — not run
```

**Times I left:** every remote-related reach for the shell during daily use
— a clone, a remote added, a merge after a diverged pull, a stalled fetch
you killed — goes in the table at the top of this file, dated, the same day
(ADR-0033). That table, not this walk, measures the promise.

## History

M5's acceptance ticket (m5-history-and-ghost-preview/11). M5 promises that
**you see the result before you confirm**. The walk is how a person checks
that: one command, 49 scenarios — every one m5/11 asked for — each a
scratch copy of a corpus shape with a history scripted by git, one of them
a work clone whose `origin` is Basic-auth HTTP so git really asks for a
password, the real app driven by keys and a mouse
drag over the real job pool and real `git`, and then **the truth read back
by running git** (`for-each-ref`, `rev-parse HEAD`, `symbolic-ref`,
`write-tree`, `ls-files -u`, `stash list`, file bytes, `.git/index` bytes,
the `.git/MERGE_HEAD`/`rebase-merge` markers, the journal file) — never
from `AppState`. The one thing read from `AppState` is **the claim**: the
plan behind the preview box (`Step::Claim`), so `ghost_equals_git` can say
whether the ghost equals what git did — each ref move's `to` is where the
ref is, each ghost's subject and parents are the new commit's, each
replayed ghost's patch-id is its source's (reversed for a revert), the
head is where the plan said, and every path the box named is among the
files that changed. `every_scenario::ghost_equals_git_is_the_only_claim_check`
greps the scenario files so no scenario judges the plan any other way.

```sh
cargo run -p graft --example m5_walk                              # every scenario, one line each
cargo run -p graft --example m5_walk -- --only rebase-two-commits-ghost-equals-git   # one
cargo run -p graft --example m5_walk -- --list                    # the names
cargo run -p graft --example m5_walk -- --stems                   # capture stems, keys, expected text
cargo run -p graft --example m5_walk -- --matrix                  # rewrites docs/shots/35-coverage-matrix.txt (all five tables)
```

`ok <name>` per scenario, `FAIL <name>: <what differs>` (20 lines at most:
refs, HEAD, index tree, files, journal length, the claim's own words), exit
1 on any failure. The same list runs in the fast tier as one test per
scenario (`tests/it/m5_walk/`, `m5_walk::every_scenario::*`). Every
scenario ends with `git fsck --no-dangling` saying nothing but notices.

**All 49 scenarios pass.** The last one to fail on purpose was
`fix-rebase-instead`, left failing by m5/38 as a finding and fixed by
m5/43: the rebase plan drew a ghost for a **merge commit** inside the
replayed range (one ghost per commit in the range
`graft-git/src/history_reads.rs` handed back — `rev-list` with no
`--no-merges`), and `git rebase` without `--rebase-merges` drops it, so the
box promised two new commits where git makes one and the branch's predicted
tip was a commit that never exists. The range the preview counts now
matches the one git replays.

The five the acceptance walk left failing on purpose before that were fixed
in m5/32:

- `merge-no-ff-ghost-equals-git`, `rebase-two-commits-ghost-equals-git`,
  `rebase-via-drop-menu`, `cherry-pick-via-menu-ghost-equals-git` — the
  box's **working-tree line named files that do not change**: a merge of
  `feature` into `main` drew `main-one.txt` (main's own file) as updated
  and said `4 files updated` where git changes two. One cause in all four:
  the line diffed the two tips the operation mentions, which also counts
  `HEAD`'s own commits. It now answers one question — which files differ
  between the tree `HEAD` has now and the tree it will have once the
  operation lands — in one place (`graft-ops/src/preview.rs`, `Landing`
  and `worktree`). Direct tests: `graft-ops::it history_ops::
  a_no_ff_merge_names_only_the_files_the_source_brings`,
  `a_rebase_names_only_the_files_onto_brings`,
  `a_cherry_pick_names_only_the_picked_patch`,
  `a_fast_forward_merge_and_the_fork_point_rule_agree` — each draws the
  plan, runs the operation, and compares the drawn paths with the file
  **bytes** that changed on disk.
- `pull-diverged-offers-m-and-r` — after `p` on a diverged branch the line
  offers `m` and `r`, and the tip it remembers for `origin/main` was the
  one from **before** the pull's own fetch, so `r` was refused once with
  `main moved since the preview · r again`. The refusal machinery
  (`tip_holds`, `Refusal::Moved`) was right and is untouched; the record
  was wrong. While the line is on screen the tip is now re-read from the
  refs at the tail of every `update`
  (`graft/src/update/remote.rs`, `forget_stale_divergence`), so both halves
  of the pair `r` pins come from one reading of the refs. Test:
  `graft update::remote::tests::
  the_diverged_line_aims_at_where_the_fetch_left_the_tracking_ref`.

Captures: `docs/shots/m5-<nn>-<what>-<WxH>.txt`, eight visual results at
80x24 and 160x48, the banner also at 120x40, the too-small preview at
38x10 only, Unicode tier
(`.scratch/m5-history-and-ghost-preview/pty-walk-m5.py <WxH>` after
`cargo build -p graft --example m5_walk`, `TRIB_TARGET` naming the target
directory). The script runs `m5_walk --capture <scenario>`, which builds
the scratch, runs the scenario's headless prefix (the time machine's three
stages), then opens the real loop with the capture's keys as the harness
prelude — fed one key per settled state, and since this ticket a pending
preview plan counts as unsettled, so `→ ⏎` land on a drawn box. A preview
capture whose box is not on screen is **not written**. **A capture is what
pyte saw, not what a person saw, and it carries no colour** — the user
judges by eye. The contact sheet is `docs/shots/m5-contact-sheet-1.txt` and `-2.txt`
(`scripts/contact-sheet.sh m5`; `contact_sheet_captions_cover_every_m5_capture`
keeps every `m5-*.txt` captioned).

### What the walk proves and what it does not

- **Proves:** for a rebase of two commits, a fast-forward and a no-ff
  merge, a cherry-pick, a revert, a mixed and a hard reset, a branch and a
  detached checkout, and a rebase onto a diverged upstream, the refs and
  the new commits after `Confirm` are what the ghost drew — parents,
  subjects, patch-ids, HEAD (`ghost_equals_git`); `Esc` and `Enter`-on-
  `Cancel` run nothing and journal nothing; a ref that moved under the box
  is refused and nothing moves; a 38x10 screen never confirms; a dirty
  tree's rebase opens the preview with the edits set aside (m6-conflicts/73,
  74; scenario 12 below); a hard reset's box says `LOSES
  base.txt (backup saved)` and `u` brings the bytes back; a conflicting
  merge stops with `MERGE_HEAD` present, the banner focused,
  `shared.txt` unmerged, `a` asks `Cancel`/`Abort` first and restores the
  tree; the stop survives a relaunch; the time machine's three steps put
  the index tree back to before the first stage, and three `U` redo it.
- **Proves the worktree forecast** since m5/32: every `ghost-equals-git`
  scenario checks each path the box named against the files that really
  changed, and the four `history_ops` tests above check the whole path set
  both ways against the bytes on disk.
- **Proves refs and names** (m5/37): `b` and `t` create straight from the
  graph with no preview, and a bad or already-taken name is refused in the
  box before `Enter` does anything; a branch or tag delete and moving a
  branch that is not checked out (drag → `Move … to here`) both go through
  the Ghost Preview and `ghost_equals_git`; `u` restores a deleted branch,
  a deleted tag, a stash `git stash branch` consumed, and a moved branch,
  each to its old commit.
- **Proves the conflict keys and the journal row** (m5/38, m5/39): `c`
  after a resolve, a rebase that stops twice under one entry, `M` refusing
  in words with no `merge.tool`, `e` asking for the editor at the first
  `<<<<<<<` marker, `u` refused mid-operation, and the journal's live row
  reading `in progress`, never `interrupted`.
- **Proves `u` after a push, the secret field and the 80-column hints**
  (m5/39): the three-choice box and all three of its answers against the
  bare's own refs; a password that draws eight glyphs whatever its length
  and reaches no frame, event, status line, state dump, askpass argument
  or environment, or file; and the footer's three-and-`. more` in five
  contexts.
- **Does not walk** the real `git mergetool` hand-over, the real `$EDITOR`
  hand-over (`e` is pressed for its commands and never dispatched), or an
  interactive rebase — all external programs or M6/M7 work. The
  cancel-mid-undo *step boundary* is `graft-ops::it undo_cancel`'s, not
  this walk's (scenario 46).
- **No drop-menu capture, no name-prompt capture, no stale-preview
  capture**: a pty run has no mouse prelude and no hook between two keys.
- **No colour, no focus, no real terminal.** pyte grids; the user's by-eye
  pass is the table at the end.

### Reading the entries

Every scenario starts on `Shape::Trivial` (four commits) unless it says
otherwise; the setup commits are timestamped from a fixed epoch, so SHAs
are stable per shape but the entries name the relation. `forked` means:
`main` gains `main moves on`, `main moves again`; `feature` forks off `add
the base` with `start the feature`, `finish the feature`; `feature` is
checked out. Box lines and status lines quoted below are asserted as
substrings (`box_says`, `title_names`, `status_containing`); button rows
and the exact git command are from the captures only.

### The scenarios

1. **preview-opens-with-ghosts-and-plan-commands** — `r` on `main`: title
   `Preview: rebase …`; box `2 commits replayed`, `feature `, `→ 2 new
   commits`, `no conflicts expected`, `undo: u takes this back`; ghost rows
   drawn; the plan is one command starting `git rebase `; nothing ran.
2. **esc-cancels-nothing-runs** — `Esc` on the box: no box, nothing
   operating, refs and files as before.
3. **cancel-is-the-default-choice** — `Enter` at once: same (ADR-0038 §1).
4. **stale-preview-refused** — `git update-ref refs/heads/main main~1`
   while the box is up; `Enter`: `main moved since the preview` and `r
   again` on the line; `main` where the shell put it, `feature` unmoved,
   no journal entry.
5. **preview-too-small-never-confirms** (38x10) — the box does not fit;
   `Enter` leaves the preview open, nothing ran, no entry.
6. **preview-refused-during-an-operation** — `conflicting`; `m` on
   `feature`, confirm, the merge stops; `Esc` off the banner, then `m` and
   then `r`: each says `finish or abort the merge first`, neither opens a
   box. `MERGE_HEAD` still present, refs and files byte-identical to the
   stop, journal length unchanged.
7. **merge-ff-preview-equals-git** — `forked_on_main`; `m` on `feature`,
   confirm: `main` equals old `feature`, tree matches; `u` puts `main`
   back.
8. **merge-no-ff-ghost-equals-git** — `m` on `feature` from `main`:
   `ghost_equals_git`, `u` restores, `U` redoes.
9. **merge-forecast-names-the-conflicting-path** — `conflicting`; box says
   `may conflict` and `shared.txt`.
10. **merge-already-up-to-date-offers-close** — `feature` left at the fork
    while `main` moved twice; `m` on `feature`: the box says `nothing to
    do` and its buttons are exactly `["Close"]`. `→` then `Enter` closes
    it: the plan moved no ref and drew no ghost, refs and files are as
    before, git is mid-nothing, the journal is empty. The one merge
    scenario `ghost_equals_git` cannot judge — it refuses a plan with no
    ref move — so the refs are asserted directly.
11. **rebase-two-commits-ghost-equals-git** — `r` on `main` from `feature`:
    `ghost_equals_git`, the two new commits' patch-ids equal the old tips'.
12. **rebase-dirty-tree-opens-the-preview** — a dirty `base.txt`; `r`
    opens the rebase preview with `Confirm`, no clean-tree refusal (the
    edits are set aside, m6-conflicts/73); `Esc`: nothing moved.
13. **rebase-via-drop-menu** — drag `feature`'s badge onto `main`'s row,
    pick `Rebase`: same holds as 11.
14. **cherry-pick-via-menu-ghost-equals-git** — `.` on `finish the
    feature` from `main`, `Cherry-pick`: one ghost, patch-id equal.
15. **revert-via-menu-ghost-equals-git** — `.` on `main moves again`,
    `Revert`: one ghost whose patch is the reverse.
16. **revert-merge-uses-mainline** — `forked_on_main` with `feature`
    merged in by git; `.` on `main`'s tip (the merge), the row
    `Revert <sha7>, -m 1`: the plan's command starts `git revert ` and
    carries `-m 1`, one ghost, `ghost_equals_git` (the ghost's patch is the
    merge's reversed **against parent 1**); after it `work-one.txt` and
    `work-two.txt` are out of the tree, `base.txt`, `main-one.txt` and
    `main-two.txt` are still in it, `main~1` is the merge, last entry
    `Revert` `Committed`.
17. **reset-hard-says-loses-and-u-brings-it-back** — dirty `base.txt`; `.`
    on `main moves on`, `Reset main to here, hard`: box `working tree:
    LOSES base.txt (backup saved)`; confirm: `main` at `main moves on`,
    the tree clean; `u`: `main` back, `base.txt` bytes back.
18. **reset-mixed-keeps-the-tree** — same row, `mixed`: `main` moved, the
    file bytes untouched.
19. **checkout-branch-via-menu** — `Checkout main`: `HEAD` symbolic to
    `refs/heads/main`, files are main's; `u` puts `feature` back.
20. **checkout-detached-via-menu** — `Checkout <sha>, detached`: `HEAD`
    detached at the row.
21. **fix-revert-the-merge** — `Shape::FoxtrotMerge` (the shape the radar's
    own tests use), `main` checked out on the foxtrot; wait for the radar's
    context, then `.` on `main`: the row `Fix: revert the merge, -m 2`.
    Confirm: the plan's command carries `-m 2`, `ghost_equals_git`,
    `feature.txt` is out of the tree, `base.txt` and `main.txt` are still
    in it, `main~1` is the merge, last entry `Revert` `Committed`.
22. **fix-reset-to-before-the-merge** — the same row, `Fix: reset main to
    before the merge`: `ghost_equals_git`, `main` is the merge's **second**
    parent (`<merge>^2`, its real previous tip), `feature.txt` out of the
    tree, last entry `Reset` `Committed`; `u` puts `main` back on the
    merge.
23. **fix-rebase-instead** — the same row, `Fix: rebase instead`: the
    replayed tip has exactly one parent and it is `<merge>^2`,
    `base.txt`/`main.txt`/`feature.txt` all still in the tree, last entry
    `Rebase` `Committed`, `u` puts `main` back on the merge, and
    `ghost_equals_git` holds. It did not until m5/43: the plan drew a ghost
    for the merge commit inside the replayed range, which `git rebase`
    without `--rebase-merges` drops, so the box promised two new commits
    where git makes one (found by m5/38, left failing as a finding). The
    claim is judged **last** in this scenario, so the eight checks above it
    are reached whatever the claim says.
24. **stop-then-esc-lists-files** — `conflicting`; `m` on
    `feature`, confirm: `MERGE_HEAD` present, journal `Merge` `Pending`,
    banner focused, `unmerged_paths` is `[shared.txt]`, the file starts
    `<<<<<<<`.
25. **banner-a-aborts-and-restores** — `a`: choices `["Cancel", "Abort"]`;
    `Abort`: no `MERGE_HEAD`, `shared.txt` is main's, entry
    `Aborted("aborted")`.
26. **conflict-survives-relaunch** — close and reopen the app mid-merge:
    the line says `still in progress from a previous run`, git still
    merging.
27. **continue-refuses-until-resolved-then-one-entry-undoes** — the stopped
    merge; `c` at once: `1 file still conflicted · ⏎ resolve  M mergetool
    e edit`, `MERGE_HEAD` still there and `write-tree`
    still refusing the index (an unmerged entry is what it refuses). Then
    `Esc`, `shared.txt` written as `resolved by hand\n`, `space` on it in
    the changes pane, `⇥` back to the banner, `c` again: git has **one**
    `Merge` entry and it is `Committed`, the new tip's parents are old
    `main` then old `feature`, `shared.txt` is `resolved by hand\n`. One
    `u` takes the whole thing back: `main` at its old commit,
    `shared.txt` back to `main's line\n`.
28. **rebase-stops-twice-same-entry** — `main` and `feature` both rewrote
    `shared.txt`, `feature` twice; `r` on `main` from `feature`, confirm:
    git stops, is resolved as `resolved once\n`, `c`, **stops again** at a
    different `HEAD`, is resolved as `resolved twice\n`, `c`. At both
    stops there is exactly **one** `Rebase` entry and it is `Pending`;
    after the second `c` that same one entry is `Committed` — one entry
    across two stops. The replayed branch's `~2` is `main`'s tip and
    `shared.txt` is `resolved twice\n`. One `u` puts `feature` back at its
    old commit with every file byte-identical to before the rebase.
29. **mergetool-absent-refuses-in-words** — the stopped merge of 28's
    shape, banner focused, `merge.tool` unset (the fixture blanks the
    machine's config): `M` says `no merge.tool configured · e edits
    shared.txt  a abort` and **runs nothing** — no write started, git still
    merging, `shared.txt` byte-identical, the journal's one `Merge` entry
    still `Pending`. The real hand-over to `git mergetool` is an external
    program and is in no walk; only the refusal is.
30. **e-opens-editor-at-first-marker** — the same stop; `e` is pressed for
    its **commands only**, never dispatched, so no `$EDITOR` starts in a
    headless run: exactly one `Command::OpenConflictEditor { path:
    "shared.txt", line }`, and `line` is the line number of the first
    `<<<<<<< ` in the file's own bytes, found by scanning them here.
    Nothing ran, git is still merging.
31. **u-refused-during-an-operation** — the same stop; `u`: `finish or
    abort the merge first`, no write started, `MERGE_HEAD` still there,
    refs, files and journal length unchanged. Waits on `Wait::
    NotInProgress` rather than sleeping, so the refusal is judged against
    a re-read in-progress state and not a stale one.
32. **journal-row-says-in-progress** (120x40) — the same stop; `Esc` off
    the banner, `J`: the drawn frame carries `in progress — c continue  a
    abort` and **nowhere says `interrupted`** — the row is live, not the
    wreck of a previous run (m5/14).
33. **time-machine-three-steps** — `three_to_stage`; stage three files;
    `J`, down to the first entry, `Enter`: title `Preview: revert to before
    "stage one.txt" (3 steps)`, box `restored from backup`; confirm: the
    index tree equals the tree before the first stage; three `Undo`
    entries; `U` ×3 stages them again.
34. **time-machine-esc-returns-to-journal** — `Esc` on that box: the
    journal browser is back, shown and ready.
35. **pull-diverged-offers-m-and-r** — `Shape::Linear` with a bare; the
    teammate commits on the bare, `ours locally`; `p`: `main and
    origin/main have diverged · m merge it  r rebase onto it`, no ref
    moved; `r`, confirm: one ghost, `ghost_equals_git`, `main~1` is
    `origin/main`, the bare untouched.
36. **u-after-push-asks-once** — `Shape::Linear` with a bare; `feature`
    merged into `main`, then `P`. `u`: a three-choice box, title `Undo
    after a push`, a line saying `undo works on your local branch only`,
    choices exactly `["Cancel", "Undo locally", "Re-push old tip"]`,
    cursor on `Cancel`. `Undo locally`: local `main` is back at its
    pre-merge commit and **the bare is untouched**. A second `u` opens no
    box — asked once per entry.
37. **u-after-push-repush-old-tip** — the same up to the box; `Re-push old
    tip`: the lease box's command carries `--force-with-lease` and
    nothing is running yet; confirm: the bare's `main` is back at its
    pre-push commit and local `main` is untouched. The walk waits on
    `Wait::RemotesCaughtUp` before `u`, because a lease built from a
    stale reading of `origin/main` is refused by git, not by the app.
38. **u-after-push-cancel-asks-again** — the same box; `Esc`: nothing ran,
    nothing journaled, refs and files as they were — and `u` again asks
    again. Cancelling is not an answer.
39. **b-creates-and-checks-out** — `forked`; cursor on `main`, `b`,
    `release`, `⇥` (checkout), `Enter`: `refs/heads/release` is `main`'s
    commit, `HEAD` symbolic to it, one journal entry naming `release`. No
    preview — a create moves no history.
40. **b-refuses-bad-and-existing-names** — `b`, `bad name` (a space):
    `name_refusal` starts `bad name:`; `Ctrl+U`, `main` (already a
    branch): `name_refusal` is `exists`; `Esc`: the prompt closes, refs
    and files unchanged, nothing journaled.
41. **t-creates-a-tag** — cursor on `feature`, `t`, `v2.0.0`, `Enter`:
    `refs/tags/v2.0.0` is `feature`'s commit, `cat-file -t` says `commit`
    (lightweight, no tag object), one journal entry.
42. **delete-branch-then-u-restores** — `.` on `main`, `Delete branch
    main`, confirm: `ghost_equals_git`, `refs/heads/main` gone, one
    `BranchDelete` entry; `u`: `main` back at its old commit.
43. **delete-tag-then-u-restores** — a tag `v1` on `main`'s tip; `.` on
    `v1`'s row, `Delete tag v1`, confirm: `ghost_equals_git`,
    `refs/tags/v1` gone, one `TagDelete` entry; `u`: `v1` back.
44. **stash-b-branches-from-stash** — one stashed entry `wip`; `S`, `b`,
    `rescued`, `Enter` opens the Ghost Preview, confirm: `ghost_equals_git`,
    `HEAD` symbolic to `refs/heads/rescued`, `stash list` empty, one
    `BranchFromStash` entry; `u`: the stash (`wip`) is back.
45. **move-a-branch-not-checked-out** — `forked_on_main`; drag `feature`'s
    badge (not current) onto `main`'s row: the drop menu offers `Move
    feature to here`, not a `Reset` row; pick it, confirm:
    `ghost_equals_git`, `refs/heads/feature` equals `main`'s old commit,
    `HEAD` stays on `main`, the tree untouched, one `Reset` entry; `u`:
    `feature` back.

46. **esc-cancels-undo-and-rolls-back** — `forked_on_main` with a dirty
    `base.txt`; `.` → `Reset main to here, hard`, confirm, then `u` and
    `Esc` on the very next `update`, through the real `CancelJob` path.
    The line says `undo cancelled`; refs, files, the index tree and
    `HEAD` are exactly what they were before `u`; no `Undo` entry is
    `Committed`. **What it does not prove:** that the cancel landed
    *between two steps* of the inverse plan. From the real key path it
    never does — `Esc` always beats the worker to `restore::execute`'s
    first `should_stop`, so no journal entry is opened at all and the
    `Aborted("cancelled")` row m5/11 asked for is unreachable here.
    `graft-ops`' own `it undo_cancel` owns every step boundary, with a
    stop keyed to the plan's effect.
47. **ctrl-r-twice-leaves-one-history-job** — two `⌃R`s with nothing
    pumped between them: one walk is awaited, none is being promoted, and
    then the rows, the refs and the working tree all come back. Reaching
    that is the assertion — a walk left parked on the two-worker pool
    starves the reads queued behind it and the wait never returns. Refs,
    files and journal length are unchanged by a refresh.
48. **password-field-is-secret** — `Shape::Linear` over **Basic-auth
    HTTP** (the one transport that makes git ask); `P`, confirm, git's
    password question. The field is masked; `hunter2` (7 characters) and
    then `correct-horse-battery-staple` (28) each draw **eight** glyphs
    in one run and nothing else — the row says nothing about the length
    (ADR-0038 §2). `Enter`: the push lands, the bare has the commit, the
    server authorised at least one request. What the secret reached is
    checked from outside the app: no drawn frame, no event, no status
    line, no `{AppState:?}` taken while the prompt was open, no argument
    or environment variable git passed its askpass helper (a wrapper
    script logs both before handing over), and no byte of any file the
    run leaves behind, the journal among them.
49. **hints-at-80-show-three-and-more** — at 80 columns, five contexts,
    each against the design note's own list: the current-branch row `⏎
    open  c commit  P push  . more`, a plain row `⏎ open  b branch  r
    rebase  . more`, the uncommitted row `⏎ changes  c commit  S stash  .
    more`, the changes pane on an unmerged file `e edit  c commit  ⇥
    graph  . more`, and the focused banner `c continue  s skip  a abort
    M mergetool  e edit  ⇥ graph  q quit` (`q quit` last since m8-13b) —
    all of them, and **no `. more`**: they measure 65 columns, so ADR-0039's ladder never shortens them. A
    sticky line takes the footer at 80, so the banner's hints are read
    after `Esc` dismisses it and `⇥` takes the keyboard back. Looking
    wrote nothing: git is still mid-rebase with one `Pending` entry.

### Not yet walked

**Nothing. The section is empty** — all 49 scenarios m5/11 listed are
built and run (m5/39 closed the last eleven). What is still **not in any
walk** is named above and below rather than here, because none of it is a
scenario a headless run can hold: the real `git mergetool` hand-over and
the real `$EDITOR` hand-over are external programs, and a real terminal
(colour, focus, reverse video) is the user's by-eye pass.

### Per breakpoint

What each capture shows; 120x40 is `not run` except for the banner.

| View | 80x24 | 120x40 | 160x48 |
|---|---|---|---|
| Rebase preview (`m5-01-preview-rebase`) | `Preview: rebase feature onto main`, two ghost rows under `after rebase` with `—` for SHA and age, `2 commits replayed`, `feature 8d6cd28 → 2 new commits`, `working tree: 2 files updated` (the two `main` brings — proved against git by the walk), `no conflicts expected`, `undo: u takes this back`, `git rebase -i --no-update-refs --no-autostash --empty=drop main`, `▸[ Cancel ]   [ Confirm ]` | not run | same box, wider graph |
| Merge preview (`m5-02-preview-merge`) | `Preview: merge feature into main`, `1 merge commit`, `main 45fa51d → 1 new commit`, `working tree: 2 files updated` (the two `feature` brings), the `git merge --no-edit --ff -m '…' feature` command | not run | same |
| Hard-reset preview (`m5-03-preview-reset-hard`) | the box after `.` → `Reset main to here, hard`: `working tree: LOSES base.txt (backup saved)` | not run | same |
| Time machine (`m5-04-time-machine`) | `Preview: revert to before "stage one.txt" (3 steps)`, `index and working tree restored from backup`, no git line | not run | same |
| Too small (`m5-05-preview-too-small`, 38x10 only) | `preview needs 40x12 · esc closes` on the last row, the ghost rows still drawn above | — | — |
| Stopped merge (`m5-06-stop-lands-in-resolver`) | The stop lands in the resolver (m6-conflicts/18): `MERGING` on the header row, the file strip `▸ shared.txt`, the identity row `▸ shared.txt · 1 conflict · 1 open`, the bar starting `o yours` (`Esc` is the way to the banner) | the same | the same |
| Branch-row hint bar (`m5-07-branch-row-hints`) | after `r` then `Esc` on `main` from `feature`: no box, `m merge  r rebase  ⏎ open  . more` then bare keys | not run | `m merge  r rebase  ⏎ open  b branch  t tag  P push  c commit  p pull  f fetch  u undo  S stash  . more  /  q …` — **`q quit` is a bare `q` at 160 since `b` and `t` went live; the user decides whether the tail is too long** |
| Diverged pull (`m5-11-diverged-pull-sticky`) | `main and origin/main have diverged · m merge it  r rebase onto it` on the footer, `[◆ main ↑1 ↓1]` and `[origin/main] the teammate's commit` one row apart (the frame after the pull's fetch is drawn, m7-power-tools/30v) | not run | same |

### Per terminal: walk the history group

Run `cargo run -p graft --example m5_walk` (expect `49 ok, 0 failed`), then
in a clone of your own: `r` on a branch
row, look at the ghost rows and the box, `Esc`; `m` on a branch that
conflicts, confirm, resolve in your editor, `c`; `.` on a commit, `Reset …
hard` with an edited file, confirm, `u`. Record the result here, naming the
scenario that failed if one did. Colour, the focused banner's reverse
video and the ghost rows' dimming are what only this pass can judge.

| Terminal | Result |
|---|---|
| Ghostty | not run |
| Zed | not run |
| VS Code | not run |

### The review's runs

The reviewer's, not the builder's (ADR-0021): figures `not run` here.

```sh
cargo test --features heavy-tests -- --test-threads=2                # the heavy tier — not run
cargo run --release -p graft --example m5_walk                       # the walk under release — not run
```

**Times I left:** every reach for the shell during a merge, rebase, pick,
revert, reset or conflict in daily use — `git rebase -i`, a `git reset`
typed because the box did not say what you needed, a conflict you resolved
with `git mergetool` from a shell — goes in the table at the top of this
file, dated, the same day (ADR-0033). That table, not this walk, measures
the promise.

## Conflicts walk

M6's acceptance ticket (m6-conflicts/07). M6 promises that **a real
conflict is resolvable without leaving the app**. The walk is the same
machine as the History walk above — one command, a scratch copy of a corpus
shape, the real app driven by keys over the real job pool and real `git`,
the truth read back by running git — with 19 more scenarios. Most start
from `Shape::StoppedMerge`: a merge begun in the shell and stopped on
**thirteen unmerged paths, one per shape git has** (`both.txt`, `three.txt`
with three blocks, `zdiff3.txt` written with `zdiff3` markers, `crlf.txt`,
`nofinal.txt`, `md.md`, `both-added.txt` (`AA`), `pic.bin` (binary), and
the five whole-file shapes `added-by-us/them`, `ours/theirs-deleted`,
`both-deleted`). `merge-resolved-with-the-keys` aborts that merge in its
setup and **begins the same merge in the app** (`m` on `feature`): the
banner's `c` finishes only an operation Tributary started (ruling F5). The
bytes a take should leave are **git's own**: `git
checkout --ours|--theirs` on a fresh copy of the same fixture
(`truth::side_bytes`), never a string typed into the test.

```sh
cargo run -p graft --example m5_walk                              # every scenario, M5's and M6's
cargo run -p graft --example m5_walk -- --only kill-9-mid-rebase-then-relaunch   # one
cargo run -p graft --example m5_walk -- --list                    # the names (M6's are the last 19)
cargo run -p graft --example m5_walk -- --matrix                  # also writes .scratch/m6-conflicts/matrix.txt
cargo nextest run -p graft --test it m5_walk::every_scenario      # the fast tier's one test per scenario
```

**All 19 pass** (2026-09-23, after fixes F1, F2, F4, F6–F8 and the
rulings on F3 and F5; findings in
`.scratch/m6-conflicts/issues/07-acceptance-conflicts.md`). The 19 M6 rows'
fast-tier time is in `docs/perf/m6-07.txt` (the ticket's bar is 60 s).

Captures: `docs/shots/m6/m6-<nn>-<what>-<WxH>.txt` at 120x40 and 80x24,
Unicode tier, from `.scratch/m6-conflicts/pty-walk-m6.py <WxH>` (build
`-p graft --example m5_walk --bin trib` and `-p graft-fixtures --bin
graft-fixtures` first; `TRIB_TARGET` names the target directory;
`TRIB_UPDATE_SHOTS=1` rewrites). `m6-06-relaunch-sticky` is taken from
the **real `trib` binary** relaunched after the script sends the capture
run `SIGKILL` and checks it died on signal 9. All nine captures exist at
both sizes, and every one shows the text it is for: the script's second
run, without `GRAFT_UPDATE_SHOTS`, is green on all 18 (2026-09-23, after
fixes F9 — the status line on the sub-100-column Detail tab — and F10 —
the start-over box drawn over the resolver). The resolver then changed
under them (side names, ticket 10; the file list and identity row, 13; a
take moves the cursor on, 15; `j`/`k` between files, 17; a stop opens the
resolver, 18; the resolver stays open after the last file, 31; the stop
line's shorter wordings, 12 and 24), and on 2026-09-24 **all eighteen were
re-taken** (m6-conflicts/45): each diff read and named to those tickets,
then a second run without `GRAFT_UPDATE_SHOTS` green on all 18.
`m6-06`'s kill point had moved because `o` in a rebase now reads `took
upstream`, not `took ours` (the script waits for the new word). One
frame no longer shows all its ticket-07 words, and its caption says so:
`m6-06` at 80x24 draws no words saying the rebase is from a previous run
(m6-conflicts/45 NOTES). `m6-09`'s keys became `k ⏎ j j j j d` on
2026-09-24 (m6-conflicts/46): the old second `⏎` was refused on the open
file and its refusal took the bottom row; now the row is the box's keys.
The contact sheet is
`docs/shots/m6/m6-contact-sheet-<n>.txt`, in parts of at most 180 000 B (headroom under the 200 KB cap)
(`scripts/contact-sheet.sh --captions-order m6 docs/shots/m6`): every M6
capture in ticket order — these nine, every fix ticket's `fix-*`, the
journeys' `m6-10` to `m6-19` — then the four M5 hint captures and the M4
credential capture M6 changed.
**A capture carries no colour** — the user judges by eye.

### What the walk proves and what it does not

- **Proves:** every take (`o`, `t`, `b`) leaves the bytes git's own
  checkout gives, CRLF and a missing final newline included; every
  whole-file card's key leaves the index and tree **exactly** as the raw
  git commands of the whole-file table do on a fresh copy (`ls-files -s`
  and bytes compared); a rebase that stops twice finishes into one
  `Committed` journal entry that one `u` takes back; `kill -9` mid-rebase
  leaves `rebase-merge/` and a `Pending` journal line, and the relaunch
  finishes or aborts from there; `$EDITOR`'s edit is re-read and never
  written over.
- **Does not prove:** the terminal hand-over itself. `e` and `M` run the
  loop's own command builders and git's own editor lookup, but headless —
  suspending and re-entering a real terminal is not exercised. **The
  mergetool is a script** (`walk-mergetool.sh`), not a real tool;
  mergetool with a real tool is **unverified**. `kill -9` in the fast tier
  is `Step::Kill` — the app dropped with no shutdown — not a signal; only
  the pty script sends a real `SIGKILL`. Start over **after** `⏎` is proven
  at the ops level only (scenario 8); the app has no way in to it yet. A
  merge **begun in the shell** finishes through the banner's `c` and the summary, journaled as its own entry that `u` puts back mid-merge. `c`/`a` in the panes never open the editor while an operation is live: the footer names the way to the banner — `merge in progress` · `⇥ banner, then c …` (usability fix M3a).
  What a box or a line looks like is proven only by the captures: the
  headless walk reads `AppState`, where the start-over box and the Sticky
  existed while no frame drew them (F9, F10 — now fixed, and pinned in the
  view by `view::frame::tests::a_live_confirm_box_is_drawn_over_every_overlay`
  and `…::the_detail_tab_below_100_columns_draws_the_status_line`).

### The scenarios

1. **merge-resolved-with-the-keys** — the fixture's merge aborted in
   setup; `m` on `feature`, confirmed: git stops (`MERGE_HEAD`), banner
   focused. `both.txt` by `o`; `three.txt` `o`, then `⏎` refused with `2
   conflicts still open in three.txt` and `n next` on the line, nothing
   staged; `t b` then `⏎` (`o` on block 1 already moved the cursor to
   block 2, and each take moves it on again — m6-conflicts/15): `three.txt`
   on disk is block 1 ours, block 2
   theirs, block 3 ours then theirs, line for line against git's
   `--ours`/`--theirs` bytes; every other path resolved, `ls-files -u`
   empty; the last file closes **to the banner**, which has the keyboard
   and says `all conflicts resolved · c continue`. `c`: `git rev-parse
   HEAD^2` equals `feature`'s tip, `ls-files -u` prints nothing, the
   journal has **one** `Merge` entry, `Committed`, whose commit's parents
   are the pre-merge `main` and `feature`. `u`: `git rev-parse HEAD` is the
   pre-merge tip, git not merging.
2. **every-whole-file-shape** — `theirs-deleted` `o`, `ours-deleted` `o`,
   `added-by-us` `t`, `added-by-them` `t`, `both-deleted` `o`, `pic.bin`
   `t`, `both-added` `o`: `ls-files -s` over the seven paths and each file's
   bytes equal a fresh copy after `git add`/`git rm`/`git checkout
   --theirs` by hand.
3. **whole-file-card-glimpses-a-large-file** — `theirs-deleted.txt` grown
   to 600 lines: the card reads 500 and says it is cut; `G` shows the
   unnumbered note row `… first 500 lines of <size> · e opens the whole
   file`; `t` deletes the file from disk and index.
4. **binary-card-shows-its-size** — `pic.bin` opens a card called binary
   with the note `binary · 22 B` and no line of the file; `o` leaves git's
   ours.
5. **crlf-and-no-final-newline-survive** — `crlf.txt` by `t` is git's
   theirs byte for byte and still has `\r\n`; `nofinal.txt` by `o` is git's
   ours and does not gain a final newline.
6. **zdiff3-and-base-on-demand** — `zdiff3.txt`'s one block carries its
   base from its own markers before `B` (no load) and, like every block,
   draws it only on `B`; `three.txt`'s three blocks
   carry none until `B`, then all three do; `B` on `both-added.txt` says
   `no base (added on both sides)`, not as a problem. Index and files
   unchanged.
7. **n-past-the-last-block-says-no-more** — setup stages every other path,
   so `zdiff3.txt` is the only file not done. With its one block open, `n`
   has no file to cross to and wraps: `wrapped · 1 still open`, cursor on
   block 1. After `t`, `n` and `N` both say `no more conflicts · ⏎ marks
   zdiff3.txt done`; the resolver stays on `zdiff3.txt`; `zdiff3.txt` on
   disk is git's `--theirs`, still unmerged, index unchanged. **The user
   decides that wording.**
8. **start-over-brings-the-markers-back** — start over **before** `⏎`
   (ruling F3): `⏎` on `three.txt`'s `CONFLICTS` row (`3 of 3` open), `o`,
   `Esc` to the pane, `⏎` on the row again — still unmerged, the taken block
   now plain lines, `2 of 2` open, two `<<<<<<< ` lines on disk; `d`: the
   box is titled `Start three.txt over`; confirmed: `git ls-files -u` lists
   `three.txt` with the **same stage oids and modes as at the stop**, the
   file on disk has the stop's **three** `<<<<<<< ` lines, and the identity
   row reads `three.txt · 3 conflicts · 3 open` again (the resolver's
   `(open, blocks)`, which the row prints). Start over **after** `⏎` is proven **at the ops level only**:
   `graft-ops` `tests/it/chained_backup.rs::restart_after_stage_brings_the_conflict_back`
   (git's REUC answers it) and `::restart_survives_a_relaunch` (the recorded
   stage tree answers it). **The app's entry point is not built** — a
   staged path has no `CONFLICTS` row and `u` is refused mid-operation; a
   keymap decision for M7 (Times I left).
9. **editor-edits-are-never-clobbered** — `e` on `three.txt` hands it to
   `core.editor` (a script replacing block 1 with `edited by hand`); back
   in the resolver the file is re-read, two blocks open; a take lands on
   what the edit left (one marker block, `edited by hand` still there);
   the edit is then changed again behind the resolver's back and `o`
   pressed: the file on disk still carries `edited twice` — the take either
   refused and re-read (file unchanged) or landed on the re-read file (no
   markers left); either is allowed, a write over the edit is not.
10. **mergetool-hand-over-and-back** — `M` with `merge.tool` a script: on
    `both.txt` it writes `resolved by the tool` and exits 0 — `both.txt
    resolved outside`, the resolver closes, `ls-files -u` empty for it; on
    `crlf.txt` it exits 1 — `mergetool exited 1 for crlf.txt`, the three
    stages still there, the file re-read.
11. **rebase-stops-twice-resolved-twice** — `r` on `main` from `feature`
    stops on `shared.txt`; `⏎` on the banner, `b`, `⏎`, `c`; the second stop
    `t`, `⏎`, `c`: `shared.txt` is `feature's second line`, the rebased
    tip's `~2` is `main`, one `Rebase` entry `Committed`; `u` puts
    `feature` back.
12. **stash-pop-conflict-resolves-without-a-banner** — `git stash pop` in
    the shell conflicts: no banner; from the changes pane the resolver
    takes, `⏎` says `every file done` with **no** `c continue`, and
    the journal has one `stage p` entry.
13. **undo-a-resolution-in-a-stash-pop-conflict** — `S`, `p`, `y`
    in the app conflicts and lands in the resolver on `p`, the `S` list
    closed, the notice `stash@{0} clashed in 1 file · esc, u puts it all
    back` (m6-conflicts/72b; the stash stays); `o`, `⏎`: every file done,
    the stash dropped (`every file done · stash@{0} dropped ✓ · u brings it
    back`, the stash list empty); `u` brings the stash back; `u` again
    brings back three stages and the bytes from
    before `⏎`; a third `u` (the drift box `Undo “pop stash@{0}”?`, confirmed)
    takes the pop back — no stages, the files and stash list as before;
    `U` puts back the unmerged index (`ls-files -u` three lines) with `p`
    as `o` left it — what the third `u` found, not the pop's markers,
    which `o` had already replaced — and the stash is **still listed**,
    where git left it (fix F2).
14. **kill-9-mid-rebase-then-relaunch** — the rebase stops on `shared.txt`
    (two blocks), `o` on the first, then `Step::Kill`: `rebase-merge/`
    present (`truth::operation_on_disk`), the journal's last line
    `Pending` (`truth::last_journal_line_is_pending`), `shared.txt` on disk
    has one `<<<<<<<` left (`half_taken_on_disk`). Relaunched: the line is
    exactly `rebase still in progress from a previous run · ⏎ resolve  c
    continue  a abort` with the changes pane open; `⏎` opens `shared.txt`
    at `shared.txt · 1 conflict · 1 open`; `t`, `⏎`, `c`, the second stop `t`, `⏎`,
    `c`: `ls-files -u` empty, `HEAD` is `feature`'s rebased tip, its `~2`
    is `main`, one `Rebase` entry `Committed`.
15. **kill-9-mid-rebase-then-abort** — the same kill; relaunched, `a`,
    confirm: not rebasing, `feature` unmoved, every file as before.
16. **u-right-after-continue-is-accepted** — a merge Tributary began stops;
    resolved, `c`, and `u` at once, before the re-read lands: accepted, the
    merge taken back.
17. **repush-lease-is-the-entrys-tip** — a teammate pushes after ours and
    the tracking ref is fetched; `u`, `Re-push old tip`: the lease is the
    tip the journal entry recorded, git refuses in words, the bare keeps the
    teammate's commit.
18. **resolver-is-modal-and-esc-returns** — opened from the banner, `u`,
    `Tab` and `/` do nothing (no search prompt behind the resolver, fix
    F1); `Esc` returns to the banner; opened from the pane, `Esc` returns
    to the pane.
19. **tiny-frame-says-so** — at 50x8, below the resolver's floor, one line
    says so, a take does nothing, `Esc` closes.

### Per breakpoint

| Capture | 120x40 | 80x24 | What right looks like |
|---|---|---|---|
| `m6-01-resolver-three` | yes | yes | identity row `▸ three.txt · 3 conflicts · 2 open` (the list `three.txt 2 of 3 open`, `2/3 open` at 80); block 2's header `conflict 2 of 3 · took theirs · feature`; blocks 1 and 3 `open`; the bottom row `base shown for 3 conflicts ✓` |
| `m6-02-whole-file-card` | yes | yes | identity row `▸ theirs-deleted.txt · modified on yours · main, deleted on theirs · feature`, then `deleted by them · o keeps the file  t deletes it`, no halves; the list row `theirs-deleted.txt keep or delete` (`keep?` at 80) |
| `m6-03-card-large-file` | yes | yes | after `G`: line 500 last, then the unnumbered `… first 500 lines of 8.1 KB · e opens the whole file` |
| `m6-04-card-binary` | yes | yes | identity row `▸ pic.bin · binary · modified on yours · main, modified on theirs · feature`, then `both modified · o keeps yours · main  t keeps theirs · feature`, then `binary · 22 B` unnumbered, no bytes |
| `m6-05-still-open-refusal` | yes | yes | `2 conflicts still open in three.txt · n next`; block 1 `took yours · main` |
| `m6-06-relaunch-sticky` | yes | yes | after a real `SIGKILL`: `rebase still in progress · 1 conflicted · ⏎ resolve  c continue  s skip  a abort`, the pane on `CONFLICTS (1)`, one marker block left. At 80x24 the Detail tab, the bottom row the keys alone (`⏎ resolve  c continue  s skip  a abort`) |
| `m6-07-all-resolved` | yes | yes | the list `files · all done ✓`, identity row `▸ shared.txt · done ✓ · d starts it over`, the finish row `✓ every file done — c continue the rebase` (ticket 60); the bottom row `shared.txt done ✓ · every file done · c continues the rebase` |
| `m6-08-no-more-conflicts` | yes | yes | `no more conflicts this way` under the identity row (a notice, ticket 61); the cursor unmoved on `conflict 1 of 1 · took theirs · feature` |
| `m6-09-start-over-box` | yes | yes | a box over the resolver, `Start both.txt over?`, `git checkout --merge -- both.txt`, `▸[ Cancel ]   [ Start over ]` (fix F10) |

**Times I left:** every reach for the shell during a conflict in daily use
— `git mergetool` from a shell, `git checkout --ours`, `git rm` on a
deleted-by-them file, editing markers in another window — goes in the
table at the top of this file, dated, the same day (ADR-0033).

## Conflict journeys

M6's journeys, **counted** (m6-conflicts/21, 43; pipeline step 7). Each is
the user's own route from where he is to done, walked over the real app and
real git by the same walk as above. The count is **read from the note**,
not restated: `crates/graft/tests/it/m5_walk/journeys.rs` parses the one
fence tagged `journey-counts` in `docs/design-notes/m6-conflicts.md`, and a
journey that sends more inputs than its row fails, naming both numbers and
the keys. A key, a chord (`Ctrl+T` = 1) or a click counts 1; waits and
truth reads count 0 (`docs/research/ux-principles.md` §1.3). After every
event the landing is asserted from the **drawn frame**, from the state where
a frame cannot say it, and from git.

```sh
cargo nextest run -p graft --test it m5_walk::every_scenario::journey      # one test per journey
cargo nextest run -p graft --test it journey_counts_match_the_note         # ids both ways, whole = parts; prints the route table
cargo run -p graft --example m5_walk -- --only two-files-in-order          # one, by name
```

Fixtures (`m5_walk/fixture.rs`): **one-file** — `shared.txt`, one block;
**three-blocks** — `three.txt`, three blocks; **two-files** — `gone.txt`
(changed on `main`, deleted on `feature`), `notes.txt` (two one-line
blocks) and a clean one-line change to `README.md` made by `feature`. Each
is merged in the app from the graph (cursor on `feature`), or **in the
shell** (`git merge feature` run by git in setup, the app opened fresh on
it); one shell variant is a `git pull` over a local remote.

### The journeys

Keys as sent; `⏎` Enter, `⇥` Tab, `^T` Ctrl+T. "Best" is the best tool's
count from the note's `journey-counts` block, where it records one.

| Journey | Start | Keys | Count | Best | What right looks like |
|---|---|---|---|---|---|
| `a-merge-to-stop` (one-file, two-files) | the graph, cursor on `feature` | `m m` | 2 | — | the merge stops and lands **in the resolver** on the list's first file (`shared.txt`; `gone.txt` for two-files), cursor on block 0; header `MERGING`; the list's `▸` on that file; the bottom row starts `o yours`; the stop reads `merge stopped · …` |
| `a-stop-to-first-conflict` | the stop | none | 0 | JetBrains 0 | the frame right after the stop is the one above: no key between the stop and the first conflict |
| `b-launch-to-first-conflict` (+ one-file, three-blocks, after a `git pull`) | the app launched on a merge begun in the shell | `⏎` | 1 | — | the launch lands on the **banner**, which has the keyboard; the pane shows `CONFLICTS`; the stop's reason starts `merge in progress · begun outside Tributary · `; the bottom row draws the fullest rung that fits (m6-conflicts/61) — at 120 columns `merge in progress · from the shell · 2 conflicted · ⏎ resolve` (the capture `m6-18` shows each size). `⏎` → the resolver on the list's first file, as in `a-merge-to-stop` |
| `one-file-one-conflict` | the resolver on `shared.txt` | `O` | 1 | JetBrains 2 | lands on the **summary** the last file opened (m6-conflicts/60): top border `╭ Preview: finish merging feature into main`, the frame has `RESOLVED BY YOU`, status `shared.txt done ✓ · every file done`; `shared.txt` on disk equals `HEAD:shared.txt` and is staged; `git ls-files -u` is empty |
| `three-conflicts-all-theirs` | the resolver on `three.txt`, three blocks open | `T` | 1 | JetBrains Accept Theirs 1 | lands on the **summary** the last file opened (m6-conflicts/60): top border `╭ Preview: finish merging feature into main`, the frame has `RESOLVED BY YOU`, status `three.txt done ✓ · every file done`; `three.txt` equals git's **per-block** theirs (`git merge-file -p --theirs` over the merge's base, ours and theirs — not `checkout --theirs`) |
| `modify-delete-keep-yours` | the resolver on `gone.txt` (two-files) | `o` | 1 | JetBrains 1 | lands on `notes.txt`: the notice `gone.txt done ✓ · ▸ notes.txt`, the list's `▸` on `notes.txt`; `gone.txt` equals `HEAD:gone.txt` and is staged |
| `two-files-in-order` | the resolver on `gone.txt` | `o T` | 2 | JetBrains 2 | lands on the **summary** the last file opened (m6-conflicts/60): top border `╭ Preview: finish merging feature into main`, the frame has `RESOLVED BY YOU`, status `notes.txt done ✓ · every file done`; both files `Done` in the list's state; `git ls-files -u` empty. Before the route, at **120x40** the strip and the card (no `FILES ·` column, no `│ result`); at **200x50** the column `FILES · 2` and the `result` pane — same count. A click on each list row selects that file at both widths (not counted) |
| `two-files-out-of-order` | the resolver on `gone.txt` | `n T o` | 3 | — | `n` → `notes.txt`; `T` → back on `gone.txt`, the file not done; `o` → lands on the **summary** the last file opened (m6-conflicts/60): top border `╭ Preview: finish merging feature into main`, the frame has `RESOLVED BY YOU`, status `gone.txt done ✓ · every file done` |
| `a-finish` | the summary the last file opened (two-files, `m m o T`) | `y` | 1 | lazygit 1 (bare confirm, rejected) | before `y`: `RESOLVED BY YOU · 2` lists `gone.txt` `kept · yours · main` and `notes.txt` `theirs · feature in 2 of 2 conflicts`; `COMBINED BY GIT ON ITS OWN · 1` with the cursor `▸` on `README.md`. `y` → the graph has the keyboard, the cursor on `main`'s badge (the merge commit), the bottom row `merged feature into main ✓`; `HEAD^2` is `feature`. `u` (a truth check, not counted) puts `main` back on its pre-merge tip |
| `b-finish` | a shell-begun merge, both files done in the app: the summary the last file opened | `y` | 1 | — | the same landing; `HEAD^2` is `feature`; exactly **one** `FinishMerge` journal entry. `u` → `MERGE_HEAD` is back and equals `feature` (git) |
| `b-finish-again-after-undo` | after that `u`: the banner has the keyboard | `c y` | 2 | — | `c` opens the summary; `y` → the same landing again, `HEAD^2` = `feature` |
| `summary-open-git-combined` | the summary, `README.md` not shown | `Space` | 1 | — | `README.md`'s diff rows (`feature's line`) appear under its row, and `MESSAGE` moves down below them. `⏎` does the same (the frame's `⏎/space diff`; the capture `m6-16` uses it) |
| `back-after-esc` | after the stop, `Esc Esc` (not counted: the start) — the graph has the keyboard | `⇥ ⏎` | 2 | — | before: the bottom row **starts** `⇥ banner`. `⇥` gives the banner the keyboard; `⏎` → the resolver on the first file not done (`gone.txt`) |
| `open-a-done-file` | the resolver on `notes.txt` after `gone.txt` was kept | `k` | 1 | Fork #1304 (no count) | `gone.txt` opens read-only: the row `▸ gone.txt` with `done ✓`, the bar offers `e edit` and `esc close` |
| `walk-back-a-done-file` | on `gone.txt` after `notes.txt` was taken theirs | `j d y` | 3 | Tower (count ?) | `notes.txt` is open again, **not** done, cursor on block 0, the list's `▸` on it; `git ls-files -u -- notes.txt` lists its three stages again |
| `walk-back-a-whole-file-choice` | the resolver on `notes.txt` after `gone.txt` was kept | `k d y` | 3 | Tower (count ?) | the box reads `the choice comes back: keep or delete` and `git checkout --ours -- gone.txt`; after `y` `gone.txt` is its card again (`deleted by them · o keeps the file  t deletes it`), the list row `keep or delete`; `git ls-files -u -- gone.txt` lists stages 1 and 2 |
| `b-abort-shell-merge` | the app launched on a two-files merge begun in the shell, the banner has the keyboard | `a y` | 2 | lazygit 3 (?) | `a` → the box `Abort the merge?` reads `main goes back to before the merge`, `u puts the merge back` and `git merge --abort`, cursor on `[ Cancel ]`; after `y` the status `abort merge of feature ✓`; `git rev-parse -q --verify MERGE_HEAD` fails; the journal holds exactly one `AbortMerge` entry |
| `b-undo-a-take-in-the-shell` | the same shell-begun merge, `gone.txt` taken whole with `t` in the resolver, then `Esc`: the banner has the keyboard | `u` | 1 | JetBrains ⌃Z 1 | `gone.txt` is conflicted again (`git ls-files -u -- gone.txt` lists its stages), the merge still open (`MERGE_HEAD` equals `feature`), and the banner **keeps** the keyboard |
| `a-whole-one-file` | the graph, cursor on `feature` (one-file) | `m m O y` | 4 | — | the merge commit landing, as in `a-finish`; the row equals the sum of its parts |
| `a-whole-two-files` | the graph, cursor on `feature` (two-files) | `m m o T y` | 5 | — | the same; equals the sum of its parts |
| `b-whole-two-files` | the app launched on a shell-begun two-files merge | `⏎ o T y` | 4 | — | the same; equals the sum of its parts |
| `editor-to-finish` | a shell-begun merge resolved in the shell, the changes pane focused | `c y` | 2 | — | `c` in the pane opens the summary (m6-conflicts/52; it used to answer in words); `y` → the merge commit landing, as in `a-finish` |
| `last-block-to-finished` | the resolver on `notes.txt` after `gone.txt` was kept and block 1 taken `o`; the cursor on block 2 | `o ⏎ y` | 3 | JetBrains ~4 (?) · VS Code 3–4 · lazygit 2 (bare, rejected) | `o` takes the last block; `⏎` marks `notes.txt` done and the summary opens by itself; `y` → the merge commit landing, as in `a-finish` |

The note's own journeys moved on from the ticket's first text, and the walk
follows the note: `O`/`T` settle a whole file (P6, ticket 17 — no `⏎`
after them); the last file done opens the finish summary by itself, a
preview that `y` confirms (P11, P12; tickets 59, 60 — the resolver stayed
open before them, P8); `k`/`j` step between files (P7). Every box `y`
confirms is also confirmed by `→ ⏎` (ticket 50). `Ctrl+T` still takes theirs in every open
block of one file without marking it done (keymap §2.6, P3); the journeys
route by `T`, and the capture `m6-13-after-ctrl-t` shows `Ctrl+T`.

**Press `Ctrl+T` once in Zed's terminal — unverified.** The pty cannot tell
whether Zed's terminal claims `Ctrl+T` for itself before Tributary sees it.
Until the user presses it once in the resolver in Zed and sees every open
block of the file read `took theirs · feature` (the notice `took theirs ·
feature in 3 blocks · ⏎ marks three.txt done` on `three.txt`), P3's key is
**unverified** there. What he sees goes in `docs/field-notes.md` the same
day.

**Times I left, row "Finish a merge begun in the shell":** closed —
`c y` on the banner (P4, P12), proven by
`crates/graft/tests/it/resolver/shell_begun.rs::c_y_finishes_a_shell_begun_merge`,
or `y` in the summary the last file opened, walked here by `b-finish` and
`b-whole-two-files`. **Row "Get back a hand edit … begun in the shell":**
closed — `a y` aborts from the banner and `u` puts the merge back
(`shell_begun.rs::a_on_the_banner_aborts_a_shell_merge`), and `u` on the
banner undoes a whole-file take
(`shell_begun.rs::u_on_the_banner_restores_a_whole_file_take_in_a_shell_merge`);
walked by `b-abort-shell-merge` and `b-undo-a-take-in-the-shell`.

### The stash put-back journeys

Picks 15–17 (m6-conflicts/75; `docs/design-notes/m6-stash-put-back.md`
"Journeys"). Walked in `crates/graft/tests/it/m5_walk/stash_journeys.rs`,
counted against the same fence as above, git's truth read at the end. The
whole pass, one command, route table to a file:

```sh
cargo nextest run -E 'test(m5_walk::every_scenario::journey) | test(journey_counts_match_the_note) | test(m6_stash_put_back::)'
# --no-capture prints `route <id>: <n> [<keys>]`; ticket 75 kept them in
# .scratch/m6-conflicts/acceptance-stash-put-back-routes.txt
```

Fixtures: **stash over notes** — `notes.md` `base` → edited `stashed` and
stashed → committed `committed`, so the pop clashes; **diverged, three
edits** — `origin/main` has the teammate's commit, `main` has `ours`, and
`one.txt` `two.txt` `three.txt` are edited, `one.txt` staged; **main and
fix** — `fix` branched after `base.txt`, one commit each side (`main moves
on`, `fix the bug`), `main` checked out; dirty with `base.txt` and
`notes.txt` edited, `base.txt` staged; **fix over main's base** — `main`
changes `base.txt`, `fix` checked out with `base.txt` edited, so the
switch's put-back clashes; **f and g** — `f.txt` and `g.txt` pushed, a
teammate's commit changing both on `origin/main`, a local `ours` changing
`f.txt`, and `g.txt` edited in the tree: the rebase stops on `f.txt`, and
putting `g.txt` back clashes once it continues.

| Journey | Start | Keys | Count | Best | What right looks like (frame, then git) |
|---|---|---|---|---|---|
| `stash-pop-clash-to-done` | the graph, stash over notes | `S p y o ⏎` | 5 | lazygit ~10 | `S p` → the pop preview, `may conflict: notes.md`; `y` → no `S` overlay; the resolver on `notes.md`, notice `stash@{0} clashed in 1 file · esc, u puts it all back`, panes `yours · main` and `stashed · stash@{0}`, bottom row starts `o yours`; the stash still listed. `o ⏎` → status `every file done · stash@{0} dropped ✓ · u brings it back`. Git: `git stash list` empty, `notes.md` = `committed`, `git status` clean |
| `stash-apply-clash-to-done` | the graph, stash over notes | `S ⏎ y o ⏎` | 5 | lazygit ~10 | the same landing from `⏎ y` (apply, its preview first); after `o ⏎` status `every file done · stash@{0} kept ✓`. Git: one stash still listed |
| `stash-look-and-leave` | the graph, stash over notes | `S ⏎ esc` | 3 | lazygit 2 (no preview) | `⏎` → a box over the list, `Preview: apply stash@{0}`, `may conflict: notes.md`, `stash@{0} stays in the list`, `y apply` on its edge; `esc` → the list on the same stash (m6-conflicts/86, `m6_stash_put_back::stash_preview::stash_look_and_leave_writes_nothing`). Git: stash list, index and status as before |
| `stash-clash-put-back` | the clash landing above (`S p y`, not counted) | `Esc u` | 2 | lazygit ~10 | `Esc` closes the resolver; `u` takes the pop back, no box. Git: one stash listed, `git status` clean, nothing unmerged |
| `pull-rebase-dirty` | the graph, diverged, three edits | `p r r` | 3 | JetBrains 3–4 | `p` refused `… have diverged …`; `r` → the rebase preview, under `working tree:` the row `your edits: 3 files set aside, put back after`, no staging words; `r` confirms → status `rebase main onto origin/main ✓ · your edits are back`. Git: `main` is `origin/main` plus one; the three files edited, `git status` `M  one.txt` (still staged, as `git switch` keeps it), ` M three.txt`, ` M two.txt`; stash list empty |
| `rejected-push-dirty-to-pushed` | the same | `p r r P y` | 5 | JetBrains 3–4 | as above, then `P` → the push box, `y` → pushed. Git: the bare's `main` = `HEAD`; its last two subjects `ours`, `the teammate's commit`; the edits still in the tree |
| `push` | the graph on `main`, clean, one commit ahead of `origin` | `P y` | 2 | lazygit 1 (no box) | `P` → the box `Push main → origin/main`; `y` → the box gone, no failure on the status line. Git: the bare's `refs/heads/main` = `HEAD` (m7-power-tools/25b, `m5_walk::every_scenario::push_journey_is_walk_gated`) |
| `switch-dirty` | the graph on `main`, dirty; cursor on `fix` (not counted) | `. ⏎ y` | 3 | lazygit 2, JetBrains 3 | on `fix`; status `… · your edits are back`. Git: `HEAD` → `fix`; `base.txt` and `notes.txt` hold the edit; `git status` `M  base.txt` (the staging kept), ` M notes.txt`; stash list empty |
| `switch-staging-not-kept` (30b) | the graph on `main`, 20-line `f.txt`, line 5 edited and staged; `fix` changed line 7; cursor on `fix` (not counted) | `. ⏎ y` | 3 | shell: `git switch fix` refuses; by hand `stash`, `switch`, `stash pop` = 3 commands, the staging lost with no word | the preview row `your edits: 1 file set aside, put back after`, no staging words; after `y`, on `fix`, the sticky line `on fix ✓ · your edits are back · staging not kept: f.txt · u undoes it, staging included` (at 80 `your edits are back` is cut first); it stays after `j`; never `could not confirm`. Git: `git status` ` M f.txt`, lines 5 and 7 both in; stash list empty. Then `u` (1 key): on `main`, `M  f.txt`, index and tree byte-equal to before (`m6_stash_put_back::set_aside::a_switch_whose_staging_cannot_come_back_says_so`) |
| …with an `AM` file (30c) | as above, plus `n.txt` new, staged at v1, then edited to v2 | `. ⏎ y` | 3 | as above, and v1 is lost with no word | the line names both: `… staging not kept: f.txt +1 more · u undoes it, staging included`. Git: ` M f.txt`, `A  n.txt` with `:n.txt` = v2. Then `u` (1 key): `:n.txt` = v1, worktree v2, everything byte-equal to before; redo byte-equal to after. Checked at the operation (`put_back_index::put_back_names_a_staged_new_file_edited_on`), not walked in the app |
| `relaunch-after-kill` (30c) | Tributary killed after the put-back brought the edits back (unstaged, fallback) and before the stash drop; he launches it again | none | 0 to see where the edits are | shell: `git status` + `git stash list`, then judge whether the stash is a duplicate = 2 commands and a judgement | HEAD on `fix`; the footer `an operation was interrupted: checkout fix · your edits are back · a copy stays in stash@{0} · J to inspect` at 120; at 80, whole, `interrupted · your edits are back · a copy stays in stash@{0} · J to inspect` (30d's ladder); never `edits in stash@{0}` alone. Git: ` M f.txt`, one stash listed. Then `u` (1 key): on `main`, `M  f.txt`, index and tree byte-equal to before the switch, stash list empty (`m6_stash_put_back::set_aside::a_relaunch_after_a_kill_before_the_drop_says_the_edits_are_back`; captures `docs/shots/m7/30c-relaunch-edits-back-{120x40,80x24}.txt`). 30f: at 160x50 with the pane open (`⏎`) the footer is whole, `J to inspect` included; after `git stash drop` + `Ctrl+R` it reads `… your edits are back · J to inspect`, no `stash@` (`…::the_relaunch_notice_fits_the_pane_and_follows_the_stash_list`); whole at every width 80–187, pane shown and hidden (`…::the_relaunch_notice_is_whole_at_every_width_pane_or_not`) |
| `switch-dirty-clash` | the graph on `fix`, fix over main's base; cursor on `main` (not counted) | `. ⏎ y o ⏎` | 5 | JetBrains ~5 (?) | `y` → the resolver on `base.txt`, notice `your edits clashed in 1 file · esc, u undoes the switch too`, panes `branch · main` and `yours · edits set aside`, bottom row `o branch  t yours …`; `o ⏎` → `every file done · set-aside copy dropped ✓ · u brings it back` (`t ⏎` instead keeps his edit, same count). Git: on `main`, `base.txt` = `main's base`, stash list empty; a `u` after lists the copy again |
| `pull-rebase-stop-then-clash` | the graph, f and g | `p r r o ⏎ c o ⏎` | 8 | none counted | the rebase stops on `f.txt`; `o ⏎` resolves it, `c` continues; putting `g.txt` back clashes: notice `… esc, u undoes the rebase too`; `o ⏎` → `every file done · set-aside copy dropped ✓ · u brings it back`. Git: `g.txt` = `theirs`, no `rebase-merge`, nothing unmerged, stash list empty |
| `menu-checkout` | the graph on `main`, clean; cursor on `fix` (not counted) | `. ⏎ y` | 3 | JetBrains 3 | `.` → the menu's first slot `Checkout fix`, the second `Cherry-pick <id>`, cursor on the first; `⏎` → the checkout preview; `y` → on `fix`. Git: `HEAD` → `fix` |
| `menu-cherry-pick` | the same | `. j ⏎ y` | 4 | JetBrains/SM 2 | `j` → the cursor on `Cherry-pick <id>`; `⏎ y` → picked. Git: `main`'s tip subject `fix the bug`, its parent `main moves on` |

`Best` is the note's; the walk proves only our count. **Kill at every
step:**
`m6_stash_put_back::kill::edits_survive_a_kill_at_every_step` — `. ⏎ y` on
`fix` from a dirty `main`, the operation killed before each of ADR-0045 §4's
steps 1–6 (a `Stop` that panics: the job's own stop is private to the app,
so `y`'s operation is run the way the job runs it), then relaunched: the
edit is in the tree or in `git stash list`, and when it is listed the
relaunch line reads `an operation was interrupted: checkout fix · edits in
stash@{0} · J to inspect`. The kill lands at the first poll after the step
before's mark, so "before step 3" came with `stash store` done; the
not-yet-listed case is graft-ops' `a_stop_before_each_step_loses_nothing`.

**Put-back failure line** (m6-conflicts/84, ADR-0045 §5 amended): a
set-aside that ends neither clean nor in a clash is sticky, the
operation's own failure first, then where `graft_ops` says the edits are
(`EditsAt`), the clause kept whole at 80 with the failure cut first:
`your edits are in stash@{n} · S p puts them back` (not in the tree);
`your edits are back · a copy stays in stash@{n}` (drop failed; no `S p`);
`u restores the backup · a copy is in stash@{n}` (stopped after the
store). Unit-proven (`operate::tests::edits_words`), not walked: the
faults are graft-ops' (`tests/it/set_aside.rs`, hooks and permissions).

**Preview warns a put-back clash** (m6-conflicts/84):
`set_aside::the_preview_warns_before_a_switch_that_clashes` — `. ⏎` on
`main`'s row over fix over main's base: the box's conflict line reads
`your edits may clash when put back: base.txt`, never `no conflicts
expected`. Capture `set-aside-clash-preview-120x40`.

**`Esc` on a history nothing is loading** (m6-conflicts/85 A):
`esc_and_the_walk::esc_then_g_on_a_long_history_draws_its_last_rows` — a
200-commit history, first screen settled, `esc G`: the header still reads
`200 commits` (no `stopped`), the graph's last rows are commits, not blank.
Capture `esc-then-g-120x40`.

**A take, then back to the pane** (m6-conflicts/85 B):
`m6_stash_put_back::pop::a_take_then_esc_shows_the_file_without_markers` —
`S p y esc ⏎ o esc` (a look at the clashed file in the pane first, which is
what caches its diff with the markers): back on the pane, `notes.md` under
`CONFLICTS`, no `<<<<<<<`, `=======` or `>>>>>>>` line in the diff the pane
serves nor on disk; header `diff vs HEAD <main's sha7>`. Capture
`stash-clash-take-esc-pane-120x40`.

**The stash preview** (m6-conflicts/86): `⏎` and `p` on the stash list
open a Ghost Preview over the list; `y` runs it, `esc` goes back to the same
stash, `⏎ ⏎` (Cancel is first) writes nothing.
`m6_stash_put_back::stash_preview::stash_look_and_leave_writes_nothing` —
`S ⏎ esc` at 3 keys; stash list, index and status unchanged; capture
`stash-apply-preview-120x40` (`Preview: apply stash@{0}`, `working tree:
notes.md`, `may conflict: notes.md`, `stash@{0} stays in the list`, `y
apply`). `the_pop_preview_fits_80_columns` — capture
`stash-pop-preview-80x24` (`Preview: pop stash@{0}`, `stash@{0} is dropped ·
u brings it back`, `y pop`, whole at 80).
`unsaved_edits_in_the_way_are_named_in_the_box` — an unsaved edit to
`notes.md`: capture `stash-preview-in-the-way-120x40` (`your unsaved edits
to notes.md are in the way`, one `[ Close ]`, no `y apply`, footer `esc
back`); `y` writes nothing. No stash-list bar under any of the three boxes;
the last row is blank (m6-conflicts/87, `no_list_bar`).
`a_clean_stash_pop_is_three_keys` — `S p y` on a stash that pops cleanly:
`no conflicts expected`, popped, within the note's `stash-pop-clean` row (87).
Colour unverified.

**A pull over his edits** (m6-conflicts/85 D, superseded by
m7-power-tools/03b): was a refusal and 7 keys (`p`, `⏎`, `S s ⏎`, `esc p`).
Since 03b the pull sets the edits aside itself: 1 key, see "M7: a pull over
his edits" below. The test was rewritten, not deleted:
`pull_over_edits::a_ff_pull_over_overlapping_edits_takes_one_key`.

### Captures

`docs/shots/m6/m6-10-…` to `m6-25-…`, one frame per landing, from
`.scratch/m6-conflicts/pty-walk-m6.py <WxH>` (see "Conflicts walk" above
for the build line). Each capture run starts **fresh on the fixture** and
presses the whole way to its landing through the real loop; the keys are
in each file's first line. A commit the run makes is dated like the
fixture's (`GIT_COMMITTER_DATE`, set by the script), so its id does not
change between runs. Sizes: **120x40** and **160x48** (strip and card),
**187x50** (the user's own width, field notes) and **200x50** (list column
and three panes), **80x24** (the floor). **Colour:** the band, the tints
and the state colours are proven by buffer tests (cell fg, bg and modifier
per block state, ADR-0041/0044):
`graft-ui conflict_resolver::tests::three_panes::band::band_follows_the_block_state`
(and its 16-colour, high-contrast, `NO_COLOR` and ASCII siblings in the
same file), `conflict_resolver::tests::card_colours::card_tints_follow_the_block_state`,
`conflict_resolver::files::tests::file_list_state_colours`,
`theme::tests::dark_tints_equal_the_mix_rule` and `light_tints_equal_the_mix_rule`.
**A capture carries no colour**; the focus reverse video and the cursor
block are colour too. The text tells are `▸`, `▏`, `?`, `◀┃` and the bottom
row. **Legibility in Zed, dark, 187 columns is unverified** until he looks.

**The put-back pass's colour** (m6-conflicts/75): the dim of `Checkout —
already on <b>` is asserted from the cells the app draws by
`m6_stash_put_back::the_checkout_slot_on_head_s_row_is_drawn_dim_and_only_it`
(every cell of that slot `DIM`, and since m7-power-tools/11 every cell of
slot 2, `Cherry-pick (o) — already on main`, too; none of `Copy SHA (y)`'s). The resolver's
roles for a stash (`stashed · stash@{0}`) and for set-aside edits (`set
aside · your edits`) are **words only** — the role picks the pane names
(`graft-ui/src/conflict_resolver/draw/block.rs:35`), the tints follow the
block state as in a merge — asserted by
`conflict_resolver::names::tests::set_aside_roles_name_the_panes_yours_and_set_aside`,
the walks' frames and the captures below. No buffer test runs the band under
those two roles. **Colour and dim here are unverified** until he looks.

Every stem from m6-10 to m6-19 exists at all five sizes; m6-20 to m6-25 at
120x40, 187x50 and 80x24, as ticket 75 asked (`.scratch/m6-conflicts/matrix.txt`
shows `—` for the other two).

| Capture | Keys | What right looks like |
|---|---|---|
| `m6-10-stop-lands-in-resolver` | `m m` | the resolver right after the stop: header `MERGING`; `▸ gone.txt · keep or delete` first in the list; identity row `▸ gone.txt · modified on yours · main, deleted on theirs · feature`; bottom row starts `o yours`. At 187 and 200: the column `FILES · 2 conflicted` and three panes titled `yours · main`, `result`, `theirs · feature` |
| `m6-11-card-on-notes` | `m m o` | on `notes.txt`: the notice `gone.txt done ✓ · ▸ notes.txt`; the list `gone.txt · done ✓` then `▸ notes.txt · 2 of 2 open` (`✓ gone.txt   ▸ notes.txt 2/2 open` at 80) |
| `m6-12-after-t` | `m m n t` | block 1's header `conflict 1 of 2 · took theirs · feature`; identity row `▸ notes.txt · 2 conflicts · 1 open`; bottom row `took theirs · feature in notes.txt · 1 of 2 open ✓` |
| `m6-13-after-ctrl-t` | `m m ⌃t` | every block `took theirs · feature`; identity row `▸ three.txt · 3 conflicts · 0 open`; notice `took theirs · feature in 3 blocks · ⏎ marks three.txt done`; the bar offers `⌃O/⌃T all` |
| `m6-14-last-file-done` | `m m o T` | the summary, **opened by itself** by the last file done (ticket 60): top border `╭ Preview: finish merging feature into main`; the first two lines inside `1 merge commit on main · nothing is committed until y` and `undo: u takes this back`; `RESOLVED BY YOU · 2` (`gone.txt   kept · yours · main`, `notes.txt  theirs · feature in 2 of 2 conflicts`); `COMBINED BY GIT ON ITS OWN · 1 · ⏎ shows the change` with `▸ README.md  modified  +1 −0`; `MESSAGE · ⇥ edit`; bottom border `╰ y commit  ⏎/space diff  j/k move  ⇥ message  esc back` |
| `m6-15-finish-row-after-esc` | `m m o T Esc` | `Esc` from that summary: the resolver on `notes.txt`, identity row `▸ notes.txt · done ✓ · d starts it over`, under it the finish row `✓ every file done — c review and finish the merge`; the list headed `files · all done ✓` (`FILES · all 2 done ✓` at 187 and 200); the bar starts `c finish` |
| `m6-16-summary-expanded` | `m m o T ⏎` | `⏎` on `README.md` (the cursor is on it): `2 + feature's line` under its row, inside the frame, and `MESSAGE · ⇥ edit` moved down below it |
| `m6-17-after-commit` | `m m o T y` | the graph, cursor on `[◆ main] Merge branch 'feature' into main`; no `MERGING`; bottom row `merged feature into main ✓` |
| `m6-18-launch-shell-begun` | none | the banner (`c finish` on a merge since m6-conflicts/70; retaken by 80), the pane `CONFLICTS (2)`. The stop line is the fullest rung that fits (ticket 61): at 120 and 200 `merge in progress · from the shell · 2 conflicted · ⏎ resolve  c finish  a abort …`; at 187 the origin dropped (`merge in progress · 2 conflicted · ⏎ resolve`); at 160 the count dropped too (`merge in progress · ⏎ resolve`); at 80 only the keys (`⏎ resolve  c finish  a abort …`) |
| `m6-19-after-esc-esc` | `m m Esc Esc` | the graph; the bottom row **starts** `⇥ banner` |
| `m6-20-stash-clash-landing` | `S p y` | no `S` overlay; the resolver on `notes.md`; notice `stash@{0} clashed in 1 file · esc, u puts it all back`; panes `yours · main` and `stashed · stash@{0}`; the bottom row starts `o yours  t stashed` |
| `m6-21-stash-clash-done` | `S p y o ⏎` | the graph on `main` with its detail, no `CONFLICTS` anywhere; the status `every file done · stash@{0} dropped ✓ · u brings it back` (the last row at 80) |
| `m6-22-set-aside-preview` | `p r` | the rebase preview box; under `working tree:` the row `your edits: 3 files set aside, put back after`, whole at every size, no staging words (30b); `[ Confirm ]`, `y confirm` |
| `m6-23-set-aside-clash-landing` | `j k . ⏎ y` | (`j k` only reach `main`'s row: the cursor opens on the uncommitted row) the resolver on `base.txt`; notice `your edits clashed in 1 file · esc, u undoes the switch too`; panes `branch · main` and `yours · edits set aside`; the bottom row starts `o branch  t yours`; `set aside ·` is no pane's name |
| `m6-24-set-aside-done` | `p r r` | the graph on the rebased `ours` over `origin/main`, `Uncommitted changes (3 unstaged)` on top; status `rebase main onto origin/main ✓ · your edits are back` |
| `m6-25-menu-fixed-slots` | `k .` | (`k` reaches `fix`'s row) the menu leads `Checkout fix`, `Cherry-pick 401cfbe (o)`, `Explain`; the slot is not dim here — it dims only on `HEAD`'s own row (`menu-fixed-slots-120x40`, frame 2) |

The Rust captures of the same pass (`check_shot`, drawn by `view::view`):
`menu-fixed-slots-120x40` (71), `stash-clash-landing-{120x40,80x24}` and
`stash-clash-done-120x40` (72b), `set-aside-preview-{120x40,80x24}`,
`set-aside-clash-landing-120x40`, `set-aside-done-120x40` (74),
`set-aside-clash-preview-120x40` (84: the checkout box, `your edits: 1 file
set aside, put back after`, then `your edits may clash when put back:
base.txt`, no `no conflicts expected`), `esc-then-g-120x40` (85 A: header
`200 commits`, not `stopped`; the last rows are commits),
`stash-clash-take-esc-pane-120x40` (85 B: the pane on `notes.md` under
`CONFLICTS`, no marker line, `diff vs HEAD` naming main's tip), and
`stash-clash-stop-line-80x24` (82): `S p y` clashed in `notes.md`, `esc` shut
the resolver, the graph focused, at 80x24 — no banner, no resolver; the
bottom row begins ` stash@{0} clashed · 1 file conflicted · ` then the keys
that fit; nothing cut mid-word. Colour unverified.

### M7 blame in a shallow clone

`docs/shots/m7/blame-shallow-187x50.txt` (m7-power-tools/20j,
`blame_place::blame_shallow_capture`): a `git clone --depth 2` of the session
fixture, cut at Priya's `0a27627`; blame of `src/auth/session.rs`, cursor on
line 18. Right: row 18 reads `▸ ^0a2762 shallow clone · can't say who`, its
trailer `◂ can't say who · this clone starts at 0a27627`; the block's other
rows are brackets, no name; `Priya` nowhere; no commit-graph offer. Colour
unverified (the heatmap's oldest bucket on the boundary rows).

### M7 history place in the frame

`cargo nextest run -E 'test(history_place)'` (m7-power-tools/21b). Blame of
`src/auth/session.rs` on the session fixture, cursor in `refresh_token`, `L`.

- `docs/shots/m7/history-lines-187x50.txt` (`capture_history_lines_187x50`).
  Right: row 1 begins `HISTORY · src/auth/session.rs · fn refresh_token`; the
  version list on the left, `▸` on a row, `the file starts here` last; the
  file pane on the right with `┃` down the traced lines; the bottom row is
  `j/k older·newer  ⇥ file  n/N next change  B blame here  ⏎ that commit  esc back`.
  No graph behind it.
- `docs/shots/m7/history-file-140x40.txt` (`capture_history_file_140x40`):
  `F` on the file, whole-file history. Right: row 1 `HISTORY ·
  src/auth/session.rs · 6 changes`; a `was src/session.rs` row; no `┃`; the
  bottom row the same list bar as above.
- `docs/shots/m7/history-100x30.txt` (`capture_history_100x30`): list only, no
  file pane; the bottom row whole, no word cut, `⇥ file` in it.
- `docs/shots/m7/esc-back-to-history-from-the-graph-120x40.txt`
  (`capture_esc_back_to_history_from_the_graph`): (a) `⏎` on a version, then
  `Tab` to the graph — the graph's bar begins `esc back to history`, never
  `back to blame`; (b) one `esc` — the history place again, same commit `▸`.
- `docs/shots/m7/history-binary-140x40.txt` (`capture_history_binary_140x40`,
  m7-power-tools/21c): `F` on `logo.bin`, a binary file grown from 600 to
  1 500 bytes. Right: row 1 `HISTORY · logo.bin · 2 changes`; the file pane's
  header carries no `this file +0 −0` (a binary has no lines to count,
  review m7-09..21c Minor 8); its second row reads exactly `binary · 600 B → 2 KB` (`size()`: 600 → `600 B`,
  1 500 → `2 KB`, whole KB rounded up); no line rows under it.

Colour unverified (the `┃` bar, the `+`/`−` rows).

### M7 submodule rows

`cargo nextest run -E 'test(submodule_truth)'` (m7-power-tools/13b). A real
submodule at `libs/parser` in a scratch superproject.

`docs/shots/m7/submodule-row-120x40.txt` (`a_moved_and_dirty_submodule_shows_both`):
pointer moved and a file changed inside. Right: under `UNSTAGED`, the row
`▣ libs/parser` ending `→ ±` right-aligned at the file list's last column
(where a file's `+3 −2` sits; m7-power-tools/30i); the diff pane's second
line, beside it, reads `changed inside`. No `M` letter, no counts, no `…`.

`docs/shots/m7/submodule-row-187x40.txt` (same test, 187 columns): the row
reads `▣ libs/parser  submodule · pointer moved <old7> → <new7> · changed
inside` in full. Colour unverified.

`docs/shots/m7/submodule-diff-120x40.txt` (`the_diff_pane_lists_the_commits_a_move_pulled_in`):
the pointer moved sideways. Right: the diff pane's first line `libs/parser ·
submodule · 5bdba05 → efdd295`, then `+ efdd295 parser 3`, `+ 8512312 parser 2`,
`− 5bdba05 side 1` — pulled in first, rewound after, same order as
`git log --left-right`. No hunks, no content diff. Colour unverified (`+` rows
added colour, `−` rows removed colour).

## Hint bar

`hint-bar-redesign` (ADR-0037 §4): the footer shows the three most relevant
keys for wherever the cursor or focus is, six from 100 columns, the
context's own keys and then the screen's from 160 — never all hidden, and
never a hint cut in half. Replaces the old fixed nineteen-entry list and its
144/155/156-column thresholds.

**Only keys that work.** Every hint carries a `HintState`: the design note's
table is kept whole, but `Planned` entries (`m merge`, `r rebase`,
`b branch`, `t tag`, the banner's `c continue`/`s skip`/`a abort`,
`M mergetool`, `space resolved`, the stash's `b branch`) are dropped before
the width rule runs, so the bar never teaches a key that does nothing
(m5-03 review, Critical 2 — at 80 columns two of a branch row's three keys
did nothing at all). The group that binds one flips that word and the hint
appears.

Unit tests (`crates/graft-ui/src/graph_screen/hints/tests.rs`): (a)
`no_context_hides_every_key` — every context but `Preview` shows something
at every width 40..=200, with and without a pane; (b)
`eighty_columns_shows_three_and_more`; (c) `hundred_twenty_shows_six`; (d)
`hundred_sixty_shows_the_context_then_the_screen`; (e)
`marked_leads_with_explain_and_omits_more`; (f)
`status_line_keeps_its_keys` — `fit_status_line` keeps ` · p pull · P
force` whole, cuts the reason with `…`; (g) `no_planned_hint_is_ever_drawn`;
(h) `the_divider_hint_follows_the_pane_not_the_context` and
`the_divider_hint_is_fourth_so_the_six_key_tier_reaches_it`. Frame test
`footer_eighty_branch_row` (`crates/graft-ui/src/graph_screen/tests.rs`).

Both directions of "only keys that work" are walked in
`crates/graft/src/update/tests.rs` against `update::keys::bound_actions()`
(43 (key, verb) pairs, each annotated with the dispatcher that answers it):
`every_hinted_key_is_bound` — every context × divider × 40/80/100/120/160/
200 columns, key *and* verb, no exemptions — and
`every_planned_hint_is_still_unbound`, which fails the build when a group
binds a planned key without flipping its hint.

**Which context the app picks** is proved from a real repository, over the
real job pool, with the keys pressed through `update` and the footer read
off a drawn frame (`crates/graft/tests/it/hint_bar.rs`):
`the_tip_row_hints_the_current_branch_context`,
`a_plain_row_hints_the_plain_context`,
`the_uncommitted_row_hints_the_working_tree_context`,
`the_changes_pane_hints_the_keys_that_write` (the changes pane is open *on*
the uncommitted row, so both `on_working_tree` and `on_changes_pane()` hold
— the ordering that hid the pane's keys for a milestone, m5-03 review,
Critical 1) and `a_failure_still_names_the_key_that_clears_it`.

Captures: `docs/shots/m5-hints-{branch,plain,uncommitted,changes}-{80x24,
120x40,160x48}.txt` come from that app path
(`cargo test -p graft --test it hint_bar::capture_hint_bar_for_docs_shots -- --ignored --nocapture`);
`m5-hints-marked-*` stays a hand-built `GraphScreen` fixture
(`cargo test -p graft-ui graph_screen::tests::capture_hint_bar_for_docs_shots -- --ignored --nocapture`),
because a Mistake Radar marker needs a corpus built to trip it. Every
capture says at its head which it is. None of them records colour (rule 5).

The last row of each capture, which is what to judge by eye:

| Context | 80x24 | 120x40 | 160x48 |
|---|---|---|---|
| Current branch row (the tip) | ` ⏎ open  c commit  P push  . more` | ` ⏎ open  c commit  P push  p pull  f fetch  u undo  . more` | ` ⏎ open  c commit  P push  p pull  f fetch  u undo  S stash  J journal  / search  q quit  F2 status  F3 renderer  x compare  + lanes  j/k move  . more` |
| Plain row | ` ⏎ open  c commit  y sha  . more` | ` ⏎ open  c commit  y sha  u undo  S stash  J journal  . more` | ` ⏎ open  c commit  y sha  u undo  S stash  J journal  / search  q quit  F2 status  F3 renderer  x compare  + lanes  j/k move  space fold  1 first-parent  . more` |
| Uncommitted-changes row | ` ⏎ changes  c commit  S stash  . more` | ` ⏎ changes  c commit  S stash  p pull  P push  f fetch  . more` | ` ⏎ changes  c commit  S stash  p pull  P push  f fetch  u undo  J journal  / search  j/k move  q quit  F2 status  F3 renderer  x compare  + lanes  . more` |
| Changes pane open | *no hint bar at all* — see the gap below | ` space stage  d discard  c commit  <> resize  ⌃A all  e edit  . more` | ` space stage  d discard  c commit  . more` |
| Marked (merge, Mistake Radar finding) | ` . explain  ⏎ open  c commit` | ` . explain  ⏎ open  c commit  y sha  u undo  S stash` | ` . explain  ⏎ open  c commit  y sha  u undo  S stash  J journal  / search  q quit  F2 status  F3 renderer  x compare  + lanes  j/k move  space fold` |

Two things in that table look wrong at a glance and are not:

- **The changes pane shows fewer keys at 160 than at 120.** The bar is the
  bottom row of the *graph*, not of the terminal. At 120 the pane is stacked
  below the graph (`DetailLayout::Below`) and the bar spans all 120 cells;
  at 160 the pane sits beside it, leaving the graph column ~93 cells — the
  three-key tier. `<> resize` is hinted fourth, so it appears wherever six
  fit (m5-03 review, Major 4: it used to be appended last, behind ten
  context keys, and no real frame ever drew it).
- **A pane keeps to its own vocabulary.** The graph's screen-wide keys
  (`q quit`, `x compare`, `+ lanes`, …) are appended only to the graph's own
  row contexts, because the changes pane does not answer them; `. more`
  leads to the rest, and the `.` menu now lists the keys the width rule
  drops first (`Fetch (f)`, `Pull (p)`, `Push (P)`, `Undo (u)`).

**Fixed — under 100 columns, with a detail pane open, the tab layout draws
its own hint bar.** `update::detail::effective_layout` picks
`DetailLayout::Tab` there: the pane takes the whole screen behind a
`[Graph] Detail` strip, and the graph screen — with its own footer — is not
drawn. `view::frame::draw_tab_hints` gives that layout a bottom row of its
own instead, through the same `hints_for` ladder every other bar uses, so
ADR-0037 §4's "never hide every key" holds here too
(m5-history-and-ghost-preview/29): `hint_bar::the_tab_layout_draws_the_panes_own_keys`
and `hint_bar::every_tab_context_draws_a_bar`.



## M7 journey: pr-contents (m7-power-tools/14)

| Journey | Start | Keys | Right looks like |
|---|---|---|---|
| `pr-contents` | cursor on `main`, pane closed, `feature-a`'s tip one row below | `x j` = 2 (note: 2) | pane opens on `x`; after `j` the header reads `changes on feature-a since it left main · 3 commits · 2 files +2 −0`; the file list holds only feature-a's files (`src/login.rs`, `src/token.rs`), never main's later `README.md`/`CHANGELOG.md`. Under 100 columns the pane is a tab: `Tab` shows it (not counted). Walk: `crates/graft/tests/it/m7_walk/pr_contents.rs::pr_contents`; captures `docs/shots/m7/compare-since-split-{187x50,80x24}.txt`. `[x base]` first on main's row (187x50). Colour unverified. |

## M7: one switch rule (m7-power-tools/10b)

`.` Checkout and `b ⇥` switch at once on a clean tree; with unsaved tracked edits (or a detached target) they open the preview, then `y`. Untracked files alone never count. Walks: `crates/graft/tests/it/m7_walk/switch.rs`. Colour unverified (rule 5).

| Journey | Start | Keys | What right looks like |
|---|---|---|---|
| clean switch by the menu | `main`, clean; cursor on `feature-b`'s row | `. ⏎` = 2 (was `. ⏎ y`) | no preview; on `feature-b`; the line `on feature-b ✓ · u goes back`; cursor on the new HEAD's row. `u` (1): back on `main` (`switch_clean_via_menu_is_two_keys`; capture `docs/shots/m7/switch-menu-clean-landed-120x40.txt`) |
| switch with edits by the menu | `main`, `base.txt` staged + a shared file unstaged; cursor on `feature-b` | `. ⏎ y` = 3 | preview first; on `feature-b`, both edits back, staged split kept, stash list empty; `on feature-b ✓ · your edits are back · u goes back`. `u`: on `main`, split as before (`switch_with_edits_via_menu`) |
| new branch with edits | same edits; cursor on `feature-b` | `b nb ⇥ ⏎ y` = 5 (typed name counts 1) | was refused (`DirtyTree`); now the preview; on `nb` at `feature-b`'s commit, edits back with staging, no stash left. `u`: `nb` gone, on `main` (`new_branch_with_edits`) |
| new branch at HEAD with edits | same edits; cursor on `main` (HEAD) | `b feature-c ⇥ ⏎` | preview: `feature-c (new) → <sha7>`, `HEAD → feature-c`, `your edits stay as they are`; nothing set aside; `esc` writes nothing (`new_branch_at_head_with_edits_previews_edits_stay`; capture `docs/shots/m7/new-branch-with-edits-preview-120x40.txt`) |
| remote-only branch (for the picker, 10) | `origin/sara/x` fetched, no local `sara/x` | — (no key yet: the picker is ticket 10) | graft-ops only: `switch --create sara/x --track refs/remotes/origin/sara/x`; `sara/x@{upstream}` = `origin/sara/x` even with `branch.autoSetupMerge=false`; `u` deletes `sara/x`, HEAD back (`graft-ops tests/it/switch_rule.rs::track_*`) |

## Switching (m7-power-tools/10, the branch picker)

`C` on the graph opens the picker over the lower graph: the branch before first, then his local branches, the remote's with no copy of his, and the one he is on last. Typing filters (a segment prefix above a substring). `⏎` switches by the one switch rule above (instant on a clean tree; a preview, then `y`, over edits); `⇥` shows the row's tip on the graph and writes nothing; `esc` closes. A click selects a row, a double-click switches. Walks: `crates/graft/tests/it/m7_walk/picker.rs`, each counted against the M7 note's fence. Fixture: `main` clean, `feature-a` before (built on `main`), `login-page` local, `origin/sara/login-page` remote-only, `main` tracking `origin/main`. Colour unverified (rule 5).

| Journey | Start | Keys | What right looks like |
|---|---|---|---|
| `switch-named-branch` | `main`, clean | `C login ⏎` = 3 (note: 3; typing counts 1) | after `C login` the box at 100 columns shows glyph, name and age only, `login-page` above `origin/sara/login-page`, footer `2 of 4 branches`; after `⏎`: no preview, on `login-page`, cursor on its row, `on login-page ✓ · u goes back`. `u`: on `main` (`switch_named_branch`; captures `docs/shots/m7/picker-filtered-100x30.txt`, `docs/shots/m7/switch-landed-120x40.txt`) |
| `switch-back` | `main`, clean, `feature-a` before | `C ⏎` = 2 (note: 2) | the box opens on `↩ before  feature-a`; `● local` / `◌ remote` / `· here` in their own column; `built on main`, `tracks origin/main`, `no copy of yours yet` on the right; `main` last, `you're on it`; the footer's notice row whole under the box; after `⏎`: on `feature-a`, cursor on its row. `u`: on `main` (`switch_back`; capture `docs/shots/m7/picker-empty-query-187x50.txt`) |
| `switch-with-edits` | `main`, `base.txt` staged + a shared file unstaged | `C login ⏎ y` = 4 (note: 4) | `⏎` opens the preview and the picker hides under it (`esc` there brings the picker back with its query: `update::picker::tests::esc_on_that_preview_returns_to_the_picker_with_the_query`); after `y`: on `login-page`, both edits back, staged split kept, no stash left, `on login-page ✓ · your edits are back · u goes back`, picker closed. `u`: on `main`, split as before (`switch_with_edits`) |
| a remote-only branch | as above | `C sara ⏎` = 3 | a local `sara/login-page` made, tracking `origin/sara/login-page`, checked out; `on sara/login-page (new, tracks origin/sara/login-page) ✓ · u goes back`. `u`: on `main`, `sara/login-page` gone (`switch_to_a_remote_only_branch_makes_its_local_copy`; capture `docs/shots/m7/picker-remote-landed-120x40.txt`) |
| `cherry-pick-other-branch` | `main`, clean; `login-page` has `fix the typo` on top of `fix the bug` | `C login ⇥ motion o y` = 6 (note: 6; typing counts 1, one motion to the commit below the tip) | `⇥` closes the picker, cursor on `login-page`'s tip, nothing written; one motion to `fix the bug`; `o` opens the take preview; after `y`: on `main`, one new commit on it carrying `fix the bug`'s change and not `fix the typo`'s; `took "fix the bug" onto main ✓ · u undoes`; cursor on the new `main`. `u`: `main` as before (`cherry_pick_other_branch`) |
| `cherry-pick-in-view` | `main`, `base.txt` edited; the graph opens with the cursor on `login-page`'s `fix the bug` | `o y` = 2 (note: `motion o y` = 3; no motion needed here) | `o` opens `Preview: take <sha7> "fix the bug" onto main`, bar `y confirm  ←→ choose  ⏎ act  esc cancel`, `your edits: 1 file set aside, put back after`; after `y`: `took "fix the bug" onto main ✓ · your edits are back · u undoes`, cursor on the new HEAD, `base.txt` edit back. `u`: the commit gone (`cherry_pick_in_view`, `y_takes_it_and_lands_on_the_new_commit`; capture `docs/shots/m7/take-preview-120x40.txt`) |
| take a merge | a merge commit on branch `merged`; cursor on it | `o` = 1 | no preview; the status line `<sha7> is a merge · taking a merge isn't in Tributary yet` (`take_refused_on_a_merge`; capture `docs/shots/m7/take-refused-merge-120x40.txt`) |
| `.` on HEAD's row | `main`, cursor on `main`'s row | `.` = 1 | slot 1 `Checkout — already on main`, slot 2 `Cherry-pick (o) — already on main`, both dim, in places 1 and 2; `⏎` on either does nothing (`menu_slot_two_dim_on_head`, `update::context_menu::tests::slot_two_dim_behaves_as_slot_one_dim`; capture `docs/shots/m7/menu-slot-two-dim-120x40.txt`) |

## M7: put staged changes into their commits (m7-power-tools/09)

`a` reads the cursor: on an older commit of his branch it previews putting the staged changes into it; on `HEAD` it is today's amend unless something is built on his branch. `.` on the uncommitted row offers absorb. Every route ends in the Ghost Preview and `y`. Walks: `crates/graft/tests/it/m7_walk/amend_into.rs`. Colour unverified (rule 5).

| Journey | Start | Keys | What right looks like |
|---|---|---|---|
| fix into an older commit | `feature-a` checked out ("Add OAuth callback", then "Handle expired tokens"), a fix to `src/auth.rs` staged; cursor on the top row (HEAD) | `j a y` = 3 | `a` shows `checking where your staged changes go…`, then `Preview: put your staged changes into <sha7> "Add OAuth callback"` with `<sha7>  Add OAuth callback  gains  src/auth.rs +1 −1`; the box says one thing about his edits (`your edits: N files set aside, put back after`, N the unstaged files only, and no such row with none; never `nothing is put back` beside it) and `undo: u takes this back`; after `y`: the fix is in "Add OAuth callback", nothing staged, "Handle expired tokens" replayed on top, an unstaged edit back as it was; `put into "Add OAuth callback" ✓ · u undoes`, or with unstaged edits `put into "Add OAuth callback" ✓ · your unstaged edits are back · u undoes`; the row reads `✎ changed` after its subject. `u`: every branch and the index as before (`fix_into_older`; captures `docs/shots/m7/amend-into-preview-187x50.txt`, `docs/shots/m7/amend-into-landed-120x40.txt`) |
| absorb | same branch; line 5 of both files fixed and a line added to `README`, all staged; cursor on the top row | `k . ⏎ y` = 4 (09b: absorb is the menu's first item and the cursor opens on it; the old five below it in their old order, capture `docs/shots/m7/absorb-menu-120x40.txt`) | box `Preview: put staged changes into the commits they belong to`: two `→ <sha7>  <subject>` rows and `README  line 2  stays staged · no commit on your branch changed these lines`; after `y`: each fix in its commit, the README line still staged. `u`: all back (`absorb_staged`; capture `docs/shots/m7/absorb-preview-120x40.txt`) |
| amend with a branch built on his | `feature-b` built on `feature-a`; a fix staged; in the changes pane | `a y` = 2 | not the editor: `checking what's built on feature-a…`, then the preview with `feature-b … built on yours · follows`; after `y`: HEAD amended, `feature-b` rebased onto it. `u`: all back (`amend_with_dependents`) |
| amend with nothing built on his | only `feature-a` (or nothing built on it) | `a` | today's amend editor, unchanged (`update::amend_into::tests::a_on_head_without_dependents_is_todays_amend`) |

## M7: a pull over his edits (m7-power-tools/03b)

A fast-forward pull whose fetch brings a file he has edited (staged or not) sets every changed tracked file aside, fast-forwards, and puts them back, in one journal entry (ADR-0045 §4 extended to Pull). The fetch runs first; what it brought decides. Diverged, up to date, or no overlap: nothing set aside, today's words. Colour unverified (rule 5).

| Journey | Start | Keys | What right looks like |
|---|---|---|---|
| `pull_ff_over_edits` | the graph on `main`, origin one ahead changing line 1 of `shared.txt`, his unsaved edit to line 10 | `p` = 1 (03 "Decided"; the note's fence has no row; no best-tool count — the prior-art study has no pull row) | HEAD at origin's tip; `shared.txt` holds both changes, his diff lines byte-equal and still unstaged; no stash left; the cursor (`▏`) on the new HEAD's row; the line `pulled origin/main ✓ · your edits are back`. Walk: `crates/graft/tests/it/m7_walk/pull.rs::pull_ff_over_edits`; capture `docs/shots/m7/pull-over-edits-120x40.txt` (taken by the walk). At 80 columns both clauses whole: `pull_over_edits::a_ff_pull_over_edits_says_they_are_back`. |
| pull clash | same, his edit on line 1 | `p` = 1 | the pull is done; the put-back clashes; he lands on the resolver on `shared.txt`, the clash is a set-aside's, his edit kept in the stash until the last file is done (`pull_over_edits::a_ff_pull_clash_lands_on_the_resolver`). No capture: the resolver's own captures cover the screen. |

graft-ops, against real repositories (`crates/graft-ops/tests/it/pull_set_aside.rs`): the journal record of a clean pull and of a failed fetch is as before 03b; `u` puts back HEAD, the staged split and every file; ADR-0045 §7's tree table round-trips; a stop before each of steps 1–6 leaves his edits in the tree or the stash list.

## M7: stashes in history (m7-power-tools/16)

| Scenario | Right looks like |
|---|---|
| A stash in the graph | One row per drawn stash (`On <branch>: <message>`, `■`, `[stash@{0}]`); **no** `index on …` and **no** `untracked files on …` row. Only `stash@{0}` is ever a row — older entries live in the reflog, not the walk. Test: `crates/graft/tests/it/stash_in_history.rs::stash_index_and_untracked_commits_are_not_drawn`. |
| A commit only a stash reaches (its branch deleted or rebased away) | That row draws `□`, dimmed, with `[only in stash@{0}]` last among its badges; a branch put back on it and `Ctrl+R` unmarks it; `H` hides it with the stash. ASCII tier: `*` like every node, the badge carries it. Pixel tier: the row is dimmed (`view/strips/tests.rs::a_stash_only_commit_is_dimmed_in_its_strip`); with a selection only an off-thread row reads dim (pre-existing, ticket 16 NOTES). Tests: `stash_in_history.rs::{a_commit_only_a_stash_reaches_is_drawn_as_only_in_stash, a_rebased_base_on_a_branch_again_loses_the_mark, h_hides_the_only_in_stash_rows_too}`. Captures `docs/shots/m7/stash-in-history-{120x40,80x24}.txt`. Colour (dimming, badge colour) unverified. |
| A stash's detail header | No `evil merge` line of any kind on a stash entry's commit. Test: `crates/graft/src/view/radar.rs::a_stash_commit_has_no_evil_merge_line`. |

## M7: the force box names what leaves (m7-power-tools/16b)

| Scenario | Right looks like |
|---|---|
| `P` after a rejected push, teammate's commits on origin | Lease box: line 1 (`origin/main <theirs> → <ours> · refused if …`), then `rewrites history for everyone who has <sha7>`, then `leaves origin/main:`, then one row per commit — sha7, subject, author, aligned in columns, subject cut with `…` — newest first, at most 3, then `+N more`; buttons row whole. `nothing on origin/main is lost` when nothing leaves; `couldn't read what leaves …: <why>` with the force still offered when the read fails. `P` before the read lands: `still reading what leaves origin/main — try again in a moment`. Tests: `update/remote/tests/lease/leaves.rs` (all ten). Captures `docs/shots/m7/force-box-leaves-{120x40,80x24}.txt` (`view::tests::capture_force_box_leaves`). Colour unverified. |

## M7: the journal, J (m7-power-tools/24-a2, 24-b)

Walks: `cargo nextest run --workspace -E 'test(/m7_walk::journal/) | test(/journal_screen/) | test(/journal_put_back/) | test(/put_back_box/)'`. Fixture: `graft_fixtures::lost_work` (journeys 13 → 14 → 15 in one repo, git's clock pinned); the put-back box on a Trivial repo.

| Scenario | Right looks like |
|---|---|
| `J` on the "lost work" fixture (journey 13's start) | Ticket 24-b (journey 13): `J` on the "lost work" fixture, every stage in. Title `JOURNAL · what happened in this repo, newest first`. Newest first: `●  Graft` fetch, then `$  outside Graft` steps each with its `git: "…"` column, a `└ ✗ lost` row under the reset (`a65d7e4 "Add rate limiter" · was main's tip · no branch holds it`, preselected `▸`) and under the switch (`419e109 "Try redis cache" · was spike's tip · spike is gone`), the undo `●` row marked `← U` with `└ ✗ lost  your edit to src/a.rs · replaced by this undo · kept in its backup` under it, the discard `undone`. Last list line: `a branch deleted outside Tributary and never checked out leaves no trace here` (muted). No status row (nothing wrong). Legend line, then the bar `⏎ put main back to it  b new branch here  / find  J/esc close`. Test `crates/graft/tests/it/m7_walk/journal.rs::the_three_lost_work_journeys`. Colour unverified (rule 5). Capture `docs/shots/m7/journal-187x50.txt`. |
| J at 140x40, cursor on the reset's `$` row | Ticket 24-b: J at 140x40 (120–179), cursor moved to the reset's `$` row: no `git:` column; the selected row's `git: "reset: moving to HEAD~1"` on the line under it; the source word (`Graft` / `outside Graft`) still drawn. Test `crates/graft/tests/it/m7_walk/journal.rs::the_three_lost_work_journeys`. Colour unverified. Capture `docs/shots/m7/journal-140x40.txt`. |
| J at 100x30 | Ticket 24-b: J at 100x30 (under 120): no source word (the legend carries it); the ✗ rows keep their words; the legend cut at a word with `…`; the bar whole. Test `crates/graft/tests/it/m7_walk/journal.rs::the_three_lost_work_journeys`. Colour unverified. Capture `docs/shots/m7/journal-100x30.txt`. |
| J at 187x24 | Ticket 24-b: J at 187x24: title, legend and bar kept; the list fits, so nothing scrolls here (scrolling is `journal_screen.rs::at_24_rows_the_frame_stays_and_the_rows_scroll`). Test `crates/graft/tests/it/m7_walk/journal.rs::the_three_lost_work_journeys`. Colour unverified. Capture `docs/shots/m7/journal-187x24.txt`. |
| The shell's `reset --hard HEAD~1` lost "Add rate limiter"; `J`, then `⏎` on the lost row | Tickets 24-a2 + 24-b: the shell's `reset --hard HEAD~1` lost "Add rate limiter"; `J`, `⏎` on the lost row. J's real rows above (24-b): the `$` reset row, its `└ ✗ lost  <sha7> "Add rate limiter" · was main's tip · no branch holds it` row selected, the commit's `$` row; the `fast-import` row is the Trivial fixture's own build, stamped by the wall clock, so against the capture's fixed clock it reads `future` (not a thing he meets). The box at the bottom titled `Preview: put main back to <sha7> "Add rate limiter"`; now / after y / your files: / u undoes it; footer starts `y put it back`; J's own bar blank under it. Test `crates/graft/tests/it/put_back_box.rs::the_put_back_box_draws_over_j`. Colour unverified. Capture `docs/shots/m7/put-back-preview-187x50.txt`. |
| `y` on the put-back box (journey 13 lands) | Ticket 24-b (journey 13 landed): after `J ⏎ y`. The graph: `main` on "Add rate limiter" tagged `↺ put back`; "Add metrics endpoint" tagged `○ main was here`; the bottom row `put back ✓ · main has "Add rate limiter" again · u undoes`. The commit-graph notice above it is the fixture's (no commit-graph file), not J's. Test `crates/graft/tests/it/m7_walk/journal.rs::the_three_lost_work_journeys`. Colour unverified. Capture `docs/shots/m7/put-back-landed-187x50.txt`. |
| `J` again after the put-back | Ticket 24-b (journey 14's start): `J` again after the put-back. A new `●  Graft  now  put main back to a65d7e4 (the shell's reset)` row on top marked `← u`; the ✗ row for "Add rate limiter" gone (main holds it); spike's ✗ row still there. Nothing preselected (the newest step is Graft's). Test `crates/graft/tests/it/m7_walk/journal.rs::the_three_lost_work_journeys`. Colour unverified. Capture `docs/shots/m7/journal-after-put-back-120x40.txt`. |
| `J n ⏎` on his lost edit, the file as the forced undo left it (journey 15, no shell step) | Ticket 30o-a: J's rows above, the `└ ✗ lost  your edit to src/a.rs · replaced by this undo · kept in its backup` row selected; the box titled `Preview: put back your edit to src/a.rs`; `your files: src/a.rs comes back · what's in src/a.rs now is replaced`; `u undoes it`; footer starts `y put it back`; J's bar blank. Test `crates/graft/tests/it/m7_walk/journal.rs::the_three_lost_work_journeys`. Colour unverified (rule 5). Capture `docs/shots/m7/put-back-edit-preview-187x50.txt`. |
| `y` on the edit's box (journey 15 lands) | Ticket 30o-a: the graph with the changes pane open on `src/a.rs` (`▾ src/`, `M a.rs`, its diff adding the lost edit's `2`; the pane's cursor row is colour only); the bottom row `your edit to src/a.rs is back ✓ · u undoes` (as `done_status` orders it). Then `u` (uncounted) puts back what was in the file: the walk reads `src/a.rs` = `DISCARDED_EDIT`. Test `crates/graft/tests/it/m7_walk/journal.rs::the_three_lost_work_journeys`. Colour unverified (rule 5). Capture `docs/shots/m7/put-back-edit-landed-187x50.txt`. |
| `J n ⏎` on his lost edit over a **staged** file (journey 15, `src/a.rs` staged then edited) | Ticket 30p-a (journey 15, `J n ⏎`, over a staged file: `src/a.rs` staged as `S`, then edited): the box titled `Preview: put back your edit to src/a.rs`, with the line `your files: src/a.rs comes back · what's in src/a.rs now is replaced · src/a.rs stays staged as you had it`, then `u undoes it`. The footer starts `y put it back`. J's rows above it, the lost edit's row selected; its two newest `$` rows are the test's shell stand-in for journeys 13 and 14. Test `crates/graft/tests/it/journal_put_back.rs::a_staged_path_put_back_lands_on_its_unstaged_row`. Colour unverified (rule 5). Capture `docs/shots/m7/put-back-staged-preview-187x50.txt`. |
| `y` on that box (the staged file lands) | Ticket 30p-a (journey 15 over a staged file, after `y`): the changes pane shows `STAGED (1)` and `UNSTAGED (1)`, each with `M a.rs`. The diff shown is the lost edit's (`2` added, over the staged `3`). The bottom row is `your edit to src/a.rs is back ✓ · u undoes`. The cursor row is colour only (30o-a NOTES). Then `u` (uncounted): the walk reads `src/a.rs` = `W` and `git show :src/a.rs` = `S`. Test `crates/graft/tests/it/journal_put_back.rs::a_staged_path_put_back_lands_on_its_unstaged_row`. Colour unverified (rule 5). Capture `docs/shots/m7/put-back-staged-landed-187x50.txt`. |
| `space` on a `●` row | Ticket 24-b: `space` on the discard's `●` row: under it `commands Tributary ran`, then `┆ git --literal-pathspecs restore --worktree -- src/a.rs` — `┆`, never `$`, so `$` only ever means outside Tributary. The bar `⏎ revert to before this  / find  space command log  J/esc close`. Test `crates/graft/tests/it/m7_walk/journal.rs::journal_expanded_capture`. Colour unverified. Capture `docs/shots/m7/journal-expanded-120x40.txt`. |
| `/spike ⏎` in J | Ticket 30o-c: `J` on the "lost work" fixture, then `/spike ⏎`. The title ends in blame's find tail, `JOURNAL · what happened in this repo, newest first · find "spike" 1 of 4` (the four rows naming spike: the two switches, spike's lost `✗` 419e109 and the commit on spike). `▸` on `switch from spike to main`, the first row naming spike at or below where the cursor stood (the preselected `✗` a65d7e4). Find is `⏎`-driven as blame's: typing alone moves nothing. The bar `⏎ switch back to spike  b new branch here  / find  n N match  J/esc close`. Test `crates/graft/tests/it/m7_walk/journal.rs::journal_find_capture`. Colour unverified (rule 5). Capture `docs/shots/m7/journal-find-187x50.txt`. |

## M7: the rewrite editor (m7-power-tools/08)

Walks: `cargo nextest run --workspace -E 'test(/m7_walk::rewrite_editor/) | test(rewrite_editor)'`. Fixture: `feature-a` 5 commits since `main` (one `fixup!`, opens pre-marked), `feature-b` built on it.

- `squash-last-three`: on HEAD, `R s s y`, 4 keys, fenced (`squash_last_three`). After it `main..feature-a` is `Handle expired tokens`, `Add OAuth callback` (three became one; the fixup melded); its message is the MESSAGE block the editor drew, line for line; `feature-b^` is the new `feature-a`; the cursor lands on the new HEAD (`moves_head` covers `Rewrite`, graft-ops `operation.rs:1278`); `u` puts every branch back.
- `swap-adjacent`: on HEAD's parent (placed, not counted), `R Alt+↑ y`, 3 keys, fenced (`swap_adjacent`). The two newest commits trade places; `u` puts every branch back.
- Look and leave: `R s esc` (`rewrite_look_and_leave_writes_nothing`): branches, index, files and the journal's entries unchanged.
- Mouse: a click selects the row drawn there; a drag moves it and replans once (`a_click_selects_the_row`, `a_drag_moves_the_row`).
- Graph bar: `R rewrite` on his commit rows, not on the default branch's, not with HEAD detached (`the_bar_offers_capital_r_on_his_commit_rows`, `the_bar_hides_capital_r_on_default_branch_rows`).
- Capture `m7/rewrite-editor-187x50.txt` — right: title `REWRITE feature-a · 5 commits since it left main · newest first`; two `squash ↓` rows over `pick <sha7> Handle expired tokens`; the `│` rule and `AFTER y · not written yet`; `3 in one`; `MESSAGE of the squashed commit`, the kept row's message first, then the melded ones newest first (`wip`, then `wip: tests`); the bar starts `y rewrite` (`capture_rewrite_editor`).
- Capture `m7/rewrite-editor-140x40.txt` — right: AFTER below the rows, labelled `AFTER y · not written yet`; the message below it.
- Capture `m7/rewrite-editor-100x30.txt` — right: rows only; `tab after` on the bar.
- Capture `m7/rewrite-refused-fork-120x40.txt` — right: status `feature-b and feature-c are both built on yours · Tributary moves one line`; no overlay.
- Not proven by the walk: colour (the bold action word, the dim notes) — unverified (rule 5). The `$EDITOR` handover from the `r` field is unit-tested only (08-a).

## M7: a squash-merged bottom, and keeping a suggested drop (m7-power-tools/05b, rebase-preview-keeps-drops)

Walks: `cargo nextest run --workspace -E 'test(/m7_walk::rebase_line/)'`. Fixture: `feature-a` (a1 a2 a3) squash-merged into `main` as `Add OAuth (#42)`, `feature-b` (b1) built on it; the keep walk adds one commit on `main` taking a3's change back out.

- `keep-a-suggested-drop`: cursor to `main`, `r R p y`, 5 keys, fenced (`keep_a_suggested_drop`). After it `main..feature-a` is `a3` alone, its parent `main`'s tip as `R` saw it (a1, a2 dropped); `feature-b^` is the new `feature-a`; `u` puts every branch back at its pre-`y` tip, by name.
- Capture `m7/rebase-squash-merged-preview-120x40.txt` — right: the `already in main:` row ends `· dropped · R to keep them`; the footer reads `y confirm  r rebase  R keep  ←→ choose  ⏎ act  esc cancel` (`capture_rebase_line`).
- Capture `m7/rebase-keep-drops-editor-120x40.txt`, after `R`, before `p` — right: each suggested row shows the drop action and `already in main · p keeps it`; no row says `marked by its name`; the last row is `─ main` at the squash commit (`onto_tip`); the cursor `▸` on the newest drop, a3 (`capture_keep_drops_editor`). Unedited, every row is dropped, so AFTER (line 10) reads `nothing kept yet · p keeps a commit` — not `every commit dropped` — and the bar has no `y rewrite` until `p`. Colour unverified (rule 5).
- `R` is live, in the row and the footer, only in a rebase preview of `HEAD`'s branch whose plan suggests drops (`r_is_unbound_in_other_previews`, `r_is_unbound_when_the_rebased_branch_is_not_head`).
- Not proven: colour of the muted subjects and notes — unverified (rule 5).

## M7: stops during a rewrite (m7-power-tools/07c, 07d)

Walks: `cargo nextest run --workspace -E 'test(rewrite_stops) | test(continue_in_git) | test(hands_over)'`.

- Conflict stop of a chain rewrite: after `y`, no key; the resolver is open on `shared.txt`; banner `REBASING  step 3 of 5 · pick <sha7> b1 · feature-b, branch 2 of 3` (`rewrite_stop_to_first_file`, capture `m7/rewrite-chain-stop-187x50.txt`).
- `edit` stop: no key; the graph has the keyboard, gutter `▸` on `[HEAD] a1  HEAD · stopped here`; a box `still to apply · not commits yet` between banner and rows, this branch's `pick` rows newest first, then one `<branch> (<author>): N commits follow, on top` row per branch above; the pending commits are never drawn as extra graph rows (`edit_stop_lands`, `no_ghost_rows_in_the_graph`).
- A click under the box selects the row drawn there, at 120x40 and 80x24 (`a_click_under_the_block_selects_the_row_drawn_there`).
- Stage, amend, continue from the edit stop: `space a c`, 3 keys, fenced. After `space` the file is staged; after `a` still stopped, HEAD's tree has the edit, same message; after `c` the rewrite is done, his branch's commit has the edit, feature-b on top of his new tip (`edit_stop_amend_continue`).
- Abort from a chain stop: `esc a y`, 3 keys, fenced; every branch back, HEAD on `feature-a`, status `aborted: rebase feature-a onto main` (`abort_from_a_chain_stop`). The note's words `✓ rewrite aborted · every branch is as before` are not what the app says (open question, ticket 07d NOTES).
- `c` on a rebase/cherry-pick begun in the shell: the terminal goes to `git <kind> --continue`; back in Tributary: finished → graph + `✓ git <kind> --continue finished`; conflicted again → resolver; non-zero, still live → `git <kind> --continue exited <code> · still stopped` (`continue_in_git` tests). A shell revert still refuses in words.
- Capture `m7/rewrite-edit-stop-187x50.txt` — right: `still to apply · not commits yet` boxed; his two `pick` rows only (`a5`, `a4`); `feature-b (Sara): 2 commits follow, on top`; `▏` on `HEAD · stopped here`; bar contains `a amend it`, `c continue`, `space stage` (`capture_edit_stop_sizes`).
- Capture `m7/rewrite-edit-stop-80x24.txt` — right: `REBASING  step 3 of 12` whole; no `change files` hint; six `pick` rows then `+1 more` (7 pending, 6 drawn); the follower row `feature-b (Sara): 2 commits follow, on top` is drawn (`capture_edit_stop_sizes`).
- Capture `m7/rewrite-edit-stop-120x40.txt` — right: `pick a2` then `feature-b` and `feature-c` follower rows; bar `⇥ banner  a amend it  c continue`, then `space` and `⏎` bare (the width ladder). Reads `1 commit follow` — a grammar defect in graft-ui's follower line, open.
- Colour unverified.

## M7 acceptance (m7-power-tools/26)

Promise: **you can rewrite, trace and recover history without leaving the app.** One command walks it:

```sh
scripts/m7-accept.sh    # raw: .scratch/m7-power-tools/accept/ · summary: docs/perf/m7-accept.txt · exits non-zero on any failure
```

Steps: (1) `cargo nextest` on `m7_walk::*` (every journey walk, `every_m7_journey_id_has_a_walk`, the promise
scenarios, every in-process M7 capture), `journey_counts_match_the_note` and `hint_bar::*`; (2) every m3–m6 pty walk
in compare mode at every size its captures exist at (M7 has no pty walk; its captures are compared in step 1);
(3) the contact sheet `docs/shots/m7/m7-contact-sheet*.txt`, journal first; (4) the perf summary lines of
`m7-04-slow-key.txt`, `lost-commit-cost.txt`, `trace-readers.txt`, verbatim; (5) `accept: <n> journeys walked, <n> at
or under count, <n> over`.

One entry alone: `cargo nextest run --workspace -E 'test(=m7_walk::<test>)'`, `<test>` as named in the entry. Each walk
reads its count from the note's fences (`docs/design-notes/m7-power-tools.md` :823 and :1262, 29 ids, as of
2026-09-30) and fails over it. Counts are keys; `motion` is the one key that reaches the row. Best tool in brackets,
from the fence comments. **colour: unverified (rule 5)** for every entry below: each check reads the colourless
carrier (words, glyphs, cursor row), never colour.

### Journeys (pass 2a fence)

- [ ] `squash-last-three` · `R s s y` = 4 (Tower ~3, JetBrains ~4, lazygit ~5) · `rewrite_editor::squash_last_three`. **Right:** `main..feature-a` reads `Handle expired tokens`, `Add OAuth callback`; feature-a's message is the one the editor showed; `feature-b^` = `feature-a`; the cursor on the new HEAD; `u` restores every branch.
- [ ] `swap-adjacent` · `R Alt+↑ y` = 3 (lazygit 1–2) · `rewrite_editor::swap_adjacent`. **Right:** `main..feature-a` starts `wip: tests`, `wip`; `u` restores every branch.
- [ ] `fix-into-older` · `motion a y` = 3 (JetBrains ~2, git-absorb 2) · `amend_into::fix_into_older`. **Right:** `feature-a~1` ("Add OAuth callback") shows `+auth 5 fixed` in `src/auth.rs`; nothing staged; `feature-a` is `Handle expired tokens` replayed; `u` puts every branch and the index back.
- [ ] `absorb-staged` · `motion . ⏎ y` = 4 (jj 1, git-absorb 2) · `amend_into::absorb_staged`. **Right:** the menu opens on absorb; `feature-a~1` gains `+auth 5 fixed`, `feature-a` gains `+tokens 5 fixed`; the README `+more` line stays staged; `u` puts every branch and the index back.
- [ ] `amend-with-dependents` · `a y` = 2 (jj 0, Graphite 1) · `amend_into::amend_with_dependents`. **Right:** `a` opens the preview, not the editor; `feature-a` shows `+tokens 9 fixed`; `feature-b~1` = `feature-a`; nothing staged; `u` puts refs and index back.
- [ ] `rebase-line-onto-main` · `motion r y` = 3 (Graphite/Git Town 1) · `rebase_line::r_y_moves_the_line`. **Right:** the plan moves feature-a, feature-b, feature-c in that order; each lands where its ghosts were drawn; `feature-a~2` = `main`.
- [ ] `squash-merged-bottom` · `motion r y` = 3 (Graphite 1) · `rebase_line::r_y_drops_the_squash_merged_branch`. **Right:** the only ghost is `b1`; feature-a = main; `feature-b^` = main.
- [ ] `keep-a-suggested-drop` · `motion r R p y` = 5 · `rebase_line::keep_a_suggested_drop`. **Right:** `R to keep them` offered; three rows read `already in main · p keeps it`, cursor on a3; after `p y`, `main..feature-a` is `a3` alone, `a3.txt` present; `feature-b^` = feature-a, subject `b1`; `u` puts every branch back.
- [ ] `rewrite-stop-to-first-file` · 0 keys (lazygit +1) · `rewrite_stops::rewrite_stop_to_first_file`. **Right:** the resolver open on `shared.txt`; the banner reads `step 3 of 5` and `feature-b, branch 2 of 3`.
- [ ] `edit-stop-lands` · 0 keys · `rewrite_stops::edit_stop_lands`. **Right:** the graph has the keyboard; the cursor row reads `stopped here`; still to apply: `a2`, followers `feature-b` (1), `feature-c` (1).
- [ ] `edit-stop-amend-continue` · `space a c` = 3 · `rewrite_stops::edit_stop_amend_continue`. **Right:** the bar offers `space stage`; `space` stages `a1.txt`; `a` amends the stopped commit in place; after `c` no rebase in progress, `feature-a~1:a1.txt` reads `a1, fixed`, `feature-b~1` = feature-a.
- [ ] `abort-from-a-chain-stop` · `esc a y` = 3 (lazygit 3) · `rewrite_stops::abort_from_a_chain_stop`. **Right:** `Esc` closes the resolver onto the banner; `a` asks first; after `y` no rebase, feature-a/-b/-c back where they were, HEAD on feature-a, status `aborted: rebase feature-a onto main`.
- [ ] `restart-after-done` · `motion d y` = 3 · `restart_after_done` (in `m7_walk.rs`; "Times I left" :109). **Right:** `shared.rs` marked done (no stages); click it, `d` asks, `y` runs; then the index holds the stop's three stages again and `shared.rs` has one `<<<<<<< ` line.
- [ ] `switch-named-branch` · `C <type> ⏎` = 3 (best 3) · `picker::switch_named_branch`. **Right:** no preview; HEAD `refs/heads/login-page`; cursor on its row; status line `on login-page ✓ · u goes back`; `u` back on main.
- [ ] `switch-back` · `C ⏎` = 2 (lazygit 3) · `picker::switch_back`. **Right:** HEAD `refs/heads/feature-a`, status `on feature-a`, cursor on its row; `u` back on main.
- [ ] `switch-with-edits` · `C <type> ⏎ y` = 4 (lazygit/JetBrains 4) · `picker::switch_with_edits`. **Right:** the preview first; HEAD on login-page; both edits back; staged split kept, no stash; status `on login-page` · `your edits are back`; `u` back on main, split as before.
- [ ] `cherry-pick-in-view` · `motion o y` = 3 (best 2 + reaching) · `picker::cherry_pick_in_view`. **Right:** preview titled `Preview: take <sha7> "fix the bug" onto main`; one commit on main, `fix.txt` present, `base.txt` edit back; status `took "fix the bug" onto main ✓ · your edits are back · u undoes`; `u` removes it.
- [ ] `cherry-pick-other-branch` · `C <type> ⇥ motion o y` = 6 · `picker::cherry_pick_other_branch`. **Right:** `⇥` puts the cursor on login-page's tip; one commit on main, `fix.txt` present, `typo.txt` absent; status `took "fix the bug"`; cursor on main; `u` removes it.
- [ ] `pr-contents` · `x motion` = 2 (GitHub 2–3) · `pr_contents::pr_contents`. **Right:** the pane compares main → feature-a; the header reads `changes on feature-a since it left main · 3 commits`.
- [ ] `pull-current-branch` · `p` = 1 (lazygit 1, VS Code 1 click) · `pull::pull_current_branch` (walk added by 26). **Right:** HEAD fast-forwarded to origin's tip; the cursor on it; status `pulled origin/main`; tree clean, no stash.

### Journeys (pass 2b fence)

- [ ] `blame-line-to-commit` · `B ⏎` = 2 (tig/Magit/GitHub 2) · `blame::blame_line_to_commit`. **Right:** blame cursor on line 22, author Lina; `⏎` lands on Lina's commit, `src/session.rs` line 21, focus in the diff; graph cursor on her commit.
- [ ] `commit-back-to-blame` · `Esc` = 1 · `blame::commit_back_to_blame`. **Right:** blame of `src/auth/session.rs` back on screen, cursor on line 22.
- [ ] `blame-who-at-line` · `j` = 1 (GitLens 0) · `blame::blame_who_at_line`. **Right:** from Tom's line 21, `j` to line 22; exactly one trailer, on the cursor row, naming Lina; no Tom trailer.
- [ ] `blame-line-before` · `B ^` = 2 (tig/Magit/GitHub 2) · `blame::blame_line_before`. **Right:** title `before <sha7 of Lina's>`; two places stacked; cursor on line 20, Tom, `if resp.status() == 401 {`.
- [ ] `function-history` · `B L` = 2 (GitLens ~2, JetBrains ~5) · `history::function_history`. **Right:** `HISTORY · ` header; a function range holding line 22; Lina's commit selected, her file loaded, the range located in it.
- [ ] `file-history` · `F` = 1 (JetBrains ~2, lazygit ~3) · `history::file_history`. **Right:** `HISTORY · ` header; path `src/auth/session.rs`; 6 rows, one renamed from `src/session.rs`; newest selected, file at top.
- [ ] `lost-to-shell-reset` · `J ⏎ y` = 3 (jj 3–4, GitUp 5) · `journal::the_three_lost_work_journeys`. **Right:** the `✗ lost` "Add rate limiter" row preselected; after `y` main back on it; graph shows `↺ put back`, `○ main was here`, a `✓` line.
- [ ] `deleted-branch-back` · `J n ⏎ y` = 4 (jj 3–4, GitUp 5) · same test. **Right:** `n` lands on "Try redis cache"'s `✗` row; after `y` spike back on it, cursor and `↺ put back` on its row; bottom `spike is back at "Try redis cache" ✓ … u undoes`; main untouched.
- [ ] `edit-in-undo-backup` · `J n ⏎ y` = 4 (today: the shell) · same test. **Right:** `n` lands on `your edit to src/a.rs`; box says `what's in src/a.rs now is replaced`; after `y` changes pane on `src/a.rs`, bottom `your edit to src/a.rs is back ✓ … u undoes`; edit on disk; `u` restores the replaced copy.

### Promise scenarios (one fixture each; no shell step after it is built)

- [ ] **Rewrite** · `R s s y` (4), `u` (1), `R e y` (3, makes the edit stop; counted), his edit (0), `space a c` (3) =
  **11** · `promise::rewrite::promise_rewrite_without_leaving`. **Right:** after `y`, `main..feature-a` reads
  `Handle expired tokens`, `Add OAuth callback`; both branches' trees unchanged; `feature-b^` = `feature-a`; the
  written message is the one the editor showed, first line `Handle expired tokens` (the kept row). After `u` every
  branch as before. At the stop HEAD is "Handle expired tokens" with the still-to-apply block drawn. After `c` no stop,
  no block, no overlay; `feature-a~2:tokens.txt` holds the edit; `feature-b^` = `feature-a`.
- [ ] **Trace** · `B ^ Esc L ⏎ Esc Esc Esc` = **8** · `promise::trace::promise_trace_without_leaving`. **Right:** `B`
  from line 22: the cursor row's trailer names Lina. `^`: title `before <sha7>`, cursor on Tom's line 20. `Esc`: back
  on her blame, line 22. `L`: Lina's row selected; the file pane's range ruled `┃`, her added lines (18, 21, 22)
  marked `+` — the walk asserts one row `┃  18 +` and line 19 drawn as context. `⏎`: the graph cursor on her commit. `Esc Esc Esc`: the diff pane, line cursor on 22 of
  `src/auth/session.rs`, focus in the diff, graph cursor on `main` as at the start.
- [ ] **Recover** · `J ⏎ y` (3), `u` (1), `J n ⏎ y` (4), `J n ⏎ y` (4), `J n ⏎ y` (4) = **16** (26's ruling) ·
  `promise::recover::promise_recover_without_leaving`, with `core.autocrlf=true` and LF working files. **Right:**
  after `J ⏎ y` main on D; after `u` main off D and **every working file byte-equal** to before (`src/a.rs`, LF,
  included); then main on D again, spike on E, and `src/a.rs` holds his lost edit.
  **Fails today (2026-10-01, 26b):** the `u` step now holds (30s); the last step fails — `J n ⏎ y` writes his lost
  edit to `src/a.rs` in CRLF, where he had LF (`recover.rs:123`; the write is `git restore --source=… --worktree`,
  `graft-ops/src/operation/recover.rs:239-247`). Gap `M7/30q-putback-fwd`.
- [ ] **Guard at 80 columns** (review 28 Q2) · `C <branch> ⏎` over an ignored
  `src/services/authentication/providers/oauth2/callbacks/session-cache.json`, branch
  `feature/sara-long-running-oauth-token-refresh-rework` ·
  `promise::guard::guard_refusal_at_80_names_the_file_and_the_next_step`. **Right:** still on `main`; the bottom row
  at 80 columns shows `session-cache.json` and `move it aside first`. Green since 30t/30t-b (2026-10-01, 26b's run).

### Coverage, captures, perf

- [ ] Every id of both fences has a walk: `m7_walk::every_m7_journey_id_has_a_walk` (id → test table `WALKS` in
  `crates/graft/tests/it/m7_walk.rs`).
- [ ] Every m3–m6 pty capture matches in compare mode (script step 2).
- [ ] Contact sheet `docs/shots/m7/m7-contact-sheet*.txt`: every M7 capture under its Right line, the journal's first,
  each file ≤ 200 KB. Judged by eye by the user only (rule 6).
- [ ] `docs/perf/m7-accept.txt` holds the three perf files' summary lines verbatim; a `missed` stays `missed`.

## M7 fix: scroll on a big repo (m7-power-tools/47)

Promise: **on a repository his size, scrolling stops when he lets go of the key, and the next key acts at once — on
both tiers** (Unicode; Pixel on Kitty with deflate, Ghostty's path, and on iTerm2). One command:

```sh
scripts/47-scroll-acceptance.sh   # summary: docs/perf/47-scroll-acceptance-<date>-<head>.txt (baseline: …-acceptance.txt, never rewritten) · raw: target/47-acceptance/ · exit 1 on any miss
```

Steps: (1) release `held_key::` twice — every test but the raw Kitty pair (must exit 0), then the raw pair alone,
judged by its `VERDICT` lines against the known-gap list; (2) release `frames::` and `heavy::budgets_hold`; (3) a pty
walk of the release binary (`scripts/scroll-lag-pty.py`) on a copy of `TeamSpec::MONOREPO`, never his repo, 187×50,
`j` every 16 ms for 3 s then `J` 50 ms later, read free and at a fixed rate, probe answered like Zed (`text`), like
Ghostty with deflate (`kitty` yes) and without (`kitty` no); (4) the journeys' key counts; (5) the two captures.
Every miss is a `SUMMARY miss` line; the known gap prints `SUMMARY known_gap M7/46-kitty-no-deflate …` and is not a miss.
What the drain models and how each field is counted: "Held keys", "The strip bound" and "Deflate off is red by
known gap" under The heavy tier above. **Colour: unverified (rule 5)** for every row.

- [ ] **Held `j`, Unicode** · `held_key::held_j_stops_on_release_unicode`; pty `text-free`, `text-500k`. **Right:**
  `frames_after_release ≤ 2`, `tail_ms ≤ 100` at a 500 KB/s drain (in-process) and over a real pty read free and at
  500 KB/s; the cursor on the row shown at release (m7-power-tools/48, with the tree dirty:
  `view::tests::held_j_keeps_the_cursor_on_screen_under_the_uncommitted_row`,
  `view::strips::tests::held_j_keeps_the_cursor_lit_under_the_uncommitted_row`,
  `update::tests::every_cursor_key_keeps_the_cursor_drawn`; at release, `ghostty_shot::fix_scroll_ghostty_png`'s
  `cursor_on_screen=true` in `docs/perf/scroll-cursor-under-uncommitted-row.txt`).
- [ ] **Held `j`, Pixel (Kitty + deflate, iTerm2)** · `held_key::held_j_stops_on_release_pixel_deflate`,
  `held_key::held_j_stops_on_release_pixel_iterm2`; pty `kitty-yes-free`, `kitty-yes-40m`. **Right:** the same at
  40 MB/s, plus `bytes_per_frame_held ≤ 512 KiB` in-process and `bytes_per_key ≤ 512 KiB`, observed `deflate=yes`,
  Tributary's own `rss_mb ≤ 300` over the pty. The strip count prints `unjudged`; the invariant is guarded in the fast
  tier (see "The strip bound").
- [ ] **`J` after held `j`** · `held_key::j_then_journal_opens_at_once_unicode`,
  `held_key::j_then_journal_opens_at_once_pixel_deflate`. **Right:** `frames_before_J ≤ 1`, `j_ms ≤ 100`.
- [ ] **Kitty without deflate: red by known gap `M7/46-kitty-no-deflate`** · `held_key::held_j_stops_on_release_pixel`,
  `held_key::j_then_journal_opens_at_once_pixel`; pty `kitty-no-free`, `kitty-no-40m` (printed, not judged).
  **Right:** exactly three items `FAILED` and nothing else — A `tail_ms<=100`, B `tail_ms<=100`, B `j_ms<=100`
  (2026-10-02: 2223, 655, 605 ms); every other raw item `ok`.
- [ ] **No regression** · `frames::` (every scroll's p99 ≤ 16 ms, asserted in release), `heavy::budgets_hold_*`,
  `frames::after_idle::g_after_idle`. **Right:** all green.
- [ ] **Journeys** · held `j` = 1 key, `J` = 1 key; every `held_key` test asserts its key list is `j`×188 (then `J`
  in B) and nothing else (`held_key.rs` `run`, "47's walk count"); every pty run prints `keys=188 then_J`.
- [ ] **Ghostty-path look** · `ghostty_shot::fix_scroll_ghostty_png` → `docs/shots/m7/fix-scroll-ghostty.png`, shown
  in `docs/shots/m7/fix-scroll-contact.md`. **Right:** the lit thread at full strength, off-thread lanes at half
  strength, no line wider than another, the same rows as the Unicode grid (`fix-scroll-release-unicode.txt`).
  Colour unverified; what Ghostty draws is the user's check.
- [ ] **His daily check** (the user, not a test): open his work repo, hold `j` ~3 s, let go — does it stop at once?
  Then `J` — does it open at once? In Zed (Unicode) and in Ghostty (Pixel); in Ghostty, does the graph look like
  `fix-scroll-ghostty.png`? A slow scroll in Ghostty means his Ghostty fell back to raw (the known gap). His words go
  to `docs/field-notes.md` the same day.

## M8: how he runs the trace (m8-polish-and-ship G-trace)

`TRIB_PERF_TRACE=<file>` makes the real app write one line of numbers per drawn frame (`crates/graft/src/trace.rs`):
where the frame's time went (`update_us`, `draw_us`, `write_us`), how many bytes it sent, how many keys it drew
(`motions`) and how old the oldest was when it finished (`input_age_us`). **Numbers only** — no path, branch, sha
or subject; the header's only words are the tier and `$TERM_PROGRAM`. `history_rows` is the rows laid out so far,
not the repository's commit count. Unset, nothing is written. A file that cannot be written shows
`trace: <reason>` on the status row and the app runs on.

He runs it once per terminal, on his work repo, from the Tributary checkout:

```sh
# Ghostty, full screen
TRIB_PERF_TRACE=$HOME/trib-trace-ghostty.txt cargo run --release -p graft -- <his repo>
# Zed's terminal
TRIB_PERF_TRACE=$HOME/trib-trace-zed.txt cargo run --release -p graft -- <his repo>
```

Then: scroll the way that felt laggy (hold `j` ~3 s, let go; a wheel flick), quit with `q`, send the two files.
`scripts/m8-trace-summary.sh <file>` turns one into `docs/perf/m8-trace-<date>.txt` (p50/p95/p99/max per field,
frames/s, `tail_ms` from the last frame that drew a key to the last frame, `pmset` power line); `--self-test` checks
it on `scripts/testdata/m8-trace-sample.trace`.

- [x] **Builder's run** · `scripts/m8-trace-monorepo.sh` → `docs/perf/m8-trace-monorepo.txt`: MONOREPO fixture
  over a real pty, 50 `j`; checks `lines`, `presses` (1–50 frames drew them, motions = keys sent), `names` (no subject
  word, author, ref or path in the file). **Right:** three `ok`.
- [ ] **His run** (the user): the two commands above; the summaries go to `docs/perf/` and feed G-trace follow-up.
- [x] (user, 2026-10-11, charms reports repo: "now it felt solid"; docs/field-notes.md) **His run after emphasis pieces** (the user; m8 05 Part 2): in Ghostty full screen, the release build, his repo
  (first command above) — hold `j` 3 s, let go; hold `k` back; `F`, then hold `j` again; quit with `q`, then
  `scripts/m8-trace-summary.sh <file>`. **Right:** it stops on the row he let go at, the thread lit at every step,
  never half-drawn; where a lit line meets a dot, the dot is on top, as in `docs/shots/m8-05-emphasis-pieces.png`
  (pieces stack by Kitty `z`, m8 05 amendment 1: unverified until he looks). Expected from the builder's after-run (a finding, not a fail if missed): `write_us` p99 ≤ 7,000
  (was 14,064), `bytes` p95 ≤ 30,700 (was 76,751). `tail_ms` is not a judge (ticket 05 NOTES).

## M8: read the manual (J17, m8-polish-and-ship/23)

A person's step: the pty cannot judge whether a man page reads well. One step, as ripgrep's (`man rg`).

| Step | Command | Right looks like |
|---|---|---|
| 1 | `man ./docs/man/trib.1` (from the checkout) | NAME, SYNOPSIS, DESCRIPTION, OPTIONS (`path`, `-h, --help`, `-V, --version`, `--generate-man`), KEYS (one heading per place, `[keys.<name>]` beside it; each key as `[keys]` takes it, then the label and the action's name), CONFIGURATION (`[ui]` … `[theme.<kind>]`, values and default under each key), ENVIRONMENT, FILES, SEE ALSO `git(1)`. No raw `\[u…]` or `\e` showing, no line running off the page at 80 columns. |

- Generated by `trib --generate-man`; `cli::docs_tests::committed_man_page_is_the_generated_one` fails when the
  committed copy is stale. `mandoc -T lint docs/man/trib.1`: 0 errors, one warning (no date in `.TH`, on purpose: a
  date would make the committed copy stale every day, or carry a false one).
- [ ] **His read** (the user): step 1, at his usual width.

## M8: install it (J16, m8-polish-and-ship/24)

**Nothing here can run until the release exists.** ADR-0057 §1: no tag, no release, no tap until a person has run the
trademark search and the rename has landed. The commands then carry the new name (`<owner>/tap/tributary`, `trib`).
What can run today runs from scripts, writing summary lines to `docs/release/`:

| Script | What it proves | Output |
|---|---|---|
| `scripts/release-local.sh` (needs `dist` 0.33 on `PATH` or `DIST=`) | `dist plan` (4 targets, installer, formula, `trib.1` in each archive); `dist build` of this Mac's archive, its `tar tf`, the binary runs; `brew style` on the formula (no tap, no install); shellcheck on the installer | `plan.txt`, `build.txt` |
| `scripts/linux-fast-tier.sh` (needs Docker) | the fast tier on Linux, in a throwaway official `rust` container, as a plain user | `linux.txt` |
| `scripts/git-matrix.sh` | the git-sensitive tests against git built from source: the floor (2.38.0) and the newest release | `git-matrix.txt` |

A failure on the floor in `git-matrix.txt` raises the floor (`GitVersion::MINIMUM`, `graft-git/src/version.rs`) to the
oldest passing version, with a line here; it is never answered by skipping a test.

**2026-10-09 (m8/24): the floor fails, by design, and is not raised yet — the user decides at the M8 stop.** git 2.38.0:
16 of 26 passed; all 10 failures are the rebase preview's conflict forecast, which `graft-ops/src/preview/chain.rs:31`
(`MERGE_BASE_MIN`, used at :78) turns off below git 2.40 ("conflicts: not checked"); 2.40.0 and 2.56.0: 26 of 26. The
fast tier as a whole assumes 2.40+: on Debian bookworm's git 2.39.5 (an early run, before `--init`) 146 failed, 50 of
them printing "git 2.40+ needed" (`docs/release/linux-git-2.39.txt`). Either the floor rises to 2.40, or 2.38–2.39 stay supported with a forecast that
says "not checked" and the fast tier is documented as needing 2.40.

`cargo install --locked --path crates/graft` from a clean tree into an empty `CARGO_HOME`: `docs/release/cargo-install.txt`.

A person's steps, after publishing, **on a clean macOS account** (one that never had Tributary; Homebrew installed). Two
steps, as homebrew-core's tools (design note J16):

| Step | Command | Right looks like |
|---|---|---|
| 1 | `brew install <owner>/tap/<name>` | taps and installs with no Gatekeeper prompt (no notarisation, ADR-0057 §3; Homebrew does not quarantine) |
| 2 | `<command>` in a repository | the graph opens |
| + | `man <command>` | the manual opens. **Today it would not**: dist's formula puts the man page in `share/<name>/`, not `man1` (see `build.txt`, the install block) |

- [ ] **His install** (the user): steps 1–2 on a clean account, after publishing.
- [ ] The curl installer, same account: `curl -LsSf https://github.com/<owner>/<name>/releases/latest/download/<name>-installer.sh | sh`, new shell, run. Three steps.

## M8 acceptance (m8-polish-and-ship/28)

Promise: **it stays smooth on your biggest repo, looks like the mock-up, installs with one command, and teaches itself
from inside the app.** One command walks it:

```sh
scripts/m8-accept.sh          # steps 1, 3, 4, 5 · raw: .scratch/m8-polish-and-ship/accept/ · summary: docs/perf/m8-accept.txt
scripts/m8-accept.sh --perf   # and step 2, the release scroll run — from your terminal, never a helper's context
```

Steps: (1) the fast tier workspace-wide — every walk below, `m8_walk::*`, the tour to `practice done ✓` with the
`trib` binary (`tour_walk`), every capture compared with what the code draws; (2) `--perf` only: held `j`/wheel, `G`
after idle, RSS, release; (3) the contact sheet `docs/shots/m8-contact-sheet-{1,2,3}.txt` (parts at 180 000 B,
colour unverified on each first line), from `docs/shots/m8-contact-sheet-captions.tsv`; (4) the install facts quoted
from `docs/release/*.txt`; (5) `5 journeys: <n> in the fence · missing a walk: none`.

One walk alone: `cargo nextest run --workspace -E 'test(/<test>$/)'`. Each reads its count from the note's fence
(`docs/design-notes/m8-polish-and-ship.md` :213–246, 31 ids) through `m8_walk::within_the_note` and fails one key over
(`m8_walk::one_key_over_the_note_fails`); `m8_walk::every_m8_journey_id_has_a_walk` holds `WALKS` to the fence.
**colour: unverified (rule 5)** for every row: the checks read words, glyphs and rows, never colour. Captures keep the
current name (`trib`/Tributary): the final rename (27d) is parked until the name is chosen.

### Journeys (the fence, 31 ids) — what right looks like on your machine

| Id | Keys | Walk | Right looks like |
|---|---|---|---|
| hold-j-stop | 1 | `held_key::held_j_stops_on_release_unicode` (heavy tier) | hold `j` 3 s on your work repo in Ghostty full screen, let go: the list stops within a blink — no rows move after your finger is up |
| wheel-flick | 1 | `m8_walk::journeys::wheel_flick` | flick the wheel hard down your work repo and stop: the list stops when the wheel does — no rows keep sliding |
| jump-oldest | 1 | `idle_advance::g_after_laid_out_lands_without_laying_out` | once the header shows the full row count, `G`: the root commit at once, no "reading…" |
| find-key-by-name | 2 | `m8_walk::journeys::find_key_by_name` | `?` then `merge`: the cursor row reads `Merge selected branch into current`, `m` beside it |
| what-does-key | 2 | `m8_walk::journeys::what_does_key` | `?` then `x`: the cursor row reads `Compare against a marked base`, `x` beside it |
| run-unrecalled | 3 | `m8_walk::journeys::run_unrecalled` | `⌃P` `stash` `⏎`: the stash list opens, its bar names `s save` and `S/esc close` |
| stopped-how-go-on | 1 | `palette_in_progress::j7_help_mid_rebase_puts_continue_first_and_enter_asks_what_c_asks` | stopped mid-rebase, `?`: continue is the first row; `⏎` asks what `c` asks |
| light-theme | 3 | `m8_walk::journeys::light_theme` | `⌃P` `theme` `⏎` on the light row: the app turns light now, palette closed (colour by eye) |
| first-launch | 0 | `first_launch::a_terminal_launch_counts_and_shows_the_line_until_the_palette` | first `trib` ever: the status row reads `first time? ? keys here · ⌃P anything`; after the first `?` it never shows again |
| start-tour | 3 | `tour::the_palette_starts_the_tour_and_esc_returns_to_his_commit` | `⌃P` `tour` `⏎`: the tour's first card over a practice repo, not yours |
| start-tour-empty-state | 1 | `tour::t_on_the_empty_state_starts_the_tour_and_esc_comes_back` | `trib` outside a repo, `t`: the tour starts; one `tributary-practice-*` folder made |
| leave-tour | 1 | `tour::the_palette_starts_the_tour_and_esc_returns_to_his_commit` | `Esc` in the tour: back in your repo on the commit you left, the practice folder gone |
| open-recent-outside | 2 | `outside_repo::open_recent_outside_opens_the_newest_at_head_on_one_terminal` | `trib` outside a repo, `⏎`: your newest repo opens on `HEAD`'s row |
| open-second-recent | 3 | `outside_repo::open_second_recent_is_j_then_enter` | `trib`, `j`, `⏎`: the second repo in the list opens |
| init-here | 2 | `outside_repo::init_here_is_i_then_y` | in an empty folder, `i` `y`: the folder is a repository and the app is on it |
| clone | 3 | `m8_walk::clone::clone_from_the_empty_state` | `c`, paste a URL, `⏎`: it clones and opens the new repo; no shell |
| which-repo-branch | 0 | `look_chrome::capture_the_chrome_at_every_designed_size` | glance at the header: repo and branch named, at 80×24 and full screen |
| cursor-row-branch | 0 | `look_chrome::capture_the_cursor_row_in_dark_and_light` | cursor on a branch's tip: its row shows `[branch]`, readable in dark and light |
| pull-nothing-new | 1 | `daily_landing::j18_a_pull_with_nothing_new_leaves_the_cursor` | cursor off `HEAD`, `p` with nothing new: the cursor stays; the line says `up to date ✓` |
| reword-head | 4 | `m7_walk::amend_into::reword_head_with_a_line_built_on_it` | with a branch built on `HEAD`: `a`, edit, `⏎`, `y`: the message changed, the branch above follows |
| undo-see-result | 1 | `daily_landing::j20_u_and_capital_u_land_on_the_result` | `u` after a commit: the cursor on the new `HEAD`; `U`: on the redone commit, wherever the cursor was |
| quit-while-write | 2 | `quit_waits::stuck::quit_while_write_is_two_keys` | `q` `y` while a merge runs: the app quits when the merge lands |
| quit-stuck-write | 4 | `quit_waits::stuck::stop_now_mid_merge_leaves_it_paused` | a hook hangs: `q` `y`, wait for the offer, `q` `y`: the app quits inside its wait; the exit line says the merge is paused |
| seek-branch | 3 | `palette_find::walk_seek_branch` | `⌃P` a branch name `⏎`: the cursor on its tip; `HEAD` unchanged |
| seek-tag | 3 | `palette_find::walk_seek_tag` | `⌃P` a tag `⏎`: the cursor on the tagged commit |
| seek-commit-sha | 3 | `palette_find::walk_seek_commit_sha` | `⌃P` a sha prefix `⏎`: the cursor on that commit |
| seek-commit-words | 3 | `palette_find::walk_seek_commit_words` | `⌃P` words from a message `⏎`: the cursor on that commit |
| file-history | 3 | `palette_find::walk_file_history` | `⌃P` a path `⏎`: that file's history |
| rerun-recent | 2 | `palette_find::walk_rerun_recent` | `⌃P` `⏎`: the last thing you ran runs again |
| fold-merge-count (J33) | 1 | `m8_walk::journeys::fold_merge_count` | fold a merge with `Space`: its row ends `+` for a moment, then `+N`, N the commits the merge brought in; press `Ctrl+R`: the number stays, it is not re-counted |
| whitespace-mode-shown (J34) | 1 | `m8_walk::journeys::whitespace_mode_shown` | in the detail pane press `w`: the title ends `· whitespace ignored`; press `w` again: the words go. In blame the title says nothing until `w`, then `· whitespace counted` |

### Teaches itself (PROMPT §9's seven tasks, from the bar or `?` where he stands)

| Task | Test | Right looks like |
|---|---|---|
| stage a hunk | `m8_walk::teaches::stage_a_hunk_is_found_in_the_changes_pane` | on a hunk the bar leads `space stage`; `?` `stage` lands on `Stage / unstage`. (`?` `hunk` finds nothing — a question, ticket 28 NOTES) |
| commit | `m8_walk::teaches::commit_is_on_the_bar` | with something staged the bar names `c commit` |
| stash and bring back | `m8_walk::teaches::stash_and_bring_back_are_found` | `?` `stash` lands on `S`; the stash list's bar names `p pop` |
| create a branch | `m8_walk::teaches::create_a_branch_is_found` | `?` `new branch` lands on a branch row |
| merge with a preview | `m8_walk::teaches::merge_is_found_on_a_branch_row` | on a branch row `?` `merge` lands on `m` |
| resolve a conflict | `m8_walk::teaches::resolve_is_on_the_stop_banner_bar` | stopped on a conflict, the bar leads `⏎ resolve` |
| push | `m8_walk::teaches::push_is_found` | `?` `push` lands on `P` |
| the whole tour | `tour_walk::the_whole_tour_in_a_pty_ticks_all_eight_and_comes_home` (the `trib` binary over a pty) | every card to `practice done ✓`, no shell step |

### By eye and person steps (yours; nothing above can judge these)

- [ ] **J35** (by eye, colour — rule 5): cursor on a commit in your work repo: lanes off its ancestry are paler than
  the ones on it, in your theme, in Ghostty and Zed; at 16 colours they are dim, not gone.
- [ ] **J36** (person step, 2+1): add `overview = true` under `[ui]`, restart: the overview strip is there at launch;
  `O` still hides it for this run.
- [ ] **J37** (by eye, Pixel terminal): at the Pixel tier open a rewrite preview: the graph turns to text lines with
  the ghost rows dashed/hollow; header `gfx` unchanged; close it: the pixel lanes come back at once, no flicker of
  re-drawing. (17c: bytes asserted in memory only; a real Kitty/iTerm2 terminal not verified.)
- [ ] **J8 rebind** (5+1, note :127): `?` the action's name, edit 1 line under `[keys.<place>]` in your
  config, restart, `?`: the row spells your key, the bar shows it, and it works.
- [ ] **J10 fix one colour** (2+1, note :129): edit 1 line (`[theme.dark] selection = "#…"`), restart: that one
  element changes, nothing else (by eye, rule 5).
- [ ] **J16 install** (2): `brew install` then `trib` on a clean account — after publishing (see "M8: install it").
- [ ] **J17 manual** (1): `man trib` — see "M8: read the manual".
- [ ] **Look** (by eye): `docs/shots/m8-contact-sheet-1.txt` onwards beside the mock-up's scenes at its top
  (`m8-proto-*`): same layout at 80×24, 120×36, 187×50.
- [ ] **Scroll** (yours, release): `scripts/m8-accept.sh --perf` from your terminal; tails after release ≤ 100 ms
  (J1, J2), `G` ≤ 16 ms (J3), RSS ≤ budget, in `docs/perf/m8-accept.txt`.
