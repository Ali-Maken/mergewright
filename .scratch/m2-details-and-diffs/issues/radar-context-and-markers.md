# radar-context-and-markers: default-chain facts, the marker column, and the pane's radar line

Status: draft
Build with: Sonnet — wiring an already-decided rule set into jobs/view/config; every transition is testable, no rule is invented here.
Needs: `mistake-radar-detection` (`Severity`, `MistakeRadarKind`, `Policy`, `CommitFacts`, `RepoFacts`, `UpstreamFacts`, `BranchFacts`, `HeadFacts`, `Finding`, `Evidence`, `Risk`, `classify_commit`, `classify_refs`), `git-runner` (`Stop`), `detail-model-and-jobs` (`Loading<T>`, `Bounded<K,V>`, `AppState::detail` pattern, `jobs::detail` as the pattern for a new `jobs::radar`, `RadarLine`, `DetailHeader::radar`, `view::detail_view`), `detail-pane-frame` (`DetailPane` widget, `view::detail_view`, `AppState::repository`/history row access)

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — Hard-to-reverse choice 2 in full; the pass-2b signature-fix note (`RadarLine::EvilNotCheckable`); Proposed changes rows 1–2; New UI "Marker column" bullet, "Detail pane" bullet's header line (`radar line when any`); §8 coverage row "Mistake Radar detection"
- `crates/graft-git/MAP.md` lines 33–61 (`history`: `Walk`, `CommitTable::id_of`/`handle_of`, `WalkError`) and 68–98 (`repo`: `RepoHandle`, `RefEntry`, `HeadState`, `RefsError`)
- `crates/graft-core/MAP.md` lines 25–29 (`CommitHandle`, `Parents`), 51–53 (`Oid`)
- `crates/graft/MAP.md` lines 56–66 (`refs`: `RefBadges`, `Upstream`, `AheadBehindCache`), 183–193 (`repos`: `RepoStore::branches`/`table`/`refresh`), 202–213 (`view`: internal split, `graph_view`), 241–262 (`jobs`), 264–321 (`history`: `History::rows` — read the doc line on this field only, not the file, to learn whether it evicts off-screen rows; if unclear, get one scout summary of `history.rs`'s `rows` field and its eviction, never the whole file), 151–181 (`config`)
- `crates/graft-ui/MAP.md` lines 35–70 (`graph_screen`: `GraphEntry`, `GraphView`, `CHROME_ROWS`)
- `.scratch/m2-details-and-diffs/issues/detail-pane-frame.md` (already landed) — `DetailPane`'s header rendering, so this ticket's addition is a diff against it, not a rewrite
- ADR-0011 (gix-primary reads: this ticket's two new git-git reads are in-memory gix, no spawn)

## Scope

In:
- `crates/graft-git/src/history.rs`: two new functions, no new module.
  - `first_parent_chain(repo, table, tip, stop) -> Result<Vec<CommitHandle>, WalkError>` — walk first parents from `tip` to the root (or until `stop`), reusing `Walk`'s existing per-handle reads; a shallow/root commit with no parent just ends the walk. `Stop` polled the same way `run.rs`'s process loop does (a plain flag check, no timer).
  - `previous_tips(repo, refname, cap) -> Result<Vec<Oid>, ReflogError>` — the ref's own reflog, old values newest-first, deduped, capped at `cap`; a ref with no reflog is `Ok(vec![])`, not an error. `ReflogError { Unreadable(String) }`, one-line `Display`.
  - MAP.md lines for both, plus `ReflogError`.
- `crates/graft/src/jobs/radar.rs` (new; `jobs::radar`, same shape as `jobs::detail`):
  - `Task::RadarContext { default_tip: Oid, default_ref: RefName }`; `Product::RadarContext(Result<Arc<RadarContext>, String>)`; `Command::RadarContext(JobId, Oid, RefName)`.
  - `pub struct RadarContext { pub default_chain: std::collections::HashSet<Oid>, pub previous_default_tips: Vec<Oid> }` — everything else `classify_commit`/`classify_refs` need (branch list, upstream, HEAD state) is already on hand in `AppState` (`RefBadges`, `RepoStore::branches`, `RepoHandle::head`); this job fetches only what nothing already reads: the default branch's first-parent chain and its reflog.
  - `pub fn run_radar_context(repo: &RepoHandle, table: &Arc<CommitTable>, default_tip: Oid, default_ref: &RefName, stop: &dyn Stop) -> Result<Arc<RadarContext>, String>` — calls the two `history` functions, converts handles to `Oid` via `table.id_of`, joins either error to one line.
- `crates/graft/src/update/radar.rs` (new): `AppState::radar: RadarState { pub context: Loading<Arc<RadarContext>>, pub policy: Policy }`, `Policy` built from config at startup (see below). Request `Command::RadarContext` wherever `Command::ReadRefs` is first issued (startup) and wherever it is re-issued (refresh) — cite that exact call site via MAP.md/a scout, don't guess a second startup path. `Product::RadarContext` sets `radar.context` (stale ids dropped, same rule as `detail`'s jobs).
- `crates/graft/src/view/radar.rs` (new):
  - `pub fn branch_facts(state: &AppState) -> Vec<BranchFacts<'_>>` — local branches from `RepoStore::branches`/refs, `UpstreamFacts` from the existing `refs::Upstream` (`None`→`None`, `InSync`→`InSync`, `Apart{ahead,behind}`→`Apart{ahead,behind}`, a straight rename, not a new read).
  - `pub fn head_facts(state: &AppState) -> HeadFacts<'_>` — from `RepoHandle::head`/`HeadState`.
  - `pub fn repo_facts<'a>(state: &'a AppState, ctx: &'a RadarContext) -> RepoFacts<'a>`.
  - `pub fn findings_for(state: &AppState, id: Oid, parents: &[Oid], subject: &str) -> Vec<Finding>` — `[classify_commit(...)].into_iter().flatten()` plus any `classify_refs` finding whose `at == id`, only once `state.radar.context` is `Ready` (else `vec![]`, no marker until the context lands). **This is the extension point**: `radar-probe-evil-merge`/`-force-push`/`-duplicates` each add one more source here, cited by name so a builder adds an arm, not a rewrite.
  - `pub fn marker_for(state: &AppState, id: Oid, parents: &[Oid], subject: &str) -> Option<MarkerView>` — highest severity of `findings_for(...)`; `Warning` wins ties.
  - `pub fn radar_lines_for(state: &AppState, id: Oid, parents: &[Oid], subject: &str) -> Vec<RadarLine>` — one `RadarLine::Finding` per `findings_for(...)` result (severity + kind phrase, listed below); plus, when `parents.len() >= 2` and `state.radar.policy.severity(EvilMerge).is_some()` and `state.radar.context` is `Ready`, one `RadarLine::EvilUnchecked` (no probe exists yet — `radar-probe-evil-merge` will look for its own `Loading<EvilVerdict>` first and only fall back to this line when that's `Idle`, cite this exact `if` as the seam).
- `crates/graft-ui` additions:
  - In the module holding `GraphEntry`/`GraphView` (its `graph_screen` MAP.md section): `pub struct MarkerView { pub severity: Severity }`; `GraphEntry` gains `pub marker: Option<MarkerView>`; `GraphView` gains `pub marker_column: bool`. Rendering: 2 cells between the graph column and badges, drawn only while `marker_column`; `⚠` (`warning`) / `ⓘ` (`info`), ASCII `!`/`i` at the 12-colour/no-icon setting (mirror however `badges` already picks ASCII vs Unicode/Nerd glyphs, don't invent a second switch).
  - `crates/graft-ui/src/detail_pane.rs`: extend the header's radar line drawing (currently a no-op on the empty `Vec`) with one line per `RadarLine`: `Finding{severity, title}` → severity glyph + `title` (already-worded English, e.g. `⚠ foxtrot merge`); `EvilUnchecked` → literal `"evil merge: unchecked"`; `EvilChecking` → `"evil merge: checking…"`; `EvilClean` → `"evil merge: clean"`; `EvilNotCheckable(reason)` → `"evil merge: not checkable — " + reason`; `ForcePushNotCheckable(reason)` → `"force-pushed: not checkable — " + reason` (the widget adds each prefix; the payload is the reason clause only, e.g. `"octopus merge"`, `"no remote-tracking reflog"`). `Info` lines `text-muted`, `Warning` lines the warning colour; ASCII fallback for the glyph, no bg tint (12-colour rule).
- Kind phrases for `Finding` titles (built once here, reused by `explain-panel`'s `ExplainView.title` is separate — this is the short pane-line phrase, not the Explain panel's caps title): `FoxtrotMerge` → `"foxtrot merge"`; `BackwardMerge` → `"backward merge"`; `AccidentalPullMerge` → `"accidental pull merge"`; `EvilMerge` → `"evil merge"`; `DuplicateCommit` → `"duplicate commit"`; `ForcePushedRemote` → `"force-pushed remote"`; `RiskyState` → `Risk::DetachedHead` → `"detached head"`, `UnpushedOnDefault{branch,ahead}` → `"unpushed on default ({branch}, {ahead} ahead)"`, `NoUpstream{branch}` → `"no upstream ({branch})"`.
- `[radar]` config: `crates/graft/src/config` (`types`/`load` split per its MAP.md) — one `RadarSetting` (`Default`/`Off`/`Info`/`Warning`) per `MistakeRadarKind::name()` key (kebab-case: `foxtrot`, `backward`, `accidental-pull`, `evil`, `duplicate`, `force-pushed`, `risky-state`), mirrored on the existing `[graph]` parsing pattern (cite its exact block via MAP.md/a scout, don't read `config/load.rs` whole). `Radar::policy() -> Policy` starts from `Policy::default()` and overrides only the keys present.
- `GraphView::marker_column`'s truth: `true` once `state.radar.context` is `Ready` and (`classify_refs` over `branch_facts`/`head_facts` is non-empty) or any row currently held in `History::rows` has a non-empty `findings_for`. If `History::rows` evicts rows that have scrolled off (check per Read-first), that can't give a stable "never shifts on scroll" answer on its own — add a `std::cell::Cell<bool>` field (`AppState::radar_column_seen`, `Clone` since `bool: Copy`) set from `view::entries` the first time a row's `marker` is `Some`, and OR it into `marker_column`. State in the report which of the two you needed.
- Tests: `first_parent_chain`/`previous_tips` on fixtures (`Shape::Linear`, `Shape::FoxtrotMerge`, ref-states' several-tips branch); `findings_for`/`marker_for`/`radar_lines_for` unit tests with a hand-built `RadarState`+`AppState` slice (fixture-backed, no job run); config round-trip for all seven keys plus an unrecognised value (`ConfigWarning`, mirror `[graph]`'s own unrecognised-key handling); marker glyph capture. MAP.md lines (`graft-git`, `graft`, `graft-ui`).
- Capture: two graph rows, one with a `Finding` marker (warning) and one with none, marker column present, to `docs/shots/m2-marker-column.txt`, ≤ 200 KB.

Out: the probes themselves (`radar-probe-*`), the Explain panel (`explain-panel`), the context-menu's `Explain` item (already landed), any `graft-ops` change (none needed here — `classify_commit`/`classify_refs`/`Policy` are unchanged).

## Signatures

```rust
// crates/graft-git/src/history.rs
pub fn first_parent_chain(repo: &RepoHandle, table: &Arc<CommitTable>, tip: Oid, stop: &dyn Stop) -> Result<Vec<CommitHandle>, WalkError>;
pub enum ReflogError { Unreadable(String) }
pub fn previous_tips(repo: &RepoHandle, refname: &RefName, cap: usize) -> Result<Vec<Oid>, ReflogError>;

// crates/graft/src/jobs/radar.rs
pub struct RadarContext { pub default_chain: std::collections::HashSet<Oid>, pub previous_default_tips: Vec<Oid> }
pub fn run_radar_context(repo: &RepoHandle, table: &Arc<CommitTable>, default_tip: Oid, default_ref: &RefName, stop: &dyn Stop) -> Result<Arc<RadarContext>, String>;
// Task::RadarContext { default_tip: Oid, default_ref: RefName }
// Product::RadarContext(Result<Arc<RadarContext>, String>)
// Command::RadarContext(JobId, Oid, RefName)

// crates/graft/src/update/radar.rs
pub struct RadarState { pub context: Loading<Arc<RadarContext>>, pub policy: Policy }
// AppState::radar: RadarState

// crates/graft/src/view/radar.rs
pub fn branch_facts(state: &AppState) -> Vec<BranchFacts<'_>>;
pub fn head_facts(state: &AppState) -> HeadFacts<'_>;
pub fn repo_facts<'a>(state: &'a AppState, ctx: &'a RadarContext) -> RepoFacts<'a>;
pub fn findings_for(state: &AppState, id: Oid, parents: &[Oid], subject: &str) -> Vec<Finding>;
pub fn marker_for(state: &AppState, id: Oid, parents: &[Oid], subject: &str) -> Option<MarkerView>;
pub fn radar_lines_for(state: &AppState, id: Oid, parents: &[Oid], subject: &str) -> Vec<RadarLine>;

// crates/graft-ui (graph_screen section)
pub struct MarkerView { pub severity: Severity }
// GraphEntry: + pub marker: Option<MarkerView>
// GraphView: + pub marker_column: bool

// crates/graft/src/config (types/load split)
pub enum RadarSetting { Default, Off, Info, Warning }
pub struct Radar { pub foxtrot: RadarSetting, pub backward: RadarSetting, pub accidental_pull: RadarSetting, pub evil: RadarSetting, pub duplicate: RadarSetting, pub force_pushed: RadarSetting, pub risky_state: RadarSetting }
impl Radar { pub fn policy(&self) -> Policy; }
// Config: + pub radar: Radar
```

## Acceptance

- [ ] `first_parent_chain` on `Shape::FoxtrotMerge`'s default tip returns exactly the first-parent commits, root last; on a linear fixture matches the whole history.
- [ ] `previous_tips` on a branch with a two-entry reflog returns both old values, newest first, deduped; on a fresh branch with no reflog entry beyond creation, `Ok(vec![])`.
- [ ] `findings_for`/`marker_for` on the foxtrot fixture's merge: one `FoxtrotMerge` finding, `Warning`; on a non-merge with no ref finding at that id: empty/`None`.
- [ ] `radar_lines_for` on the same merge: one `RadarLine::Finding{severity: Warning, title: "foxtrot merge"}` plus one `EvilUnchecked` (policy default has `EvilMerge` on); with `[radar] evil = off`, no `EvilUnchecked` line.
- [ ] `[radar]` config: all seven keys parse `off`/`info`/`warning`; an unrecognised key is a `ConfigWarning`, not a load failure; `Radar::default().policy() == Policy::default()`.
- [ ] `marker_column` false before `state.radar.context` is `Ready`; true once a finding is known, and — the concrete check named in Scope — stays true across a scroll that leaves the marked row off the laid-out set (a test that scrolls past it and re-checks).
- [ ] MAP.md: `graft-git` (`history`), `graft` (`jobs::radar`, `update::radar`, `view::radar`, `config::Radar`), `graft-ui` (`graph_screen`, `detail_pane`).
- [ ] Capture at `docs/shots/m2-marker-column.txt`, ≤ 200 KB; user judges by eye.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · purity guard (`update/radar.rs`: no I/O; `view/radar.rs`: no I/O beyond the `Cell` write if needed).

## Size

Four crates touched but three are small (git ~90 lines incl. tests; ui ~60; config ~70): the bulk is `graft` (`jobs/radar.rs` ~60, `update/radar.rs` ~50, `view/radar.rs` ~180, config wiring ~40). Tests ~220. This is the pass's second-largest ticket. **Named seam**: finish and commit `crates/graft-git` + the `[radar]` config + `graft-ui`'s `MarkerView`/column additions first (a clean, testable slice); if `view/radar.rs` plus the job wiring then passes 70% of budget before `radar_lines_for`/the capture are done, stop, commit, and report `partial` with `radar_lines_for` + the marker-column stickiness check + the capture as the remainder. Target ≤ 32 tool calls for the whole ticket, ~22 for the seam above.
