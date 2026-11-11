# pp-12 identity rename: what moved, for the user's eye

Colour is unverified (a capture holds no colour). Lane-slot lines are the only colour evidence: a changed slot is a changed palette entry.

`SUMMARY compared=289 equal=272 reordered=0 lane-slot-files=3 other=14 png_changed=2 png_size_changed=0 added=0 removed=0 lane-slot changes=18`

## Reordered captures (same lines, other order)

Cause site: oid tie-break in `mergewright-git/src/history/node.rs` (equal commit time, ties on oid) or `mergewright/src/map/walk.rs:92` (branch order). Fixture commits sharing a timestamp are the inputs.


## Snapshot files whose lane-slot letters moved (approximate class)

- `crates/mergewright-ui/tests/snapshots/graph_column__ref-states.snap` line 7
  - was: `  ●       ..l.....  #  Archived work`
  - now: `  ●       ..e.....  #  Archived work`
- `crates/mergewright-ui/tests/snapshots/graph_column__stash.snap` line 7
  - was: `  ■─┬─╮   ..ehhcc.  #  On main: wip: with an untracked file`
  - now: `  ■─┬─╮   ..iddhh.  #  On main: wip: with an untracked file`
- `crates/mergewright-ui/tests/snapshots/lanes_without_hue__criss-cross-merge-16.snap` line 35
  - was: `5.68788...`
  - now: `5.67777...`

## Lane-slot changes (probe, by corpus fixture row)

Cause site: `BranchIdentity::of_short_sha` (`mergewright-graph/src/identity.rs`) for an unnamed lane; named lanes hash the name and cannot move.

- `criss-cross-merge` row `alpha: merge beta#1`: `0:2,1:5,2:3,3:9,4:0` -> `0:2,1:5,2:3,3:4,4:5`
- `criss-cross-merge` row `alpha: work#1`: `0:2,1:5,2:3,3:9` -> `0:2,1:5,2:3,3:4`
- `criss-cross-merge` row `beta: merge alpha#1`: `0:2,1:5,2:3,3:9` -> `0:2,1:5,2:3,3:4`
- `criss-cross-merge` row `beta: work#1`: `0:2,1:5,2:3,3:9,4:0` -> `0:2,1:5,2:3,3:4,4:5`
- `ref-states` row `Add the base#1`: `0:2,1:11` -> `0:2,1:4`
- `ref-states` row `Advance main again#1`: `0:2,1:11,2:0,3:7` -> `0:2,1:4,2:0,3:7`
- `ref-states` row `Advance main#1`: `0:2,1:11,2:0,3:7` -> `0:2,1:4,2:0,3:7`
- `ref-states` row `Archived work#1`: `1:11` -> `1:4`
- `ref-states` row `ahead: not pushed yet#1`: `1:11,2:0,3:7` -> `1:4,2:0,3:7`
- `ref-states` row `ahead: pushed#1`: `1:11,2:0,3:7` -> `1:4,2:0,3:7`
- `ref-states` row `behind: shared#1`: `1:11,2:0` -> `1:4,2:0`
- `ref-states` row `behind: theirs#1`: `1:11,2:5` -> `1:4,2:5`
- `stash` row `Add a.txt#1`: `0:2,1:4,2:7` -> `0:2,1:8,2:3`
- `stash` row `On main: wip: with an untracked file#1`: `1:4,2:7,3:2` -> `1:8,2:3,3:7`
- `stash` row `index on main: d96881b Add a.txt#1`: `1:4,2:7,3:2` -> `None`
- `stash` row `index on main: de6923a Add a.txt#1`: `None` -> `1:8,2:3,3:7`
- `stash` row `untracked files on main: d96881b Add a.txt#1`: `1:4,2:7,3:2` -> `None`
- `stash` row `untracked files on main: de6923a Add a.txt#1`: `None` -> `1:8,2:3,3:7`

## Changed PNGs

- `docs/shots/22-raster-criss-cross-merge-on-dark.png` (187, 259) -> (187, 259)
- `docs/shots/22-raster-criss-cross-merge.png` (187, 259) -> (187, 259)

## Other (stop; causes)

Causes, attributed by reading each pair: (1) criss-cross lane-letter snapshots: unnamed lane, `BranchIdentity::of_short_sha` (a lane-slot change, same as the probe's); (2) every other entry: the author column shows the NAME'S FIRST WORD, so `Graft` (5) became `Fixture`/`Fixtu…` (7/6), which moves where a detail line, a blame cell or an overlay row is cut. D1's premise (equal-length names keep every truncation) holds for the whole name, not for the first word.

- `crates/mergewright-graph/tests/it/snapshots/it__raster__raster-criss-cross-merge.snap` line 45
- `crates/mergewright-ui/tests/snapshots/graph_column__criss-cross-merge.snap` line 9
- `crates/mergewright-ui/tests/snapshots/lanes_without_hue__criss-cross-merge-256.snap` line 35
- `docs/shots/ease-pass/help-graph-mode-120.txt` line 9
- `docs/shots/ease-pass/main-cursor-teammate-120.txt` line 9
- `docs/shots/m7/history-100x30.txt` line 2
- `docs/shots/m7/history-lines-187x50.txt` line 2
- `docs/shots/m7/rewrite-refused-fork-120x40.txt` line 7
- `docs/shots/m7/switch-landed-120x40.txt` line 12
- `docs/shots/m7/switch-menu-clean-landed-120x40.txt` line 12
- `docs/shots/m8-bar-head-row-100x30.txt` line 5
- `docs/shots/m8-help-rebase-stop-120x36.txt` line 5
- `docs/shots/m8-look-help-graph-187x50.txt` line 5
- `docs/shots/m8/came-back-landed-120x40.txt` line 15
