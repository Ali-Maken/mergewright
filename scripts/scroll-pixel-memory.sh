#!/usr/bin/env bash
# m8 04 (G-pixel-mem) step 1: where the Pixel tier's memory goes while `j` is
# held on MONOREPO, measured before any code change.
#
#   scripts/scroll-pixel-memory.sh [rev]      (default HEAD)
#
# Exports <rev> into a scratch tree (git archive; the worktree is not touched),
# applies scripts/scroll-pixel-memory.patch — a counting allocator and one line
# per pixel frame to TRIB_MEM_STATS, measurement only, never committed to the
# source — and builds it --release under target/mem-instr. Then runs
# scripts/scroll-lag-pty.py on a copy of the team-17700 fixture (MONOREPO),
# 5 s load, `j` every 16 ms for 3 s, reader free, for:
#   kitty deflate yes at the user's 17x37 cell, and at 9x19 (a narrower graph
#   in pixels) and 24x52 (wider) — the strip's pixel width follows the cell,
#   not the terminal's columns (a first run at 120/187/280 columns drew
#   850 px strips every time); kitty deflate no; iTerm2 (renderer = "pixel"
#   in a scratch config: detection settles iTerm2 with truecolor on Unicode,
#   detect/tier.rs). All at 187x50.
# Appends SUMMARY lines to docs/perf/scroll-pixel-memory-budget.txt:
#   per layer (backdrop, emphasis, composite): entries and KiB resident at the
#   end, and rasterisations during the hold; Kitty uploads and the KiB of
#   frames they keep alive; iTerm2 bands; large allocations (> 127 KiB, macOS
#   malloc's large class) per held second; vmmap `Malloc Large (empty)`
#   resident; Tributary RSS. Raw stats under target/mem-instr/runs/.
#
# Needs the fixture under target/fixtures (any heavy `held_key::` test builds it).
set -uo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT="$ROOT/docs/perf/scroll-pixel-memory-budget.txt"
. "$ROOT/scripts/lib/machine-state.sh"
machine_state_begin "$OUT"
load() { echo "SUMMARY load $1 $(uptime | sed 's/.*load averages*: //') (other builds on this machine: a number under load is recorded, not judged)" | tee -a "$OUT"; }
REV=${1:-HEAD}
WORK="$ROOT/target/mem-instr"
SRC="$WORK/src"
RUNS="$WORK/runs"
mkdir -p "$RUNS"
HASH=$(git -C "$ROOT" rev-parse --short "$REV") || exit 1
rm -rf "$SRC" && mkdir -p "$SRC"
git -C "$ROOT" archive "$REV" | tar -x -C "$SRC" || exit 1
( cd "$SRC" && patch -s -p1 <"$ROOT/scripts/scroll-pixel-memory.patch" ) || {
  echo "SUMMARY mem rev=$HASH patch=FAILED" | tee -a "$OUT"; exit 1; }
export CARGO_TARGET_DIR="$WORK/target"
( cd "$SRC" && cargo build --release -q -p graft --bin trib ) >"$WORK/build.log" 2>&1 || {
  echo "SUMMARY mem rev=$HASH build=FAILED (target/mem-instr/build.log)" | tee -a "$OUT"; exit 1; }
BIN="$CARGO_TARGET_DIR/release/trib"
FIXTURE=$(ls -dt "$ROOT"/target/fixtures/team-17700-* 2>/dev/null | head -1)
[ -n "$FIXTURE" ] || { echo "no MONOREPO fixture under target/fixtures" >&2; exit 1; }
REPO="$WORK/repo"
WAIT=5
HOLD=3

CONFIG="$WORK/config"
mkdir -p "$CONFIG/tributary" && printf '[graph]\nrenderer = "pixel"\n' >"$CONFIG/tributary/config.toml"

run() { # label profile deflate cell [config dir]
  local label=$1 profile=$2 deflate=$3 cell=$4 xdg=${5:-$WORK/no-config} cols=187
  rm -rf "$REPO" && cp -R "$FIXTURE" "$REPO" && chmod -R u+w "$REPO"
  rm -f "$RUNS/$label.stats"
  local line
  line=$(env PROFILE="$profile" DEFLATE="$deflate" CELL="$cell" XDG_CONFIG_HOME="$xdg" TRIB_MEM_STATS="$RUNS/$label.stats" \
    MEM_OUT="$RUNS/$label.vmmap" python3 "$ROOT/scripts/scroll-lag-pty.py" \
    "$BIN" "$REPO" "$cols" 50 "$WAIT" 0.016 "$HOLD" "$label" 2>"$RUNS/$label.err" | grep '^SUMMARY')
  python3 - "$label" "$cell" "$RUNS/$label.stats" "$RUNS/$label.vmmap" "$line" "$WAIT" "$HOLD" "$HASH" <<'PY' | tee -a "$OUT"
import re, sys
label, cell, stats, vmmap, pty, wait, hold, rev = sys.argv[1:9]
wait, hold = float(wait) * 1000, float(hold)
def fields(line):
    return {k: v for k, v in re.findall(r"(\w+)=(\S+)", line)}
try:
    lines = [fields(l) for l in open(stats) if l.startswith("t_ms=")]
except FileNotFoundError:
    lines = []
pf = fields(pty)
if not lines:
    print(f"SUMMARY mem rev={rev} label={label} cell={cell} frames=0 (no pixel frame: tier not Pixel) rss_mb={pf.get('rss_mb','?')} broken={pf.get('broken','?')}")
    sys.exit(0)
before = [l for l in lines if int(l["t_ms"]) <= wait] or lines[:1]
start, end = before[-1], lines[-1]
held = lambda k: int(end[k]) - int(start[k])
empty = "?"
try:
    for l in open(vmmap):
        m = re.match(r"\s*Malloc Large \(empty\)\s+\S+\s+(\S+)", l)
        if m:
            empty = m.group(1)
except FileNotFoundError:
    pass
peak_mb = max(int(l["resident_kb"]) for l in lines) >> 10
wins = sum(int(l.get("minimum_wins", 0)) for l in lines)
print(
    f"SUMMARY mem rev={rev} label={label} cell={cell} strip_px_w={end['strip_px_w']}"
    f" backdrop={end['backdrop_n']}/{int(end['backdrop_kb'])>>10}MB rast_held={held('backdrop_rast')}"
    f"(new={held('rast_new')},again={held('rast_again')},rekeyed={held('rast_rekeyed')}:layout={held('rekeyed_layout')},alpha={held('rekeyed_alpha')})"
    f" emphasis={end['emphasis_n']}/{int(end['emphasis_kb'])>>10}MB rast_held={held('emphasis_rast')}"
    f" composite={end['composite_n']}/{int(end['composite_kb'])>>10}MB rast_held={held('composite_rast')}"
    f" cache_mb={int(end['resident_kb'])>>10} cache_peak_mb={peak_mb} budget_mb={int(end['budget_kb'])>>10}"
    f" min_strips={end['min_strips']} minimum_wins_frames={wins}/{len(lines)}"
    f" buffers_allocated={end['buffers_allocated']}(held+{held('buffers_allocated')})"
    f" uploads={end['uploads_n']} scratch_mb={int(end['scratch_kb'])>>10}"
    f" bands={end['bands_n']}/seq{int(end['bands_seq_kb'])>>10}MB"
    f" large_allocs_per_held_s={held('large_allocs')/hold:.0f} large_mb_per_held_s={held('large_mb')/hold:.0f}"
    f" malloc_large_empty_resident={empty} rss_mb={pf.get('rss_mb','?')}"
    f" transmits={pf.get('transmits','-')} deflate={pf.get('deflate','-')} broken={pf.get('broken','?')}"
)
PY
}

load before
run kitty-yes kitty yes 17x37
run kitty-yes-narrow kitty yes 9x19
run kitty-yes-wide kitty yes 24x52
run kitty-no kitty no 17x37
run iterm2 iterm2 - 17x37 "$CONFIG"
load after
