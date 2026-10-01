#!/usr/bin/env bash
# m7-power-tools/42: held `j` at his scale, both tiers, into a draining
# terminal (`held_key::` in crates/graft/tests/it/held_key.rs). Release
# build. Writes docs/perf/fix-scroll-held-key.txt (or $1): the HEAD, the
# machine and its load, the measured macOS pty capacity beside the model's
# 16 KiB pipe, then every `SUMMARY held_key` and `VERDICT held_key` line and
# each test's result. The full test output stays in a temp dir.
#
# Red until 46 (Unicode green after 43): exits 1 when any test failed.
set -u
cd "$(dirname "$0")/.."
OUT=${1:-docs/perf/fix-scroll-held-key.txt}
WORK=$(mktemp -d)
LOAD=$(uptime | sed "s/.*load averages*: //")
trap 'rm -rf "$WORK"' EXIT

# The real pty's buffer: non-blocking writes into a fresh pty until EAGAIN.
pty_capacity() {
  python3 - <<'PY' 2>&1
import os, pty, fcntl, errno
master, slave = pty.openpty()
fcntl.fcntl(slave, fcntl.F_SETFL, fcntl.fcntl(slave, fcntl.F_GETFL) | os.O_NONBLOCK)
total, chunk = 0, b"x" * 1024
while True:
    try:
        n = os.write(slave, chunk)
    except OSError as e:
        if e.errno in (errno.EAGAIN, errno.EWOULDBLOCK):
            break
        raise
    total += n
    if n == 0:
        break
print(total)
PY
}

cargo test --release --features heavy-tests --test it --no-run --quiet 2>"$WORK/build.err" || {
  echo "release build failed:" >&2
  tail -20 "$WORK/build.err" >&2
  exit 1
}

cargo test --release --features heavy-tests --test it -- held_key:: \
  --test-threads=1 --nocapture >"$WORK/run.out" 2>&1
status=$?

{
  echo "# fix-scroll-held-key (m7-power-tools/42): held j at his scale, release"
  echo "# HEAD $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- crates || echo ' (+ uncommitted changes under crates/)')"
  echo "# machine: $(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m), $(sysctl -n hw.ncpu 2>/dev/null) cpus, $(uname -sr)"
  echo "# load before: $LOAD"
  echo "# pipe model 16384 B; real macOS pty capacity (non-blocking write until EAGAIN): $(pty_capacity) B"
  grep -E '^(SUMMARY|VERDICT) held_key' "$WORK/run.out"
  grep -E '^test held_key::' "$WORK/run.out"
  grep -A1 'panicked at' "$WORK/run.out" | grep -vE 'panicked at|^--' | cut -c1-400 | sed 's/^/# panic: /'
  grep -E '^test result' "$WORK/run.out"
  echo "# exit $status"
} >"$OUT"

grep -E '^(SUMMARY|test held_key|test result)' "$OUT"
[ "$status" -eq 0 ] || exit 1
