#!/usr/bin/env bash
# m8-polish-and-ship/12: `⌃P` find anything on 50 000 refs, release, real pty.
# Writes docs/perf/m8-palette.txt, summary lines only, numbers only:
#   loop_us        per frame update+draw+write from the GRAFT_PERF_TRACE
#                  frame trace, p50/p95/max; check: p95 < 16 ms
#   open_to_branches_ms   `⌃P` `4` -> BRANCHES drawn, settled (p50/p95)
#   key_to_names_ms       one query key -> its answer drawn (p50/p95)
#   then scripts/m8-trace-summary.sh's per-field lines.
# Times on the pty side include pyte (scripts/perf-palette-pty.py).
#
# The fixture: one small repository with 40 000 branches (team/<word>-N) and
# 10 000 tags (v1.N), packed; built under $SCRATCH (default
# $CARGO_TARGET_DIR/perf-palette) if none is there. Never his repo.
# Needs the release binary: `cargo build --release -p graft`.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
TARGET=${CARGO_TARGET_DIR:-$ROOT/target}
SCRATCH=${SCRATCH:-$TARGET/perf-palette}
BIN=$TARGET/release/graft
[ -x "$BIN" ] || { echo "no release binary at $BIN" >&2; exit 1; }
FIXTURE=$SCRATCH/refs-50000

if [ ! -d "$FIXTURE/.git" ]; then
  mkdir -p "$FIXTURE"
  git -C "$FIXTURE" init --quiet --initial-branch=main
  for n in 1 2 3; do
    echo "$n" >"$FIXTURE/file.txt"
    git -C "$FIXTURE" add file.txt
    GIT_AUTHOR_DATE="2026-01-0$n 12:00 +0000" GIT_COMMITTER_DATE="2026-01-0$n 12:00 +0000" \
      git -C "$FIXTURE" -c user.name=perf -c user.email=perf@example.invalid commit --quiet -m "commit $n"
  done
  tips=$(git -C "$FIXTURE" rev-list main | tr '\n' ' ')
  awk -v tips="$tips" 'BEGIN {
    split(tips, t, " ")
    split("feature rebase fix chore spike", w, " ")
    for (i = 0; i < 40000; i++) printf "create refs/heads/team/%s-%d %s\n", w[i % 5 + 1], i, t[i % 3 + 1]
    for (i = 0; i < 10000; i++) printf "create refs/tags/v1.%d %s\n", i, t[i % 3 + 1]
  }' | git -C "$FIXTURE" update-ref --stdin
  git -C "$FIXTURE" pack-refs --all
fi

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
REPO=$WORK/repo
# Graft writes .git/graft/ in what it opens: a copy per run.
cp -R "$FIXTURE" "$REPO"
TRACE=$WORK/trace.txt

status=0
GRAFT_PERF_TRACE=$TRACE python3 "$ROOT/scripts/perf-palette-pty.py" "$BIN" "$REPO" 120 36 5 \
  >"$WORK/pty.txt" 2>"$WORK/pty.err" || status=$?

loop=$(awk '
  $1 == "frame" {
    s = 0
    for (i = 2; i <= NF; i++) {
      split($i, kv, "=")
      if ((kv[1] == "update_us" || kv[1] == "draw_us" || kv[1] == "write_us") && kv[2] != "-") s += kv[2]
    }
    v[++n] = s
  }
  END {
    if (n == 0) { print "loop_us n=0"; exit }
    for (i = 2; i <= n; i++) { t = v[i]; for (j = i - 1; j >= 1 && v[j] > t; j--) v[j + 1] = v[j]; v[j + 1] = t }
    p50 = v[int((50 * n + 99) / 100)]; p95 = v[int((95 * n + 99) / 100)]
    printf "loop_us n=%d p50=%d p95=%d max=%d\n", n, p50, p95, v[n]
  }' "$TRACE")
p95=$(echo "$loop" | grep -oE 'p95=[0-9]+' | cut -d= -f2 || true)
if [ -n "$p95" ] && [ "$p95" -lt 16000 ]; then check="check loop_p95_under_16ms ok"; else check="check loop_p95_under_16ms FAIL"; fi
# update alone: the palette's own per-key work, apart from drawing the
# graph under it (every frame here draws ~16 000 ref badges on one row).
up95=$(awk '$1 == "frame" { for (i = 2; i <= NF; i++) { split($i, kv, "="); if (kv[1] == "update_us" && kv[2] != "-") v[++n] = kv[2] } }
  END { for (i = 2; i <= n; i++) { t = v[i]; for (j = i - 1; j >= 1 && v[j] > t; j--) v[j + 1] = v[j]; v[j + 1] = t }
        if (n) print v[int((95 * n + 99) / 100)] }' "$TRACE")
if [ -n "$up95" ] && [ "$up95" -lt 16000 ]; then upcheck="check update_p95_under_16ms ok p95=$up95"; else upcheck="check update_p95_under_16ms FAIL p95=${up95:-none}"; fi

OUT=$ROOT/docs/perf/m8-palette.txt
{
  echo "# m8/12 palette find: 40 000 branches + 10 000 tags, real pty 120x36, release, Unicode."
  echo "# $(date -u +%Y-%m-%dT%H:%MZ) $(uname -sm); pty driver exit $status"
  echo "$loop"
  echo "$check"
  echo "$upcheck"
  sed -n 's/^SUMMARY //p' "$WORK/pty.txt"
  "$ROOT/scripts/m8-trace-summary.sh" "$TRACE" "$WORK/summary.txt" >/dev/null
  cat "$WORK/summary.txt"
} >"$OUT"
cat "$OUT"
[ "$status" -eq 0 ] && [[ "$check" == *" ok" ]]
