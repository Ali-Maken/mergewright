# docs-sweep: README/CHANGELOG for M2, the bare `cargo run --` sweep (KG 375), KNOWN_GAPS expiries and the new gap rows

Status: draft
Build with: Sonnet — documentation edits with every target line named below; no code.
Needs: every M2 feature ticket that lands before it (the README/CHANGELOG describe what shipped; run this after `explain-panel`, `map-view-screen`, `overview-strip`, `refresh-reopens-walk`, before the acceptance ticket)

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — Promise; Proposed changes table (only rows the checkpoint has since turned into an ADR are documented as behaviour; the rest are not mentioned); New UI "Detail pane → Keys" bullet (the key list to document); §8 coverage map rows "KG 127", "KG 197/246/247", "KG 326", "KG 375", "KG 52, 175, 227", "PROMPT §4 image diffs"
- `KNOWN_GAPS.md` lines 50–54 (table header + rows 52, 54), 175, 227, 373–375, and the rows numbered 127, 197, 246, 247, 326 (open the file at those lines only)
- `README.md` lines 100–150 (the "Try it" block and the exit-code examples), `CHANGELOG.md` top entry (its M1 shape is the template), `TESTING.md` lines 315–350, 430–470, 495–525 (the bare `cargo run --` lines), `scripts/clone-git-git.sh` line 78
- `docs/adr/` — list the files (Glob) to see which Proposed-changes rows have an ADR; read none whole

## Scope

In:
- **Bare `cargo run --` sweep (closes KG 375):** rewrite every bare mention to `cargo run -p graft --`: `README.md` 114, 146, 147, 148; `TESTING.md` 321, 348, 436, 467, 503, 506, 510, 515, 520 (only the second `cargo run` on 520); `scripts/clone-git-git.sh` 78. Not touched: `CLAUDE.md` 176 (an example of report style, not a command; and CLAUDE.md is the user's), `PROGRESS.md` 38 (`--manifest-path`, unambiguous), `scripts/m1-checkpoint.sh` (already `-p`). Decision recorded in the KNOWN_GAPS row: the sweep, not `[workspace] default-members` — `default-members` changes what `cargo build`/`cargo test` do everywhere without `-p`, and the fast tier relies on the whole workspace running. Close row 375's section with a one-line "Closed by m2 docs-sweep: sweep, not default-members".
- **README.md:** a "Details and diffs" paragraph under the M1 feature text: `Enter` opens the detail pane (files tree, unified diff; `v` side-by-side; `W` word tint; `w` ignore whitespace; `[`/`]` context; `Tab` cycles Graph→Files→Diff; `Esc` back), `x` marks a compare base, `.` context actions (Explain, Copy SHA, Compare, Open details), `M` Map view, `O` Overview strip, Mistake Radar markers `⚠`/`ⓘ` with `.` → Explain, `[radar]` and `[diff]` config keys (one line each: the seven radar keys with `off`/`info`/`warning`, `diff.context`, `diff.ignore_whitespace`, `diff.word_tint`). Keys listed only if their ticket landed (check each spec's `Status:`); a key whose ticket is `partial` is left out and named in the report.
- **CHANGELOG.md:** an M2 entry in the M1 entry's shape: Added (the paragraph above, as bullets), Changed (`Enter` no longer peeks — ADR-0023; markers column; `Ctrl+R` reopens the walk on the graph path), Known gaps (pointer to KNOWN_GAPS rows added below).
- **KNOWN_GAPS.md expiries and closures** (each closure names its ticket, same strike-through form the file already uses):
  - row 52 (snapshot cost on index ops) → expiry `M3` (no staging exists in M2 to measure)
  - row 175 (older stash badges) → expiry `M3` (no stash list in M2)
  - row 227 (`full_topological` residual) → expiry `Not planned`, one clause why (the offer is built; the residual is the user's own decline)
  - row 127 (`cargo-fuzz` targets) → closed by `diff-parser` if its `Status:` is done and the targets exist under `fuzz/` (check with Glob); else expiry `M2 review`
  - rows 197/246/247 (contained-in, body, parents) → closed by `git-runner`/`detail-model-and-jobs`/`detail-pane-frame` when their `Status:` is done; a `partial` leaves the row open with the seam named
  - row 326 (Copy SHA under `.`) → closed by `context-actions-menu`
  - row 54 → already closed by `refresh-reopens-walk`; verify, don't redo
  - **new rows:** "PROMPT §4 image diffs in the pixel tier — not built; M2 shows `binary · <old> → <new>` for images" → expiry `M8 polish`; "Map view has no mouse interaction and no pixel-tier density curve (Proposed changes)" → `M8 polish`; "Overview strip default is off; no `[ui]` key to start with it on" → `M8 polish`; "Map view shows local branches only; a remote does not collapse into its upstream's row (DESIGN §5.1)" → `M8 polish`; "Map view's merge elbow marks only merges into the default branch, at the tip's bucket" → `Not planned unless asked`.
- **DESIGN.md:** no edits unless an ADR for a Proposed-changes row exists; then only the keymap table rows §2.3/§2.5 that ADR names, with an `(ADR-00nn, m2)` suffix like the m1 rows carry.

Out: TESTING.md's "Details and diffs" section (the acceptance ticket writes it); ARCHITECTURE.md (each code ticket that added a module updated it, or reports the gap); any ADR (the conductor/user write those at the checkpoint).

## Signatures

none (docs only)

## Acceptance

- [ ] `rg 'cargo run --' README.md TESTING.md scripts/` finds no bare form (only `-p graft --` and `--manifest-path`).
- [ ] KNOWN_GAPS row 375 section closed with the decision; rows 52/175/227 carry the new expiries; 127/197/246/247/326 closed or left open with the seam named, per the tickets' real `Status:`.
- [ ] The five new gap rows exist with owners/expiries as above.
- [ ] README has the M2 paragraph, listing only landed keys; CHANGELOG has the M2 entry.
- [ ] `docs/design-notes/m2-details-and-diffs.md` untouched (the note is the planner's).

## Gates

None for code; `cargo test --quiet` once to be sure no doc-test or `include_str!` broke.

## Size

Docs only, ~120 changed lines. ≤ 16 tool calls: open each file at the cited lines, never whole (`TESTING.md` is 800+ lines; `KNOWN_GAPS.md` 370+). No seam expected.
