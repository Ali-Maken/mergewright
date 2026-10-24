#!/bin/bash
# usage: shoot27.sh label scenario onscreen "keys"   -> five sizes
cd "$(dirname "$0")/../.."
export MWR_TARGET=target MWR_FIXTURES_BIN=target/debug/mergewright-fixtures
for s in 80x24 120x40 160x48 187x50 200x50; do
  python3 .scratch/ease-pass/pty-ease27.py $s "$2" "$1" "$4" "$3" >/dev/null 2>&1
done
