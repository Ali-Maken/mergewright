# Review: m8-polish-and-ship 51 — every env name is `MWR_*`

Commit `d5616e4d` against base `049d5f07` (merged at `f2809cf0`). Ticket
`.scratch/m8-polish-and-ship/issues/51-mergewright-env-names.md`. It also takes the fix for
`docs/reviews/m8-mergewright-50.md` Major 1 (the practice-folder prefix).

**Counts: Critical 0 · Major 1 · Minor 5 · Questions 0.**

## What was verified, and how

- **Gates: not run.** The conductor said no cargo. I have no fmt, clippy, nextest, heavy-tier or release-perf result
  of my own. The builder's NOTES claim the gates are green; I did not observe that.
- **Old env names:** `git grep -nE "TRIB_|GRAFT_[A-Z]" d5616e4d` over `crates`, `scripts`, `ARCHITECTURE.md`,
  `README.md`, `docs/man`, the pty-walk and shot scripts, `.claude`, `CLAUDE.md` and `PROMPT.md` finds nothing
  outside `graft-core/src/on_disk` (the names table, allow-listed).
  - In TESTING.md the only hits are lines 2310 and 2318. Both are dated records, which the ticket keeps on purpose.
  - Elsewhere the only hits are `.scratch/m6-conflicts/try-stash.sh` and `.scratch/m7-power-tools/48-recapture.sh`.
    Both are history, outside the ticket's file set.
  - Nothing reads an env name through `older()` except the test `clock_pin.rs:70`, which sets the old name to show it
    is ignored.
- **Tour delete guard:** I read `tour.rs:198-235` and `build.rs:50-71`, then grepped every constructor of
  `PracticeRepo`.
  - The guard accepts a name only if it starts with `<dir>-practice-` for one of the 3 rows of `GENERATIONS`
    (`graft`, `tributary`, `mergewright`). The other checks are unchanged: not a link, canonical, the parent is
    `under`, and the marker is a regular file.
  - `removal_accepts_every_earlier_generations_folder` builds its names from `old.dir()`, not through `practice_prefix`,
    so it checks the function independently. `removal_refuses_a_root_with_the_wrong_name` still refuses
    `my-project`.
  - The guard was widened safely. See Minor 2 for what the widening reaches.
- **Cache move:** I read `scripts/clone-git-git.sh:41-59`, then ran it myself twice. Each run used a temp `HOME`
  holding a bare repo at `~/.cache/tributary/git-git.git` plus a file of my own (`MYDATA`), and a fake `git` on
  `PATH` that fails on `clone`.
  - Run 1 printed `moving … to …/mergewright/git-git.git` and then `updating`.
  - Run 2 printed only `updating`, so the move is idempotent.
  - `MYDATA` survived. `clone` was called 0 times.
  - When old and new both exist, the move is skipped and the old one is left as it is (Minor 3). No data was lost in
    any case.
- **Rule 3:** I grepped the added lines under `crates/` and `scripts/` for `let _`, `unwrap_or_default`, `Err(_)`,
  `|| true` and `2>/dev/null`. There are no hits. The `unwrap`/`expect` hits are all in tests.

## Acceptance spot-check: 6 of 6 boxes checked, all 6 hold

| Box | Checked against | Holds? |
|---|---|---|
| Grep test | `rename_names/scripts.rs:115-148` reads `Generation::CURRENT.older().map(env_prefix)` over the stated set (README and shot scripts now included). My own grep, above, agrees: zero hits. Allow-list has 1 entry, as NOTES says. | yes |
| Askpass | The four tests exist (`askpass_mode.rs:62,82,93`, `graft-git/src/run/tests.rs:139`). `git grep '"MWR_ASKPASS'` over `crates` returns `on_disk.rs:173` only. | yes (existence and grep; tests not run) |
| Old name not read | `clock_pin.rs:68-86` loops `older()` and sets each `<prefix>NOW=1.5` alone. | yes |
| clone-git-git dry run | `.scratch/m8-polish-and-ship/mwr-clone-git-git-dry-run.txt` ends `clone called: 0`. My own re-run, above, agrees. | yes |
| Captures drift | `mwr-env-shot-drift.txt` ends `changed 10 · name-only 2 · drift-files 8 · png 0`, matching NOTES. NOTES lists the 8 drift files (7 m5-ui header comments, plus the tour step-1 capture). | yes |
| 27-shots-left | 450 lines at `d5616e4d`, and the sorted set is identical to the one at `049d5f07`. | yes (see Minor 4) |

## Major

### 1. The cache moved, but three readers still point at the old paths, two of them on lines this commit edited

- **Where:**
  - `crates/graft/tests/it/gallery.rs:671-677`: the default git-git root is `<cache>/graft/git-git.git`. The doc
    comment just above it (`:662-663`) says it follows "`scripts/clone-git-git.sh`'s own rules". The script writes
    `<cache>/mergewright/git-git.git` (`clone-git-git.sh:42`).
  - `scripts/m1-checkpoint.sh:78`: the error text names `${XDG_CACHE_HOME:-$HOME/.cache}/tributary/git-git.git`. This
    commit edited that same line (`TRIB_` → `MWR_`) and left the path.
  - `TESTING.md:595`: the "Where" row of the git/git table says `…/tributary/git-git.git`. It too was edited in this
    commit, and the path was left.
  - `TESTING.md:619` (Linux kernel suggestion, `~/.cache/tributary/linux.git`) is the same kind of leftover, less
    important.
- **Why it matters:**
  - The gallery (`cargo run -p mergewright --example gallery`) is the M1 checkpoint harness, and KNOWN_GAPS:286 sends
    the user to it ("the gallery re-walk"). Its git-git stop shows "not available" for a clone the script says is
    present. Following the prompt and re-running the script prints `updating`, and the gallery still cannot find it.
  - The gallery mismatch has been there since 27c (`0a21f0eb` moved the script to `tributary/`). This ticket owns the
    cache move (ticket §Decisions, "Dev caches"), and its Touches line lists "every script exporting one". The
    knock-on check (CLAUDE.md "Things are connected") should have found every reader of the path, not only every
    writer of the env name.
  - The two documentation lines now point readers at a path the script no longer uses.
  - No row in KNOWN_GAPS covers any of this.
- **Suggested fix:**
  - Take the default from the script, and do not keep a copy. Either have `roots()` run `scripts/clone-git-git.sh --where`, or
    build the path from `Generation::CURRENT.dir()`, which `graft_core` already exposes.
  - Change `m1-checkpoint.sh:78` and `TESTING.md:595,619` to `mergewright`. Add a test to `rename_names` that compares
    the gallery's default with the script's `--where` output under a temp `HOME`.

## Minor

### 1. The ratchet was raised for 3 files. I accept two as they are; the third would have been smaller with a helper

- `crates/graft/tests/it/over_400.txt`:
  - `clock_pin.rs` 443→448: the loop over `older()`. Fine.
  - `tour_walk.rs` 928→929: a rustfmt line split. Fine.
  - `crates/graft-ops/src/tour/tests.rs` 846→893 (+47). About 29 lines are the two new tests. Most of the other ~18
    come from rustfmt breaking `&format!("{}N-N", practice_prefix(Generation::CURRENT))` over 4 lines, at six call
    sites (`tests.rs:295,378,409,427,458,733`).
- **Suggested fix:** add a test helper, `fn current_name(rest: &str) -> String`. It folds each site back to one line
  and pulls the file back to about 875. This is not blocking, because test files sit under the ratchet by name.

### 2. A doc comment claims older practice folders "still clean up", but nothing ever hands one to the guard

- **Where:** `crates/graft-ops/src/tour.rs:64` ("Removal accepts every generation's, so older folders still clean
  up") and `tests.rs:343` ("A practice folder an earlier generation made is still cleaned up").
- **Why it matters:** `PracticeRepo` has exactly one constructor, `build.rs:71`, and it always names the folder with
  `practice_prefix(Generation::CURRENT)` (`build.rs:56,64`). The app keeps it only in memory
  (`event_loop/places/practice.rs:19`), and nothing scans `$TMPDIR`.
  - In production, then, the guard only ever sees the current prefix. A `tributary-practice-*` folder left by an
    earlier build is never removed by the app: it ages out of `$TMPDIR`.
  - The guard also accepts `graft-practice-`. No build ever wrote that (`git log -S'graft-practice'` hits only
    prototype and capture text). The marker and root-equality checks still hold, so this is not a safety problem.
    But the comment overstates what the code does (rule 1).
- **Suggested fix:** reword it: "Removal accepts every generation's prefix, so a rename never makes the guard refuse
  a root this build recorded". Or narrow the guard to `CURRENT`, since this is the only path that reaches it. Either is
  safe. The conductor's ask was to accept every generation's prefix, which the code does.

### 3. When old and new caches both exist, the old one is left with no word

- **Where:** `scripts/clone-git-git.sh:55`. The move is skipped when `$CACHE` exists, and nothing is printed about
  `$OLD_CACHE`. I observed it in my run: `tributary/git-git.git` stayed beside `mergewright/git-git.git`.
- **Why it matters:** it is not data loss. But a stale clone of about 42 MB stays in his cache with no hint. The move
  also leaves an empty `tributary/` directory behind.
- **Suggested fix:** in the both-exist case, print `left <old>: <new> already exists; remove it with rm -rf <old>`.
  After a move, `rmdir` the parent `tributary/` if it is empty (`rmdir` refuses a non-empty one, so it is safe).

### 4. Re-running `27-shots-left.txt` rewrote 788 lines with the same 450 entries

- **Where:** `.scratch/m8-polish-and-ship/27-shots-left.txt`. Sorted, it is identical to the file at `049d5f07`, so
  only the order changed.
- **Why it matters:** the diff is noise, and a real change to the list could hide in it.
- **Suggested fix:** sort the generator's output (`LC_ALL=C sort`) wherever 27a's command produces it.

### 5. `tour_walk.rs` spells the practice prefix as a literal

- **Where:** `crates/graft/tests/it/tour_walk.rs:222,275` (`"mergewright-practice-"`), and the doc comments at `:96`
  and `:211`.
- **Why it matters:** the ticket's aim is that the next rename touches only the table. The rest of the diff reads
  `practice_prefix` or `Generation`; these two literals will need a hand edit. A test that checks against a literal is
  defensible as an independent oracle, but `tour/tests.rs:340` already does that job.
- **Suggested fix:** use `graft_ops::tour::practice_prefix(Generation::CURRENT)`.

## Not findings, noted

- **His machine:** a `TRIB_*` export in his shell rc is now silently ignored. The builder reported this; it is the
  ticket's §1 decision. It is for the conductor to pass on.
- `README.md:52` still says "the files Tributary writes". That is out of scope here; `mergewright-docs-sweep` (55)
  owns README prose.
