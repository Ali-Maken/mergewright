#!/bin/bash
cd "$(dirname "$0")/../.."
export MWR_TARGET=target MWR_FIXTURES_BIN=target/debug/mergewright-fixtures
run() { python3 .scratch/ease-pass/pty-ease27.py "$1" "$2" "$3" "$4" "$5" >/dev/null 2>&1; }
for s in 80x24 120x40 160x48 187x50 200x50; do
 ( run $s b-launch-to-first-conflict shell-landing - CONFLICTS
   run $s b-launch-to-first-conflict shell-pane-focused esc CONFLICTS
   run $s b-launch-to-first-conflict shell-pane-closed "esc esc" CONFLICTS ) &
 ( run $s a-stop-to-first-conflict app-landing - "▸ gone.txt"
   run $s a-stop-to-first-conflict app-banner esc "▸ gone.txt"
   run $s a-stop-to-first-conflict app-pane-closed "esc esc" "▸ gone.txt" ) &
 wait
done
