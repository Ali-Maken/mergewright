# M5 design pass: merge, rebase, reset, Ghost Preview (write) (2026-09-20)

Read: CLAUDE.md; PROMPT.md §1.4 Fix…, §1.5, §3 (undo, long operations, feedback), §6.1 crate rules, §8 M5 row + the
no-resolver paragraph; docs/design 01 §1.1–1.3, 1.7, 1.8, 1.10, 1.13, 1.16; 02 (whole, binding); 06 §6.3–6.7; 08 §8.2
(Ghost Preview lines); ADR-0029/0031/0032/0035/0037; every `crates/*/MAP.md`; docs/reviews/m4-07 (+ m4-05 on `Esc`);
KNOWN_GAPS M5 rows; the m4 note for shape. No source. **No scouts were dispatched: this dispatch had no agent tool.**
Every brownfield spec lists the scout questions; the conductor sends them (ADR-0035) before setting `Build with`.

**Promise:** *you see the result before you confirm.* Every history-changing key — merge, rebase, cherry-pick, revert,
reset, checkout, branch move, the journal's time machine — draws its predicted result in the graph as ghost rows, with
the exact commands, the refs that move, what the working tree loses, and a conflict forecast, before anything runs.
The acceptance walk proves it: for each operation, the ghost picture equals what git then does.

**Scope in:** the four ADR-0037 requirements (zeroizing password field first; hint bar redesign; `u` after a push
explains and offers the re-push; the two bugs as tickets); `m r b t`, `.` menu verbs (checkout, cherry-pick, revert,
reset, delete branch/tag, Fix…); Ghost Preview planning + drawing; the honest conflict interim (banner, conflicted
files, `M` mergetool / `e` editor hand-off, Abort always); journal time machine (ADR-0031); branch from stash;
`p` on a diverged branch pointing at `m`/`r`; M5 gap rows 217, 218, 229, 455, 474, 484, 485.
**Out (Proposed changes):** submodule/worktree fixtures → M7 (nothing in M5 reads them); `refs.rs` split → M7
housekeeping; `last_rejected_push` persistence → M7 (needs params in the journal entry); drag-and-drop stays in M5
but in pass 2; pixel-tier ghosts → M8 (previews draw at the Unicode/ASCII tier, see choice 1).

## Interfaces and data shapes

Seam rule unchanged: `app` builds an `Operation`, `ops` decides plan/check/journal/preview, `git` runs and reads.
New: `ops` predicts (`preview`), `app` synthesises ghost commits into a bounded slice and lays it out with
`graft_graph::layout` (DESIGN 08 §8.2), `ui` paints `RowPaint::Ghost`.

```rust
// graft-ui/src/text_input.rs                                                       (G1 secret-field)
pub struct SecretField;  // chars in a Vec<char> zeroed on Drop (zeroize crate, no unsafe); Clone zeroes its own copy
impl SecretField { pub fn new() -> Self; pub fn insert(&mut self, c: char); pub fn insert_str(&mut self, s: &str);
    pub fn backspace(&mut self); pub fn delete(&mut self); pub fn left/right/home/end(&mut self); pub fn clear(&mut self);
    pub fn len(&self) -> usize; pub fn is_empty(&self) -> bool; pub fn cursor(&self) -> usize;
    pub fn into_bytes(self) -> Zeroizing<Vec<u8>>; }   // the only way out; UTF-8; caller hands it to Answer::new
// Debug prints `SecretField(len)`; no text(), no Display, no PartialEq on contents (Eq by len only, for Command)
pub struct FieldView { …, pub secret: Option<usize> /* Some(len): draw len mask glyphs, no text */ }  // replaces `masked`
// graft/src/update/credential.rs: CredentialPrompt.field: SecretField (username asks use TextField: `field: PromptField`)
pub enum PromptField { Open(TextField), Secret(SecretField) }

// graft-ops/src/restore.rs                                                          (G2 undo-cancel-rollback)
pub enum RestoreError { …, Cancelled { rolled_back: usize } /* every applied step reversed, entry Aborted("cancelled") */,
    CancelledPartway { applied: Vec<String>, rollback: Box<RestoreError> } /* rollback failed; entry Aborted, after-state real */ }
pub fn execute(ctx: &Context, plan: &InversePlan, force: bool) -> Result<Applied, RestoreError>;   // unchanged signature;
//   checks ctx.stop before every step; on stop runs roll_back(applied) from the entry's own before-fields (CAS on refs)
// graft/src/update/remote.rs: UNDO_CANNOT_STOP deleted; undo launches Stoppable::Yes

// graft/src/update/mod.rs + jobs                                                     (G3 refresh-race)
pub enum Command { …, OpenRepository(JobId /* minted by update via LocalJobs */, RepoOpen) }   // runner: queue_as
// AppState::awaiting is set at dispatch, not on Started; refresh_key cancels it unconditionally

// graft-git/src/history_reads.rs (new)                                              (G4 git-history-reads)
pub fn merge_base(repo: &RepoHandle, a: Oid, b: Oid) -> Result<Option<Oid>, RunError>;
pub struct CommitSummary { pub id: Oid, pub parents: Vec<Oid>, pub subject: String, pub author: Identity }
pub fn commits_between(repo: &RepoHandle, exclude: Oid, include: Oid, cap: usize, stop: &dyn Stop) -> Result<Range, RunError>;
pub struct Range { pub commits: Vec<CommitSummary> /* oldest first */, pub capped: bool }
pub enum Forecast { Clean, Conflicts { paths: Vec<PathBuf> }, NotCheckable(String) }   // git < 2.38 → NotCheckable
pub fn forecast_merge(repo, git: &GitVersion, base: Option<Oid>, ours: Oid, theirs: Oid, stop) -> Result<Forecast, RunError>;
//   `merge-tree --write-tree --name-only --no-messages [--merge-base=<base>] <ours> <theirs>`; exit 1 = conflicts
pub fn changed_paths(repo, from: Oid, to: Oid, cap: usize) -> Result<(Vec<PathBuf>, bool), RunError>; // diff-tree --name-only
pub fn first_marker_line(path: &Path) -> io::Result<Option<usize>>;                    // `^<<<<<<< ` for `e`
// graft-git/src/state.rs (extended)
pub struct InProgressDetail { pub kind: InProgress, pub step: Option<(u32, u32)>, pub head_name: Option<String>, pub onto: Option<Oid> }
pub fn in_progress_detail(repo: &RepoHandle) -> Result<Option<InProgressDetail>, StateError>;  // rebase-merge/{msgnum,end,head-name,onto}
pub fn mergetool(repo: &RepoHandle) -> Result<Option<String>, RunError>;              // `git config merge.tool`

// graft-core/src/journal.rs                                                          (G5 ops-history-operations)
pub enum OperationKind { …, Merge, Rebase, CherryPick, Revert, Reset, Checkout, BranchCreate, BranchDelete, TagCreate,
    TagDelete, BranchFromStash, RevertTo }
// graft-ops/src/operation.rs (+ operation/history.rs)
pub enum Operation { …, Merge(MergeParams), Rebase(RebaseParams), CherryPick(PickParams), Revert(RevertParams),
    Reset(ResetParams), Checkout(CheckoutParams), BranchCreate(BranchCreateParams), BranchDelete(RefDeleteParams),
    TagCreate(TagCreateParams), TagDelete(RefDeleteParams), BranchFromStash(BranchFromStashParams) }
pub struct Tip { pub name: String /* full ref, or "HEAD" */, pub oid: Oid }   // what the preview was drawn against
pub struct MergeParams { pub source: Source, pub into: Tip, pub source_tip: Oid, pub ff: FastForward /* Default|Only|Never */ }
pub enum Source { Branch(String /* full */), Commit(Oid) }
pub struct RebaseParams { pub branch: Tip, pub onto: Source, pub onto_tip: Oid }
pub struct PickParams { pub commits: Vec<Oid>, pub head: Tip }        // one commit in M5, Vec for M7
pub struct RevertParams { pub commit: Oid, pub mainline: Option<u8>, pub head: Tip }
pub struct ResetParams { pub target: ResetTarget, pub to: Oid }
pub enum ResetTarget { Head { branch: Tip, mode: ResetMode /* Soft|Mixed|Hard */ }, Branch(Tip) /* update-ref with CAS */ }
pub struct CheckoutParams { pub to: CheckoutTarget /* Branch(String)|Detached(Oid) */, pub head_before: HeadRef }
pub struct BranchCreateParams { pub name: String /* short */, pub at: Oid, pub checkout: bool }
pub struct RefDeleteParams { pub name: String /* full */, pub expected: Oid }
pub struct TagCreateParams { pub name: String, pub at: Oid, pub message: Option<String> }
pub struct BranchFromStashParams { pub name: String, pub index: usize, pub commit: Oid }
pub enum Refusal { …, Moved { name: String } /* a Tip moved since the preview: "press m again" */,
    DirtyTree { count: usize } /* rebase/checkout-with-switch: "S stashes N changed files first" */, BadRefName { name, rule },
    RefExists { name }, MarkersRemain { path } /* stage of an unmerged path with `<<<<<<<` still in it */, NotInProgress }
// plan (exact args; verify each in a throwaway repo): merge → `merge --no-edit [--ff-only|--no-ff] <source>`;
//   rebase → `rebase <onto>` (stderr sink for `Rebasing (n/m)`); cherry-pick → `cherry-pick --no-edit <oid>…`;
//   revert → `revert --no-edit [-m N] <oid>`; reset Head → `reset --<mode> <oid>`; reset Branch → `update-ref <name> <to> <expected>`;
//   checkout → `switch <branch>` / `switch --detach <oid>`; branch create → `branch <name> <oid>` [+ `switch <name>`];
//   delete → `update-ref -d <name> <expected>`; tag → `tag [-a -m <msg>] <name> <oid>` (stdin `-F -` when message);
//   branch from stash → `stash branch <name> stash@{n}`. Every invocation env `GIT_EDITOR=true`.
// touches: merge/rebase/pick/revert {[branch], index, worktree}; reset Head {[branch], index (Mixed/Hard), worktree (Hard)};
//   reset Branch {[name]}; checkout {[HEAD], index, worktree}; create/delete {[name]}; stash-branch {[name, refs/stash], HEAD, index, worktree}
// graft-ops/src/apply.rs
pub enum ApplyError { …, Conflicted { entry: Box<JournalEntry> /* still Pending */, kind: InProgress, paths: Vec<PathBuf> } }
pub enum Resume { Continue, Skip, Abort }
pub fn resume(ctx: &Context, resume: Resume) -> Result<Applied, ApplyError>;   // finds the Pending entry whose kind matches
//   `in_progress`; runs `<kind> --continue|--skip|--abort` (GIT_EDITOR=true); Continue with unmerged paths → Refused(Unmerged);
//   success → Outcome Committed (after-state now); Abort → Outcome Aborted("aborted"), after == before; conflict again → Conflicted
pub fn recover_interrupted(ctx) -> Result<Recovered, …>;   // unchanged signature; SKIPS a Pending entry whose kind matches a live
//   `in_progress` state (the conflict survived a relaunch: banner offers Continue/Abort); closes every other Pending as before
// graft-ops/src/undo.rs — §6.4 inverses come free from the generic before/after steps (merge/rebase/pick/revert/reset/checkout);
//   BranchDelete/TagDelete inverse = UpdateRef { expect: None, to: expected }; BranchFromStash inverse adds StashStore(pins[0])
pub fn plan_revert_to(entries: &[JournalEntry], target: EntryId) -> Result<Vec<InversePlan>, NotInvertible>;  // newest → target, §6.6
pub fn revert_to(ctx: &Context, target: EntryId, force: bool) -> Result<Vec<Applied>, UndoError>;  // one Undo entry per step, stop between
// graft-ops/src/preview.rs (new) — Ghost Preview planning
pub struct PreviewPlan { pub description: String, pub commands: Vec<String> /* == plan(&facts) lines, never hand-written */,
    pub ghosts: Vec<GhostCommit> /* parents-first */, pub ref_moves: Vec<RefMove>, pub head: Option<HeadMove>,
    pub worktree: WorktreeEffect, pub conflicts: ConflictForecast, pub undo: UndoNote, pub against: Vec<Tip> }
pub enum Target { Real(Oid), Ghost(usize) }
pub struct GhostCommit { pub oid: Oid /* synthetic: sha1("graft-ghost\0"+description+"\0"+n); never written */,
    pub parents: Vec<Target>, pub subject: String, pub author: Identity, pub replays: Option<Oid> }
pub struct RefMove { pub name: String, pub from: Option<Oid>, pub to: Target }
pub enum HeadMove { ToBranch(String), Detached(Target) }
pub enum WorktreeEffect { Untouched, Updated { paths: Vec<PathBuf>, more: bool }, Lost { paths: Vec<PathBuf> }, RestoredFromBackup }
pub enum ConflictForecast { Clean, Likely { paths: Vec<PathBuf> }, NotChecked(String) }
pub enum UndoNote { Undoable, NotUndoable(String) }
pub const FORECAST_CAP: usize = 50;   // replayed commits forecast one by one; past it → NotChecked("more than 50 commits")
pub fn plan(repo: &RepoHandle, git: &GitVersion, op: &Operation, facts: &RepoFacts, stop: &dyn Stop) -> Result<PreviewPlan, PreviewError>;
pub fn plan_revert_to(entries: &[JournalEntry], target: EntryId, now: &RepoStateNow) -> Result<PreviewPlan, PreviewError>;
// graft-ops/src/operation.rs: PushParams += source_commit: Option<Oid>  (push that commit, not the branch tip: the guided re-push)

// graft-ui                                                                             (G6 ui-ghost-banner-conflict)
pub enum RowPaint { Normal, Dimmed, Highlighted, Ghost }   // graph_column: dashed `┊ ┈ ╭┈ ╰┈`, dim; ASCII `: . ,`; node `○` / `o`
pub struct ConfirmView { …, pub choices: Vec<SanitizedText>, pub cursor: usize }   // N choices; `new` keeps 2; `with_choices`
pub struct PreviewBox { pub title, pub lines: Vec<SanitizedText> /* refs, worktree, conflicts, undo */, pub commands, pub choices, pub cursor, pub planning: bool }
pub struct BannerView { pub kind: SanitizedText /* REBASING */, pub step: Option<(u32,u32)>, pub conflict: Option<SanitizedText>, pub focused: bool, pub skippable: bool }
pub enum WorkingTreeSection { Unmerged, Staged, Unstaged, Untracked }   // `CONFLICTS (3)` first
pub enum HintContext { WorkingTreeRow, BranchRow { current: bool }, PlainRow, Marked(Box<HintContext>), Banner { skippable: bool },
    Preview, ChangesPane { unmerged: bool }, Journal, Stash }
pub fn hints_for(context: &HintContext, columns: u16) -> Vec<Hint>;   // never empty; <100: 3 + `. more`; 100–159: 6 + `. more`; 160+: all
pub fn fit_status_line(reason: &str, keys: &str, columns: u16) -> String;  // cuts the reason first, keeps ` · <keys>` whole

// graft (app), pass 2                                                                  (G8–G10)
// Command::PlanPreview(JobId, Box<Operation>) / PlanRevertTo(JobId, EntryId) → Product::Preview(Result<Arc<PreviewPlan>, String>)
// AppState::preview: Option<PreviewState { op: PreviewSubject, plan: Loading<Arc<PreviewPlan>>, rows: Vec<Row>, ghost_ids: HashSet<Oid> }>
// Command::Resume(JobId, Resume); Command::OpenMergetool { path } (loop-owned hand-over like $EDITOR); AppState::banner_focus: bool
// AppState::name_prompt: Option<NamePrompt { kind: Branch|Tag, at: Oid, field: TextField }>  → in text_field_open()
```

## Hard-to-reverse choices

1. **How ghost rows reach the graph.** (a) Intern synthetic commits into the live `CommitTable` and re-walk through
   `StreamingLayout`; (b) **modal overlay over a bounded slice**: `app` takes the resident rows around the cursor
   (viewport ± 1 screen), substitutes the plan's ref moves, inserts `GhostCommit`s as `core::Commit`s with synthetic
   oids, and calls `graft_graph::layout(slice, seeds)` once; the result replaces the graph rows while the preview is
   open; (c) draw ghosts as text rows with no lanes. **(b).** No fake ids in the table, no walk, pure and testable
   (`layout` is a fn), well under 16 ms for a few hundred rows. Cost: the slice's lanes can differ from the live graph
   at the slice edge; the preview is modal, so the eye compares real rows *inside* the same picture (§1.7's rule
   holds). Pixel tier: the preview draws at the text tier (dashed strokes in the rasteriser → M8).
2. **A conflicted operation is one journal entry, Pending from stop to finish** (DESIGN 06 §6.4), not two entries
   (op `Aborted("conflicts")` + a separate `Continue`). **One entry.** Undo of a finished merge then restores the
   pre-merge state, as §6.4 says; `Continue`/`Skip`/`Abort` write the Outcome line. Cost: `recover_interrupted` must
   leave a Pending entry alone while git's own state says the operation is still live, and `in_force` must treat a
   Pending entry as not in force (today's rule). A `kill -9` mid-rebase then relaunches into the banner (PROMPT §9).
3. **Preview and operation share one `Operation` value, pinned to the tips it was drawn against.** Every history
   params carries `Tip { name, oid }`; `check` refuses `Moved` when facts differ at confirm time. The alternative
   (re-plan silently at confirm) can run something other than the picture. **Pin.** Cost: a fetch between preview
   and confirm makes the user press the key again — one line says so.
4. **Rebase on a dirty tree** (KNOWN_GAPS row 50): (a) `--autostash`; (b) **refuse before git runs**, `DirtyTree`,
   line `rebase needs a clean tree · S stashes 3 changed files`. **(b).** Autostash's pop can conflict outside any
   journal entry and §6.3's snapshot does not cover it; the refusal costs one key (`S`) and keeps undo one entry.
5. **Conflict forecast is per commit, not chained.** Rebase/cherry-pick forecast = `merge-tree` of (onto, c) with
   base c^ for each replayed c, capped at 50. Exact chaining needs temporary commits (object writes). Wording says
   `may conflict (forecast)`; never `will`.

## Where I'd differ from DESIGN.md (Proposed changes)

1. §1.7 buttons `[ Confirm ] [ Cancel ]` → `[ Cancel ] [ Confirm ]`, `Cancel` under the cursor (PROMPT §3 safe default;
   matches the push box). 2. §6.4 Push row gains the ADR-0037 §3 transparency rule: `u` with a push newer than the
   entry opens a three-choice box (below), never a silent step-over. 3. §1.13's banner takes focus when a conflict
   lands (`c s a` live, `⇥` leaves it) rather than needing a click; the mockup's keys assume focus. 4. §2.9 `b` in
   the stash overlay = `stash branch` (one op, journaled, undoable) — kept as designed, noted because it is M5 work.
   5. Deferrals listed under Scope. 6. §1.14 mask: fixed 8 glyphs regardless of length (m4-07 Minor 9) — user's call.

## New UI (captures carry no colour: every fact is text)

**Hint bar** — `graph_screen` footer, rebuilt from `hints_for`. Priority lists (the width tiers here were replaced by the shrink ladder, ADR-0039; `. more` is never dropped and since m5/28 leads the bare keys rather than ending the row; `? help`/`⌃P` absent until M8 binds them):
```
Uncommitted row : ⏎ changes  c commit  S stash  | p pull  P push  f fetch  | u undo  J journal  / search  j/k move
Branch row      : m merge  r rebase  ⏎ open     | b branch  t tag  P push   | c commit  p pull  f fetch  u undo  S stash  / search
Current branch  : ⏎ open  c commit  P push      | p pull  f fetch  b branch | t tag  u undo  S stash  J journal  / search
Plain row       : ⏎ open  b branch  r rebase    | t tag  c commit  y sha    | m merge  u undo  S stash  J journal  / search
Marked row      : . explain, then the row's own list (`. more` collapses into it)
Banner focused  : c continue  s skip  a abort   | M mergetool  e edit  ⇥ graph          (merge: no `s skip`)
Preview open    : ←→ choose  ⏎ act  esc cancel  (the box's own footer; main footer empty)
Changes pane    : space stage  d discard  c commit | ⌃A all  e edit  ⇥ graph   (unmerged file: M mergetool  e edit  space resolved)
```
```
80  ╰ m merge  r rebase  ⏎ open  . more ─────────────────────────────────────────╯
120 ╰ m merge  r rebase  ⏎ open  b branch  t tag  P push  . more ─────────────────────────────────────────────────────╯
160 ╰ m merge  r rebase  ⏎ open  b branch  t tag  P push  c commit  p pull  f fetch  u undo  S stash  / search  . more ──╯
80, sticky failure: ╰ push rejected: origin/main has commits you… · p pull · P force ─╯   (reason cut, keys whole)
```
**Ghost Preview** — modal over the graph rows; the box docks to the bottom rows above the footer. Eye: the ghost rows
(`(ghost)` in the ref column, `—` sha), then the box title. `Esc`/`Cancel` closes; `⏎` on `Confirm` runs the pinned op.
```
80x24 (r on the row [main], HEAD on feature/auth)                 160x48 (same, right half)
│ ●  [main]         Fix token refresh race       2m  a1b2c3d │  …AUTHOR WHEN SHA     │
│ ┊○ (ghost)        Add OAuth provider           —   —       │  AK     1h   —        │
│ ┊○ (ghost)        Wire provider into login     —   —       │  AK     1h   —        │
│ │●  [feature/auth] Wire provider into login    1h  7f6e5d4 │
│ │●                 Add OAuth provider          1h  2b3c4d5 │
│ ◉─╯               Merge 'feature/login'        3h  3c2b1a0 │
╭ Preview: rebase feature/auth onto main ─────────────────────╮
│ 2 commits replayed · feature/auth 7f6e5d4 → ghost           │
│ working tree: 3 files updated · may conflict: src/auth.rs   │
│ undo: u restores feature/auth and the tree                  │
│ git rebase refs/heads/main                                  │
│                        [ Cancel ]   [ Confirm ]             │
╰ ←→ choose  ⏎ act  esc cancel ───────────────────────────────╯
```
Planning: box shows `predicting…` for ≤ 1 s of forecast; the ghost rows appear when the plan lands. Forecast
`NotChecked`: `conflicts: not checked (git 2.38+ needed)`. Reset `--hard` with dirty files: `working tree: LOSES
src/a.rs, src/b.rs (backup saved)`. Time machine (journal `⏎` on an older row): no ghosts; lines `refs/heads/main
a1b2c3d → 9f8e7d6 · HEAD stays · index and working tree restored from backup · 3 steps`. Too small (< 40 cols or
< 12 rows): one line `preview needs 40x12 · esc closes`; `⏎` does nothing — a picture that cannot be seen is never
confirmed.

**Operation banner + conflict interim** — banner replaces the header's ahead/behind slot whenever `in_progress`;
focused on the frame a conflict lands. The changes pane opens on `CONFLICTS (n)` with the files; `M` runs
`git mergetool -- <path>` (loop hand-over), `e` opens `$EDITOR` at the first `<<<<<<<`; `space` marks resolved
(refused while markers remain); `c` continues when none are unmerged. `a` always aborts (box: `[ Cancel ] [ Abort ]`).
```
80x24                                                          120x40 (header)
╭◆ graft ~/code/api ⚠ REBASING 3/7 — conflict src/auth.rs ─╮   ╭─ ◆ graft ─ ~/code/api ─ ⚠ REBASING step 3 of 7 — conflict in src/auth/token.rs (+1) ─╮
│[Graph]  Changes  Detail                                  │   │ …graph…                                                                              │
│ CONFLICTS (2)                                            │   ├──────────────────────────────────────────────────────────────────────────────────────┤
│ ▸ src/auth.rs        both modified                       │   │ CONFLICTS (2)   ▸ src/auth/token.rs  both modified   src/login.rs  deleted by them   │
│   src/login.rs       deleted by them                     │   │ STAGED (1) …                                                                         │
│ STAGED (1) …                                             │   ╰ c continue  s skip  a abort  M mergetool  e edit  ⇥ graph ────────────────────────╯
╰ c continue  s skip  a abort  M mergetool  e edit  ⇥ graph╯
```

*(Corrected 2026-09-22, m5/39 NOTES 2: this footer read `c continue  s skip  a abort  . more`. Under ADR-0039 the ladder shortens a list only when it does not fit, and all six fit at 80 — 57 columns. The schematic box is drawn 60 wide, narrower than the 80 it is labelled, which is why it looks tight here; the walk scenario `hints-at-80-show-three-and-more` asserts the drawn string (`crates/graft/tests/it/m5_walk/regressions.rs:395-400`).)*
**`u` after a push** — three-choice confirm (no undo runs yet): title `Undo after a push`, lines `commit "x" is on
origin/main (pushed 2m ago) · undo works on your local branch only`, `[ Cancel ]  [ Undo locally ]  [ Re-push old tip ]`.
Re-push opens the existing lease box with `push --force-with-lease=refs/heads/main:<tracking tip> origin 9f8e7d6:refs/heads/main`
(`PushParams.source_commit`), consequence `rewrites origin/main for everyone who has a1b2c3d`. Asked once per entry.
**Name prompt** (`b`, `t`, stash `b`) — one-field box like the credential prompt: title `New branch at a1b2c3d`, field,
note `checked out after creating · ⇥ toggles` (`(•) check out`), refusal preview under it (`exists`, `bad name: …`).
**`.` menu additions** (badge row): `Checkout <branch>`, `Merge <branch> into main`, `Rebase main onto <branch>`,
`Cherry-pick`, `Revert`, `Reset main to here (mixed)`, `Reset main to here (hard)`, `Delete branch <b>`, `Delete tag <t>`;
marked merge row: `Fix: revert the merge (-m 1)`, `Fix: reset main to before the merge`, `Fix: rebase instead`.

## Genuinely risky groups

- **ops-history-operations (G5)** — Fable (signature). Meets all three tests: a wrong journal shape for the Pending
  conflict span or a wrong `PreviewPlan` is structural for M6 (resolver), M7 (chain rebase, ADR-0032) and the browser;
  correctness of "the ghost equals what git did" is not readable from the diff; the builder still decides the
  per-kind inverse details and the resume/recovery rules. Judged by the walk's ghost-vs-git equality tests.
- **undo-cancel-rollback (G2)** — Opus, not Fable: approach fixed here (roll back from the entry's own before-fields);
  correctness is testable with a counting `Stop` and `for-each-ref`/tree ids.
- **app-preview-and-keys (pass 2)** — the slice re-layout and the modal state machine; judged by captures and the
  walk, not the diff. Opus.

## Pending: the border/background distinction (→ ticket 28)

Accepted with a clause, m5/27: a bordered box (the Ghost Preview, the operation
banner) makes its edge explicit, so content beside it reads as *background*
rather than as part of the box — **but only while whole columns survive beside
the box**. Fragment a column and the border does not save you; a reader cannot
tell a clipped column from a box that leaked into it. The geometry rule that
follows from this, and its ADR, are ticket 28's — not written or implemented
here.

Fable builds: G5 (signature) + acceptance. The design is not underspecified: each hard problem is decided above.
Group count 11 → **pass 1 = this note + G1–G7** (`secret-field`, `undo-cancel-rollback`, `refresh-race`,
`git-history-reads`, `ops-history-operations`, `ui-ghost-banner-conflict`, `hint-bar-redesign`); pass 2 =
`app-preview-and-keys`, `app-conflict-interim`, `app-drag-drop-and-stash-branch`, `acceptance-history` (walk
`m5_walk` split by seam per m4-07 M3/M4, `TESTING.md` "History", contact sheet for M5 **and retroactive M3/M4
sheets** through one script, KNOWN_GAPS "no resolver" row expiring M6, `docs/shots/m5-*` ≤ 200 KB each).
