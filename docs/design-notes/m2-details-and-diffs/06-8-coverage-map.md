# M2 design pass: details and diffs (2026-09-15) — §8 coverage map

[See also: m2-details-and-diffs/README.md](README.md) for the full section index.
Previous: [New UI](05-new-ui.md) · Next: [Genuinely risky tickets](07-genuinely-risky-tickets.md)

---

## §8 coverage map

| Item | Ticket slug(s) |
|---|---|
| Commit detail pane | `git-runner` (contained-in), `detail-model-and-jobs` (p1), `detail-pane-frame`, `detail-keys-and-focus` |
| Working-tree row → `DetailSpec::WorkingTree` (PROMPT §1.3 virtual row, §5 "diff against the working tree") | `working-tree-row-selectable` (reach it), `detail-pane-frame` (`UNCOMMITTED` header) |
| File tree | `detail-model-and-jobs` (p1, tree assembly), `detail-pane-frame` |
| Unified diff | `git-runner`, `diff-parser`, `diff-reads` (p1), `diff-widget` |
| Side-by-side | `diff-parser` (`pairs`, p1), `side-by-side-view` |
| Word diffs | `syntax-highlighting` (`word_spans`, p1), `diff-widget` |
| Syntax highlighting | `syntax-highlighting` (p1), `diff-widget` |
| Compare any two commits | `compare-two-commits` |
| Mistake Radar detection | `mistake-radar-detection` (p1, Fable sig), `radar-context-and-markers`, `radar-probe-evil-merge`, `radar-probe-force-push`, `radar-probe-duplicates` |
| Explain | `context-actions-menu`, `explain-panel` |
| Map view (M1 deferral) | `map-view-data`, `map-view-screen` |
| Overview strip (M1 deferral) | `overview-strip` |
| KG 54 refresh reopens walk after `B` | `refresh-reopens-walk` |
| KG 127 `cargo-fuzz` targets | `diff-parser` (p1); row closed by `docs-sweep` |
| KG 197/246/247 contained-in, body, parents | `git-runner`, `detail-model-and-jobs` (p1), `detail-pane-frame`; rows closed by `docs-sweep` |
| KG 326 Copy SHA under `.` | `context-actions-menu`; row closed by `docs-sweep` |
| KG 375 bare `cargo run --` sweep | `docs-sweep` (sweep, not `default-members`) |
| KG 52, 175, 227 | **uncovered by design** — expiries moved by `docs-sweep` (Proposed changes) |
| PROMPT §4 "image diffs in the pixel tier" | **uncovered** → KNOWN_GAPS row by `docs-sweep`, M8 polish |
| DESIGN §5.1 remote rows, cross-branch elbows, pixel curve, mouse | **uncovered by design** → KNOWN_GAPS rows by `docs-sweep`, M8 polish |
| DESIGN §1.16 `?` help overlay (was in `detail-keys-and-focus` group 2 / 10b) | **uncovered by design** → M8 (spec fix 2026-09-18); KNOWN_GAPS rows 199 and m2/10b re-owned |
| README/CHANGELOG for M2 | `docs-sweep` |
| Acceptance | `acceptance-details-and-radar` (Fable; planned as two contexts) |

Nothing else in PROMPT §8 M2 or the M1 deferrals is without a slug. **24 tickets** (23 + `working-tree-row-selectable`).
