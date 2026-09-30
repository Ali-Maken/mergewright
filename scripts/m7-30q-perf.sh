#!/usr/bin/env bash
# m7-power-tools/30q-c (ADR-0052, ticket 30q R2's cost): what the
# before-snapshot costs now that it keeps raw bytes.
# Release `graft-ops --example snapshot_cost`, one run: three cases on a
# 5 000-file repository the example builds (no graft-fixtures Scale grows
# the tree), 20 runs each; `take` and `raw_record` timed apart.
#   case1 no conversion, 5 changed files
#   case2 no conversion, clean worktree
#   case3 core.autocrlf=true, 50 changed LF files
# Writes docs/perf/30q-snapshot.txt (or $1): a header, `SUMMARY case<k> …`
# per case, then `SUMMARY raw-record gate(case1 p50<=10ms)=ok|OVER|FAIL`.
# Exits 1 unless the gate is ok. OVER: the conductor files the follow-up.
#
# Measurement never runs in a helper's context (CLAUDE.md, Gates): the
# conductor runs this and reads the SUMMARY lines.
set -u
cd "$(dirname "$0")/.."
OUT=${1:-docs/perf/30q-snapshot.txt}
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

cargo build --release -p graft-ops --example snapshot_cost --quiet 2>"$WORK/build.err" || {
  echo "release build failed:" >&2
  tail -20 "$WORK/build.err" >&2
  exit 1
}

t0=$SECONDS
cargo run --release -p graft-ops --example snapshot_cost --quiet >"$WORK/run.out" 2>&1
status=$?

{
  echo "# m7-power-tools/30q the snapshot's cost — $(date -u +%FT%TZ)"
  echo "# HEAD $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- crates || echo ' +uncommitted')"
  echo "# $(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m), $(sysctl -n hw.ncpu 2>/dev/null) cpus, $(uname -sr)"
  echo "# $(git --version)"
  echo "# release, graft-ops example snapshot_cost, 5000 files, 20 runs per case; exit $status, $((SECONDS - t0))s"
  echo
  sed 's/^case/SUMMARY case/' "$WORK/run.out"
  python3 - "$WORK/run.out" "$status" <<'PY'
import re, sys
text = open(sys.argv[1], errors="replace").read()
status = int(sys.argv[2])
m = re.search(r'^case1 .*raw_p50_us=(\d+)', text, re.M)
if status != 0 or m is None or "snapshot_cost: done" not in text:
    verdict = f"FAIL(exit={status})"
elif int(m.group(1)) > 10000:
    verdict = "OVER"
else:
    verdict = "ok"
print(f"SUMMARY raw-record gate(case1 p50<=10ms)={verdict}")
PY
} 2>&1 | sed "s|$HOME|~|g" >"$WORK/new.txt"

mkdir -p "$(dirname "$OUT")"
cp "$WORK/new.txt" "$OUT"
echo "wrote $OUT"
if ! grep -q '^SUMMARY raw-record gate(case1 p50<=10ms)=ok$' "$OUT"; then
  echo "m7-30q-perf: $(grep '^SUMMARY raw-record ' "$OUT" || echo 'no verdict line')" >&2
  exit 1
fi
