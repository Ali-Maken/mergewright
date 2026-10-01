# perf-machine-state-guard — a release perf run on battery or Low Power Mode is recorded, not judged

Status: ready (planner 2026-10-02, 47 ruling 1). Build before the M7 review release perf run.
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
- [ ] `bash scripts/lib/machine-state.sh --self-test` prints the parsed line for three fixture `pmset` outputs
      (AC + LPM off; battery; AC + LPM on), with exits 0 / 3 / 3. The fixtures sit beside the script.
- [ ] Every sourcing script writes the `SUMMARY machine` line first in its perf file. Check with
      `grep -c '^SUMMARY machine' docs/perf/<file>` after one dry run per script, or show by `--help`/a dry-run flag
      where a full run is too heavy.
- [ ] `shellcheck` clean on the new file and each edited script.

## Size
Small (~15k).
