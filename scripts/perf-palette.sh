#!/usr/bin/env bash
# m8-polish-and-ship/12: `⌃P` find anything on 50 000 refs, release, real pty.
# 12b: the 100 000-paths case. 12c: a fixture argument, held `j` on the piled
# refs fixture, and the spread-refs case.
#
#   scripts/perf-palette.sh [piled|with-refs|paths|all]     (default: all)
#
# Writes docs/perf/m8-palette.txt, one section per run, summary lines only,
# numbers only:
#   loop_us        per frame update+draw+write from the MWR_PERF_TRACE
#                  frame trace, p50/p95/max; check: p95 < 16 ms
#   open_to_<group>_ms    `⌃P` `4` -> BRANCHES (refs cases) or FILES (paths)
#                         drawn, settled (p50/p95)
#   key_to_names_ms       one query key -> its answer drawn (p50/p95)
#   then scripts/m8-trace-summary.sh's per-field lines.
#   piled only: held `j`, no palette (scripts/scroll-lag-pty.py): its trace
#   summary and `check held_j_draw_p95_under_16ms`.
# Times on the pty side include pyte. Each section's header line carries the
# load average when it started.
#
# Fixtures, never his repo:
#   piled      one small repository with 40 000 branches (team/<word>-N) and
#              10 000 tags (v1.N) on 3 commits, packed: ~16 000 refs on one
#              row. Built under $SCRATCH (default $CARGO_TARGET_DIR/perf-palette)
#              if none is there.
#   with-refs  the heavy tier's `Scale::HEAVY.with_commit_graph()
#              .with_refs(50_000)` (crates/mergewright/tests/it/many_refs.rs), refs
#              spread over history; built or found by mergewright-fixtures'
#              `synthetic` example.
#   paths      one commit whose tree holds 100 000 paths (src/mNNN/fN.rs),
#              every entry one blob, written with `update-index --index-info`;
#              no worktree files: every entry is marked skip-worktree so the
#              status is clean. Built under $SCRATCH if none is there.
# Needs the release binary: `cargo build --release -p mergewright`.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
TARGET=${CARGO_TARGET_DIR:-$ROOT/target}
SCRATCH=${SCRATCH:-$TARGET/perf-palette}
BIN=$TARGET/release/mwr
[ -x "$BIN" ] || { echo "no release binary at $BIN" >&2; exit 1; }
WHICH=${1:-all}
case "$WHICH" in
  piled | with-refs | paths | all) ;;
  *) echo "usage: $0 [piled|with-refs|paths|all]" >&2; exit 2 ;;
esac
runs() { [ "$WHICH" = all ] || [ "$WHICH" = "$1" ]; }

paths_fixture() {
  local fixture=$SCRATCH/paths-100000
  if [ ! -d "$fixture/.git" ]; then
    mkdir -p "$fixture"
    git -C "$fixture" init --quiet --initial-branch=main
    local blob tree commit
    blob=$(echo "perf" | git -C "$fixture" hash-object -w --stdin)
    awk -v blob="$blob" 'BEGIN {
      for (i = 0; i < 100000; i++) printf "100644 %s\tsrc/m%03d/f%d.rs\n", blob, i % 1000, i
    }' | git -C "$fixture" update-index --add --index-info
    tree=$(git -C "$fixture" write-tree)
    commit=$(GIT_AUTHOR_DATE="2026-01-01 12:00 +0000" GIT_COMMITTER_DATE="2026-01-01 12:00 +0000" \
      git -C "$fixture" -c user.name=perf -c user.email=perf@example.invalid commit-tree "$tree" -m "100k paths")
    git -C "$fixture" update-ref refs/heads/main "$commit"
    git -C "$fixture" ls-files -z | xargs -0 git -C "$fixture" update-index --skip-worktree
  fi
  echo "$fixture"
}

piled_fixture() {
  local fixture=$SCRATCH/refs-50000
  if [ ! -d "$fixture/.git" ]; then
    mkdir -p "$fixture"
    git -C "$fixture" init --quiet --initial-branch=main
    for n in 1 2 3; do
      echo "$n" >"$fixture/file.txt"
      git -C "$fixture" add file.txt
      GIT_AUTHOR_DATE="2026-01-0$n 12:00 +0000" GIT_COMMITTER_DATE="2026-01-0$n 12:00 +0000" \
        git -C "$fixture" -c user.name=perf -c user.email=perf@example.invalid commit --quiet -m "commit $n"
    done
    local tips
    tips=$(git -C "$fixture" rev-list main | tr '\n' ' ')
    awk -v tips="$tips" 'BEGIN {
      split(tips, t, " ")
      split("feature rebase fix chore spike", w, " ")
      for (i = 0; i < 40000; i++) printf "create refs/heads/team/%s-%d %s\n", w[i % 5 + 1], i, t[i % 3 + 1]
      for (i = 0; i < 10000; i++) printf "create refs/tags/v1.%d %s\n", i, t[i % 3 + 1]
    }' | git -C "$fixture" update-ref --stdin
    git -C "$fixture" pack-refs --all
  fi
  echo "$fixture"
}

with_refs_fixture() {
  (cd "$ROOT" && cargo run --release -q -p mergewright-fixtures --features heavy-tests \
    --example synthetic -- 100000 --commit-graph --refs 50000) | cut -f1
}

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

p95_of() { # field trace -> that frame field's p95, or nothing
  awk -v f="$1" '$1 == "frame" { for (i = 2; i <= NF; i++) { split($i, kv, "="); if (kv[1] == f && kv[2] != "-") v[++n] = kv[2] } }
    END { for (i = 2; i <= n; i++) { t = v[i]; for (j = i - 1; j >= 1 && v[j] > t; j--) v[j + 1] = v[j]; v[j + 1] = t }
          if (n) print v[int((95 * n + 99) / 100)] }' "$2"
}

load() { uptime | sed 's/.*load average[s]*: //'; }

# A writable copy per run: Mergewright writes .git/mergewright/ in what it opens.
copy_of() { # fixture dest
  cp -R "$1" "$2"
  chmod -R u+w "$2"
}

run_palette() { # name fixture-dir description group queries
  local name=$1 repo=$WORK/$1-repo trace=$WORK/$1-trace.txt status=0
  copy_of "$2" "$repo"
  local started_load
  started_load=$(load)
  MWR_PERF_TRACE=$trace python3 "$ROOT/scripts/perf-palette-pty.py" "$BIN" "$repo" 120 36 5 \
    "$4" "$5" >"$WORK/$name-pty.txt" 2>"$WORK/$name-pty.err" || status=$?

  local loop p95 check up95 upcheck
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
    }' "$trace")
  p95=$(echo "$loop" | grep -oE 'p95=[0-9]+' | cut -d= -f2 || true)
  if [ -n "$p95" ] && [ "$p95" -lt 16000 ]; then check="check loop_p95_under_16ms ok"; else check="check loop_p95_under_16ms FAIL"; fi
  # update alone: the palette's own per-key work, apart from drawing the
  # graph under it.
  up95=$(p95_of update_us "$trace")
  if [ -n "$up95" ] && [ "$up95" -lt 16000 ]; then upcheck="check update_p95_under_16ms ok p95=$up95"; else upcheck="check update_p95_under_16ms FAIL p95=${up95:-none}"; fi

  echo "## $name: $3; palette, real pty 120x36, release, Unicode."
  echo "# $(date -u +%Y-%m-%dT%H:%MZ) $(uname -sm); load at start $started_load; pty driver exit $status"
  echo "$loop"
  echo "$check"
  echo "$upcheck"
  sed -n 's/^SUMMARY //p' "$WORK/$name-pty.txt"
  "$ROOT/scripts/m8-trace-summary.sh" "$trace" "$WORK/$name-summary.txt" >/dev/null
  cat "$WORK/$name-summary.txt"
}

run_held_j() { # name fixture-dir: held `j` for 3 s at 60 Hz, no palette
  local name=$1 repo=$WORK/$1-j-repo trace=$WORK/$1-j-trace.txt status=0
  copy_of "$2" "$repo"
  local started_load d95 dcheck
  started_load=$(load)
  MWR_PERF_TRACE=$trace python3 "$ROOT/scripts/scroll-lag-pty.py" "$BIN" "$repo" 120 36 5 0.016 3 "$name" \
    >"$WORK/$name-j-pty.txt" 2>"$WORK/$name-j-pty.err" || status=$?
  d95=$(p95_of draw_us "$trace")
  if [ -n "$d95" ] && [ "$d95" -lt 16000 ]; then dcheck="check held_j_draw_p95_under_16ms ok p95=$d95"; else dcheck="check held_j_draw_p95_under_16ms FAIL p95=${d95:-none}"; fi
  echo "## $name: held j, no palette, real pty 120x36, release, Unicode."
  echo "# $(date -u +%Y-%m-%dT%H:%MZ) $(uname -sm); load at start $started_load; pty driver exit $status"
  echo "$dcheck"
  "$ROOT/scripts/m8-trace-summary.sh" "$trace" "$WORK/$name-j-summary.txt" >/dev/null
  cat "$WORK/$name-j-summary.txt"
}

OUT=$ROOT/docs/perf/m8-palette.txt
{
  echo "# m8/12, 12b, 12c: palette find on many refs and many paths, held j. scripts/perf-palette.sh $WHICH"
  # The refs cases' queries are the pty driver's defaults (12's).
  if runs piled; then
    piled=$(piled_fixture)
    echo
    run_palette piled "$piled" "40 000 branches + 10 000 tags on 3 commits" BRANCHES "4417,v1.99,-2024"
    echo
    run_held_j piled "$piled"
  fi
  if runs with-refs; then
    with_refs=$(with_refs_fixture)
    echo
    run_palette with-refs "$with_refs" "100 000 commits, 50 000 refs spread (Scale::with_refs)" \
      BRANCHES "4417,v1.99,-2024"
  fi
  if runs paths; then
    paths=$(paths_fixture)
    echo
    run_palette paths "$paths" "100 000 tracked paths in one commit" FILES "4417,m99/f,f12"
  fi
} >"$OUT"
cat "$OUT"
# Every check ok and every pty driver clean, read back off the file.
! grep -qE '^check .* FAIL|pty driver exit [1-9]' "$OUT"
