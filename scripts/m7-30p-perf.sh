#!/usr/bin/env bash
# m7-power-tools/30p-b (review m7-30m-30o M2, ruling R4): J's find, measured.
# Release `frames::journal_find` only, 3 runs, --test-threads=1: J over its
# caps (2 000 reflog records, 500 lost), `/step⏎`, then n×40, N×40, j×40.
# Writes docs/perf/30p-journal-find.txt (or $1).
#
# Per run: `SUMMARY run<k> rows=<n> hits=<h> p50_us=… p99_us=… max_us=…
# exit=<s>`, from the test's `frames:` line. Then the verdict:
# `SUMMARY journal-find gate(p99<=16ms)=ok|OVER|FAIL(<run> failed)`, where
# FAIL is any run that exited non-zero for any reason but the budget, or
# printed no `frames:` p99 line, and OVER is any run's p99 over 16 ms. In
# release the test itself asserts the p99 after printing its line, so an
# over-budget run exits non-zero: that exit, with its line over 16 ms and
# the gate's own panic (`input-to-frame budget in J`), is OVER, not FAIL. The script exits 1 unless every run
# passed and the gate is ok. OVER dispatches 30p-d; ok closes M2.
#
# Measurement never runs in a helper's context (CLAUDE.md, Gates): the
# conductor runs this and reads the SUMMARY lines.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib/machine-state.sh"  # ticket 49: battery / Low Power Mode guard
cd "$(dirname "$0")/.."
OUT=${1:-docs/perf/30p-journal-find.txt}
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
machine_state_begin "$OUT"
FEATURES=heavy-tests
FILTER=frames::journal_find

cargo test --release --features "$FEATURES" -p graft --test it --no-run --quiet 2>"$WORK/build.err" || {
  echo "release build failed:" >&2
  tail -20 "$WORK/build.err" >&2
  exit 1
}

for n in 1 2 3; do
  t0=$SECONDS
  cargo test --release --features "$FEATURES" -p graft --test it --quiet -- "$FILTER" --nocapture --test-threads=1 \
    >"$WORK/run$n.out" 2>&1
  status=$?
  echo "run$n exit $status, $((SECONDS - t0))s" >>"$WORK/runs.txt"
done

{
  echo "# m7-power-tools/30p J's find — $(date -u +%FT%TZ)"
  echo "# HEAD $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- crates || echo ' +uncommitted')"
  echo "# $(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m), $(sysctl -n hw.ncpu 2>/dev/null) cpus, $(uname -sr)"
  echo "# $(git --version)"
  echo "# release, --features $FEATURES, $FILTER, --test-threads=1; harness: synthetic keys, in-memory terminal, 187x50 Unicode"
  echo
  echo "## runs"
  cat "$WORK/runs.txt"
  for n in 1 2 3; do
    echo
    echo "## run$n: summary lines"
    grep -oE '(frames: .*|^test result.*|panicked.*)' "$WORK/run$n.out" |
      grep -v '0 passed; 0 failed' |
      awk '{ if (length($0) > 600) print substr($0, 1, 600) " (cut)"; else print }'
  done
  echo
  python3 - "$WORK" <<'PY'
import re, sys
work = sys.argv[1]

def fields(line):
    return dict(re.findall(r'(\w+)=(\S+)', line))

exits = {}
for line in open(f"{work}/runs.txt"):
    m = re.match(r'(run\d) exit (\d+),', line)
    if m:
        exits[m.group(1)] = int(m.group(2))

failed, over = [], []
for run in ("run1", "run2", "run3"):
    status = exits.get(run)
    line = next((l for l in open(f"{work}/{run}.out", errors="replace")
                 if l.startswith("frames: ") and "p99_us=" in l and "pane=JournalFind" in l), None)
    if line is None:
        print(f"SUMMARY {run} no frames: p99 line exit={status}")
        failed.append(run)
        continue
    f = fields(line)
    print(f"SUMMARY {run} rows={f['refs']} hits={f.get('hits', '-')} p50_us={f['p50_us']} p99_us={f['p99_us']} max_us={f['max_us']} exit={status}")
    text = open(f"{work}/{run}.out", errors="replace").read()
    budget_only = "input-to-frame budget in J" in text and int(f["p99_us"]) > 16000
    if status != 0 and not budget_only:
        failed.append(run)
    elif int(f["p99_us"]) > 16000:
        over.append(run)

if failed:
    verdict = f"FAIL({','.join(failed)} failed)"
elif over:
    verdict = "OVER"
else:
    verdict = "ok"
print(f"SUMMARY journal-find gate(p99<=16ms)={verdict}")
PY
} 2>&1 | sed "s|$HOME|~|g" >"$WORK/new.txt"
SUMMARISED=("${PIPESTATUS[@]}")

cp "$WORK/new.txt" "$OUT"
echo "wrote $OUT"
if [ "${SUMMARISED[0]}" != 0 ] || [ "${SUMMARISED[1]}" != 0 ]; then
  echo "m7-30p-perf FAILED: summarising the runs failed (status ${SUMMARISED[*]})" >&2
  exit 1
fi
if ! grep -q '^SUMMARY journal-find gate(p99<=16ms)=ok$' "$OUT"; then
  echo "m7-30p-perf FAILED: $(grep '^SUMMARY journal-find ' "$OUT" || echo 'no verdict line')" >&2
  exit 1
fi
