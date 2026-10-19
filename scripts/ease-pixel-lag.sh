#!/usr/bin/env bash
# ease-pass/13 (Q4 a): what colouring the pixel strips by role, and then the
# tall row, cost a held `j` at the Pixel tier. Writes docs/perf/ease-pixel.txt
# (or $OUT). Lines a reader needs first start with SUMMARY. Numbers only: no
# cut, fallback or switch hangs on them (the user decides, with Q4's options).
#
# Three release binaries, each built from `git archive` of its rev into its
# own CARGO_TARGET_DIR under $EASE_PIXEL_WORK (default $TMPDIR/ease-pixel),
# never the worktree's target/:
#   base   60e07871  before ease-pass/13 (strips one colour per branch)
#   roles  bcc58b40  13 commit 1: lanes stroked and coloured by role
#   tall   $TALL_REV (default HEAD): roles + the cursor row's second line
#
# Each run: scripts/scroll-lag-pty.py, a real pty, the probe answered like
# Ghostty (PROFILE=kitty, DEFLATE=yes), `j` at 30 Hz for 3 s, with
# MWR_PERF_TRACE on. Sizes 120x40 and 200x50. Walks:
#   open     MONOREPO team fixture (TeamSpec::MONOREPO, 17 700 commits; the
#            fixture 41-scroll-team-fixture.sh measures), keys from 0.3 s
#   settled  the same, keys from 5 s
#   focus    a two-branch history whose rows alternate branch every 5 rows
#            (committer dates in blocks of 5), keys from 5 s: every 5th `j`
#            moves the focus onto the other branch, so roles re-key strips
#            (ADR-0053's "a j re-sends no strip" no longer holds there).
#
# Per binary x size x walk, from the trace's frames that applied a key
# (motions>0), nearest rank: input_age_us p50/p95/p99, draw_us p95, bytes
# p95; tail_ms from the whole trace (scripts/m8-trace-summary.sh: last frame
# minus last frame that applied a key) and pty_tail_ms (last key -> last
# byte, scroll-lag-pty.py). Then deltas against base.
set -uo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT=${OUT:-$ROOT/docs/perf/ease-pixel.txt}
WORK=${EASE_PIXEL_WORK:-${TMPDIR:-/tmp}/ease-pixel}
mkdir -p "$WORK"
RUN=$(mktemp -d)
trap 'rm -rf "$RUN"' EXIT
. "$ROOT/scripts/lib/machine-state.sh"  # ticket 49: battery / Low Power Mode guard
machine_state_begin "$OUT"

BASE_REV=60e07871
ROLES_REV=bcc58b40
TALL_REV=${TALL_REV:-HEAD}

fail() { echo "SUMMARY ease-pixel FAILED $*" | tee -a "$OUT" >&2; exit 1; }

{
  echo "# ease-pass/13 pixel lag — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# $(sysctl -n machdep.cpu.brand_string 2>/dev/null), $(sysctl -n hw.ncpu 2>/dev/null) cpus; $(uptime | sed 's/.*load/load/')"
  echo "# revs: base=$(git -C "$ROOT" rev-parse --short "$BASE_REV") roles=$(git -C "$ROOT" rev-parse --short "$ROLES_REV") tall=$(git -C "$ROOT" rev-parse --short "$TALL_REV")"
  echo "# pty: PROFILE=kitty DEFLATE=yes (Ghostty's answers, cell 17x37), j every 33 ms for 3 s"
} >"$OUT"

# ── binaries ────────────────────────────────────────────────────────────────
build() { # name rev -> $WORK/target-<name>/release/mwr
  local name=$1 hash src
  hash=$(git -C "$ROOT" rev-parse "$2") || fail "no rev $2"
  src=$WORK/src-$name
  # Re-extracted only when the rev changed, so cargo's fingerprints hold.
  if [ "$(cat "$src/.ease-pixel-rev" 2>/dev/null)" != "$hash" ]; then
    rm -rf "$src" && mkdir -p "$src"
    git -C "$ROOT" archive "$hash" | tar -x -C "$src" || fail "archive $name"
    echo "$hash" >"$src/.ease-pixel-rev"
  fi
  ( cd "$src" && CARGO_TARGET_DIR="$WORK/target-$name" cargo build --release -q -p mergewright --bin mwr ) \
    >"$RUN/build-$name.txt" 2>&1 || { tail -20 "$RUN/build-$name.txt" >&2; fail "build $name"; }
}
for name in base roles tall; do
  case $name in base) rev=$BASE_REV ;; roles) rev=$ROLES_REV ;; tall) rev=$TALL_REV ;; esac
  build "$name" "$rev"
done

# ── fixtures ────────────────────────────────────────────────────────────────
# MONOREPO from the fixture cache (built on first use, then reused).
export MWR_FIXTURES_DIR="$ROOT/target/fixtures"
TEAM=$( cd "$ROOT" && CARGO_TARGET_DIR="$ROOT/target" cargo run -q -p mergewright-fixtures --example team_path 2>"$RUN/fixture.txt" ) \
  || { tail -20 "$RUN/fixture.txt" >&2; fail "no MONOREPO fixture"; }
[ -d "$TEAM/.git" ] || [ -f "$TEAM/HEAD" ] || fail "MONOREPO fixture path is not a repository"

FOCUS=$WORK/focus-fixture
if [ ! -d "$FOCUS/.git" ]; then
  rm -rf "$FOCUS" && mkdir -p "$FOCUS"
  git -C "$FOCUS" init -q -b main || fail "focus init"
  # 400 commits after a root: blocks of 5 on `main`, then 5 on `other`, one
  # minute apart, so the date-ordered graph alternates branch every 5 rows.
  python3 - <<'EOF' | git -C "$FOCUS" fast-import --quiet || fail "focus fast-import"
import sys
out = sys.stdout
t = 1_700_000_000
def commit(ref, mark, parent, n):
    global t
    t += 60
    msg = f"commit {n}\n".encode()
    body = f"{ref} {n}\n".encode()
    out.write(f"commit {ref}\nmark :{mark}\ncommitter Walk <walk@example.invalid> {t} +0000\n")
    out.write(f"data {len(msg)}\n{msg.decode()}")
    if parent:
        out.write(f"from :{parent}\n")
    out.write(f"M 644 inline file\ndata {len(body)}\n{body.decode()}\n")
commit("refs/heads/main", 1, None, 0)
tips = {"refs/heads/main": 1, "refs/heads/other": 1}
for n in range(1, 401):
    ref = "refs/heads/main" if ((n - 1) // 5) % 2 == 0 else "refs/heads/other"
    commit(ref, n + 1, tips[ref], n)
    tips[ref] = n + 1
EOF
  git -C "$FOCUS" reset -q --hard main || fail "focus checkout"
fi

# ── runs ────────────────────────────────────────────────────────────────────
pct() { # summary-file field -> "p50 p95 p99" (nearest rank, m8-trace-summary)
  awk -v f="$2" '$1 == f { for (i = 2; i <= NF; i++) { split($i, kv, "="); v[kv[1]] = kv[2] } }
    END { printf "%s %s %s", (v["p50"] == "" ? "-" : v["p50"]), (v["p95"] == "" ? "-" : v["p95"]), (v["p99"] == "" ? "-" : v["p99"]) }' "$1"
}
ROWS=$RUN/rows.tsv
: >"$ROWS"
for name in base roles tall; do
  bin=$WORK/target-$name/release/mwr
  for size in "120 40" "200 50"; do
    set -- $size
    cols=$1 lines=$2
    for walk in open settled focus; do
      case $walk in open) src=$TEAM wait=0.3 ;; settled) src=$TEAM wait=5 ;; focus) src=$FOCUS wait=5 ;; esac
      repo=$RUN/repo trace=$RUN/trace.txt
      # Mergewright writes .git/mergewright/ in what it opens; the cache is sealed read-only.
      rm -rf "$repo" "$trace" && cp -R "$src" "$repo" && chmod -R u+w "$repo" || fail "copy $walk"
      pty=$(MWR_PERF_TRACE=$trace PROFILE=kitty DEFLATE=yes python3 "$ROOT/scripts/scroll-lag-pty.py" \
        "$bin" "$repo" "$cols" "$lines" "$wait" 0.033 3 "$name-$walk" 2>"$RUN/pty.err" | grep '^SUMMARY')
      [ -n "$pty" ] || { tail -5 "$RUN/pty.err" >&2; fail "pty $name $walk ${cols}x$lines"; }
      [ -s "$trace" ] || fail "no trace $name $walk ${cols}x$lines"
      awk '$1 == "header" || ($1 == "frame" && / motions=[1-9]/)' "$trace" >"$RUN/keyed.txt"
      "$ROOT/scripts/m8-trace-summary.sh" "$RUN/keyed.txt" "$RUN/keyed.sum" >/dev/null
      "$ROOT/scripts/m8-trace-summary.sh" "$trace" "$RUN/all.sum" >/dev/null
      read -r a50 a95 a99 <<<"$(pct "$RUN/keyed.sum" input_age_us)"
      read -r _ d95 _ <<<"$(pct "$RUN/keyed.sum" draw_us)"
      read -r _ b95 _ <<<"$(pct "$RUN/keyed.sum" bytes)"
      tail_ms=$(awk -F= '$1 == "tail_ms" { split($2, t, " "); print t[1] }' "$RUN/all.sum")
      keyed=$(grep -oE '^frames=[0-9]+' "$RUN/keyed.sum" | cut -d= -f2)
      field() { echo "$pty" | grep -oE " $1=[^ ]+" | cut -d= -f2; }
      echo "SUMMARY bin=$name size=${cols}x$lines walk=$walk input_age_us_p50=$a50 input_age_us_p95=$a95 input_age_us_p99=$a99 draw_us_p95=$d95 bytes_p95=$b95 tail_ms=$tail_ms pty_tail_ms=$(field tail_ms) keys=$(field keys) keyed_frames=$keyed transmits=$(field transmits) broken=$(field broken)" >>"$OUT"
      printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$name" "${cols}x$lines" "$walk" "$a50" "$a95" "$a99" "$d95" "$b95" "$tail_ms" >>"$ROWS"
    done
  done
done

# ── deltas against base ─────────────────────────────────────────────────────
awk -F'\t' '
  { key = $2 SUBSEP $3; for (i = 4; i <= 9; i++) v[$1, key, i] = $i; if (!(key in seen)) { seen[key] = 1; order[++n] = key } }
  function d(a, b) { return (a == "-" || b == "-") ? "-" : sprintf("%+d", a - b) }
  END {
    split("input_age_us_p50 input_age_us_p95 input_age_us_p99 draw_us_p95 bytes_p95 tail_ms", names, " ")
    for (b = 1; b <= 2; b++) {
      bin = (b == 1) ? "roles" : "tall"
      for (k = 1; k <= n; k++) {
        split(order[k], sw, SUBSEP)
        line = "SUMMARY delta bin=" bin "-base size=" sw[1] " walk=" sw[2]
        for (i = 4; i <= 9; i++) line = line " " names[i - 3] "=" d(v[bin, order[k], i], v["base", order[k], i])
        print line
      }
    }
  }' "$ROWS" >>"$OUT"

cat >>"$OUT" <<'EOF'
# In Ghostty, by you (what no pty answers: the terminal's own parse and paint).
# Copy your work repo first (Mergewright writes .git/mergewright/ in what it opens):
#   cp -R <your work repo> /tmp/mwr-ease-copy
#   MWR_PERF_TRACE=/tmp/ease-pixel.trace mwr /tmp/mwr-ease-copy    then hold j 3 s, then q
#   scripts/m8-trace-summary.sh /tmp/ease-pixel.trace docs/perf/ease-pixel-ghostty.txt
EOF
echo "# done $(date -u +%H:%M:%SZ)" >>"$OUT"
grep '^SUMMARY' "$OUT"
