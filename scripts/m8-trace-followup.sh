#!/usr/bin/env bash
# m8-polish-and-ship/05 (G-trace follow-up): the builder's MWR_PERF_TRACE run
# at the shape of his Ghostty run (docs/perf/m8-trace-2026-10-08-ghostty.txt:
# Pixel tier over Kitty graphics, 222x70, his 17x37 cell), through a real pty
# that answers the probe like Ghostty (scroll-lag-pty.py PROFILE=kitty,
# DEFLATE=yes). 50 `j` at 33 ms after a 5 s load, then quiet.
#
#   scripts/m8-trace-followup.sh <label> [repo]
#
# repo defaults to the newest MONOREPO fixture under $CARGO_TARGET_DIR/fixtures
# (built by any heavy `held_key::` test). The repo is copied; never his.
# Prints `<label>` then the pty's SUMMARY and m8-trace-summary.sh's lines;
# the caller appends them to docs/perf/m8-trace-followup.txt.
#
# Needs the release binary (`cargo build --release -p mergewright`).
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
TARGET=${CARGO_TARGET_DIR:-$ROOT/target}
LABEL=${1:?usage: m8-trace-followup.sh <label> [repo]}
SRC=${2:-$(ls -dt "$TARGET"/fixtures/team-17700-* 2>/dev/null | head -1)}
[ -n "$SRC" ] || { echo "no repo given and no MONOREPO fixture under $TARGET/fixtures" >&2; exit 1; }
BIN=$TARGET/release/mwr
[ -x "$BIN" ] || { echo "no release binary at $BIN" >&2; exit 1; }
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
REPO=$WORK/repo
# Mergewright writes .git/mergewright/ in what it opens; fixtures are sealed read-only.
cp -R "$SRC" "$REPO" && chmod -R u+w "$REPO"
TRACE=$WORK/trace.txt

line=$(PROFILE=kitty DEFLATE=yes SCROLL_LAG_DUMP=$WORK/stream.bin MWR_PERF_TRACE=$TRACE python3 "$ROOT/scripts/scroll-lag-pty.py" \
  "$BIN" "$REPO" 222 70 5 0.033 1.65 m8-trace-followup 2>"$WORK/pty.err" | grep SUMMARY || true)
[ -n "$line" ] || { echo "the pty run printed no SUMMARY:" >&2; cat "$WORK/pty.err" >&2; exit 1; }

echo "# $LABEL: $(date -u +%Y-%m-%dT%H:%MZ) $(uname -sm) $(git -C "$ROOT" rev-parse --short HEAD)"
echo "${line#*SUMMARY }"
"$ROOT/scripts/m8-trace-summary.sh" "$TRACE" "$WORK/summary.txt" >/dev/null
# TRACE_KEEP=<path>: keep the raw trace (per-frame lines) for reading the
# over-bound frames; never committed.
if [ -n "${TRACE_KEEP:-}" ]; then cp "$TRACE" "$TRACE_KEEP"; fi
cat "$WORK/summary.txt"
# Frames after the last frame that drew a key, and what each carried: the
# tail's shape, which the summary's one tail_ms cannot show.
awk '$1 == "frame" { n++; for (i = 2; i <= NF; i++) { split($i, kv, "="); f[n, kv[1]] = kv[2] }
       if ($0 ~ / motions=[1-9]/) last = n }
     END { printf "after_last_motion frames=%d", n - last
           for (k = last + 1; k <= n; k++) printf " [bytes=%s draw_us=%s]", f[k, "bytes"], f[k, "draw_us"]
           print "" }' "$TRACE"
# Where the bytes went: Kitty uploads by layer (an id with bit 31 set is an
# emphasis image, graft-term/src/kitty/mod.rs:47), with the most common
# upload sizes in pixels. Read from the raw stream, which is deleted with $WORK.
python3 -I - "$WORK/stream.bin" <<'PY'
import collections, re, sys
d = re.sub(rb"\n@@READ -?[0-9.]+ ms\n", b"", open(sys.argv[1], "rb").read())
count, size, shapes, layer = collections.Counter(), collections.Counter(), collections.Counter(), None
for m in re.finditer(rb"\x1b_G([^;\x1b]*)(?:;[^\x1b]*)?\x1b\\", d):
    kv = dict(p.split("=", 1) for p in m.group(1).decode(errors="replace").split(",") if "=" in p)
    if kv.get("a") == "t":
        layer = "emphasis" if int(kv["i"]) >= 1 << 31 else "strip"
        count[layer] += 1
        shapes[(layer, kv.get("s"), kv.get("v"))] += 1
    elif kv.get("a") == "p":
        layer = None
        count["place"] += 1
    elif "a" in kv:
        layer = None
    if layer:  # a transmit and its continuation chunks
        size[layer] += len(m.group(0))
total = len(d)
print("uploads " + " ".join(f"{k}={count[k]}/{size[k]}B({100 * size[k] // max(total, 1)}%)" for k in ("strip", "emphasis"))
      + f" placements={count['place']} stream={total}B")
print("upload_shapes " + " ".join(f"{l}:{s}x{v}={n}" for (l, s, v), n in shapes.most_common(6)))
PY
