#!/usr/bin/env bash
# m8-polish-and-ship/12 and 12b: `⌃P` find anything, release, real pty.
# Writes docs/perf/m8-palette.txt, summary lines only, numbers only. Two
# cases, each a block:
#   refs   50 000 refs (12): `⌃P` `4` -> BRANCHES drawn
#   paths  100 000 tracked paths (12b): `⌃P` `4` -> FILES drawn
# Per case:
#   loop_us        per frame update+draw+write from the GRAFT_PERF_TRACE
#                  frame trace, p50/p95/max; check: p95 < 16 ms
#   update_us      update alone (p95 < 16 ms)
#   open_to_<group>_ms    `⌃P` `4` -> the group drawn, settled (p50/p95)
#   key_to_names_ms       one query key -> its answer drawn (p50/p95)
#   then scripts/m8-trace-summary.sh's per-field lines.
# Times on the pty side include pyte (scripts/perf-palette-pty.py).
#
# Fixtures, built under $SCRATCH (default $CARGO_TARGET_DIR/perf-palette) if
# none is there; never his repo:
#   refs-50000    one small repository, 40 000 branches (team/<word>-N) and
#                 10 000 tags (v1.N), packed.
#   paths-100000  one commit whose tree holds 100 000 paths (src/mNNN/fN.rs),
#                 every entry one blob, written with `update-index
#                 --index-info`; no worktree files: every entry is marked
#                 skip-worktree so the status is clean.
# Needs the release binary: `cargo build --release -p graft`.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
TARGET=${CARGO_TARGET_DIR:-$ROOT/target}
SCRATCH=${SCRATCH:-$TARGET/perf-palette}
BIN=$TARGET/release/graft
[ -x "$BIN" ] || { echo "no release binary at $BIN" >&2; exit 1; }
REFS=$SCRATCH/refs-50000
PATHS=$SCRATCH/paths-100000

if [ ! -d "$REFS/.git" ]; then
  mkdir -p "$REFS"
  git -C "$REFS" init --quiet --initial-branch=main
  for n in 1 2 3; do
    echo "$n" >"$REFS/file.txt"
    git -C "$REFS" add file.txt
    GIT_AUTHOR_DATE="2026-01-0$n 12:00 +0000" GIT_COMMITTER_DATE="2026-01-0$n 12:00 +0000" \
      git -C "$REFS" -c user.name=perf -c user.email=perf@example.invalid commit --quiet -m "commit $n"
  done
  tips=$(git -C "$REFS" rev-list main | tr '\n' ' ')
  awk -v tips="$tips" 'BEGIN {
    split(tips, t, " ")
    split("feature rebase fix chore spike", w, " ")
    for (i = 0; i < 40000; i++) printf "create refs/heads/team/%s-%d %s\n", w[i % 5 + 1], i, t[i % 3 + 1]
    for (i = 0; i < 10000; i++) printf "create refs/tags/v1.%d %s\n", i, t[i % 3 + 1]
  }' | git -C "$REFS" update-ref --stdin
  git -C "$REFS" pack-refs --all
fi

if [ ! -d "$PATHS/.git" ]; then
  mkdir -p "$PATHS"
  git -C "$PATHS" init --quiet --initial-branch=main
  blob=$(echo "perf" | git -C "$PATHS" hash-object -w --stdin)
  awk -v blob="$blob" 'BEGIN {
    for (i = 0; i < 100000; i++) printf "100644 %s\tsrc/m%03d/f%d.rs\n", blob, i % 1000, i
  }' | git -C "$PATHS" update-index --add --index-info
  tree=$(git -C "$PATHS" write-tree)
  commit=$(GIT_AUTHOR_DATE="2026-01-01 12:00 +0000" GIT_COMMITTER_DATE="2026-01-01 12:00 +0000" \
    git -C "$PATHS" -c user.name=perf -c user.email=perf@example.invalid commit-tree "$tree" -m "100k paths")
  git -C "$PATHS" update-ref refs/heads/main "$commit"
  git -C "$PATHS" ls-files -z | xargs -0 git -C "$PATHS" update-index --skip-worktree
fi

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
status=0
ok=1

# p50/p95/max of the frames' update+draw+write, or of one field.
loop_line() {
  awk -v only="${2:-}" '
    $1 == "frame" {
      s = 0; seen = 0
      for (i = 2; i <= NF; i++) {
        split($i, kv, "=")
        if (kv[2] == "-") continue
        if (only == "" && (kv[1] == "update_us" || kv[1] == "draw_us" || kv[1] == "write_us")) { s += kv[2]; seen = 1 }
        if (only != "" && kv[1] == only) { s = kv[2]; seen = 1 }
      }
      if (seen) v[++n] = s
    }
    END {
      name = only == "" ? "loop_us" : only
      if (n == 0) { print name " n=0"; exit }
      for (i = 2; i <= n; i++) { t = v[i]; for (j = i - 1; j >= 1 && v[j] > t; j--) v[j + 1] = v[j]; v[j + 1] = t }
      printf "%s n=%d p50=%d p95=%d max=%d\n", name, n, v[int((50 * n + 99) / 100)], v[int((95 * n + 99) / 100)], v[n]
    }' "$1"
}

# One case: its repository copied (Graft writes .git/graft/ in what it
# opens), the pty driver run, its block of summary lines.
measure() {
  local name=$1 fixture=$2 group=$3 queries=$4
  local repo=$WORK/$name trace=$WORK/$name.trace
  cp -R "$fixture" "$repo"
  local run=0
  GRAFT_PERF_TRACE=$trace python3 "$ROOT/scripts/perf-palette-pty.py" "$BIN" "$repo" 120 36 5 \
    "$group" "$queries" >"$WORK/$name.pty" 2>"$WORK/$name.err" || run=$?
  [ "$run" -eq 0 ] || status=$run
  local loop up p95 up95
  loop=$(loop_line "$trace")
  up=$(loop_line "$trace" update_us)
  p95=$(echo "$loop" | grep -oE 'p95=[0-9]+' | cut -d= -f2 || true)
  up95=$(echo "$up" | grep -oE 'p95=[0-9]+' | cut -d= -f2 || true)
  echo "## $name: pty driver exit $run"
  echo "$loop"
  if [ -n "$p95" ] && [ "$p95" -lt 16000 ]; then echo "check loop_p95_under_16ms ok"; else echo "check loop_p95_under_16ms FAIL"; ok=0; fi
  echo "$up"
  if [ -n "$up95" ] && [ "$up95" -lt 16000 ]; then echo "check update_p95_under_16ms ok"; else echo "check update_p95_under_16ms FAIL"; fi
  sed -n 's/^SUMMARY //p' "$WORK/$name.pty"
  "$ROOT/scripts/m8-trace-summary.sh" "$trace" "$WORK/$name.summary" >/dev/null
  cat "$WORK/$name.summary"
}

OUT=$ROOT/docs/perf/m8-palette.txt
{
  echo "# m8/12, 12b palette find: real pty 120x36, release, Unicode."
  echo "# $(date -u +%Y-%m-%dT%H:%MZ) $(uname -sm)"
  # (12) 40 000 branches + 10 000 tags.
  measure refs "$REFS" BRANCHES "4417,v1.99,-2024"
  # (12b) 100 000 tracked paths in one commit.
  measure paths "$PATHS" FILES "4417,m99/f,f12"
} >"$OUT"
cat "$OUT"
[ "$status" -eq 0 ] && [ "$ok" -eq 1 ]
