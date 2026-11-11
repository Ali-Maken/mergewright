# pp-12 identity rename: what moved, for the user's eye

Colour is unverified (a capture holds no colour). Lane-slot lines are the only colour evidence: a changed slot is a changed palette entry.

`SUMMARY compared=288 equal=274 reordered=1 lane-slot-files=5 other=8 png_changed=2 png_size_changed=0 added=0 removed=0 lane-slot changes=18`

## Reordered captures (same lines, other order)

Cause site: oid tie-break in `mergewright-git/src/history/node.rs` (equal commit time, ties on oid) or `mergewright/src/map/walk.rs:92` (branch order). Fixture commits sharing a timestamp are the inputs.

- `docs/shots/m6/set-aside-preview-80x24.txt` line 14
  - was: `                      after rebase`
  - now: `▸ ●                   ours                                           Maker   6m`

## Snapshot files whose lane-slot letters moved (approximate class)

- `crates/mergewright-ui/tests/snapshots/graph_column__criss-cross-merge.snap` line 9
  - was: `│ ◉─│─╮     c.fjdjj...  #  beta: merge alpha`
  - now: `│ ◉─│─╮     c.ffdff...  #  beta: merge alpha`
- `crates/mergewright-ui/tests/snapshots/graph_column__ref-states.snap` line 7
  - was: `  ●       ..l.....  #  Archived work`
  - now: `  ●       ..c.....  #  Archived work`
- `crates/mergewright-ui/tests/snapshots/graph_column__stash.snap` line 7
  - was: `  ■─┬─╮   ..ehhcc.  #  On main: wip: with an untracked file`
  - now: `  ■─┬─╮   ..geeee.  #  On main: wip: with an untracked file`
- `crates/mergewright-ui/tests/snapshots/lanes_without_hue__criss-cross-merge-16.snap` line 35
  - was: `5.68788...`
  - now: `5.66766...`
- `crates/mergewright-ui/tests/snapshots/lanes_without_hue__criss-cross-merge-256.snap` line 35
  - was: `c.fjdjj...`
  - now: `c.ffdff...`

## Lane-slot changes (probe, by corpus fixture row)

Cause site: `BranchIdentity::of_short_sha` (`mergewright-graph/src/identity.rs`) for an unnamed lane; named lanes hash the name and cannot move.

- `criss-cross-merge` row `alpha: merge beta#1`: `0:2,1:5,2:3,3:9,4:0` -> `0:2,1:5,2:3,3:5,4:7`
- `criss-cross-merge` row `alpha: work#1`: `0:2,1:5,2:3,3:9` -> `0:2,1:5,2:3,3:5`
- `criss-cross-merge` row `beta: merge alpha#1`: `0:2,1:5,2:3,3:9` -> `0:2,1:5,2:3,3:5`
- `criss-cross-merge` row `beta: work#1`: `0:2,1:5,2:3,3:9,4:0` -> `0:2,1:5,2:3,3:5,4:7`
- `ref-states` row `Add the base#1`: `0:2,1:11` -> `0:2,1:2`
- `ref-states` row `Advance main again#1`: `0:2,1:11,2:0,3:7` -> `0:2,1:2,2:0,3:7`
- `ref-states` row `Advance main#1`: `0:2,1:11,2:0,3:7` -> `0:2,1:2,2:0,3:7`
- `ref-states` row `Archived work#1`: `1:11` -> `1:2`
- `ref-states` row `ahead: not pushed yet#1`: `1:11,2:0,3:7` -> `1:2,2:0,3:7`
- `ref-states` row `ahead: pushed#1`: `1:11,2:0,3:7` -> `1:2,2:0,3:7`
- `ref-states` row `behind: shared#1`: `1:11,2:0` -> `1:2,2:0`
- `ref-states` row `behind: theirs#1`: `1:11,2:5` -> `1:2,2:5`
- `stash` row `Add a.txt#1`: `0:2,1:4,2:7` -> `0:2,1:6,2:4`
- `stash` row `On main: wip: with an untracked file#1`: `1:4,2:7,3:2` -> `1:6,2:4,3:4`
- `stash` row `index on main: 923159b Add a.txt#1`: `None` -> `1:6,2:4,3:4`
- `stash` row `index on main: d96881b Add a.txt#1`: `1:4,2:7,3:2` -> `None`
- `stash` row `untracked files on main: 923159b Add a.txt#1`: `None` -> `1:6,2:4,3:4`
- `stash` row `untracked files on main: d96881b Add a.txt#1`: `1:4,2:7,3:2` -> `None`

## Changed PNGs

- `docs/shots/22-raster-criss-cross-merge-on-dark.png` (187, 259) -> (187, 259)
- `docs/shots/22-raster-criss-cross-merge.png` (187, 259) -> (187, 259)

## Other (stop; causes)

Every entry here is a stop. Attributed by reading each pair (2026-11-11 run): the six `docs/shots/m6/*` files are rows that swapped place (two fixture commits with equal times: tie-break on oid, `mergewright-git/src/history/node.rs`), so they also show a moved cursor or lane glyph; `it__raster__raster-criss-cross-merge.snap` is lane-slot letters (unnamed lane, `of_short_sha`) hit by the hex mask; `m8-look-help-graph-187x50.txt` is the tail of the author word (`aft` from Graft, now the tail of Maker) showing past an overlay edge.

- `crates/mergewright-graph/tests/it/snapshots/it__raster__raster-criss-cross-merge.snap` line 45
- `docs/shots/m6/fix-abort-reads-aborted-120x40.txt` line 3
- `docs/shots/m6/fix-landing-b-launch-120x40.txt` line 4
- `docs/shots/m6/fix-landing-b-launch-187x50.txt` line 4
- `docs/shots/m6/fix-radar-bookkeeping-120x40.txt` line 25
- `docs/shots/m6/fix-tab-words-120x40.txt` line 5
- `docs/shots/m6/set-aside-preview-120x40.txt` line 15
- `docs/shots/m8-look-help-graph-187x50.txt` line 5
