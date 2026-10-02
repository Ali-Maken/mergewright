# perf-machine-state-guard — a release perf run on battery or Low Power Mode is recorded, not judged

Status: built 2026-10-02; shellcheck box open (tool not installed). Was: ready (planner 2026-10-02, 47 ruling 1). Build before the M7 review release perf run.
Build with: Sonnet. Shell only and fully specified; the output can be checked by eye.
Needs: none.
Blocked by: none

## Need
47's `g_after_idle` miss cost a debugger round. The cause turned out to be machine state: battery and Low Power Mode,
with a CPU-only control running 2.07× slower (`docs/perf/47-debug.txt:14-16`). Nothing in the perf files recorded
the machine's state, so nobody could see it. "Remember to plug in" is a be-careful rule, and those don't hold. A
check in the script does.

## Decided
- New `scripts/lib/machine-state.sh`, sourced by every script that runs a `--release` measurement. Find them with
  `grep -l -- '--release' scripts/` and list them in the report.
- It writes one line at the head of the perf file and to stdout: `SUMMARY machine ac=<1|0> lowpower=<1|0>
  os=<darwin|other>`, read from `pmset -g batt` and `pmset -g` on macOS.
- On other systems: `ac=unknown lowpower=unknown`, and the run is judged as normal. The guard does not block where
  it cannot see.
- **macOS, judged run, `ac=0` or `lowpower=1`:** the script still runs and records everything, but prints `SUMMARY
  not_judged machine` and exits 3. Exit 3 is neither a pass nor a miss. The milestone review treats it as "re-run on
  mains".
- `ALLOW_LOW_POWER=1` exists only for diagnosis runs like the debugger's. It prints `SUMMARY machine override` and
  exits 3 all the same.
- No retry, sleep or wait for power. The script reports and stops being a verdict (rule 2).

## Touches
- The release scripts found by the grep: `47-scroll-acceptance.sh`, `m7-perf-key.sh`, `fix-scroll-held-key.sh`,
  `47-debug-idle.sh`, and any others. Confirm each one's exit-code contract is still read correctly by its caller.
- `TESTING.md` §"The heavy tier": one line on exit 3.
- `.claude/skills/conduct/SKILL.md` is not edited here. The conductor adds "exit 3 → re-run on mains" to its review
  step itself.
- The redaction guard: the new line carries no paths.

## Acceptance (each tick names its check)
- [x] (check: `bash scripts/lib/machine-state.sh --self-test` → ac-lpm-off exit=0, battery exit=3, ac-lpm-on exit=3, rc 0) `bash scripts/lib/machine-state.sh --self-test` prints the parsed line for three fixture `pmset` outputs
      (AC + LPM off; battery; AC + LPM on), with exits 0 / 3 / 3. The fixtures sit beside the script.
- [x] (check: `MACHINE_STATE_DRY_RUN=<file> bash scripts/<s>.sh` for all 25, `grep -c` = 1 each, docs/ untouched; real insert/exit checked by harness on this battery+LPM machine: appended block and rewritten file both get the line first, exit 3) Every sourcing script writes the `SUMMARY machine` line first in its perf file. Check with
      `grep -c '^SUMMARY machine' docs/perf/<file>` after one dry run per script, or show by `--help`/a dry-run flag
      where a full run is too heavy.
- [x] (check, conductor 2026-10-02, shellcheck 0.11.0 `-S warning`: `scripts/lib/machine-state.sh` clean; the 26 edited scripts show 2×SC2034, 20×SC2164, 1×SC2296 — identical counts on the same files at f851fdc4~1, so 49 added none; pre-existing findings → gap M7/scripts-shellcheck) `shellcheck` clean on the new file and each edited script.

## Size
Small (~15k).

## NOTES (builder, 2026-10-02)
- No scout: shell only, each script grepped directly.
- 25 scripts wired (`grep -l -- '--release' scripts/*.sh`); `grep -L machine-state.sh $(grep -l -- '--release' scripts/*.sh)` empty.
- Insertion: line goes first in what the run wrote: file head if rewritten, block head if appended (byte offset + cksum).
- Exit mapping: 0/1 → 3 when not judged; other codes (setup errors) pass through.
- Nested guard (m7-12-final-perf → heap-slope): inner records, outer judges (MACHINE_STATE_OUTER).
- m7-perf-key-selftest feeds the mains fixture (canned data): selftest still "every check held".
- Dry-run seam: MACHINE_STATE_DRY_RUN=<file>.
- Old-rev worktrees run by m7-12/m7-key37 lack the lib; they run their own copy, unaffected.
- Did not compact.
