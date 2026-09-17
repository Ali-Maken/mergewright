# acceptance-details-and-radar: M2's acceptance ticket — the gallery's "details and radar" stops, TESTING.md, captures

Status: ready
Blocked by: 01, 02, 03, 04, 05, 06, 06b, 07, 10b, 08, 09, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22

Status: draft
Build with: Fable — defining what good looks like for the promise is a decision, not plumbing (CLAUDE.md, acceptance-ticket rule). Doesn't count against the milestone's Fable build expectation.
Needs: `git-runner`, `diff-parser`, `diff-reads`, `syntax-highlighting`, `mistake-radar-detection`, `detail-model-and-jobs`, `working-tree-row-selectable`, `detail-pane-frame`, `diff-widget`, `side-by-side-view`, `detail-keys-and-focus`, `compare-two-commits`, `context-actions-menu`, `radar-context-and-markers`, `radar-probe-evil-merge`, `radar-probe-force-push`, `radar-probe-duplicates`, `explain-panel`, `map-view-data`, `map-view-screen`, `overview-strip`, `refresh-reopens-walk`, `docs-sweep` — every other M2 ticket; a `partial` among them is walked as far as it got and its gap is a stop that says so.

**M2's promise:** *what a commit changed is readable without leaving the graph, and a bad merge is flagged and explained.* This ticket builds the means to judge that promise by eye, not more tests of it. M2 isn't done until this ticket is.

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — Promise; New UI in full (the detail pane at three breakpoints, marker column, Explain panel, context actions, Map view, Overview strip: these bullets are the source of every "right looks like" line below); Proposed changes (what is deliberately *not* on screen: no `?` on unchecked merges, no `Fix…`)
- `.scratch/m1-the-graph/issues/35-graph-gallery-acceptance.md` — Scope items 1–5, "Guard (context)", and "Part a: what was built" (the `Harness`, the cycle, `--list`/`--matrix`); `35c` "Built" (one stdin reader per process; the `pty-gallery.py` route and its `]q` ending)
- `TESTING.md` lines 579–660 (Graph gallery: the banner, how entries are written — mirror this section's form exactly) and 813–833 (per-terminal walk)
- `crates/graft/MAP.md` lines 19 (`examples/gallery`), 94–96 (`Harness`, `Banner`), 229 (`run` re-entrancy)
- `crates/graft-fixtures/MAP.md` (`Shape::ALL`, names) — header lines only
- Each `Needs:` ticket's **Acceptance** section only, for what it proved and where its capture is (never its Scope; never source)

## Scope

In:
1. **A second progression in the gallery example** (`cargo run -p graft --example gallery -- --group details`; `--list` prints both groups; `--group graph` is M1's, the default stays M1's so the M1 checklist doesn't move). Each M2 stop is a repo **plus a key prelude** the harness feeds before the first frame: `Harness::prelude: Vec<KeyPress>` (new field; applied in order after the first history window lands and the refs job has answered, so `Enter` opens on a real row and the radar context is `Ready` — wait for those products, don't sleep). Stops, in order, each with its `shows:`/`differs:` banner lines (≤ 80 columns, the M1 width test extended to the new group):
   1. `feature-branches · Enter` — pane on `Merge branch 'feature/two'`: header, files tree, first file's diff.
   2. `feature-branches · Enter j` — cursor on the second file: its diff replaces the first, the header didn't move.
   3. `feature-branches · Enter Tab v` — side-by-side.
   4. `feature-branches · Enter W` — word tint on a changed line.
   5. `feature-branches · x j j` — compare base marked, pane reads `COMPARE a → b · N files`.
   6. `foxtrot-merge` — `⚠` in the marker column on the merge, `. explain` in the hints, pane radar line `⚠ foxtrot merge`.
   7. `foxtrot-merge · .` then `Enter` on `Explain` — the Explain panel: `⚠ FOXTROT MERGE`, paragraphs, 5-line sketch, faded rows.
   8. `backward-merge` — `⚠ backward merge` on `feature`'s merge; `main`'s rows unmarked.
   9. `accidental-pull-merge` — `ⓘ accidental pull merge`.
   10. `evil-merge · Enter` — radar line goes `evil merge: unchecked` → `checking…` → `⚠ evil merge · evil.txt` (the capture waits for the last).
   11. `cherry-pick-duplicates` — `ⓘ duplicate commit` on both copies once the probe lands.
   12. `ref-states` — `ⓘ` on `ahead: not pushed yet` (`unpushed on default` or `no upstream`, whichever `classify_refs` assigns — the entry states which and why).
   13. `unicode-names · Enter` — CJK path in the files tree, columns aligned.
   14. `non-utf8-path · Enter` — the Latin-1 path shows one `�`, nothing else disturbed; the diff loads.
   15. `feature-branches · M` — Map view: 4 rows, elbows on the two merged branches, `▸ main … ●`.
   16. `feature-branches · O` — strip: `◉` twice, `●` at the top, `█` over the viewport.
   17. `graft-self · Enter` — a real repo's commit: `Contained in: main`, message row `✉`, real syntax colours.
   18. `git-git · Enter` on a release merge (prelude includes `/v2.4` … or a `G`-free path the builder chooses; state it) — a 100+-file tree, first diff within the frame budget; `L` on a truncated diff.
   19. `git-git · O` — the strip is "honestly incomplete" (buckets below the walked range blank) and fills in.
   Heavy repos only if present, as M1's rule (`not available — build with:`).
   The working-tree row has **no gallery stop** (the gallery walks fixtures as they are; dirtying one is not this ticket's job): its TESTING.md entry (item 2) points at the captures `working-tree-row-selectable` and `detail-pane-frame` already committed.
2. **TESTING.md gains "Details and diffs"** after "Graph gallery", same form: the command, the keys, "Reading the entries" (glyphs: `⚠`/`ⓘ`, `✉`, `M/A/D/R/C/T`, `+N −N`, `bin`, the `▏` compare rule, the tint vocabulary at 256 colours vs 16), then **one concrete entry per stop** — which row carries the marker, what the header's four lines say, which file is first and what its first hunk's heading is, which words tint, what the sketch's three labels are, what `Esc` closes first — drawn from the fixture facts in `crates/graft-fixtures/src/shapes/` via the M1 entries (never "looks correct"). Plus one entry **"Working-tree row"** with no stop: in a dirty checkout, `k` from the top row lands on `Uncommitted changes (…)` (highlight moves, HEAD's row un-highlights), `Enter` opens `UNCOMMITTED · N staged N unstaged N untracked` / `diff vs HEAD <sha>`, staged and unstaged files listed, an untracked file's body reads `untracked · no diff yet (M3)`, `j` from the row returns to HEAD's commit and the pane follows; captures: `docs/shots/m2-working-tree-row.txt` (graph) and the `UNCOMMITTED` pane in `docs/shots/m2-detail-pane-breakpoints.txt`. Then a per-breakpoint block: what changes at 160+ (right column), 100–159 (below), < 100 (tab strip) — the same stop at three sizes. Then the per-terminal walk, extending the M1 procedure by two lines (walk the `details` group too; at stop 10 wait for the evil verdict; in a dirty checkout of graft itself press `k`, `Enter` at the top and check the "Working-tree row" entry).
3. **Captures:** `.scratch/m2-details-and-diffs/pty-gallery-m2.py` modelled on `.scratch/m1-the-graph/pty-gallery.py` (pyte; verify the config file name the way 35b did). Every stop at **160×40** Unicode (the wide layout is where the pane lives) and at 80×24 Unicode; stops 1, 6, 7, 15, 16 also at 100×30 (medium) and 80×24 ASCII. Files `docs/shots/m2-<nn>-<stop>-<WxH>-<tier>.txt`, each trimmed to the frame, **≤ 200 KB** (a 160×40 frame is ~7 KB; never commit a raw stream — raw runs stay under `target/`). Each file verified by `grep -c` for its own stop name and one marker string from its entry (e.g. `FOXTROT MERGE`), never `cat`.
4. **Coverage matrix** re-run (`--matrix`) so the `m2-*` captures appear; gaps (a stop with no capture at some size) listed in KNOWN_GAPS with owner `M2 review`.
5. The exact commands for the user written into this ticket's Built section, as 35 did.

Out: judging by eye (the user); fixing what a stop reveals (a new ticket or the M2 review — say what you saw, in the entry's words); heavy fixtures generated here (never); a dirty-fixture gallery stop (see item 1).

## Guard (context)

Captures go **straight to files** under `docs/shots/m2-*`. Never print a capture into your own context: check with `wc -c`, `grep -c`, or a haiku scout. Perf lines for stop 18 (first diff latency on git/git) come from the script writing `docs/perf/acceptance-details-and-radar.txt`; read its summary line only. Report file paths and the matrix only.

## Signatures

```rust
// crates/graft/src/update (Harness)
// Harness: + pub prelude: Vec<KeyPress>   — keys the loop feeds, in order, once the first window and the refs product have landed; empty in the binary
// tests/it/gallery.rs
pub enum Group { Graph, Details }
pub fn stops(group: Group) -> Vec<Entry>;   // Entry gains `prelude: Vec<KeyPress>` and `group`
```

## Acceptance

- [ ] `cargo run -p graft --example gallery -- --group details` walks the 19 stops with `]`/`[`; `--list` prints both groups; every banner line ≤ 80 columns (test); the M1 default group is untouched (its `--list` output byte-identical).
- [ ] `Harness::prelude` feeds keys only after the first window + refs product (a scripted-`Wakes` test: `Enter` in the prelude opens the pane on row 0, not on nothing); default-empty in `main.rs`.
- [ ] TESTING.md "Details and diffs": one concrete entry per stop plus the "Working-tree row" entry, a per-breakpoint block, the per-terminal lines; no entry reads "looks correct"/"renders fine" (grep).
- [ ] Captures under `docs/shots/m2-*` per item 3, each ≤ 200 KB and each grep-verified to name its stop and one marker string; the matrix re-run and committed.
- [ ] `docs/perf/acceptance-details-and-radar.txt` holds stop 18's first-diff latency line (informational; no gate).
- [ ] Exact user commands in this ticket's Built section.
- [ ] **By eye (user):** walks the `details` group in Ghostty and the VS Code terminal at ≥ 160 columns against TESTING.md. Unticked until the user does.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet`. Fast tier only; nothing heavy generated.

## Size

Over the 80k target if done in one go — M1's 35 needed a/b/c. **Planned as two contexts from the start; say so in the report rather than compacting:** (a) `Harness::prelude` + the `Details` group + `--list`/banner tests + this ticket's Built section (~250 lines, ≤ 28 tool calls); (b) TESTING.md entries + the pty script + captures + matrix (~200 lines of prose, ≤ 30 tool calls, mostly file writes). Stop at the seam with a handoff note in this ticket; the conductor files (b) as its own ticket with the same `Build with`.
