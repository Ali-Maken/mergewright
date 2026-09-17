# detail-model-and-jobs: detail state, its jobs and the view-model types, no drawing

Status: partial
Blocked by: 01, 02, 03, 05

Status: draft
Build with: Opus — cancellation, stale-product rules and the update-purity guard are where a wrong shape becomes structural for six pass-2 tickets.
Needs: `git-runner` (`Stop`, `contained_in`, `Contained`), `diff-reads` (`DiffSpec`, `DiffOptions`, `list_files`, `file_diff`, `DiffError`), `diff-parser` (`FileList`, `FileEntry`, `FileDiff`, `pairs`), `syntax-highlighting` (`language_for_path`, `language_for_first_line`, `highlight`, `word_spans`, `SyntaxSpan`)

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — Sizing from M1; Interfaces (`graft-ui::detail`, app `detail`); Hard-to-reverse choices 1 and 3; New UI "Detail pane" (regions, states, keys — only `Enter`/`Esc`/`j`/`k` on the files list are wired here) and "Compare" (types only)
- `DESIGN.md` §1.1 lines 43–71 (what the pane must carry), §8.2 `graft` app paragraph (line 1548: `Command`, jobs, `update` pure — no I/O, no clock)
- `ADR-0023` (Enter opens the detail pane in M2, the peek goes)
- `docs/reviews/m1-the-graph-round-2.md` — the update-purity guard finding (grep `purity`)
- `crates/graft/MAP.md` — `jobs` section (`Task`, `Product`, `Progress`, `JobId`, `JobFailure`, the pool), `Command` (line 128), `AppState` fields (92–121), `History::selected`/`row_at`/`select_at` (299–301), `RepoStore` (184–192)
- `crates/graft-ui/MAP.md` — `graph_screen` (`GraphView`, `GraphEntry`) header lines only; `crates/graft-term/MAP.md` lines 71–72 (`sanitize*`)
- `crates/graft-git/MAP.md` — `run`, `contained`, `diff` sections; line 56 (`read_commit`); `crates/graft-core/MAP.md` lines 22–27

## Scope

In:
- **ui types** — `crates/graft-ui/src/detail.rs` (`pub mod detail`): the view-model types below, `Default`/constructors, no widget. `ui` learns nothing about git: every string is `SanitizedText`, every path already sanitised.
- **app state** — `crates/graft/src/update/detail.rs` (new submodule of `update`; `mod.rs` gains one `mod` line and the dispatch arms below, nothing else moves): `DetailSpec`, `Loading`, `DetailState`, `DiffKey`, `Bounded` (a `VecDeque`-backed bounded map, evict oldest), and the transitions:
  - `open(spec)` (from `Enter` on a graph row: `Commit(selected id)`; Enter on the working-tree row: `WorkingTree`): sets `spec`, `shown = true`, `focus = Files`, `files = Pending`, emits `Command::ListFiles(id, spec, options)`; cancels a pending list job (`Command::Cancel(old)`).
  - Selection moves while `shown`: same, keeping `focus`. Selection moves while hidden: nothing (the pane costs nothing when off).
  - `Product::Files(id, result)`: dropped unless `id` is the pending one; on `Ok` builds `FileListView` (tree from `/`-split paths, `✉ message` entry first when the commit body has a non-trailer line — `co_authors` and `Signed-off-by`/`Co-authored-by`/`Reviewed-by` are trailers), `cursor = 0`, then asks for the first file's diff (`Command::ReadFileDiff`), or the message body when the first entry is `✉`.
  - Files cursor move (`j`/`k`/`↑`/`↓` while `focus == Files`): `diff_cache` hit → `Ready` at once; else `Pending` + `Command::ReadFileDiff(id, spec, entry, options, max_lines)`; the previous pending diff job is cancelled.
  - `Product::FileDiff(id, key, result)`: stale-dropped by `id`; `Ok` → cache (`Bounded`, 8) + `Ready`; `Err` → `Failed(display string)`.
  - `Product::ContainedIn(oid, result)`: cached (`Bounded`, 64) whatever the selection now is; `open` asks for it once per oid.
  - `Esc`: `focus = Graph`, `shown = false` (the pane closes; a second `Esc` does what it did in M1). `Enter` while `focus == Files` → `focus = Diff`. `Tab`/`Shift+Tab` cycle Graph→Files→Diff→Graph when shown. Every other key of the pane is `detail-keys-and-focus`.
  - `Ctrl+R`/refresh: `diff_cache` and `contained` cleared, `spec` kept and re-asked.
  - Options (`context`, `ignore_whitespace`, `DiffMode`, `max_lines` 5,000) live in `DetailState` with `Default`; no key changes them here.
  - **The peek goes** (ADR-0023): `AppState::peek` removed with its footer reveal and tests; the hover reveal stays. If removing it spreads beyond `update/keys.rs`, the footer in `view`, and their tests, keep the field, leave `Enter` opening the pane, and report `partial` with that seam.
- **jobs** — `crates/graft/src/jobs/detail.rs` (or `jobs/kinds/detail.rs` if `kinds` is a directory): `Task::{ListFiles, FileDiff, ContainedIn}`, `Product::{Files, FileDiff, ContainedIn}`, `Command::{ListFiles, ReadFileDiff, ContainedIn, Cancel(JobId)}`. The runner: opens `RepoStore::shared().handle()`, runs the read with the job's stop flag, then **assembles the view model on the job thread**: sanitise every line (`sanitize_line`) and path (`sanitize_path_bytes`), `pairs` → `LineView::pair`, `word_spans` on paired lines, `language_for_path` (then `language_for_first_line` on the first context/added line) → `highlight` over the new-side text of every line (removed lines highlighted with the old-side state, same call, both sides concatenated per hunk is fine), CRLF → trailing `␍` shown by the widget (the `\r` is kept in the sanitised text as U+240D by the assembler). Hands back one `Arc<DiffView>`.
  - `Command::Cancel(id)` flips that job's stop flag if it is still running; a cancelled job's product is dropped by id anyway.
  - Untracked entry (`FileStatus::Untracked`) → `DiffView` with `hunks` empty and `binary: None`, `truncated: None`; the widget words it.
- **`AppState::detail: DetailState`**, `Default` hidden. `view` untouched except the peek footer removal; nothing draws the pane yet (`detail-pane-frame`).
- Tests: see Acceptance. MAP.md lines (app, ui). `update`-purity guard still passes: no `Instant`, no I/O in `update/detail.rs`.

Out: drawing, layout (`DetailLayout::compute` is `detail-pane-frame`), keys beyond the list above, `x` compare (`compare-two-commits`), radar lines (`radar-context-and-markers`), `L` load-all and options keys (`detail-keys-and-focus`), the config file.

## Signatures

```rust
// crates/graft-ui/src/detail.rs
pub enum Subject { None, Commit { sha: SanitizedText }, Compare { base: SanitizedText, head: SanitizedText }, WorkingTree { staged: u32, unstaged: u32, untracked: u32 } }
pub struct DetailHeader { pub subject: Subject, pub summary: SanitizedText, pub author: SanitizedText, pub author_date: SanitizedText, pub relative: SanitizedText,
    pub committer: Option<SanitizedText>, pub co_authors: Vec<SanitizedText>, pub parents: Vec<SanitizedText>, pub against: Option<SanitizedText>, pub contained: ContainedIn, pub radar: Vec<RadarLine> }
pub enum ContainedIn { NotAsked, Checking, Known { branches: Vec<SanitizedText>, tags: Vec<SanitizedText>, more: usize }, Failed(SanitizedText) }
pub enum RadarLine { Finding { severity: Severity, title: SanitizedText }, EvilUnchecked, EvilChecking, EvilClean, ForcePushNotCheckable(SanitizedText) }   // Severity from graft_core
pub enum FileStatusView { Added, Deleted, Modified, Renamed, Copied, TypeChanged, Unmerged, Untracked }
pub enum FileRowKind { Message { lines: usize }, Dir { collapsed: bool }, File { status: FileStatusView, added: Option<u32>, removed: Option<u32> } }
pub struct FileRow { pub depth: u8, pub name: SanitizedText, pub kind: FileRowKind, pub entry: Option<u32> }   // entry: index into the git FileList, None for Message/Dir
pub struct FileListView { pub rows: Vec<FileRow>, pub cursor: usize, pub top: usize, pub files: u32, pub added: u64, pub removed: u64 }
impl FileListView { pub fn visible(&self) -> impl Iterator<Item = (usize, &FileRow)>; pub fn toggle_dir(&mut self, row: usize); pub fn move_cursor(&mut self, delta: isize); pub fn selected_entry(&self) -> Option<u32>; }
pub enum LineKindView { Context, Added, Removed, HunkHeading }
pub struct LineView { pub kind: LineKindView, pub old_no: Option<u32>, pub new_no: Option<u32>, pub text: SanitizedText, pub words: Vec<std::ops::Range<usize>>, pub syntax: Vec<SyntaxSpan>, pub pair: Option<u32> }
pub struct HunkView { pub heading: SanitizedText, pub lines: Vec<LineView> }
pub enum DiffMode { Unified, SideBySide }
pub struct DiffOptionsView { pub context: u8, pub ignore_whitespace: bool, pub word_tint: bool }
pub struct DiffView { pub path: SanitizedText, pub old_path: Option<SanitizedText>, pub status: FileStatusView, pub hunks: Vec<HunkView>, pub binary: Option<SanitizedText>, pub truncated: Option<(usize, usize)>, pub untracked: bool }
pub enum BodyView { Empty(SanitizedText), Message { lines: Vec<SanitizedText> }, Loading(SanitizedText), Failed(SanitizedText), Diff(Arc<DiffView>) }
pub enum DetailFocus { Graph, Files, Diff }
pub struct DetailView { pub header: DetailHeader, pub files: Option<Arc<FileListView>>, pub body: BodyView, pub body_top: usize, pub focus: DetailFocus, pub mode: DiffMode, pub options: DiffOptionsView, pub explain: Option<ExplainView> }
pub struct ExplainView { pub title: SanitizedText, pub severity: Severity, pub paragraphs: Vec<SanitizedText>, pub sketch: Vec<SanitizedText> }   // built by explain-panel; type lives here

// crates/graft/src/update/detail.rs
pub enum DetailSpec { Commit(Oid), Compare { base: Oid, head: Oid }, WorkingTree }
pub enum Loading<T> { Idle, Pending(JobId), Ready(T), Failed(String) }
#[derive(Clone, PartialEq, Eq, Hash)] pub struct DiffKey { pub spec: DetailSpec, pub entry: u32, pub options: DiffOptions, pub max_lines: usize }
pub struct Bounded<K, V> { /* VecDeque<(K, V)>, cap */ }
impl<K: Eq, V> Bounded<K, V> { pub fn new(cap: usize) -> Self; pub fn get(&self, k: &K) -> Option<&V>; pub fn insert(&mut self, k: K, v: V); pub fn clear(&mut self); }
pub struct DetailState { pub spec: Option<DetailSpec>, pub shown: bool, pub focus: DetailFocus, pub files: Loading<Arc<FileListView>>, pub diff: Loading<Arc<DiffView>>,
    pub diff_cache: Bounded<DiffKey, Arc<DiffView>>, pub contained: Bounded<Oid, ContainedIn>, pub options: DiffOptions, pub mode: DiffMode, pub word_tint: bool, pub max_lines: usize, pub body_top: usize }
impl DetailState { pub fn open(&mut self, spec: DetailSpec, next_id: &mut impl FnMut() -> JobId) -> Vec<Command>; pub fn close(&mut self); pub fn selected_entry(&self) -> Option<u32>; }
// AppState::detail: DetailState (field). view::detail_view(&AppState) -> DetailView is detail-pane-frame's; this ticket adds nothing to view but the peek removal.

// crates/graft/src/jobs/detail.rs
// Task::ListFiles { spec }, Task::FileDiff { key }, Task::ContainedIn { oid };  Product::Files(Result<Arc<FileListView>, String>), Product::FileDiff(DiffKey, Result<Arc<DiffView>, String>), Product::ContainedIn(Oid, Result<Contained, String>)
// Command::ListFiles(JobId, DiffSpec, DiffOptions), Command::ReadFileDiff(JobId, DiffKey, FileEntry), Command::ContainedIn(JobId, Oid), Command::Cancel(JobId)
pub fn assemble_diff(diff: &FileDiff, word_tint: bool) -> DiffView;                 // pure; sanitise, pairs, words, syntax
pub fn assemble_files(list: &FileList, message: Option<&[u8]>) -> FileListView;      // pure; tree, ✉ row
```

## Acceptance

- [ ] `update` tests (fixture `Shape::FeatureBranches`, no jobs run): `Enter` on a row → `detail.shown`, `focus == Files`, one `Command::ListFiles` with that row's id; `j` on the graph while shown → `Cancel(old)` + a new `ListFiles`; `Esc` → hidden, no command; a second `Enter` on the same row while shown → no new command.
- [ ] Stale drop: `Product::Files` with an id that is not `detail.files`'s pending id changes nothing.
- [ ] `Product::Files(Ok)` → `files == Ready`, `cursor == 0`, exactly one `ReadFileDiff` (or none when row 0 is `✉` and the body shows the message); `k`/`j` across files issue diffs and hit the cache on the way back (second visit → no command).
- [ ] `assemble_files` on a list with `a/b/c.rs`, `a/d.rs`, `e.rs` gives rows `a/`, `b/`, `c.rs`, `d.rs`, `e.rs` with depths 0,1,2,1,0; with a two-line body the first row is `Message { lines: 2 }`; a body that is only `Signed-off-by:` gives no message row; `toggle_dir` hides `b/`'s children from `visible()`.
- [ ] `assemble_diff` on a hand-built `FileDiff` (one hunk, two paired lines): `HunkHeading` first, `pair` set on both, `words` non-empty on the changed word, `syntax` non-empty for a `.rs` path, an ESC byte in a line arrives as the sanitiser's replacement, a CRLF line ends in `␍`.
- [ ] Jobs test with the pool (fixture temp copy): `Command::ListFiles` → `Product::Files(Ok)` with the fixture's file count; `Command::ReadFileDiff` → `Product::FileDiff(Ok)`; `Command::ContainedIn` → `Product::ContainedIn(Ok)` naming `main`; `Cancel` on a running `FileDiff` for `git-git`-sized input is not tested here (no heavy tier) — the runner's own stop test covers it.
- [ ] Purity guard passes for `update/detail.rs`; edge test passes (ui has no git/ops import: `SyntaxSpan` and `Severity` come from ui/core).
- [ ] `peek` removed (or the seam reported); the M1 hover reveal test still passes.
- [ ] MAP.md: `graft` (update/detail, jobs/detail, `AppState::detail`, `Command`/`Task`/`Product` lines amended) and `graft-ui` (`detail` section).

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · edge test · purity guard.

## Size

Two crates: ui types (~180 lines, no logic beyond `FileListView` methods), app (`update/detail.rs` ~250, `jobs/detail.rs` ~220), tests (~250). This is the milestone's largest ticket; it fits only if the builder reads `jobs` and `update` through MAP.md plus one scout summary of `jobs/kinds` (how a `Task` is run and a `Product` delivered) and of `update/keys.rs`'s `Enter`/`Esc` arms — never those files whole. ≤ 30 tool calls. If `assemble_diff` plus the jobs wiring passes 60 % of the budget before `update/detail.rs` starts, stop, commit the ui types + jobs, and report `partial` with `update/detail.rs` as the seam.

## Handoff (builder-opus, 2026-09-17, stopped at 40-call hard stop; no compaction)

Done (commit below): `graft-ui::detail` types + FileListView methods + tests; `update/detail.rs` (DetailSpec, Loading, DiffKey, Bounded, DetailState transitions, LocalJobs, detail_key, follow_selection, detail_event); `jobs/detail.rs` (assemble_files, assemble_diff, read_* runners, `impl Stop for Cancel`); runner `queue_as` + `dispatch_detail` + `detail_job`; Task/Product/Command variants; peek removed (state, keys, view/rows reveal, 5 update tests, view test now hover-only). `cargo test` 987 passed / 0 failed; fmt clean.

Left:
1. **clippy fails, 2 findings** (`too_many_lines`): `jobs/runner.rs:349` `dispatch` 62/60 and `update/mod.rs:56` `answer` 65/60. Next step: move the three `Command::ListFiles|ReadFileDiff|ContainedIn` arms out of `dispatch` behind one `command @ (...)` arm already calling `dispatch_detail` (check formatting didn't expand it), and in `answer` fold the detail `Produced` arm + detail `Task` names into one call (e.g. route `AppEvent::Started`/`Produced` for detail through a helper before the `match`, keeping exhaustiveness with a `=> {}` arm).
2. Jobs test with the pool on a fixture temp copy (ListFiles → Files(Ok) count; ReadFileDiff → FileDiff(Ok); ContainedIn names `main`). Not written.
3. MAP.md: `graft` (update/detail, jobs/detail, AppState::detail/local_jobs, Task/Product/Command lines, peek line removed) and `graft-ui` (`detail` section). Not written.
4. graft-ui footer still has a peek hint (`graph_screen/footer.rs:40,50,162,166,200`, test `the_peek_hint_names_itself_only_when_the_cursor_row_is_truncated`): now wording a key that opens the pane. Seam for `detail-keys-and-focus` or a small follow-up.

Deviations from spec (say so at review):
- Job ids: runner issues ids, so `update` ids carry `LOCAL_JOB_BIT` (1<<63) via `AppState::local_jobs: LocalJobs`; runner `queue_as(id, ..)` accepts them.
- `Command::Cancel(JobId)` not added: existing `Command::CancelJob` reused.
- `Product::Files` carries `(Arc<FileListView>, Arc<FileList>)`; `DetailState::list` added — `ReadFileDiff` needs the `FileEntry`, which ui types can't hold.
- `FileListView::message: Vec<SanitizedText>` added — the `✉` row's body had nowhere to live.
- `Task::ListFiles { spec: DiffSpec }`; runner reads the commit body for `DiffSpec::Commit`.
- `DiffOptions` gained `Hash` derive in graft-git (DiffKey derives Hash).
- Update tests use `merge_on_screen`, not `Shape::FeatureBranches` (update tests build no fixture repos).
- WorkingTree `Enter` not wired (no working-tree row id found within budget).
- Runner always computes word spans (`assemble_diff(.., true)`); `DetailState::word_tint` is the widget's switch.
