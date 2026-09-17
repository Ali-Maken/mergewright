# explain-panel: turn a marked commit's finding into a read-only explanation

Status: done
Blocked by: 04, 07, 10, 12, 13, 13b, 13c
Build with: Sonnet — assembling and rendering data `explain()`/`findings_for` already produce; no new rule, judged by the capture and the transition tests.
Needs: `mistake-radar-detection` (`explain`, `Explanation`), `radar-context-and-markers` (`findings_for`, `RadarState`), `context-actions-menu` (`DetailState::explain_requested`, already landed), `detail-pane-frame` (`DetailPane`, `view::detail_view`, and whatever it used to resolve commit facts — parents/subject — for `AppState.detail.spec`'s oid; report `partial` inheriting its gap rather than adding a new git read if that lookup was itself left unresolved), `detail-keys-and-focus` (the `Esc` arm, already landed with an interim `focus = Graph; shown = false` in place of an Explain-priority branch)

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — New UI "Explain panel" bullet in full; "Marker column" bullet's last clause (`. explain` footer hint); §8 coverage row "Explain"
- `.scratch/m2-details-and-diffs/issues/mistake-radar-detection.md` — `Explanation { title, paragraphs, evidence, highlight }`, `explain` signature, and its acceptance test for the foxtrot fixture (exact `title`, `highlight` order, three `evidence` labels top→bottom) — this ticket's sketch formatting must reproduce that order
- `.scratch/m2-details-and-diffs/issues/context-actions-menu.md` — exactly what it set (`state.detail.explain_requested = Some(oid)`, nothing else) and its own note that Explain "cannot appear end to end" from that ticket alone
- `.scratch/m2-details-and-diffs/issues/detail-pane-frame.md` — `DetailView.explain`, and its `view::detail_view` scope note ("files/body/focus/mode/options/explain come straight from `AppState::detail`") — this ticket is what makes that field real
- `.scratch/m2-details-and-diffs/issues/detail-keys-and-focus.md` — the landed `Esc` arm (its Acceptance line: "closes the pane (no Explain in this ticket's scope, or closes Explain first if that seam is already there — state which)")
- `crates/graft/MAP.md` line 114 (`AppState::focus`, Focus mode: "rows off the selection's thread fade; paint only") and line 212 (`graph_view`: "the selection's Ancestry over the window lights lanes, fades rows in Focus mode")
- `crates/graft-ui/MAP.md` lines 35–70 (`graph_screen`: fade/hint rendering)

## Scope

In:
- `crates/graft/src/update/detail.rs` (extends `DetailState`, same additive pattern `compare-two-commits`/`context-actions-menu` used): `pub explain: Option<ExplainView>`, `pub explain_highlight: Vec<Oid>` (raw ids for the graph fade; `ExplainView` itself stays ui-safe text only, per `detail-model-and-jobs`' rule that `ui` never sees raw git types).
  - `pub fn open_explain(state: &mut AppState, next_id: &mut impl FnMut() -> JobId) -> Vec<Command>` — called once per tick while `explain_requested.is_some()`: opens the pane on that oid if it isn't already there (`detail.open(DetailSpec::Commit(oid), next_id)`, a no-op command-wise if already open on it, per `detail-model-and-jobs`' existing dedupe); resolves `(parents, subject)` for that oid via the same path `view::detail_view` uses for the header of an arbitrary open spec (cite it exactly, don't add a second lookup); calls `findings_for(state, oid, parents, subject)`, takes the first (there is normally exactly one, since Explain only ever appears on a marked row); if found, builds an `Explanation` via `graft_ops::mistake_radar::explain(&finding, &short)` (`short` = the same short-sha formatter the SHA header line already uses), converts it to `ExplainView` via `build_explain_view` below, sets `state.detail.explain`/`explain_highlight`; if no finding is found (a stale marker — the row's finding disappeared, e.g. policy changed underneath), clears `explain_requested` and does nothing else — not a bug, just nothing to show. Always clears `explain_requested` at the end.
  - `pub fn close_explain(state: &mut AppState)` — `explain = None`, `explain_highlight.clear()`.
- `crates/graft/src/view/explain.rs` (new): `pub fn build_explain_view(e: &Explanation) -> ExplainView` — `title`/`paragraphs` sanitised as-is; `sketch` from `build_sketch(&e.evidence, short)`.
  - `pub fn build_sketch(evidence: &[(Oid, String)], short: impl Fn(Oid) -> String) -> Vec<SanitizedText>` — one line per entry (`"{label}  {short(oid)}"`, sanitised), a connector line between each consecutive pair alternating `"        ──╮"` / `"        ──╯"` (matches DESIGN §1.8's shape the mistake-radar-detection test already pins the label order for); for the foxtrot fixture this is exactly 5 lines (label, connector, label, connector, label).
- `crates/graft/src/update/keys.rs`: extend the landed `Esc` arm with the priority branch it left as a seam: `if state.detail.explain.is_some() { close_explain(state); return …; }` before its existing pane-close behaviour — cite the exact landed arm, add the one branch, don't restructure it.
- `crates/graft/src/view/rows.rs` (or wherever `view::graph_view`'s fade is computed, per its MAP.md line 212): while `state.detail.explain.is_some()`, fade every row whose id is not in `explain_highlight` (same fade rendering Focus mode already uses — reuse the function, feed it a different id set, don't fork the renderer); `AppState::focus` (the plain `F` toggle) is untouched and unrelated — Explain's fade is independent and always wins while open (state precedence in the report if both could somehow be true at once, though `F` and Explain aren't reachable together in normal use).
- `crates/graft-ui/src/graph_screen.rs`-equivalent (per MAP.md section): the cursor row's key hints gain `. explain` when that row's `marker.is_some()` (small addition beside wherever hints are already assembled from row state).
- `crates/graft-ui/src/detail_pane.rs`: when `view.explain.is_some()`, replace the files+body regions with: title line (severity glyph + `title`, e.g. `⚠ FOXTROT MERGE`, severity colour), paragraphs wrapped to width, a blank line, the sketch lines verbatim, footer `Esc close`. At every breakpoint (`Right`/`Below`/`Tab`) this occupies the same area the files+body split otherwise would.
- Tests: `open_explain`/`close_explain` transitions (found finding → view set; stale/no finding → cleared, no view); `build_sketch` against the foxtrot fixture's known evidence (exact 5 lines, connector glyphs, order); `Esc` closes Explain first, a second `Esc` then closes the pane; fade test (`graph_view` on a fixture with an open Explain fades every row outside `explain_highlight`); hint test (`. explain` appears only on a marked cursor row). MAP.md lines (`graft`, `graft-ui`).
- Capture: the foxtrot fixture's merge, Explain open (title, two paragraphs, sketch, faded graph rows visible behind/around the pane depending on breakpoint), to `docs/shots/m2-explain-panel.txt`, ≤ 200 KB.

Out: the `[ Fix… ]` button (deferred to M5, per Proposed changes), any change to `classify_commit`/`explain`'s wording (`mistake-radar-detection`'s job), the probes (their own tickets — a probe-only finding like `EvilMerge` or `ForcePushedRemote` explains exactly the same way once `findings_for` reports it, nothing extra needed here).

## Signatures

```rust
// crates/graft/src/update/detail.rs (extends DetailState)
// DetailState: + pub explain: Option<ExplainView>, + pub explain_highlight: Vec<Oid>
pub fn open_explain(state: &mut AppState, next_id: &mut impl FnMut() -> JobId) -> Vec<Command>;
pub fn close_explain(state: &mut AppState);

// crates/graft/src/view/explain.rs
pub fn build_explain_view(e: &Explanation) -> ExplainView;
pub fn build_sketch(evidence: &[(Oid, String)], short: impl Fn(Oid) -> String) -> Vec<SanitizedText>;
```

## Acceptance

- [x] `.` → `Explain` on the foxtrot fixture's merge opens the pane on it and sets `detail.explain` with `title == "FOXTROT MERGE"`, two-ish paragraphs, a 5-line sketch in evidence order.
- [x] `build_sketch` on the foxtrot fixture's `Explanation.evidence`: exact 5 lines, connectors alternating `──╮`/`──╯`.
- [x] `Esc` while Explain is open closes only Explain (pane stays, on the same commit); a second `Esc` then closes the pane (matches `detail-keys-and-focus`'s existing order otherwise).
- [x] `graph_view` with Explain open fades every row not in `explain_highlight`; with Explain closed, fading reverts to `F`'s existing ancestry rule, untouched.
- [x] The cursor row's hints show `. explain` only when `marker.is_some()` for that row.
- [x] A stale `explain_requested` (no finding found) clears the flag and sets no `explain` view, without error.
- [x] MAP.md: `graft` (`update/detail` additions, `view/explain`), `graft-ui` (`detail_pane`, `graph_screen` hint line).
- [x] Capture at `docs/shots/m2-explain-panel.txt`, ≤ 200 KB; user judges by eye.

## Deviations from spec

- `open_explain`'s `(parents, subject)` lookup reuses `view::detail::commit_header`'s existing path, which resolves facts for whatever oid is currently selected/open — not an arbitrary oid. This is fine in practice (a marked row is always the cursor row: Explain is only reachable from the context-actions menu on the selected commit), but it is an inherited constraint, not a general-purpose oid→facts lookup. Documented here rather than silently generalised or left implicit.
- No `[ Fix… ]` button — out of scope per the ticket, deferred to M5.

## Notes

- Compaction occurred once mid-ticket (context ran long across `update`/`view`/`graft-ui` wiring plus tests). All remaining work after compaction was: writing the transition/fade/hint tests, the `foxtrot_explain_state`/`foxtrot_explain_history` fixture and its capture test, fixing three clippy lints (`if_then_some_else_none`-style bool-to-int, `assigning_clones`, two `too_many_lines` splits — `detail_key`→`escape_priority`, `foxtrot_explain_state`→`foxtrot_explain_history`), updating both crates' MAP.md, running the capture, and this closeout. No scope was lost or changed by the compaction.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · purity guard.

## Size

Two crates (`graft` ~150 lines: update ~60, view/explain ~50, keys/rows edits ~40; `graft-ui` ~70: detail_pane ~55, hint line ~15). Tests ~150. ≤ 24 tool calls. Named seam: if the commit-facts-for-an-arbitrary-oid lookup (inherited from `detail-pane-frame`) isn't there yet, stop at that exact point, commit `build_sketch`/`build_explain_view` (fixture-tested, no app wiring) and report `partial` naming the missing lookup, not a workaround git read.
