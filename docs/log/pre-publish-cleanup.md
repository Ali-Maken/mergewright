# Pre-publish cleanup — live log

One line per helper, ending with its tool count (harness figure).

- pre-publish start badd3e87
- planner-fable: 12 tickets + index (.scratch/pre-publish-cleanup/issues/), 14 rows covered, no Fable tier, 2 debuggers; U1 to user (old-name folder move: keep/drop) · 45 tools.
- scout (sonnet) 04 lane identity: ticket contradicted — upstream read via handle.upstream() (refs/mod.rs:202-209), not %(upstream) parse; preview_seeds passes empty followed map (history/seeds.rs:125,146) while main graph uses seeds_with_upstreams (stream.rs:59); no fixture with differently-named upstream. Builder reads scout over ticket · 16 tools.
- scout (sonnet) 05 blame ink/narrow graph: BlameView has no theme field, blame.rs never calls ink_marks (only caller overlays.rs:201); no-gutter only at screen ≤3 cols (layout.rs:298), graph ≥40 by resize cap (state.rs:37) → narrow-graph case reachable only on a ≤3-col terminal; user's Ghostty narrow check dropped · conductor ruling. Unconfirmed: stale override_cells after shrink (layout.rs:51 lower clamp only).
- builder-sonnet (sonnet) 01 merged partial (17af5032): A/B script + perf file; noise (A/A 135 s) > delta (54 s); new tests +1.8 s; 3 boxes open (fixes, ≤360 s twice) — not measurable on this load; gap ease/28 → unverified · skills: none.
- conductor gates on main after 01: fmt/clippy/heavy/doc ok; fast 6844/6844 628.8 s (beside debugger 02 — not valid timing); walks 463/0 169.7 s. User killed dasd + fseventsd (both respawned; fseventsd 109%→<7% once tests stopped).
