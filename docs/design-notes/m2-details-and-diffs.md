# M2 design pass: details and diffs (2026-09-15)

Read: CLAUDE.md standing rules; PROMPT.md §1.3–1.4, §2, §3, §4 Comparing, §5 Diffs,
§6.1, §6.3, §7.2, §8 M2; DESIGN.md §1.1–1.4, §1.8, §2.2–2.5, §2.15, §3.1, §5.1–5.3,
§8; ADR-0007, 0011, 0023; every `crates/*/MAP.md`; KNOWN_GAPS → M2 rows;
PROGRESS.md compaction lines; AUTHORSHIP.md; M1 review round 1. No source.

**Promise:** *what a commit changed is readable without leaving the graph, and a
bad merge is flagged and explained.* The acceptance ticket proves it on the
fixtures (foxtrot, backward, accidental-pull, evil, cherry-pick-duplicates,
unicode-names, non-utf8-path) and on git/git.

## Sizing from M1

20 of ~100 M1 builds compacted (AUTHORSHIP.md). Causes, from the log lines:
- **Whole big files read** — 41 (~100k), 70/71 (splitting a 1.5k-line `update.rs`), 48 (131k).
- **Measurement or pty runs in the builder's context** — 07 ("after pty runs"), 26 (pty+pyte at 100k), 27 (memory), 47/51b (heavy runs; 51b "ended turn 3× waiting").
- **Several modules in one ticket** — 12 (git status + app wiring + chrome), 08 (164k), 09 (160k), 51 (168k: graph + app + fixes).
- **Visual iteration** — 17 (132k: mouse + tooltip + sanitising), 41.
- **Spec question mid-build** — 52 blocked, then compacted; 36 stopped for a design pass.
- Fable and Sonnet compacted as readily as Opus: it is the ticket, not the model.

Rules applied to every M2 spec: (1) at most two crates per ticket and new modules
named up front, never "edit `update/`" wholesale; (2) captures and measurements
go to files by script (`docs/shots/`, `docs/perf/<ticket>.txt`), the builder reads
`wc -c`/grep/summary lines only; (3) at most six snapshots per ticket; (4)
consumers read the producer's MAP.md, never its source (interface tickets ship
real code, so that is enough); (5) a spec gap is a `partial` report at once, not
a guess; (6) no ticket waits on a heavy run — nothing in M2 needs one. Budget:
~600 new lines, ≤ 30 tool calls. Each spec names its seam for a `partial`.

## Interfaces and data shapes

```rust
// graft-git::run (git-runner) — the typed runner PROMPT Technology names; first use is reads
pub struct GitInvocation;   // `git -C <dir> --no-pager …`; env GIT_TERMINAL_PROMPT=0, LC_ALL=C, GIT_OPTIONAL_LOCKS=0; user config kept
impl GitInvocation { pub fn new(repo: &RepoHandle) -> Self; pub fn arg(self, a: GitArg) -> Self; pub fn args(self, a: impl IntoIterator<Item = GitArg>) -> Self;
    pub fn stdin(self, bytes: Vec<u8>) -> Self; pub fn output_cap(self, bytes: usize) -> Self;   // default 64 MiB
    pub fn run(&self, stop: &dyn Stop) -> Result<GitOutput, RunError>; pub fn command_line(&self) -> String; }   // kills the child on stop; display only
pub enum GitArg { Flag(&'static str), Count(&'static str, u32), Oid(Oid), Ref(Vec<u8>), Path(Vec<u8>), Separator, Word(String) }
pub trait Stop: Sync { fn should_stop(&self) -> bool; }   // impl for AtomicBool, ()
pub struct GitOutput { pub stdout: Vec<u8>, pub stderr: Vec<u8>, pub truncated: bool }
pub enum RunError { GitNotFound, Spawn(io::Error), Stopped, Exited { status: i32, stderr_first_line: String, command: String }, Signalled { command: String } }
// graft-git::contained (git-runner)
pub struct Contained { pub branches: Vec<Vec<u8>>, pub remotes: Vec<Vec<u8>>, pub tags: Vec<Vec<u8>>, pub more: usize }
pub fn contained_in(repo: &RepoHandle, commit: Oid, stop: &dyn Stop) -> Result<Contained, RunError>;   // for-each-ref --contains
// graft-git::history (radar-context-and-markers, pass 2) — first-parent chain and a ref's own reflog, both in-memory gix, no spawn
pub fn first_parent_chain(repo: &RepoHandle, table: &Arc<CommitTable>, tip: Oid, stop: &dyn Stop) -> Result<Vec<CommitHandle>, WalkError>;
pub fn previous_tips(repo: &RepoHandle, refname: &RefName, cap: usize) -> Result<Vec<Oid>, ReflogError>;   // old values, newest first, deduped, capped; no reflog = Ok(vec![])

// graft-git::diff (diff-parser: types + parsers + pairs; diff-reads: DiffSpec/DiffOptions/list_files/file_diff)
pub enum DiffSpec { Commit { commit: Oid, against: Option<Oid> }, Range { base: Oid, head: Oid }, WorkingTreeVsHead }
pub struct DiffOptions { pub context: u8, pub ignore_whitespace: bool }   // -M -C always on
pub enum FileStatus { Added, Deleted, Modified, Renamed { similarity: u8 }, Copied { similarity: u8 }, TypeChanged, Unmerged, Untracked }
pub struct FileEntry { pub path: Vec<u8>, pub old_path: Option<Vec<u8>>, pub status: FileStatus, pub added: Option<u32>, pub removed: Option<u32> }  // None: binary
pub struct FileList { pub files: Vec<FileEntry>, pub added: u64, pub removed: u64 }
pub enum LineKind { Context, Added, Removed }
pub struct DiffLine { pub kind: LineKind, pub text: Vec<u8>, pub old_no: Option<u32>, pub new_no: Option<u32>, pub no_newline_at_eof: bool }
pub struct Hunk { pub old_start: u32, pub old_lines: u32, pub new_start: u32, pub new_lines: u32, pub heading: Vec<u8>, pub lines: Vec<DiffLine> }
pub struct FileDiff { pub entry: FileEntry, pub hunks: Vec<Hunk>, pub binary: Option<BinaryNote>, pub truncated: Option<Truncated>, pub old_mode: Option<u32>, pub new_mode: Option<u32> }
pub fn parse_unified(bytes: &[u8]) -> Result<Vec<FileDiff>, ParseError>;                // pure; fuzz target
pub fn parse_file_list(raw_z: &[u8], numstat_z: &[u8]) -> Result<FileList, ParseError>; // pure; fuzz target
pub fn list_files(repo: &RepoHandle, spec: &DiffSpec, opts: &DiffOptions, stop: &dyn Stop) -> Result<FileList, DiffError>;
pub fn file_diff(repo: &RepoHandle, spec: &DiffSpec, entry: &FileEntry, opts: &DiffOptions, max_lines: usize, stop: &dyn Stop) -> Result<FileDiff, DiffError>;
pub struct Pair { pub removed: Range<usize>, pub added: Range<usize> }   // index ranges into hunk.lines; side-by-side and word diff both key on it
pub fn pairs(hunk: &Hunk) -> Vec<Pair>;

// graft-ui::syntax + diff_words (syntax-highlighting; syntect + similar, on sanitised &str, char-index spans)
pub enum SyntaxRole { Plain, Comment, String, Keyword, Type, Function, Number, Punctuation, Attribute }
pub fn language_for_path(path: &str) -> Option<Language>;  pub fn language_for_first_line(line: &str) -> Option<Language>;
pub struct SyntaxSpan { pub range: Range<usize>, pub role: SyntaxRole }
pub fn highlight(lang: &Language, lines: &[&str]) -> Vec<Vec<SyntaxSpan>>;   // state carried line to line; > 5,000 lines → Plain
impl Theme { pub fn syntax(&self, role: SyntaxRole) -> TokenColor; }
pub struct WordSpans { pub removed: Vec<Vec<Range<usize>>>, pub added: Vec<Vec<Range<usize>>> }
pub fn word_spans(removed: &[&str], added: &[&str]) -> WordSpans;

// graft-ui::detail (detail-model-and-jobs: types; detail-pane-frame / diff-widget: widgets)
pub struct DetailView { pub header: DetailHeader, pub files: Option<Arc<FileListView>>, pub body: BodyView, pub body_top: usize, pub focus: DetailFocus, pub mode: DiffMode, pub options: DiffOptionsView, pub explain: Option<ExplainView> }
pub enum Subject { None, Commit { sha: SanitizedText }, Compare { base: SanitizedText, head: SanitizedText }, WorkingTree { staged: u32, unstaged: u32, untracked: u32 } }
pub enum ContainedIn { NotAsked, Checking, Known { branches: Vec<SanitizedText>, tags: Vec<SanitizedText>, more: usize }, Failed(SanitizedText) }
pub enum RadarLine { Finding { severity: Severity, title: SanitizedText }, EvilUnchecked, EvilChecking, EvilClean, EvilNotCheckable(SanitizedText), ForcePushNotCheckable(SanitizedText) }
pub enum FileRowKind { Message { lines: usize }, Dir { collapsed: bool }, File { status: FileStatusView, added: Option<u32>, removed: Option<u32> } }
pub enum BodyView { Empty(SanitizedText), Message { lines: Vec<SanitizedText> }, Loading(SanitizedText), Failed(SanitizedText), Diff(Arc<DiffView>) }
pub struct DiffView { pub path: SanitizedText, pub old_path: Option<SanitizedText>, pub status: FileStatusView, pub hunks: Vec<HunkView>, pub binary: Option<SanitizedText>, pub truncated: Option<(usize, usize)>, pub untracked: bool }
pub struct LineView { pub kind: LineKindView, pub old_no: Option<u32>, pub new_no: Option<u32>, pub text: SanitizedText, pub words: Vec<Range<usize>>, pub syntax: Vec<SyntaxSpan>, pub pair: Option<u32> }
pub enum DetailFocus { Graph, Files, Diff }   pub enum DiffMode { Unified, SideBySide }
pub enum DetailLayout { Right { columns: u16 }, Below { rows: u16 }, Tab { shown: bool } }   // detail-pane-frame
pub struct MarkerView { pub severity: Severity }   // GraphEntry::marker: Option<MarkerView>; GraphView::marker_column: bool (radar-context-and-markers)

// graft-core::mistake_radar (mistake-radar-detection)
pub enum Severity { Info, Warning }
pub enum MistakeRadarKind { BackwardMerge, FoxtrotMerge, AccidentalPullMerge, EvilMerge, DuplicateCommit, ForcePushedRemote, RiskyState }
// graft-ops::mistake_radar (mistake-radar-detection; ops gets its MAP.md here)
pub struct CommitFacts<'a> { pub id: Oid, pub parents: &'a [Oid], pub subject: &'a str, pub on_default_chain: bool, pub parents_on_default_chain: &'a [bool] }
pub struct RepoFacts<'a> { pub default_branch: Option<&'a str>, pub remotes: &'a [&'a str], pub previous_default_tips: &'a [Oid] }
pub enum UpstreamFacts { None, InSync, Apart { ahead: u32, behind: u32 } }
pub struct BranchFacts<'a> { pub name: &'a str, pub tip: Oid, pub is_default: bool, pub upstream: UpstreamFacts }
pub enum HeadFacts<'a> { Branch(&'a str), Detached(Oid), Unborn }
pub struct Policy;   // Option<Severity> per kind; Default = PROMPT §1.4; Clone (map-view-data carries one into its job)
pub struct Finding { pub kind: MistakeRadarKind, pub severity: Severity, pub at: Oid, pub evidence: Evidence }
pub enum Evidence { Foxtrot { merge: Oid, first_parent: Oid, real_tip: Oid, branch: String }, Backward { merge: Oid, default_tip: Oid, into: Option<String> },
    AccidentalPull { merge: Oid, branch: String, remote: String }, Risk(Risk), Evil { files: Vec<Vec<u8>> }, Duplicate { copies: Vec<Oid> }, ForcePushed { remote_ref: String, previous: Oid, current: Oid } }
pub enum Risk { DetachedHead, UnpushedOnDefault { branch: String, ahead: u32 }, NoUpstream { branch: String } }
pub fn merged_branch_from_subject(subject: &str) -> Option<MergedName>;
pub fn classify_commit(c: &CommitFacts, repo: &RepoFacts, policy: &Policy) -> Option<Finding>;   // pure, O(1); paint-time lens
pub fn classify_refs(branches: &[BranchFacts], head: &HeadFacts, policy: &Policy) -> Vec<Finding>;
pub struct Explanation { pub title: String, pub paragraphs: Vec<String>, pub evidence: Vec<(Oid, String)>, pub highlight: Vec<Oid> }
pub fn explain(f: &Finding, short: &dyn Fn(Oid) -> String) -> Explanation;
pub enum EvilVerdict { Clean, Evil { files: Vec<Vec<u8>> }, NotCheckable(String) }      // probes (pass 2) fill: probe_evil_merge / probe_force_push / probe_duplicates
pub enum ForcePushVerdict { NotCheckable(String), Clean, Rewritten { remote_ref: String, previous: Oid, current: Oid } }
pub struct Duplicates { pub groups: Vec<Vec<Oid>>, pub window: u32, pub complete: bool }

// graft (app)::update::detail + jobs::detail (detail-model-and-jobs)
pub enum DetailSpec { Commit(Oid), Compare { base: Oid, head: Oid }, WorkingTree }
pub enum Loading<T> { Idle, Pending(JobId), Ready(T), Failed(String) }
pub struct DetailState { pub spec: Option<DetailSpec>, pub shown: bool, pub focus: DetailFocus, pub files: Loading<Arc<FileListView>>, pub diff: Loading<Arc<DiffView>>,
    pub diff_cache: Bounded<DiffKey, Arc<DiffView>> /* 8 */, pub contained: Bounded<Oid, ContainedIn> /* 64 */, pub options: DiffOptions, pub mode: DiffMode, pub word_tint: bool, pub max_lines: usize, pub body_top: usize }
// Task::{ListFiles, FileDiff, ContainedIn, RadarContext, RadarProbe, Map}; Product::{Files, FileDiff, ContainedIn, RadarContext, Probe, Map}; Command::{ListFiles, ReadFileDiff, ContainedIn, BuildMap, Cancel(JobId), …}
pub fn assemble_diff(diff: &FileDiff, word_tint: bool) -> DiffView;   // job thread: sanitise → pairs → word_spans → highlight

// graft (app)::update + update::selection (working-tree-row-selectable, spec fix 2026-09-17) — the selection may rest on the virtual uncommitted row; History's cursor model is untouched
// AppState: + pub on_working_tree: bool;  impl AppState { pub fn uncommitted_row_shown(&self) -> bool; pub fn selected_spec(&self) -> Option<DetailSpec>; }   // Enter opens selected_spec(); the pane follows it
pub fn move_from_graph(state: &mut AppState, motion: Motion) -> Option<Motion>;   // virtual-row rules first; the motion History still applies, or None
pub fn reconcile(state: &mut AppState, cursor_before: usize) -> bool;            // end of every update: flag off when the row is gone or the history cursor moved
pub fn follow_selection(state: &mut AppState, next_id: &mut impl FnMut() -> JobId) -> Vec<Command>;

// graft (app)::map + jobs::map + update::map (map-view-data, pass 2c) — one bounded date walk per local branch into fixed buckets; no lanes
pub struct MapRequest { pub buckets: usize, pub clock: Clock, pub policy: Policy, pub previous_default_tips: Vec<Oid> }
pub enum MapMark { Tag(Vec<u8>), Radar(Severity) }
pub struct MapBranch { pub name: RefName, pub identity: BranchIdentity, pub tip: Oid, pub tip_date: i64, pub counts: Vec<u32>, pub is_current: bool, pub is_default: bool, pub merged_into: Option<(usize, usize)>, pub marks: Vec<(usize, MapMark)>, pub complete: bool }
pub struct MapData { pub branches: Vec<MapBranch>, pub more: usize, pub from: i64, pub to: i64, pub buckets: usize, pub complete: bool }
pub fn build_map(store: &RepoStore, req: &MapRequest, progress: &mut dyn FnMut(u32, u32), stop: &dyn Stop) -> Result<MapData, MapError>;
pub struct MapState { pub shown: bool, pub data: Loading<Arc<MapData>>, pub progress: Option<(u32, u32)>, pub selected: usize }   // AppState::map
// graft-ui::map_screen (map-view-screen)  MapView { title_branch, span_days, rows: Vec<MapRowView>, max_count, buckets, selected, more, complete, status: MapStatus, ascii }; MapScreen widget; ramp_glyph(count, max, ascii)
// graft-ui::overview_strip (overview-strip)  StripMarks { rows_total, viewport: Range<usize>, marks: Vec<(usize, StripMark)> }; StripMark { Merge, Radar(Severity), SearchHit, Head }; cell_for / row_for_cell; OverviewStrip widget; AppState::strip_shown
// graft::repos (refresh-reopens-walk)  RepoStore::refresh(&mut self) -> Result<Refreshed { table_rebound: bool }, StoreError>
// graft::update::Harness (acceptance)  + prelude: Vec<KeyPress>
```

Callers: `jobs::detail` runs `list_files`/`file_diff`, then `assemble_diff` on the job
thread and hands back one `Arc<DiffView>`; `view::rows` calls `classify_commit` per
visible row against `AppState::radar` (default-chain handles + ref facts, one job) and
fills `GraphEntry::marker`; `update` asks `explain` on `.`→Explain; `view::detail`
builds `DetailView`; trailers (`Co-authored-by`) are parsed in the app assembler.
`jobs::map` runs `build_map` (which computes its own default chain through
`first_parent_chain`, so the map never waits on the radar context); `view::map` and
`view::strip` assemble the two new views from `AppState` alone.

**Signature fix (pass 2b):** `RadarLine` gains `EvilNotCheckable(SanitizedText)`.
The original enum had no way to show an evil-merge probe's `NotCheckable` verdict
(octopus merge, git too old, remerge failed) — only `ForcePushNotCheckable`
existed, which is worded for the wrong probe. `radar-probe-evil-merge` needs it.

**Signature fixes (pass 2c):** `StripMark::Head(char)` → `StripMark::Head` — M1's
graph column draws no lane letters (`graft-ui` MAP.md `GraphLine`), so the `char`
had nothing to carry. `MapBranch` gains `tip_date`, `is_default` and `complete`,
`MapData` gains `more` and `complete` (the 200-row and 50,000-commit caps must be
visible). `Policy` must be `Clone` (a derive, not a shape change).

## Hard-to-reverse choices

1. **Diffs from system git through a typed runner, per file on demand.**
   Options: (a) gix tree diff + own unified rendering; (b) `git diff-tree` parsed
   (PROMPT Technology). **(b)**: follows the user's diff config, renames and
   copies for free; the runner (`GitInvocation`: typed args, `-z` byte paths,
   kill-on-stop, output cap) is what M3–M6 writes build on, so its shape is the
   real decision. File list first (`--raw -z --numstat -z`), hunks only for the
   selected file — a 2,000-file merge on git/git costs one small spawn.
2. **Mistake Radar is a paint-time lens over facts, not a layout input.**
   Options: (a) DESIGN §8.2: `graph::layout` takes a `Map<CommitId, Kind>` from
   a whole-history scan (O(history) subject reads before any marker shows;
   re-layout when it lands); (b) `classify_commit` per hydrated row, O(1), with
   one job building the global context (default-branch first-parent chain; ref
   facts from `RefBadges`; previous default tips from its reflog), plus lazy
   probes (evil per selected merge, force-push per tracked remote, duplicates
   over a 500-commit window). **(b)**: markers appear with the rows, cost nothing
   off-screen, and the fixture test still classifies every commit of every shape.
3. **"Contained in" via `git for-each-ref --contains <oid>`.** Options: (a) own
   reachability index over `CommitTable` (a ticket of its own, O(history)
   memory); (b) gix `revision` feature (drags gix-index, rejected in M1); (c) one
   git spawn per selection, cached 64 deep, cancelled on move. **(c)**; result
   bounded to "main, release/2.1 +47 more". PROMPT Technology says gix for reads:
   this is a query gix lacks cheaply (ADR-0011 names such gaps). Flagged below.

## Where I'd differ from DESIGN.md (Proposed changes)

| Change | Text | Trade-off |
|---|---|---|
| Markers are a paint-time lookup; `Row.marker` stays unused (remove in M8 pass) | DESIGN §8.2 "layout takes a `Map<CommitId, MistakeRadarKind>`", §3.1 `Row.marker` | strip/map see findings only for rows on hand (§5.3 already accepts this) |
| No column glyph for an unchecked merge; the pane's radar line says `evil merge: unchecked` → `checking…` → `clean`/finding | PROMPT §1.4 "the marker reads unchecked" | a `?` on every merge of git/git would be the loudest thing on screen |
| Word and syntax spans computed on sanitised text in `ui`; `git` keeps line pairing only | DESIGN §8.2 `WordDiff` in `git` | byte offsets survive sanitising (`�`/`␛` change lengths); ui gains `similar` + `syntect` |
| "Contained in" through the git binary | PROMPT Technology "gix for reads" | one spawn per selection, off-thread, cached |
| Map view draws text glyphs at every tier in M2; pixel curve deferred | DESIGN §5.1 tiers | a toggle the user reaches rarely; Unicode is the daily view |
| Map view rows are local branches only; a remote never collapses into its upstream's row; the merge elbow marks merges into the default branch only, at the tip's own bucket | DESIGN §5.1 "Shows" | one walk per local branch, no merge-commit search; remotes and cross-branch merges are M8 polish gap rows |
| No `[ Fix… ]` in the Explain panel until M5 | DESIGN §1.8 | nothing implies Fix exists |
| Keymap: `x` mark for compare; `Tab` = Graph→Files→Diff; `<`/`>` shrink/grow the pane on the focused divider (ADR-0028; was `Ctrl+arrows`, which macOS takes); `L` load the rest of a truncated diff; `Enter` opens the pane and drops the peek (ADR-0023) | DESIGN §2.2/2.3/2.5 | four new letters plus one bracket pair; `+`/`-` stay off (`+` unfolds lanes) |
| `Ctrl+R` after `B` rebinds the commit table and restarts the walk on the graph path, re-anchored on the selection; `Written` footer hints `Ctrl+R to use it`; no automatic restart | KNOWN_GAPS row 54 | the user asks; an unasked restart mid-scroll is worse than a hint |
| KNOWN_GAPS expiries: snapshot-cost row (52) and older-stash badges (175) → M3 (no staging or stash list exists in M2); `full_topological` residual (227) → not planned | KNOWN_GAPS.md | honest owners |
| **`?` help overlay (DESIGN §1.16) is not built in M2; no footer ever shows `? help` until it exists** | DESIGN §1.1–1.3 mockups' `? help` footer hint | keys off the footer (`Space` `1` `H` `x` `w W v [ ]` `<>`) are learned from README/TESTING.md until M8 (spec fix 2026-09-18, below) |

## New UI

**Detail pane** (positions per DESIGN §1.1–1.3; content here). Regions:
*header* (fixed), *files* (list, focusable), *body* (message or diff, focusable).
- **Wide 160+:** right column, 42 % of width (min 48 cols), divider `│`.
  Header 4–5 lines: `COMMIT a1b2c3d` (accent when focused) · `Rui Mehdi <r@x> · 2026-09-13 14:02 +0500 · 2 min ago`
  (+ `committed by …` when different; `with: <co-authors>`) · `parents 9f8e7d6, 3c2b1a0 · diff vs first parent` ·
  `Contained in: main, release/2.1 +3 more` (`checking…` while the job runs) · radar line when any.
  Files below the header, up to 40 % of the column: first entry `✉ message (14 lines)` when the body has a
  non-trailer line, then a tree split on `/` (dirs collapsible with `Space`; `M/A/D/R/C/T` letter + `+12 −4` right-aligned,
  binary shows `bin`). Body takes the rest: the message when the cursor is on `✉`, else the selected file's diff. Cursor starts on the first file.
- **Medium 100–159:** pane below the graph, 40 % of height (min 8 rows), divider `─`. Header 2 lines
  (`COMMIT a1b2c3d · Rui M · 2 min ago · parents 9f8e7d6 · Contained in: main` / subject + radar). Files left (30 cols) | body right.
- **Narrow <100:** tab strip `[Graph] Detail` (DESIGN §1.3); Detail tab = header 3 lines, files ≤ 40 % height, body below.
- **Eye lands on:** the tinted diff lines; then the short SHA + subject. Chrome is rules, not boxes; the focused region's title is accent, others `text-muted`.
- **Keys:** `Enter` on a graph row → focus Files; `Tab`/`Shift+Tab` cycle Graph→Files→Diff; `Esc` → back to Graph (closes Explain first);
  `j/k ↑↓` move (files: loads that file's diff; body: scroll), `PgUp/PgDn/g/G`, wheel in each region; `Enter` on a file → focus Diff;
  `Space` folds a dir; `w` ignore-whitespace, `W` word tint, `v` unified/side-by-side, `[`/`]` context 0–20 (each `w`/`[`/`]` re-reads the file, options are per session and in `[diff]` config);
  `L` loads the rest past `max_lines` (5,000); `y` copies the SHA anywhere; divider drag, `<`/`>` shrink/grow the pane in 4-cell steps on the **focused divider** — the layout's one divider: columns in `Right`, rows in `Below`; a no-op in the `Tab` layout, with the pane hidden or the Map shown (ADR-0028) — remembered per orientation in `<common_dir>/graft/layout`; hints list `<> resize` at ≥ 100 columns.
- **States:** no selection → `select a commit` muted; loading → header from the row at once, files `reading…`; error → `git diff-tree: <first stderr line>` in `warning`, hints stay;
  binary → `binary · 12 KiB → 13 KiB`; truncated → `… 4,120 more lines · L load all`; untracked file under the working-tree spec → `untracked · no diff yet (M3)`.
- **Diff lines:** gutter `old new` numbers (width from the last line number, min 3), sign cell, text. `added`/`removed` soft bg, changed words a stronger bg (`W`), syntax roles as fg, hunk heading `@@ … @@ fn refresh()` in `text-muted`
  pinned on the top line while its hunk is scrolled through. CRLF shown as `␍` at the end. At 16 colours/`NO_COLOR`: sign + bold/dim only (m1/12c rule).
  Side-by-side: two halves, own gutters, `pairs()` alignment, unpaired lines blank on the other side, `│` between.
- **Compare (`x`):** `x` on row A marks it (accent `▏` left rule on that row; footer `compare base a1b2c3d — move, x again clears`); the pane follows the cursor:
  `COMPARE a1b2c3d → 7f6e5d4 · 14 files +120 −33`, files = `git diff-tree base head`. `Shift`/`Ctrl`+click a second row does the same. `Esc`/`x` on the base clears.
- **Working tree row selected:** `UNCOMMITTED · 2 staged 1 unstaged 1 untracked` (a zero count omitted) / `diff vs HEAD a1b2c3d`, no author, date, contained or radar lines; one diff vs HEAD (read-only; staging is M3).
  **Reaching it** (`working-tree-row-selectable`): the row stays where m1/12b put it — above HEAD, only while HEAD is the top visible history row (or unborn) and the tree is dirty. The selection is a flag beside `History`'s cursor (`AppState::on_working_tree`), never a history row:
  `k`/`↑` from the top visible row (cursor on HEAD) lands on it, the highlight moves up, HEAD's row un-highlights; `j`/`↓` returns to HEAD; `PgDn`/`G` leave it and move as usual; `k`/`PgUp`/`g` on it do nothing at the very top, else scroll up with the row leaving alongside HEAD;
  a click on its screen row selects it. It clears itself whenever the row is no longer shown (tree clean after `Ctrl+R`, HEAD moved, wheel scrolled HEAD off the top) or the history cursor moved (search hit, click, `G`), and the pane follows to HEAD's commit.
  On it: `Enter` opens the working-tree diff; `y` copies nothing; `x` is a no-op and a live compare shows the working tree with the base kept; `.` lists `Open details` only; Focus mode never fades it.

**Marker column** (graph screen): 2 cells between the graph column and badges, present only while the repo has any finding
(stable within a session, never shifts on scroll). `⚠` in `warning`, `ⓘ` in `info`; ASCII `!`/`i`; a marked row's footer hint gains `. explain`.

**Explain panel** (`.` → Explain, or right-click; only on a marked row): replaces files+body while open. Title `⚠ FOXTROT MERGE`
(severity colour, caps), paragraphs wrapped to width, then the evidence sketch (three lines like DESIGN §1.8: labelled short SHAs joined by `──╮`/`──╯`, `(this merge)`),
footer `Esc close`. The main graph fades every row not in `Explanation::highlight` (Focus-mode vocabulary; `Esc` restores). No Fix… button.

**Context actions (`.`, right-click):** popup list anchored under the row (bottom-left of the graph when no room): `Explain` (marked rows), `Copy SHA`,
`Compare with this (x)`, `Open details`. `j/k/Enter/Esc`, click. Width = longest item + 4, `surface-raised` bg, accent cursor.

**Map view (`M`)** — as DESIGN §5.1; fixed here: rows = local branches (current first, then default, then by tip date; cap 200, `+N more` in the footer), one column per bucket, `buckets = max(10, width − 24)`, time span = oldest tip… now, clamped to 1 day…366 days;
Unicode `▁▂▃▄▅▆▇█` by count/max (a non-zero count never blank), ASCII ` .:-=+*#%@`, colour = branch identity; the default row counts every commit in the span, any other row only commits off the default first-parent chain (a merged branch keeps its bars);
`●╮` (ASCII `o\`) at the tip's bucket when the tip is reachable from the default tip; tag marks `┴` + `[tag]` and radar marks `⚠`/`ⓘ` over the bucket; `▸` + ` ●` and the `NOW` header for the current branch; ` (active)` on a row whose tip is in the newest bucket; ` · partial` in `warning` when a cap was hit.
Header `MAP · <branch> · last N days|months` / `BRANCH … NOW`; footer `↑↓ select  ⏎ zoom to commits  M/Esc back`. `↑↓` select, `Enter` → `History::seek(tip)` in the normal graph, `M`/`Esc` back (data discarded). Empty: `no local branches`. Loading: `building map… n/N branches`, `Esc cancels`. Failed: the line in `warning`. Resize re-requests. No mouse in M2 (gap row).
Text glyphs at every tier in M2 (Proposed above).

**Overview strip (`O`)** — as DESIGN §5.3; fixed here: one column, the graph area's rightmost column in every layout (narrow: the screen edge; wide/medium with the pane shown: just left of the detail divider); bucket = `ceil(rows_total / strip_rows)` with `rows_total = Window::laid_out` (honestly incomplete while streaming); priority when a bucket holds several: `⚠` > `ⓘ` > `◆` search > `●` HEAD > `◉` merge > `█` viewport (only where nothing else drew) > `┆`; ASCII `! i * o + # :`; blank past the last bucket;
merges and radar known for resident rows only; not drawn while Map view is shown (no restore state needed); off by default, `O` flips it; click on the strip selects its bucket's first row.

**`?` help overlay:** not in M2 (spec fix 2026-09-18, below). Nothing in M2 draws one or hints at one.

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

## Genuinely risky tickets

- **`mistake-radar-detection`** — Fable (signature). The foxtrot/backward/accidental rules rest on one heuristic
  (merged-branch name from the subject + "is on the default chain"); a wrong rule is a wrong product, and no test can say the rule is right, only that the fixtures agree.
- **`git-runner`** — Opus. Structural for every M3–M6 write; judged by tests → not Fable.
- **`detail-model-and-jobs`** — Opus. Cancellation and stale-product rules shape six later tickets; testable → not Fable.
- No other ticket passes all three Fable tests. **Planned Fable builds: 1** (+ acceptance).

## Tickets

**Pass 1 (written):** `git-runner` (O) · `diff-parser` (S) · `diff-reads` (S) · `syntax-highlighting` (S) · `mistake-radar-detection` (F sig) · `detail-model-and-jobs` (O).

**Pass 2a (written):** `detail-pane-frame` (S) · `diff-widget` (S) · `side-by-side-view` (S) · `detail-keys-and-focus` (S) · `compare-two-commits` (S) · `context-actions-menu` (S).

**Pass 2b (written):** `radar-context-and-markers` (S: `first_parent_chain`/`previous_tips` in git, `RadarContext` job, `view::radar` facts + `findings_for`/`marker_for`/`radar_lines_for`, marker column, `[radar]` config, pane radar line rendering) ·
`radar-probe-evil-merge` (O: `probe_evil_merge` — `git show --remerge-diff --format= --name-only`, `Bounded` cache, unchecked/checking/clean/not-checkable) ·
`radar-probe-force-push` (O: remote-tracking reflog via `previous_tips`, `merge-base --is-ancestor` via `git-runner`, `NotCheckable` on an empty reflog) ·
`radar-probe-duplicates` (S: `log --all -n 500 -p` piped as stdin into `patch-id --stable` via `GitInvocation::stdin`, groups, "checked newest 500") ·
`explain-panel` (S: `ExplainView` from `explain`, the evidence sketch, `DetailState::explain_requested` consumed, graph highlight/fade, `Esc` priority).

**Pass 2c (written):**
`map-view-data` (S: `graft::map` — `MapData`/`build_map`, one date walk per local branch, default-chain exclusion, caps; `jobs::map`; `update::map` with `M`/`Esc`/`↑↓`/`Enter`/resize) ·
`map-view-screen` (S: `graft-ui::map_screen` — `MapView`/`MapScreen`/`ramp_glyph`; `view::map`; frame swap; captures) ·
`overview-strip` (S: `graft-ui::overview_strip` — `StripMarks`/`cell_for`/`row_for_cell`/widget; `view::strip`; `AppState::strip_shown` + `O`; the graph area's last column; click) ·
`refresh-reopens-walk` (S: `RepoStore::refresh -> Refreshed { table_rebound }`, walk restart re-anchored on the selection, `Written` footer hint; closes KG 54) ·
`docs-sweep` (S: bare `cargo run --` → `-p graft` in README/TESTING/clone script, KG closures and expiries, five new gap rows, README/CHANGELOG M2 entries) ·
`acceptance-details-and-radar` (F: `Harness::prelude`, gallery `--group details` with 19 stops, TESTING.md "Details and diffs", `docs/shots/m2-*` at 160×40/100×30/80×24, matrix; **two contexts by design** — (a) harness + group, (b) entries + captures).

**Spec-only pass (2026-09-17, written):** `working-tree-row-selectable` (S: `AppState::on_working_tree` + `uncommitted_row_shown`/`selected_spec`, `update::selection` rules, key/click arms, highlight; Needs `detail-model-and-jobs`; goes before `detail-pane-frame`, `compare-two-commits`, `context-actions-menu`, whose Needs now name it).

**Can't fit under 80k:** only `acceptance-details-and-radar`, and it says so with its seam; the conductor files part (b) when (a) hands off.

**Spec fix (2026-09-17, ADR-0028):** detail-pane resize keys are `<`/`>` (shrink/grow the pane on the focused divider), not `Ctrl+arrows`; the stored size is per orientation (`PaneSize { right_columns, below_rows }`), since a column count from `Right` means nothing as a row count in `Below`. Only `detail-keys-and-focus` changes.

**Spec fix (2026-09-17, working-tree row):** 06 shipped `DetailSpec::WorkingTree` with no way to select the row it belongs to (KNOWN_GAPS m2/06). Options: (a) defer to M3 — leaves a visible dead end in M2 (a row `Enter` ignores) and dead `WorkingTree` code, and M3's `space`/`d` on that row would need the same selection anyway; (b) make the virtual row a `History` row — shifts every row index; (c) a one-bit selection beside `History`'s cursor, M1's placement rule kept. **(c)**, one Sonnet ticket (`working-tree-row-selectable`), ~150 lines in `graft` only. Cost accepted: the row is reachable only while HEAD is the top visible history row (M1's rule), so with a newer branch above HEAD the user scrolls HEAD to the top first — a KNOWN_GAPS row for M3 to decide whether to pin the row above HEAD wherever it sits. Fable build count unchanged (1 + acceptance). Touched specs: `detail-pane-frame` (`UNCOMMITTED` header from `selected_spec()`), `compare-two-commits` (`x` no-op on the row; live compare keeps the base), `context-actions-menu` (`for_working_tree()`, `oid: Option<Oid>`), `acceptance-details-and-radar` (one TESTING.md entry, no gallery stop).

**Spec fix (2026-09-18, `?` help): deferred to M8, no new ticket.** 10b's group-2 line "`?` help" was a slip of mine: it named a screen this note never designed, and two builders confirmed no help surface exists. Options: (a) build DESIGN §1.16 now as a hand-typed key list per screen; (b) M8. **(b)**: PROMPT §8 already puts "keybinding help and remapping" in M8, and §1.16 defines the overlay as the §2 keymap table *filtered to the focused pane* — that table is the remapping data structure M8 builds, so (a) is a throwaway list rewritten in M8, in a read-only milestone already at 25 tickets with two partials. Cost: until M8 the keys the 80-column footer can't fit (`Space` `1` `H` `x` `w W v [ ]` `<>` `L` `M` `O`) are learned from README/TESTING.md; the acceptance ticket's TESTING.md entries carry them. Rule while the gap stands: **no footer or mockup-derived hint shows `? help`** (none does today, checked against `docs/shots/`, README, TESTING.md); `?` stays unbound rather than bound to nothing. Touched: `detail-keys-and-focus`/10b's `?` line closes as deferred; KNOWN_GAPS rows 199 ("M1, with the help overlay") and m2/10b re-owned to M8. For M8's designer: `crates/graft/src/update/` has a purity guard that rejects any mention of ratatui or `graft_ui` types in that module, doc comments included (10b hit it) — the overlay's geometry and widget live in `graft-ui` (`help_overlay`, sibling of `map_screen`); `update` holds only a `help_shown: bool`-shaped state and the focused-pane id, and the keymap table itself sits in `graft-core` or `graft-ui`, never in `update`.
