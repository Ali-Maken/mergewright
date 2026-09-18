# M2 design pass: details and diffs (2026-09-15) — Interfaces and data shapes

[See also: m2-details-and-diffs/README.md](README.md) for the full section index.
Previous: [Sizing from M1](01-sizing-from-m1.md) · Next: [Hard-to-reverse choices](03-hard-to-reverse-choices.md)

---

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
