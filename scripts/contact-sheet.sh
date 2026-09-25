#!/usr/bin/env bash
# Assembles one milestone's contact sheet from its per-scenario captures
# (docs/shots/<milestone>-*.txt), per ADR-0037 decision 2.
#
# Usage: scripts/contact-sheet.sh [--captions-order] <milestone> [<shots-dir>]
#        scripts/contact-sheet.sh m4
#        scripts/contact-sheet.sh --captions-order m6 docs/shots/m6
#
# <shots-dir> defaults to docs/shots; M6 keeps its captures in docs/shots/m6.
#
# Reads:
#   <shots-dir>/<milestone>-*.txt                    the captures, in order
#   <shots-dir>/<milestone>-contact-sheet-captions.tsv   filename<TAB>caption
#     one line per capture: what right looks like, in the user's own terms.
#     Missing entries print as NOT RECORDED rather than being invented.
#
# Order:
#   default           the milestone's own captures by name (ticket order: the
#                     numeric prefix), then any caption naming a file outside
#                     the milestone's prefix, in the captions file's order.
#   --captions-order  every capture the captions file names, in its order —
#                     so fix tickets' captures (`fix-*`, M6) sit in ticket
#                     order among the milestone's own — then any
#                     <milestone>-*.txt it does not name, NOT RECORDED.
#   A name reaching outside <shots-dir> (`../m5-…`) is an earlier milestone's
#   capture this one changed, and its heading says so.
#
# Writes:
#   <shots-dir>/<milestone>-contact-sheet.txt when the sheet fits in 180 000 B
#   (CLAUDE.md "Captures stay small"); otherwise parts
#   <shots-dir>/<milestone>-contact-sheet-<n>.txt, each at most 180 000 B
#   (headroom under the 200 KB cap), split between captures, never inside
#   one. A run removes the sheet files
#   an earlier run wrote (whole or in parts) first, so none goes stale.
#
# The captions file is hand-authored per milestone (by that milestone's
# acceptance group), sourced in this order of preference: 1) TESTING.md's
# checklist rows for the scenario, 2) the capture file's own header line,
# 3) the walk's scenario name. This script only assembles; it does not
# invent captions.
set -euo pipefail

# The hard cap is 200 KB read the strict way (1000, not 1024; CLAUDE.md,
# "Captures stay small"). Parts split at 180 000 B so each keeps 20 KB of
# headroom: a retaken capture that grows a little must not push a part over.
max_bytes=$((180 * 1000))

captions_order=false
if [ "${1:-}" = "--captions-order" ]; then
  captions_order=true
  shift
fi

if [ $# -lt 1 ] || [ $# -gt 2 ]; then
  echo "usage: $0 [--captions-order] <milestone-prefix> [<shots-dir>]   e.g. m3, m4; --captions-order m6 docs/shots/m6" >&2
  exit 1
fi

milestone="$1"
shots_dir="${2:-docs/shots}"
captions_file="$shots_dir/${milestone}-contact-sheet-captions.tsv"
sheet_stem="${milestone}-contact-sheet"

if [ ! -d "$shots_dir" ]; then
  echo "no $shots_dir directory — run from the repo root" >&2
  exit 1
fi

# The milestone's own captures by name. Ticket order: the numeric prefix
# after "<milestone>-" sorts first under a plain C-locale sort (digits <
# letters in ASCII), then non-numbered captures fall in after,
# alphabetically. Excludes the sheet (whole or in parts) and the captions.
own=()
while IFS= read -r name; do
  own+=("$name")
done < <(
  find "$shots_dir" -maxdepth 1 -name "${milestone}-*.txt" \
    ! -name "${sheet_stem}.txt" \
    ! -name "${sheet_stem}-*.txt" \
    -exec basename {} \; | LC_ALL=C sort
)

captioned=()
if [ -f "$captions_file" ]; then
  while IFS=$'\t' read -r name _; do
    captioned+=("$name")
  done < "$captions_file"
fi

in_list() {
  local wanted="$1"
  shift
  local item
  for item in "$@"; do
    [ "$item" = "$wanted" ] && return 0
  done
  return 1
}

# `${a[@]+"${a[@]}"}`: an empty array under `set -u` in macOS's bash 3.2.
captures=()
if $captions_order; then
  captures=(${captioned[@]+"${captioned[@]}"})
  for name in ${own[@]+"${own[@]}"}; do
    in_list "$name" ${captioned[@]+"${captioned[@]}"} || captures+=("$name")
  done
else
  captures=(${own[@]+"${own[@]}"})
  for name in ${captioned[@]+"${captioned[@]}"}; do
    case "$name" in
      "${milestone}-"*) ;;
      *) captures+=("$name") ;;
    esac
  done
fi

if [ "${#captures[@]}" -eq 0 ]; then
  echo "no captures matching $shots_dir/${milestone}-*.txt" >&2
  exit 1
fi

caption_for() {
  local name="$1"
  if [ -f "$captions_file" ]; then
    local line
    line=$(awk -F'\t' -v n="$name" '$1 == n { print substr($0, index($0, "\t") + 1); exit }' "$captions_file")
    if [ -n "$line" ]; then
      printf '%s' "$line"
      return
    fi
  fi
  printf 'NOT RECORDED — judge against the %s ticket or walk scenario named in this capture.' "$milestone"
}

# The milestone's promise (PROMPT.md / CLAUDE.md's "Running work" list, or
# TESTING.md's own wording for the milestone's acceptance ticket). Add a
# case here when a new milestone's sheet is generated for the first time.
case "$milestone" in
  m3) promise="you cannot lose work — every write is journaled with a backup and comes back byte for byte on u (TESTING.md \"Working tree, undo and stash\")." ;;
  m4) promise="you never leave Graft to fetch, pull or push — not for a password, a rejection or a hang (TESTING.md \"Remotes\")." ;;
  m5) promise="you see the result before you confirm — the ghost the preview draws is what git then does, and u takes it back (TESTING.md \"History\")." ;;
  m6) promise="a real conflict is resolvable without leaving the app — each shape a merge, rebase or stash pop stops on is taken, staged and continued by keys, and the bytes are what the command line's own checkout gives (TESTING.md \"Conflicts walk\", \"Conflict journeys\")." ;;
  *) promise="NOT RECORDED — add this milestone's promise to scripts/contact-sheet.sh." ;;
esac

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# One file per capture: its heading, then the capture.
not_recorded_count=0
index=0
for name in "${captures[@]}"; do
  if [ ! -f "$shots_dir/$name" ]; then
    echo "$shots_dir/$name is named but does not exist" >&2
    exit 1
  fi
  caption=$(caption_for "$name")
  if [[ "$caption" == NOT\ RECORDED* ]]; then
    not_recorded_count=$((not_recorded_count + 1))
  fi
  label=""
  case "$name" in
    ../*) label="   (an earlier milestone's capture, changed by ${milestone})" ;;
  esac
  index=$((index + 1))
  {
    echo "================================================================================"
    echo "$shots_dir/$name$label"
    echo "what right looks like: $caption"
    echo "================================================================================"
    echo
    cat "$shots_dir/$name"
    echo
  } > "$work/section-$(printf '%04d' "$index")"
done

header() {
  local part="$1" parts="$2"
  echo "# ${milestone} contact sheet"
  if [ "$parts" -gt 1 ]; then
    echo "# Part $part of $parts (${sheet_stem}-1.txt … ${sheet_stem}-$parts.txt), each at most 180 000 B."
  fi
  echo "#"
  echo "# Promise: $promise"
  echo "#"
  echo "# Generated by scripts/contact-sheet.sh — do not hand-edit; edit"
  echo "# ${captions_file##*/} and re-run instead."
  echo "#"
  echo "# ${#captures[@]} captures in all, in ticket order."
  echo "#"
  echo "# These are character-grid captures (pyte/TestBackend), not pixels:"
  echo "# colour is never verifiable from this file. Judge colour only in a"
  echo "# real terminal, never from a capture (CLAUDE.md rule 5)."
  echo "#"
  echo "# Scroll once: each capture is delimited by a heading naming its"
  echo "# file (docs/shots/<name>) and what right looks like for it."
  echo
}

# Split between captures: a part takes sections while the header (sized
# for the most parts there could be) plus its sections stay under the cap.
header_bytes=$(header 9999 9999 | wc -c)
part=1
part_bytes=$header_bytes
: > "$work/part-1"
for section in "$work"/section-*; do
  bytes=$(wc -c < "$section")
  if [ $((header_bytes + bytes)) -gt "$max_bytes" ]; then
    echo "${section##*/} alone is $bytes bytes: no part can hold it under $max_bytes" >&2
    exit 1
  fi
  if [ $((part_bytes + bytes)) -gt "$max_bytes" ]; then
    part=$((part + 1))
    part_bytes=$header_bytes
    : > "$work/part-$part"
  fi
  echo "$section" >> "$work/part-$part"
  part_bytes=$((part_bytes + bytes))
done
parts=$part

# Remove what an earlier run wrote, whole or in parts, then write.
find "$shots_dir" -maxdepth 1 \( -name "${sheet_stem}.txt" -o -name "${sheet_stem}-[0-9]*.txt" \) -delete
written=()
for part in $(seq 1 "$parts"); do
  if [ "$parts" -eq 1 ]; then
    out_file="$shots_dir/${sheet_stem}.txt"
  else
    out_file="$shots_dir/${sheet_stem}-$part.txt"
  fi
  {
    header "$part" "$parts"
    while IFS= read -r section; do
      cat "$section"
    done < "$work/part-$part"
  } > "$out_file"
  written+=("$out_file ($(wc -c < "$out_file" | tr -d ' ') bytes)")
done

echo "wrote ${#captures[@]} captures, $not_recorded_count without a recorded caption, in $parts part(s):"
for line in "${written[@]}"; do
  echo "  $line"
done
