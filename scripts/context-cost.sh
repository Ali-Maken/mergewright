#!/usr/bin/env bash
# Prints the byte and estimated-token cost of what a helper pays before doing
# any work: CLAUDE.md, plus (if given) the agent definition it runs as, plus
# any extra file named on the command line. Estimated tokens = bytes / 4
# (rough, chars-to-tokens; good enough to compare before/after a doc change).
#
# Usage:
#   scripts/context-cost.sh [agent-file] [extra-file ...]
#
# Always appends its result to docs/perf/context-cost.txt (never overwrites —
# each run adds a dated block, so a before/after pair stays in the file).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/docs/perf/context-cost.txt"
mkdir -p "$ROOT/docs/perf"

est_tokens() {
    # bytes / 4, integer
    awk -v b="$1" 'BEGIN { printf "%d", b / 4 }'
}

files=("$ROOT/CLAUDE.md")
for f in "$@"; do
    if [[ "$f" = /* ]]; then
        files+=("$f")
    else
        files+=("$ROOT/$f")
    fi
done

total_bytes=0
{
    echo "## context-cost run: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    for f in "${files[@]}"; do
        if [[ -f "$f" ]]; then
            b=$(wc -c < "$f" | tr -d ' ')
            t=$(est_tokens "$b")
            total_bytes=$((total_bytes + b))
            printf '%-70s %8d bytes  ~%7d tokens\n' "${f#"$ROOT"/}" "$b" "$t"
        else
            printf '%-70s MISSING\n' "${f#"$ROOT"/}"
        fi
    done
    total_tokens=$(est_tokens "$total_bytes")
    printf 'TOTAL %63d bytes  ~%7d tokens\n' "$total_bytes" "$total_tokens"
    echo
} | tee -a "$OUT"

echo "Appended to $OUT"
