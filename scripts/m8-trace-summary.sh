#!/usr/bin/env bash
# m8-polish-and-ship G-trace: summarise a GRAFT_PERF_TRACE file.
#
#   scripts/m8-trace-summary.sh <trace> [out]   summary -> out (default
#                                               docs/perf/m8-trace-<date>.txt)
#   scripts/m8-trace-summary.sh --self-test     the checked-in sample against
#                                               its expected summary; exit 1 on
#                                               any difference
#
# Summary lines only, numbers only (the trace has no names in it to leak):
#   header ...                 the last header line, and how many there were
#   frames=N span_ms= frames_per_s=
#   <field> n= p50= p95= p99= max=   per numeric field; `-` values skipped,
#                                    n counts the rest (nearest-rank)
#   tail_ms=                   last frame minus the last frame that applied
#                              an input (motions>0): how long frames kept
#                              coming after the last key was drawn
#   power ...                  `pmset -g batt`'s first line (macOS), else
#                              `power=unknown`
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)

summarise() { # trace power-line
  awk -v power="$2" '
    function sortn(a, n,   i, j, t) {
      for (i = 2; i <= n; i++) { t = a[i]; for (j = i - 1; j >= 1 && a[j] > t; j--) a[j + 1] = a[j]; a[j + 1] = t }
    }
    function rank(a, n, p,   k) { k = int((p * n + 99) / 100); if (k < 1) k = 1; return a[k] }
    $1 == "header" { header = $0; headers++; next }
    $1 == "frame" {
      frames++
      for (i = 2; i <= NF; i++) {
        split($i, kv, "=")
        name = kv[1]; value = kv[2]
        if (!(name in seen)) { seen[name] = 1; order[++fields] = name }
        if (value == "-") continue
        count[name]++
        values[name, count[name]] = value + 0
      }
      at = 0
      for (i = 2; i <= NF; i++) if ($i ~ /^at_us=/) at = substr($i, 7) + 0
      if (frames == 1) first = at
      last = at
      for (i = 2; i <= NF; i++) if ($i ~ /^motions=/ && substr($i, 9) + 0 > 0) moved = at
      next
    }
    END {
      if (headers) printf "%s headers=%d\n", header, headers; else print "header none"
      span = (last - first) / 1000
      fps = (span > 0) ? (frames - 1) * 1000 / span : 0
      printf "frames=%d span_ms=%.1f frames_per_s=%.1f\n", frames, span, fps
      for (f = 1; f <= fields; f++) {
        name = order[f]
        if (name == "at_us") continue
        n = count[name]
        if (n == 0) { printf "%s n=0\n", name; continue }
        delete a
        for (i = 1; i <= n; i++) a[i] = values[name, i]
        sortn(a, n)
        printf "%s n=%d p50=%d p95=%d p99=%d max=%d\n", name, n, rank(a, n, 50), rank(a, n, 95), rank(a, n, 99), a[n]
      }
      if (moved == "") print "tail_ms=- (no frame applied an input)"
      else printf "tail_ms=%.1f\n", (last - moved) / 1000
      print power
    }
  ' "$1"
}

if [ "${1:-}" = "--self-test" ]; then
  sample=$ROOT/scripts/testdata/m8-trace-sample.trace
  expected=$ROOT/scripts/testdata/m8-trace-sample.summary
  if diff -u "$expected" <(summarise "$sample" "power self-test"); then
    echo "m8-trace-summary self-test: ok"
  else
    echo "m8-trace-summary self-test: FAIL" >&2
    exit 1
  fi
  exit 0
fi

trace=${1:?usage: m8-trace-summary.sh <trace> [out] | --self-test}
out=${2:-$ROOT/docs/perf/m8-trace-$(date +%Y-%m-%d).txt}
if command -v pmset >/dev/null; then
  power="power $(pmset -g batt | head -1)"
else
  power="power=unknown"
fi
summarise "$trace" "$power" >"$out"
cat "$out"
