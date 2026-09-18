# m1-the-graph/13 — Lanes at 16 colours and `NO_COLOR`: identity from shape

Status: done (by-eye box open until the user looks)
Build with: **Opus** (high) — design judgment; one mechanism for two constraints
Blocked by: 12b, 12c
Leaves runnable: yes — the graph stays readable in 16-colour terminals and with `NO_COLOR`.

Why: six hues cannot carry twelve lanes, and `NO_COLOR` carries none. One second channel must solve both.

## Read first
- PROMPT.md §1.2 (colour rules only) and §5
- docs/design/04-terminal-capability-detection.md §4.6
- KNOWN_GAPS.md M0 rows "Twelve lanes collapse to six at 16 colours", "The lane palette is argued, not seen", "Colour depth is inferred from environment strings, not terminfo"
- Code: `crates/graft-ui/src/theme/`, 08's widget

## Scope
In:
- A second channel for lane identity when hue can't carry it: dash pattern, glyph variation, or a hue cycle plus a mark. Pick one; say why the others lost.
- The same mechanism carries `NO_COLOR`.
- 08's paint attribute (dimmed / highlighted) still distinguishable at 16 colours and `NO_COLOR`.
- Settle colour depth: a terminfo lookup (`max_colors`, `RGB`), or an ADR that environment strings are enough — with its consequence for the tier boundary.

Out:
- ASCII (19).

## Acceptance
- [x] Snapshots of criss-cross, foxtrot, octopus, the widest fixture and a linear history at 256, 16 and `NO_COLOR`, 80×24.
- [x] A snapshot where two adjacent lanes share a hue at 16 colours and are still told apart.
- [x] Colour-depth decision recorded as an ADR; the three M0 rows closed or re-owned with a reason.
- [ ] Exact commands prepared for the user to judge in Zed and VS Code on criss-cross and foxtrot. **The by-eye box stays unticked until the user looks.**

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.

## Notes (m1/13, builder-opus)

**Mechanism: dashes by column parity, at 16 colours and `NO_COLOR` only.** Lanes in odd columns draw straight runs as `╎`/`╌`; even columns stay `│`/`─`. Hue still comes from branch identity. Rule is `graph_column::dashes_odd_columns`.

Why the others lost:
- Dash by branch identity (hash): neighbours still collide one time in k; `NO_COLOR` would need 12 legible patterns, box drawing has ~3.
- Node glyph variation: node shape already says commit kind (HEAD, merge, root).
- Hue cycle plus a mark: breaks per-branch colour stability (PROMPT.md §1.2), and a lane has no spare cell for a mark.
- Parity: neighbours always differ; row-local (needs only the `Row`); a lane keeps its column for life, so the pattern never flips along a line.

Truecolor/256 unchanged (all existing snapshots identical). Highlight at `NO_COLOR` now gets 12c's `▏` rule too (no background step there either); dimmed stays `DIM`.

Colour depth: ADR-0019 (env strings are enough). KNOWN_GAPS: "Twelve lanes collapse" and "Colour depth inferred" closed; "palette argued, not seen" re-owned to the user's by-eye pass; 08's "Neighbouring lanes can share a hue" partly closed (truecolor still position-only). New rows: compaction (14) must keep parity; corners have no dashed forms.

Evidence: `crates/graft-ui/tests/lanes_without_hue.rs` (15 snapshots; widest shape computed = criss-cross) and a corpus-wide test that no same-coloured neighbours share a glyph at 16/`NO_COLOR`. pty captures: `docs/shots/13-{foxtrot-merge,criss-cross-merge}-{16,no-color}.txt`.

### For the user to judge by eye (Zed and VS Code)

Build once: `cargo build -p graft && cargo test -p graft-fixtures` (the fixture corpus lands in `target/fixtures/<key>/`).

```sh
F=$(dirname $(ls -d target/fixtures/*/foxtrot-merge | head -1))
# truecolor (as you run it daily)
target/debug/graft $F/foxtrot-merge
target/debug/graft $F/criss-cross-merge
# 16 colours
env -u COLORTERM -u TERM_PROGRAM TERM=xterm target/debug/graft $F/foxtrot-merge
env -u COLORTERM -u TERM_PROGRAM TERM=xterm target/debug/graft $F/criss-cross-merge
# NO_COLOR
NO_COLOR=1 target/debug/graft $F/foxtrot-merge
NO_COLOR=1 target/debug/graft $F/criss-cross-merge
```
Look for: can you follow each lane; do dashed neighbours read as separate branches; is truecolor fine without dashes (else dashes at every depth is a one-line change).
