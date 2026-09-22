# M6 design pass: conflicts (2026-09-22)

Read: CLAUDE.md; PROMPT.md §4 Conflicts, §8 M6 row + no-resolver paragraph, §9; docs/design 01 §1.5, §1.13; 02 (whole,
binding); 06 §6.1–6.4; 08 §8.2 (`ui` never imports `git`/`ops`; `SanitizedText` at the seam); ADR-0031/33/35/36/37/38/39/41;
CONTEXT.md; the m5 note (conflict interim, its 2026-09-22 correction); m5 reviews 09 and 10-30-31; TESTING.md "Times I
left" + History scenarios 24–32; docs/field-notes.md; every `crates/*/MAP.md`; every KNOWN_GAPS row expiring M6. No source.
**No scouts were dispatched (no agent tool in this dispatch)**; every brownfield spec lists its scout questions.

**Promise:** *a real conflict is resolvable without leaving the app.* The acceptance walk proves it: a merge, a rebase
that stops twice, a stash pop and a `kill -9` mid-rebase each end with git's own state clean (`ls-files -u` empty, the
operation finished or aborted, the resolved bytes exactly what the keys chose) with no `$EDITOR`, no `git mergetool`, no
shell. **"Times I left" expectation:** the scheduled mergetool/`$EDITOR` reach falls to **0 for two-sided text conflicts
and modify/delete conflicts**; residual reaches (submodule, rename/rename, custom `conflict-marker-size`) are named as
gaps expiring M7, not counted as failures.

## Decision 1: what "three-pane" means here

**Result-centred, sides shown inside the conflict.** The resolver shows **one file** — the working-tree file itself,
as a document: plain lines drawn as they are, and each conflict block (between git's `<<<<<<<` and `>>>>>>>`) drawn as a
labelled card with **OURS and THEIRS side by side** (≥100 columns) or **stacked** (<100). BASE is a third labelled
section inside the card, on demand. `o`/`t`/`b` rewrite that block on disk; the card collapses to the taken lines and
keeps its header so the take can be changed until the resolver closes. Ours, theirs, result are all on screen — the
result *is* the page. Why not §1.5's three full-height columns: (1) three independent columns need a live three-way
row alignment that changes with every edit — a real algorithm with no ground truth; the in-place model has none, git
already delimited the sides; (2) at 80/120 columns (where the user works) three columns are 25/38 cells — code is
unreadable; halves at 120 are 58, stacked at 80 are 78; (3) what is drawn is byte-for-byte what `⏎`/`space` stages —
no result buffer that can diverge from disk, and `e` (`$EDITOR`) and `M` (mergetool) act on the same file and the
resolver re-reads on return. "Edit inline" (§2.6 `e`) is `$EDITOR`, per ADR-0038 §3 — no text editor is built.
Labels are git's own marker labels verbatim (`HEAD`, `feature/auth`, `3f2a1b (Add OAuth provider)`), plus one explainer
line on a rebase (`ours = main, what you are rebasing onto · theirs = the commit being replayed`) — the classic
confusion, answered where it happens (ADR-0041: can the user tell).

## Decision 2: where recovery on relaunch lives

**Not its own group.** M5/14 built the mechanism (`Recovered::live`, `launch_into_live`: banner focused, `c`/`a`
offered) and scenario 26 proves a relaunch. What M6 adds: (a) the relaunch **lands on the conflict** — changes pane open
on `CONFLICTS`, banner focused, `⏎` one key from the resolver (app piece, in `app-answers-beat-rereads`); (b) the
walk gains a `kill -9` scenario — the harness is *forgotten* (no shutdown, no outcome line) while git's child runs,
which is byte-identical on disk to SIGKILL, then relaunched — and the pty script kills the real binary with `SIGKILL`
for one capture; (c) resolver writes are temp+rename, so a kill mid-take never leaves a torn file. The journal side is
already right: a `Pending` line with no outcome is live while git says so, `interrupted` otherwise (§6.2).

## Interfaces and data shapes

```rust
// graft-git/src/conflict.rs (new)                                                     G1 git-conflict-model (Fable, signature)
pub struct Line(pub Vec<u8>);   // terminator included when present → render() is byte-exact, CRLF/mixed/no-final-newline all kept
pub struct Block { pub ours_label: Vec<u8>, pub ours: Vec<Line>, pub base: Option<(Vec<u8>, Vec<Line>)>, pub theirs_label: Vec<u8>, pub theirs: Vec<Line> }
pub enum Region { Plain(Vec<Line>), Conflict(Block), Resolved { block: Block, take: Take } }   // Resolved renders the taken lines only
pub enum Take { Ours, Theirs, Both /* ours then theirs, git's smerge order */ }
pub struct ConflictFile { pub regions: Vec<Region>, pub fingerprint: Oid /* git's blob id of the bytes as read: sha1("blob <len>\0"+bytes) */ }
pub struct ParseError { pub line: usize, pub what: &'static str }   // Unterminated, StrayMarker (a `=======`/`>>>>>>>` outside a block)
pub fn parse(bytes: &[u8]) -> Result<ConflictFile, ParseError>;   // markers: exactly 7 chars at column 0, `<<<<<<< `/`||||||| `/`>>>>>>> ` + label, `=======` alone; zdiff3 = diff3
impl ConflictFile { pub fn render(&self) -> Vec<u8>; pub fn take(&mut self, block: usize, take: Take) -> Result<(), NoSuchBlock>;
    pub fn blocks(&self) -> Vec<BlockAt> /* index, state, first line (1-based, in render()) */; pub fn open(&self) -> usize; }
pub enum ReadError { Io(io::Error), Binary, TooLarge { bytes: u64 } }   // Binary: NUL in the first 8 KiB (git's rule); cap 8 MiB
pub fn read(path: &Path) -> Result<ConflictFile, ReadError>;
pub fn write_back(path: &Path, file: &ConflictFile, expected: Oid) -> Result<Oid, WriteError>;   // Changed { now: Oid } when the file's blob id ≠ expected; temp in the same dir + rename, mode kept
pub fn stage_bytes(repo: &RepoHandle, path: &Path, stage: u8 /* 1|2|3 */) -> Result<Option<Vec<u8>>, RunError>;   // `git show :N:<path>`
pub fn diff3_blocks(repo: &RepoHandle, path: &Path) -> Result<Option<Vec<Block>>, RunError>;   // `merge-file -p --diff3` over the three stages in a scratch dir; None when a stage is missing
pub fn attach_base(file: &mut ConflictFile, diff3: &[Block]) -> Result<usize, BaseMismatch>;   // by (ours, theirs) byte-equality in order; after a hand edit → BaseMismatch, never a guess
// graft-git/src/status.rs (extended)
pub enum UnmergedHow { BothModified, BothAdded, BothDeleted, AddedByUs, AddedByThem, DeletedByUs, DeletedByThem }   // porcelain v2 XY; words(): "both modified" …
pub struct StatusEntry { …, pub unmerged_how: Option<UnmergedHow> }
// graft-fixtures: Shape::StoppedRebase — a corpus repo left at a rebase conflict (rebase-merge/, stages 1–3), one file per UnmergedHow + a binary UU + a CRLF UU + a zdiff3 UU

// graft-core/src/journal.rs                                                            G2 ops-conflict-operations
pub enum OperationKind { …, ResolveConflict, RestartConflict }
// graft-ops/src/operation.rs
pub enum Operation { …, ResolveWholeFile(ResolveWholeParams), RestartConflict(RestartParams) }
pub struct ResolveWholeParams { pub path: PathBuf, pub side: Side, pub how: UnmergedHow }   pub enum Side { Ours, Theirs }
pub struct RestartParams { pub path: PathBuf }   // `checkout --merge -- <path>`: the conflict back from the stages; folded into the live entry, not undoable — the box says so
pub enum Refusal { …, NotUnmerged { path }, ConflictsOpen { path, open: usize } /* ⏎ with blocks still open */, NotMarkerShape { path, how } /* o/t on a UD file goes to ResolveWholeFile */ }
// plan: UU/AA → `checkout --ours|--theirs -- <p>` then `add -- <p>`; UD ours / DU theirs / AU ours / UA theirs → `add -- <p>`; the other side of each → `rm --quiet -- <p>`; DD → `rm --quiet -- <p>`; every path `--literal-pathspecs`, verified in a throwaway repo
// graft-ops/src/conflict.rs (new)
pub struct Opened { pub file: Option<ConflictFile> /* None: whole-file shape or binary */, pub how: UnmergedHow, pub binary: bool }
pub fn open_conflict(repo: &RepoHandle, status: &WorkingTreeStatus, path: &Path) -> Result<Opened, OpenError>;   // refuses NotUnmerged
pub fn apply_take(repo: &RepoHandle, path: &Path, expected: Oid, block: usize, take: Take) -> Result<ConflictFile, TakeError>;   // read → fingerprint → take → write_back; Changed never clobbers
pub fn base_for(repo: &RepoHandle, path: &Path, file: &mut ConflictFile) -> Result<usize, BaseError>;   // diff3_blocks + attach_base

// graft-ui/src/conflict_resolver.rs (new)                                              G3 ui-conflict-resolver
pub struct DocView { pub title: SanitizedText, pub explainer: Option<SanitizedText>, pub regions: Vec<RegionView>, pub cursor_block: Option<usize>, pub base_shown: bool, pub notice: Option<(SanitizedText, bool)> }
pub enum RegionView { Plain { first_line: usize, lines: Vec<SanitizedText> }, Block(BlockView), WholeFile { how: SanitizedText, keys: SanitizedText, lines: Vec<SanitizedText> } }
pub struct BlockView { pub index: usize, pub of: usize, pub state: BlockState, pub ours_label: SanitizedText, pub theirs_label: SanitizedText, pub ours: Vec<SanitizedText>, pub base: Option<(SanitizedText, Vec<SanitizedText>)>, pub theirs: Vec<SanitizedText>, pub first_line: usize }
pub enum BlockState { Open, Took(TakeView) }   pub enum TakeView { Ours, Theirs, Both }
pub enum ScreenRow { Plain { line_no, text }, Header { block }, Pair { block, ours: Option<usize>, theirs: Option<usize> } /* ≥100 */, Side { block, side: SideKind, i } /* <100, and BASE at every width */, End { block }, WholeFile { .. } }
pub struct Layout { pub rows: Vec<ScreenRow>, pub block_row: Vec<usize> /* first screen row of block i */ }
pub fn layout(doc: &DocView, columns: u16) -> Layout;   // pure; the app scrolls and hit-tests against the same rows the widget draws (no second copy)
pub const SIDE_BY_SIDE_MIN: u16 = 100;
pub struct ResolverScreen<'a> { pub doc: &'a DocView, pub layout: &'a Layout, pub top: usize, pub theme: &'a Theme, pub ascii: bool }   // Widget; draws nothing outside its Rect
pub fn row_at(layout: &Layout, top: usize, screen_row: u16) -> Option<&ScreenRow>;
pub enum HintContext { …, Resolver { base: bool, restart: bool } }   // modal: its whole list, no `. more`; ChangesPane { unmerged } gains `⏎ resolve`; Banner gains `⏎ resolve` when conflicted

// graft (app)                                                                            G4 app-conflict-resolver, G5 app-resolver-whole-files-and-base
pub struct ResolverState { pub path: PathBuf, pub opened_from: OpenedFrom /* Banner | Pane */, pub doc: Loading<Arc<Opened>>, pub file: Option<ConflictFile>, pub cursor_block: Option<usize>, pub top: usize, pub base_shown: bool, pub job: Option<JobId> }
pub enum GraphOverlay { …, Resolver }   // drawn over graph AND pane, header kept (banner word stays), its own footer
pub enum Command { …, OpenConflict(JobId, PathBuf), TakeSide(JobId, TakeRequest), LoadBase(JobId, PathBuf, Box<ConflictFile>) }
pub struct TakeRequest { pub path: PathBuf, pub expected: Oid, pub block: usize, pub take: Take }
pub enum Product { …, ConflictOpened(Result<Arc<Opened>, String>), Taken(Result<Arc<ConflictFile>, String>), BaseLoaded(Result<(Arc<ConflictFile>, usize), String>) }
pub fn resolver_key(state, commands, key, next_id) -> bool;   // sits right after preview_key, before banner_key; modal
pub fn open_resolver(state, commands, path: PathBuf, from: OpenedFrom, next_id) -> bool;
pub fn resolver_view(state: &AppState) -> Option<DocView>;   // view::resolver — the one place git bytes become SanitizedText for this screen
```

## Hard-to-reverse choices

1. **The document model** (above) vs a three-column diff-aligned view. **Document.** Reasons in Decision 1. Cost: the
   user does not see ours/theirs for *unconflicted* lines (git already merged them; the diff pane still shows the file
   against `HEAD`).
2. **A take writes the file at once** (temp+rename, fingerprint-checked) vs a buffer written on `⏎`. **At once.** What is
   on screen is on disk: `e`, `M`, a crash and a second Graft all see the takes; `⏎` is the same `Stage` write `space`
   already is (`MarkersRemain` refusal untouched). Cost: a take is not journaled — it is a step inside the live `Pending`
   span, like a hand edit; the entry's backup still covers the pre-operation tree and `a` still restores everything.
3. **Whole-file shapes go through `Operation`s, block takes do not.** UD/DU/AU/UA/DD/binary have no markers; their
   resolution *is* `add`/`rm`/`checkout --ours`, which are git writes, so they are `Operation::ResolveWholeFile` (folded
   into the live entry, seam rule kept). Block takes are fs writes with no git command, so they live in `ops::conflict`
   as plain functions on a job — no `read_facts`/status spawn per keypress. Cost: two paths to "resolved"; one test
   walks both.
4. **Base is attached by equality, never by alignment.** `merge-file --diff3` over the stages reproduces git's own
   blocks; a block is given its base only if its (ours, theirs) bytes equal the reproduced block's, in order. After a
   hand edit the answer is `base unavailable after edits`, not a guess. Cost: base is unavailable for a file the user
   already touched — honest, and rare.

## Where I'd differ from DESIGN.md (Proposed changes — tickets follow DESIGN.md until approved)

1. **§1.5 layout**: document-with-cards (Decision 1) replaces the three full-height columns and the `CONFLICTED FILES`
   strip; the title row carries `file 1 of 2 · 2 of 3 conflicts open` and `n`/`N` walk files. Trade-off above.
2. **Keys the doc does not give, needed by the resolver** (each is a `02-keymap.md` edit the conductor makes before the
   binding ticket builds; the specs mark them *pending*): (a) `⏎` on a `CONFLICTS` row (§2.4) and on the focused banner
   (§2.6a) **opens the resolver** — §2.2's global "open / drill in" applied, no new letter; (b) `B` **show/hide base** in
   the resolver — UPPERCASE "toggles a mode" (§2.1), `b` stays take-both; (c) `d` in the resolver **start this file over**
   (`checkout --merge`, confirm box, not undoable while the operation is open) — the discard verb, its one meaning.
3. **§2.1's `M` contradiction** (KNOWN_GAPS 482): a hand-over *is* "a place you go to and come back from"; `M` joins the
   opens-a-place set with that sentence. Doc-only.
4. **§2.6 `e` "edit the result inline"** → `e` opens `$EDITOR` at the cursor block (ADR-0038 §3 already decided this).
5. **Commit editor** (KNOWN_GAPS 571): modal, as §2.10 already says; one line in §2.1's list. Doc-only.
6. **`Esc` in the resolver closes the resolver** (back to banner or pane); it never aborts. The banner's own `Esc` is left
   as M5 built it (row 561 → M7).

## New UI — the conflict resolver (`docs/design/01` §1.5 replaced; everything else per DESIGN)

Full frame below the header (graph and pane both covered; header keeps `REBASING step 3 of 7`); title row; document;
footer = its own complete key list (modal, no `. more`, ADR-0039). Eye lands on the **cursor block's header** (reversed,
`▏` rule on its lines per ticket 35's convention); second, the two labelled sides; third, the title's counts.
```
120x40 (≥100: sides paired)                                                              80x24 (<100: stacked)
╭ ◆ graft ─ ~/code/api ─ REBASING step 3 of 7 ────────────────────────────────╮  ╭ ◆ graft ─ ~/code/api ─ REBASING step 3 of 7 ─╮
│ src/auth/token.rs · file 1 of 2 · 2 of 3 conflicts open                      │  │ src/auth/token.rs · 1/2 · 2 of 3 open        │
│ ours = main, what you are rebasing onto · theirs = the commit being replayed │  │ ours = main (onto) · theirs = replayed commit│
│   38 fn refresh(&mut self) {                                                 │  │   38 fn refresh(&mut self) {                 │
│▏  39 ── conflict 1 of 3 · open ── o ours  t theirs  b both ──────────────── │  │▏  39 ── conflict 1 of 3 · open ─────────────│
│▏     OURS · HEAD                        │ THEIRS · 3f2a1b (Add OAuth provider)│  │▏     OURS · HEAD                             │
│▏  40   if expired() {                   │   if is_expired() {                 │  │▏  40   if expired() {                        │
│▏  41     self.renew();                  │     self.do_refresh();              │  │▏  41     self.renew();                       │
│▏     ───────────────────────────────────┴──────────────────────────────────── │  │▏     THEIRS · 3f2a1b (Add OAuth provider)    │
│   42 }                                                                       │  │▏       if is_expired() {                     │
│   57 ── conflict 2 of 3 · took theirs ── o ours  b both ──────────────────── │  │▏         self.do_refresh();                  │
│   58   let tok = self.store.get();                                           │  │▏     ────────────────────────────────────── │
╰ o ours  t theirs  b both  e edit  n next  N prev  ⏎ resolved  M mergetool  esc close ╯  ╰ o ours  t theirs  b both  e edit  n next  ⏎ resolved  esc ╯
```
- **Line numbers** are the file's current line numbers (`render()`); a taken block keeps its header (`took ours`, keys
  reduced to the other takes) until the resolver closes; reopening shows plain lines — the title then says `0 open`.
- **Keys** (§2.6): `o t b` act on the cursor block **and scroll it into view in the same press**; `n`/`N` next/previous
  *open* block, crossing to the next conflicted file at the end (`no more conflicts · ⏎ marks token.rs resolved` at the
  very end); `⏎` = `Stage` of this file (refusal `2 conflicts still open · n next`), then the next conflicted file, or —
  none left — closes to the banner with `all conflicts resolved · c continue` (no banner: to the pane, `all conflicts
  resolved`); `e` `$EDITOR` at the cursor block's first line, file re-read on return; `M` mergetool on this file, same;
  `j k ↑ ↓ PgUp PgDn g G` scroll; `Esc` closes to where it was opened from; `B` base (pending), `d` start over
  (pending); `Ctrl+C` quits. Mouse: wheel scrolls; click on a block's row makes it the cursor block.
- **BASE** (`B`): a third stacked section `BASE · merged common ancestor` inside every open block at every width;
  `base unavailable after edits` / `no base (added on both sides)` as the notice line when not attachable.
- **Whole-file shapes** (UD/DU/AU/UA/DD, binary): the document is the file as it is (or empty), one card at the top:
  `deleted by them · o keeps the file  t deletes it` — `o`/`t` here are `ResolveWholeFile` and also mark it resolved.
- **Empty/error states**: a file with no markers → `no conflict markers · ⏎ marks it resolved`; `TooLarge` → `too large to
  show (9 MB) · e edit  M mergetool`; `Changed` on a take → `token.rs changed on disk · re-read` and the doc re-reads;
  frame < 60×10 → one line `resolver needs 60x10 · esc closes`, no key acts.
- **Changes pane** `CONFLICTS` row bar becomes `⏎ resolve  M mergetool  e edit  space resolved` (KNOWN_GAPS 558 flips in
  the same commit); **banner** gains `⏎ resolve` first while anything is unmerged.

## Genuinely risky tickets

- **git-conflict-model (G1)** — Fable (signature). Meets all three tests: `ConflictFile` is the spine every later group
  and M7 (rerere, interactive rebase stops) hang on, and a wrong marker rule is structural; whether "a `=======` in a
  Markdown file inside a block is a separator" or "a label with a trailing space" is right is not readable from a diff;
  the builder decides the parse rules, the fixture shapes and the fuzz target. Judged by byte-exact round trips over the
  fixture and a fuzzed parser.
- **app-conflict-resolver (G4)** — Opus, not Fable: the state machine is fixed here; judged by the walk and captures.
- Everything else is decided above. The design is not underspecified. **One Fable build** (G1) + acceptance.

## KNOWN_GAPS rows expiring M6 — disposition (the conductor moves them)

Taken: 121 (fixture → G1), 229 (stash-pop conflicts open in the resolver, no banner → G4; the redo-markers half → M7),
235 (→ G4/G7), 472 (→ G1), 481, 505, 557, 569 (→ G6 `app-answers-beat-rereads`), 482, 571 (doc, Proposed 3/5), 558 (→ G4
seam close), 550 (assert at the write site → G6), 556 (new variant done through the type → G3), 480 (superseded: the
buttons row is gone, m5/16 — close). **Re-expire to M7 with reason "not on the conflict path"**: 148, 493, 494, 495, 498–503,
504, 506–509, 513, 526, 527, 529–532, 534–536, 538, 539, 541, 545–547, 552, 553, 559, 564–568. 211/498/538/568 → the M6
review's runs. 553 was the conductor's own ruling (drop `space fold` from the uncommitted row) — one line in G6.

Groups: **7** → one pass. G1 `git-conflict-model` (Fable sig; graft-git + graft-fixtures), G2 `ops-conflict-operations`
(Opus; graft-ops + graft-core), G3 `ui-conflict-resolver` (brownfield in `hints.rs`; graft-ui), G4 `app-conflict-resolver`
(brownfield; graft, + `hints.rs` at the seam close), G5 `app-resolver-whole-files-and-base` (brownfield; graft), G6
`app-answers-beat-rereads` (brownfield; graft), G7 `acceptance-conflicts` (Fable; graft tests + docs). Disjoint sets:
G1 ∥ G2 ∥ G3 can run concurrently; G4 → G5 → G6 → G7 are all `graft` and serial.
