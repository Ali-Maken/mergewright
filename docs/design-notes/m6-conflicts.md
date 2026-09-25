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

> **Superseded by ADR-0043 (Amendment 2):** file list always; three panes at ≥180 columns; this card below.

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
// amended 2026-09-24 (ticket 14 D1): WholeFile also carries `ours_absent, theirs_absent: Option<SanitizedText>` — the words a side with no file shows in its pane, sanitised in `view`
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
   against `HEAD`). *(Amendment 2 / ADR-0043: three panes aligned by git's regions at ≥180 columns; still no diff alignment.)*
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

> **Amended by Amendment 2 / ADR-0043:** a file list above (strip) or beside (column ≥180); names `role · name`; numbering per projection; no explainer line.

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
  *open* block, crossing to the next conflicted file at the end (`no more conflicts · ⏎ marks token.rs done` at the
  very end); `⏎` = `Stage` of this file (refusal `2 conflicts still open · n next`), then the next conflicted file, or —
  none left — closes to the banner with `all conflicts resolved · c continue` (no banner: to the pane, `all conflicts
  resolved`); `e` `$EDITOR` at the cursor block's first line, file re-read on return; `M` mergetool on this file, same;
  `j k ↑ ↓ PgUp PgDn g G` scroll; `Esc` closes to where it was opened from; `B` base (pending), `d` start over
  (pending); `Ctrl+C` quits. Mouse: wheel scrolls; click on a block's row makes it the cursor block.
- **BASE** (`B`): a third stacked section `BASE · merged common ancestor` inside every open block at every width;
  `base unavailable after edits` / `no base (added on both sides)` as the notice line when not attachable.
- **Whole-file shapes** (UD/DU/AU/UA/DD, binary): the document is the file as it is (or empty), one card at the top:
  `deleted by them · o keeps the file  t deletes it` — `o`/`t` here are `ResolveWholeFile` and also mark it resolved.
  At ≥180 columns (ticket 14, 2026-09-24): the card row and its note span all three panes full width (D4); the pane of a
  side with no file reads `(not on <side>)` for added-by shapes, `(deleted on <side>)` for deleted ones (D1, D2).
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

---

## Amendment 2026-09-23 — the journeys (pipeline steps 1–4, done late)

Read: CLAUDE.md "The feature pipeline"; `docs/research/conflict-resolution.md` (**CR**); `docs/research/ux-principles.md`
(**UX**); `docs/field-notes.md` 2026-09-23; `.scratch/m6-conflicts/scout-stuck-after-merge.md` (**scout**); 02-keymap
§2.1, §2.2, §2.6, §2.6a; ADR-0038 §1, ADR-0042; graft MAP (`update::banner`, `::resolver`, `::relaunch`, `::preview`,
`Status`), graft-ui MAP (`conflict_resolver`, `hints`, `preview_box`), graft-ops MAP (`apply`, `preview`); TESTING.md
:104-109, :2110-2144. No source. Nothing above is withdrawn unless named here.

**Promise (unchanged), now counted:** every journey below has a key count; the acceptance walk replays it and fails
over the count in the `journey-counts` block.

### Need
- **Who / task:** the user, daily, Zed's terminal, team on dependent branches. A merge (or a pull) stops on a conflict;
  get it resolved and committed.
- **What hurt, his words** (field notes 2026-09-23): stuck after the stop — `Esc`, move to the uncommitted row, open it,
  before reaching the conflicts. *"A human needs not take 5 to 6 steps; whatever is enough is enough."* In the
  resolver: numbers on OURS only; a stray `2` before `── conflict 1 of 1`; key bar not in his screenshot
  (**unconfirmed** — may be below his window; ask his terminal size before the prototype).
- **Why** (scout, file:lines): path A — the stop's sticky line (`view/status.rs:56`) takes the bottom row and shows only
  `esc dismiss`; `Esc` hands focus away (`banner.rs:302` → `:265-279`); the preview opens on Cancel, so `m ⏎` cancels.
  Path B — `update/relaunch.rs:43` returns early with no journal entry, so a shell-begun merge never focuses the
  banner. **Which path he took is unknown**; both are fixed.

### Prior art (CR §3 · T take · R reject · I invent)
| Aspect | Best answer | Ours | Rec · why |
|---|---|---|---|
| Landing on a stop | JetBrains opens its dialog itself; lazygit "Conflicts!" menu (CR 1.1, 1.5) | banner focused, keys hidden | **T.** In-spec: the banner's keys own the bottom row. **P5**: open the resolver itself (UX 3.1) |
| Landing on launch, op begun in the shell | not in CR | nothing focused | **I**: git's state decides the landing, not the journal; same landing as a stop |
| Keys at the point of use | lazygit's bottom line (UX 3.4) | a sticky line replaced them | **T**: a stop is a state (H1) and never replaces keys (H6) |
| Way back after `Esc` | not in CR | `Tab` works (`banner::take_focus`), shown nowhere | **I**: `⇥ banner` leads the bar while an op is live (H3, H6). No new key |
| Confirm a previewed op | not in CR; ADR-0038 §1 (his): cursor on Cancel, everywhere | `m → ⏎` | **Keep §1. I, P1**: the verb key again confirms (`m m`). The decision was made at `m` and while reading the picture; the confirm is anticipated (UX 1.1 Rule 1), and `→` is a K with no work in it |
| Advance after a take | KDiff3 auto-advance (CR 1.8) | stays; `n` | **T, P2** — with `n`/`N` reaching taken blocks, so a take stays fixable by keyboard (H3) |
| Whole-file take (text) | JetBrains Accept Theirs; Tower; asked of Sublime #316, lazygit #1608 | none | **T, P3** as "every open block", not `checkout --theirs` (lazygit maintainer's caveat, CR 1.5); `Ctrl+O`/`Ctrl+T` (§2.1: bulk = modifier) |
| Line numbers | each pane its own file (JetBrains/KDiff3, **(?)**) | OURS only | **I**: every line that is on disk carries its disk number, both sides — what `e` and his editor show (H2) *(superseded by Amendment 2: per-pane projection numbers)* |
| Number on the marker | nobody numbers a marker | `2` on the header | **I**: rows that stand for markers carry no number |
| Finish | lazygit's "Continue the merge?" (1 key) | `c` (A); refused when begun in the shell (TESTING.md:108) | Keep A. **P4**: `c` finishes a shell-begun merge |
| Landing after finish | UX 3.1: cursor on the new commit | unspecified | **T**: cursor on the merge commit, `merged … ✓` |
| Three columns / AI | VS Code went back to inline (#160806) | document (ADR-0042) | **R**, unchanged *(reopened by Amendment 2: #160806 changed the default only, LA §0)* |

### Journeys
UX 1.3: key = one chord (`⇥` = Tab); `[M]` = a decision. CR counts are read from docs, not measured. "As built" = scout.

- **J1 merge in the app until it stops.** Start: graph, cursor on `feature` (under the cursor, as the scout found).
  1 `m` → preview, cursor on Cancel [M: read the picture] · 2 `→` · 3 `⏎` → merge runs and stops. **3** (P1: `m m`, 2).
  Lands: banner focused; bottom row `merge stopped · 1 file conflicted · ⏎ resolve  c continue  a abort  M mergetool  e edit  ⇥ graph`.
  CR: no start counts.
- **J2 stop → first conflict.** 1 `⏎` → resolver on the first conflicted file, cursor on block 1. **1** (P5: **0**).
  As built: 1 but unhinted; he took ~5. CR: JetBrains 0 + 1; lazygit 2.
- **J3 begun in the shell → first conflict.** Start: `git merge feature` or `git pull` stopped; he runs `graft`. Lands
  on launch: banner focused, pane on CONFLICTS, bottom row `merge in progress · begun outside Graft · 1 file conflicted
  · ⏎ resolve …`. 1 `⏎` → resolver. **1**. As built: unfocused; `⏎` opened `main`'s commit (scout B). CR: none.
- **J4 one file, one conflict.** Start: resolver on it. 1 `o` [M] · 2 `⏎` → staged. **2**. Lands: next conflicted file;
  none left → banner focused, `all conflicts resolved · c continue`. CR: lazygit 1 + auto-stage; JetBrains 2; Sublime 2.
- **J5 one file, 3 conflicts, theirs everywhere.** Start: resolver on it, cursor on block 1. In-spec `t n t n t ⏎`
  **6**; P2 `t t t ⏎` **4**; P3 `Ctrl+T ⏎` **2**. Lands as J4. CR: KDiff3 "B everywhere" 1 + save **(?)**; JetBrains
  Accept Theirs 1 (whole-file semantics, not the same act).
- **J6 finish the merge.** Start: J4's landing. 1 `c` → `merge --continue`. **1**. Lands: graph focused, cursor on the
  new merge commit, `merged feature into main ✓`. Begun in the shell: in-spec refused; as built (ticket 39) the commit editor's words
  name the banner, `esc, ⇥ banner, then c`; P4 **1**. CR: lazygit 1.
- **J7 back after `Esc`.** Start: J1's landing, `Esc` pressed → graph focused, bar led by `⇥ banner`. 1 `⇥` → banner ·
  2 `⏎` → resolver. **2**. As built: nothing on screen said how.
- **Whole, 1 file 1 conflict.** Path A `m → ⏎ ⏎ o ⏎ c` **7** (from the stop **4** = lazygit 4); P1 6; P1+P5 5.
  Path B `⏎ o ⏎` **3 + shell**; P4 **4**.

**Landing per event:** stop → banner focused, its keys on the bottom row (P5: resolver on the first conflicted file;
`Esc` → banner). Rebase stops again → same, `rebase stopped again at step 4 of 7 · …`. Last file's `⏎` → banner,
`all conflicts resolved · c continue`. Committed → graph, cursor on the new commit. `c` refused or failed → banner stays
focused; the reason leads its key row (H9). Launch into a live op, journal or not → banner focused, pane on CONFLICTS.

```journey-counts-superseded
# Superseded by the journey-counts block in Amendment 2 (counts before P1–P5 were approved). Not parsed.
# id                          keys  once approved
a-merge-to-stop               3     P1:2
a-stop-to-first-conflict      1     P5:0
b-launch-to-first-conflict    1
one-file-one-conflict         2
three-conflicts-all-theirs    6     P2:4 P3:2
a-finish                      1
b-finish                      -     P4:1
back-after-esc                2
```
`-` = leaves the app: the walk asserts the refusal words, not a count. When a P is approved, the conductor moves its
number into `keys` (a spec edit), and the walk tightens with it.

### Interfaces and data shapes (amendment)
```rust
// graft-git/src/conflict.rs                                                     fix-resolver-line-numbers
pub struct SideLines { pub ours: Option<usize>, pub base: Option<usize>, pub theirs: Option<usize> }  // 1-based line in render() of each side's first line; None = no line on disk (empty side, attached base)
impl ConflictFile { pub fn side_lines(&self, block: usize) -> Option<SideLines>; }   // None: no such block, or Resolved
// graft-ui/src/conflict_resolver.rs
pub struct SideNumbers { pub ours: Option<usize>, pub base: Option<usize>, pub theirs: Option<usize> }
pub struct BlockView { …, pub numbers: SideNumbers }   // Header, Labels, End rows never draw a number
// (SideLines / SideNumbers superseded by Amendment 2: Projection / first_lines / BlockNumbers)

// graft/src/update/mod.rs                                                       fix-landing-after-a-stop
pub enum Status { …, Stopped { reason: String } }   // a state: leads the bottom row, never replaces keys, no `esc dismiss`; cleared when the op ends
// graft/src/update/relaunch.rs
pub enum LiveOrigin { ThisRun, PreviousRun, OutsideGraft }
pub(super) fn land_on_live_operation(state: &mut AppState, commands: &mut Vec<Command>, origin: LiveOrigin, next_id: /* launch_into_live's type */) -> bool;  // replaces launch_into_live; gate = in_progress.is_some()
// graft/src/update/banner.rs
pub fn stop_reason(state: &AppState, origin: LiveOrigin, again: bool) -> String;
pub fn one_tab_to_banner(state: &AppState) -> bool;   // take_focus lands on the banner in one Tab from here
// graft-ui/src/graph_screen/hints.rs
pub enum HintContext { …, OperationLive(Box<HintContext>) }   // the row's list, `⇥ banner` (Live) first
pub fn stopped_row(reason: &str, hints: &[Hint], columns: u16) -> String;   // bar fitted first; reason takes the rest, cut with …, dropped under 12 cells

// graft/src/update/resolver (walk.rs, take.rs)                                   fix-take-advances-to-next-block (P2)
pub fn next_block(file: &ConflictFile, from: Option<usize>, forward: bool) -> Option<usize>;   // any state
pub fn after_take(file: &ConflictFile, taken: usize) -> Option<usize>;   // next open after `taken`, wrapping; None = none open
pub struct ResolverState { …, pub in_flight: Option<(TakeAt, Take)>, pub queued: VecDeque<(TakeAt, Take)> }   // in graft/src/update/resolver.rs; in_flight = the take whose TakeSide is out; queued = takes pressed while one is in flight, in order, cap 64, excluding the one in flight; pub enum TakeAt { Block(usize), Every } in update/events.rs (ticket 16); a bulk take is one queue entry   // supersedes (usize, Take)

// graft-ui/src/preview_box.rs                                                   fix-preview-verb-confirms (P1)
pub struct PreviewBox { …, pub again: Option<(SanitizedText, SanitizedText)> }   // ("m", "merge"), drawn first in the footer border

// graft-git conflict.rs + graft-ops conflict.rs                                  fix-take-every-open-block (P3)
impl ConflictFile { pub fn take_all(&mut self, take: Take) -> usize; }   // every Conflict region; how many
pub fn apply_take_all(repo: &RepoHandle, path: &Path, file: &ConflictFile, take: Take) -> Result<ConflictFile, TakeError>;   // the document, as apply_take (ticket 16 D1; supersedes `expected: Oid`)

// graft-core journal + graft-ops                                                fix-continue-a-merge-begun-in-the-shell (P4)
pub enum OperationKind { …, FinishMerge }
pub enum Operation { …, FinishMerge(FinishMergeParams) }   pub struct FinishMergeParams { pub merge_head: Oid, pub message: Option<Vec<u8>> }
pub struct MergeState { pub merge_head: Oid, pub merge_msg: Vec<u8>, pub merge_mode: Option<Vec<u8>> }  // JournalEntry gains merge_before / merge_after, recorded on every entry
// undo: Step::SetMergeState(Option<MergeState>) — last step; Some writes MERGE_HEAD/MERGE_MSG/MERGE_MODE back, None removes them (redo) (ticket 20, spec pass 2026-09-24)
```

### Hard-to-reverse choices (amendment)
1. **What decides the landing: git's state or the journal.** Options: journal live entry (as built) · git's
   `in_progress`. **Git's state**; the journal only picks the words. A shell-begun op has no entry, ever.
2. **Line numbers: the file on disk, markers unnumbered** · vs each side numbered in its stage file. **Disk.** It is
   what `e` and his editor show, and `side_lines` is exact against `render()`; stage numbering needs the alignment
   ADR-0042 reason 1 refused. *(Superseded by Amendment 2 / ADR-0043 §5: per-pane projection numbers; `e` still opens at the disk line.)*
3. **(P4) What `u` after finishing a shell-begun merge restores.** Options: the resolved result as uncommitted changes,
   not merging · the exact mid-merge state, `MERGE_HEAD` included. **Exact.** Every other `u` puts back what was; losing
   the second parent turns the next `c` into a plain commit, silently. Cost: a new inverse step writing git's own files.

### Approved changes P1–P5 (user, 2026-09-23: "keep all 5") — were Proposed; now in-spec
Keymap rows are still to be written into `docs/design/02-keymap.md` (the conductor routes the edit; rows listed in Amendment 2).
- **P1 — preview: the verb key again confirms** (`m m`, `r r`), only once the plan has landed. ADR-0038 §1 is kept
  (cursor on Cancel everywhere, `⏎` = the choice under the cursor). Keymap: Ghost Preview gains `m`/`r` for `.`-less
  opens only (`.` still closes what `.` opened). Trade-off: a double tap merges after one glance — the same glance
  `→ ⏎` needed.
- **P2 — takes advance; `n`/`N` walk every block**, taken ones included (§2.6 row "next/previous *open* block").
  ~~Past the last block: first open block if any (`wrapped · 2 still open`), else today's `no more conflicts` notice.~~
  Amended by the user 2026-09-24 (field notes): past the last block, the next file not done, in list order; else this
  file's first open block (`wrapped · 2 still open`); else `no more conflicts · ⏎ marks <file> done`. Crosses first, then wraps.
  TESTING.md scenario 1's `n t n b` becomes `t b`. Trade-off: `n` stops on taken blocks; the title's count says what
  is left.
- **P3 — `Ctrl+O` / `Ctrl+T`: ours / theirs in every open block of this file**, one write, then `⏎` as usual. No
  bulk-both (`Ctrl+B` is tmux's prefix). **(?)** Zed's terminal may claim `Ctrl+T` — the acceptance capture from his
  terminal checks it.
- **P4 — `c` finishes a merge begun in the shell** (reverses ruling F5; pulls TESTING.md:108 from M7). Merge only;
  shell-begun rebase and cherry-pick stay M7. Journaled as its own entry, exact inverse (choice 3).
- **P5 — a stop with anything unmerged lands in the resolver** on the first conflicted file (§2.6a says the banner
  takes focus). UX 3.1; saves the `⏎` on every stop. `Esc` → banner. A stop with nothing unmerged, and any launch,
  keep the banner. Trade-off: abort from there is `Esc a`, 2 keys, for the rarer act.

### New UI (amendment)
Resolver per §New UI above; bottom rows and numbering change:
```
stop, 120 cols      │ merge stopped · 1 file conflicted · ⏎ resolve  c continue  a abort  M mergetool  e edit  ⇥ graph │
after Esc           │ merge stopped · 1 file conflicted · ⇥ banner  ⏎ open  c commit  . more  …                        │
launch, begun in shell  merge in progress · begun outside Graft · 1 file conflicted · ⏎ resolve  c continue  a abort …
card, ≥100:         │   1 line-one                                                                                  │
                    │▏    ── conflict 1 of 3 · open ── o ours  t theirs  b both ───────────────────                 │
                    │▏    OURS · HEAD                             │ THEIRS · feature                                 │
                    │▏  3 one-ours                                │    5 one-theirs                                  │
                    │▏    ────────────────────────────────────────┴─────────────────────────                        │
                    │   7 b1                                                                                        │
after t (P2)        │     ── conflict 1 of 3 · took theirs ── o ours  b both ──   then  │   2 one-theirs  │  cursor on 2 │
preview (P1)        ╰ m merge  ←→ choose  ⏎ act  esc cancel ╯
```
- Eye first: the bottom row's keys after any stop; in the card, the cursor header, then both numbered sides.
- Stacked (<100): THEIRS lines numbered like OURS. BASE numbered only when its lines are on disk (diff3/zdiff3 markers).
- Focus is still shown by reverse video on `MERGING` — **colour, unverified in captures (rule 5)**; the text tell is
  which keys the bottom row lists.

### Genuinely risky tickets (amendment)
- **fix-continue-a-merge-begun-in-the-shell (P4)** — **Fable**, all three tests: a new journal kind and inverse step
  every later shell-begun op (M7 rebase, pick) builds on; whether `u` truly restores a merging repository is not
  readable from a diff; the builder decides which of git's merge files are state and how they are written back.
- **fix-take-advances-to-next-block (P2)** — Opus: the take queue is new state; judged by tests.
- Acceptance on Fable. **One Fable build if P4 is approved, none otherwise.** Not underspecified.

### Prototype (pipeline step 5) — before any fix builds
Wizard-of-Oz (UX §4): canned 120x40 text frames, a script steps on his keys; one task in his words — *"merge
`feature` into `main` and get it finished."* Ask his terminal size first; add that size. Frames, smallest set:
1 after the stop (bottom row with keys) · 2 after `Esc` (`⇥ banner`) · 3 launch into a shell-begun merge · 4 the card,
both sides numbered, no header number · 5 after `t` with advance (P2) and after `Ctrl+T` (P3) · 6 after `c`, cursor on
the merge commit · 7 the preview footer with `m merge` (P1). Record first key per frame and every stall (UX 4.1);
field notes the same day; findings → ADR.

---

## Amendment 2 — 2026-09-23 — the layout (ADR-0043), the finish summary, P1–P5 approved

Read: `docs/research/conflict-layout-audit.md` (**LA**); prototype v2 frames `docs/shots/m6/prototype-journeys.txt`
(`--layout jb` runs, :12–:2280; frames only); field notes 2026-09-23 (last two); 02-keymap §2.1, §2.2, §2.6, §2.6a,
§2.10; ADR-0042, ADR-0043; graft, graft-ui, graft-ops, graft-git MAPs. No source. **P1–P5 approved** (above) and
counted as in-spec below. ADR-0043 supersedes ADR-0042. New proposals are P6–P9.

### Need
- **Who / task:** unchanged. A merge or a pull stops with conflicts in **several files**; he gets each resolved and the
  merge committed.
- **His words, prototype v1:** *"confusing at which file I was talking with."* His ask: left, middle, right, *"the chosen
  side lands in the middle"*, plus a file list to choose from (field notes, 2026-09-23).
- **His words on finishing:** approved *"yes build it that way"*. `c` shows one screen of what will land: merging X → Y,
  what he resolved (per file, which side), what git combined on its own (each can be opened), and an editable message.
  `⏎` commits and `Esc` goes back. Why: *"the conflicts are only part of what lands on main; git-combined changes are
  never seen otherwise."* **Not** a yes/no confirm, because `u` already undoes, and a bare confirm teaches pressing `⏎`
  without reading.
- **Why it hurt** (LA §C, frames :175, :257, :262): the current file was named in one header row that changed silently;
  each side had 3–4 names; line numbers skipped the marker lines.

### Prior art (LA · T take · R reject · I invent)
| Aspect | Best answer | Ours before | Rec · why |
|---|---|---|---|
| File list | JetBrains Conflicts dialog, Accept Yours/Theirs per file (LA §0, §A); lazygit files panel | one header row | **T**: strip below 180 columns, column at 180 and wider (ADR-0043 §1). H1, H6 |
| Three panes | JetBrains yours / result / theirs (LA §0); lazygit #2133 asks for it | card | **T at ≥180 columns** (LA §B; 48–53 columns a pane at 180, gutter included — ADR-0043 §2; was "46–52 chars", superseded 2026-09-24). **I**: rows line up by git's regions, with no diff (ADR-0043 §2) |
| Names per side | JetBrains: one label per pane | 3–4 names per side | **T + I**: `role · name`, the same everywhere; no parentheses (§2.1) |
| Line numbers | each pane numbers its own file (JetBrains) | skipped the markers | **T**: every pane is numbered with no gaps, by projection (ADR-0043 §5) |
| Whole-file take | JetBrains Accept Theirs, 1 key a file; asked of Sublime (#316) | P3 `Ctrl+T ⏎`, 2 keys | **P6**: `O`/`T` take every open block and mark the file done, 1 key |
| Moving between files | JetBrains dialog list; lazygit `j`/`k` | `n` crosses only at the last block | **P7**: `j`/`k` move between files |
| Finish | lazygit "Continue the merge?" (CR:289) | `c` runs at once | **R** the bare confirm. **I** (his approval): the summary |
| Merge message | not in CR | git's message, never shown | **I**: shown and editable in §2.10's own fields |

### Journeys
Counting as in UX 1.3. The multi-file fixture is the prototype's: `gone.txt` (changed on main, deleted on feature) and
`notes.txt` (2 blocks). "In-spec" now includes P1–P5 and ADR-0043; `P6:` gives the count once P6 is approved.

- **J1 merge until it stops.** Graph, cursor on `feature`: `m` [M: read the picture] · `m`. **2**. Lands in the resolver
  (P5) on the list's first file, cursor on block 0, list `▸` on it, header `MERGING`, bottom row = the resolver's keys.
- **J2 stop → first conflict.** **0**. JetBrains 0 + 1; lazygit 2.
- **J3 begun in the shell → first conflict.** Lands on the banner (a launch is not a stop). `⏎`. **1**.
- **J4 one file, one conflict.** `o` [M] · `⏎`. **2**; P6 `O` **1**. JetBrains 2. Lands on the next file not yet done
  (list order = git's byte-wise path order, stable as files become done — fix-stable-file-order-and-done-file-open D1; wrapping), with the list `▸` moved and the notice `notes.txt done ✓ · ▸ gone.txt`. If no file is left,
  the resolver closes to the banner, status line `every file done ✓ · c finishes the merge` (`fix-resolver-file-list`
  R4/R5). After `31-finish-from-the-resolver` (P8) it stays in the resolver instead, the words on its notice row; a
  stash pop still closes to the graph, status line `every file done ✓`.
- **J5 one file, 3 conflicts, theirs everywhere.** `Ctrl+T ⏎` **2** (the P2 route, `t t t ⏎`, is 4); P6 `T` **1**.
  JetBrains Accept Theirs 1.
- **J5b modify/delete, keep yours.** `o` on the whole-file card (a git write, marks it done). **1**. JetBrains 1.
- **J8 two files in order.** gone `o` · notes `Ctrl+T ⏎`. **3**; P6 `O T` **2**. JetBrains 2.
- **J9 two files, notes first.** Resolver on gone: `n` (the card crosses to the next file) · `Ctrl+T ⏎` (lands on
  gone, the next file not done) · `o`. **4**; P6 `n T O` **3** (P7's `j T O` is also 3).
- ~~**J6 finish, merge begun in the app.** From `every file done`: `c` → summary, cursor on the first row git combined ·
  `⏎` → commit. **2**.~~ Superseded 2026-09-24 (P11, P12 approved): the summary is already open, `y`, **1**
  (Amendment 2026-09-24, J6). Landing unchanged: graph focused, cursor on the merge commit, `merged feature into main ✓`.
- ~~**J6b finish, merge begun in the shell (P4).** `c ⏎`. **2**.~~ Superseded 2026-09-24: `y` **1** when the last file
  was done in the app; `c y` **2** from the banner. Same landing, and one `FinishMerge` entry.
- **J10 see a change git combined on its own.** In the summary: `space` expands that file's diff under its row. **1**.
- **J7 back after `Esc`.** Resolver `Esc` → banner, `Esc` → graph with `⇥ banner` leading the bar: `⇥ ⏎`. **2**.
- **Whole journeys.** A, one file, one conflict: `m m o ⏎ c ⏎` **6** (P6: 5). A, two files: `m m o ^T ⏎ c ⏎` **7**
  (P6 `m m O T c ⏎`: **6**; v1 in-spec was 11). B, two files: `⏎ o ^T ⏎ c ⏎` **6** (P6: 5).

**Where each event lands him (additions).**
- ~~A take → the cursor moves to the next open block (P2).~~ A take → the cursor moves to the next open block **at the
  keypress** (P2); takes typed faster than the disk queue, and a failed one drops the queue with a count.
- A file done (`⏎`, `O`/`T`, or `o`/`t` on a whole-file card) → the next file not done. A failed or changed-on-disk take under `O`/`T` → the same file, notice `… · nothing staged` (ticket 17 D3).
- The last file done → the banner, status line `every file done ✓ · c finishes the merge` (rebase / cherry-pick /
  revert: `c continues the …`); stash pop → the graph, `every file done ✓` (ticket 13 R4/R5). ~~P8: the resolver, with
  the notice~~ superseded 2026-09-24: that is ticket 31's (split from 19 Part B), words on the resolver's notice row; stash pop still closes.
- `c` while files are still open → refused with `1 file still conflicted · ⏎ marks done`.
- `c` while a take or write is landing → refused in the resolver's own words (`a take is still landing …` / `still writing the last take`), nothing sent.
- `c` on a rebase, cherry-pick or revert → continues with no summary (merge only this milestone; gap row → M7). From the resolver it closes to the banner as it sends, so the next stop opens the resolver fresh (P5).
- In the summary:
  - `Esc` → back to where `c` was pressed, with the draft message kept.
  - `⏎` → `committing…`, then the graph with the cursor on the merge commit. Superseded 2026-09-24 (P12): `y` in the
    file list, `⏎` in the subject; `⏎` on a file row shows its diff.
  - A hook refuses → the summary stays, the reason goes above its bar, and the draft is kept.
  - The summary could not be read → `summary unavailable: <why> · ⏎ commits with git's message`. Superseded
    2026-09-24 (P12): `· y commits with git's message`.
  - `y` on a merge begun in the shell → as J6; one `FinishMerge` entry. `u` after it → the merge is back in progress, resolved; header `MERGING`, banner focused, status `undo: finish merge of <from> ✓ · c finishes the merge`; `c y` finishes it again, 2 (ticket 41; P12, 2026-09-24: was `c ⏎`, still 2). No summary opens by itself after `u` (P11: the trigger is the event).
- `c`/`a` in the changes pane while an operation is live → the commit/amend editor, `[ Commit ]` dimmed, words `… esc, ⇥ banner, then c …`. `Esc` → the pane; `⇥` (×n, ticket 39 NOTES) → the banner; `c` → the summary (ticket 39). Superseded by usability fix M3a (m6-usability-fixes.md): the editor does not open; the pane answers in words, `merge in progress` · `⇥ banner, then c …`.
- The commit editor's refusal has its own row above the options row, full width, budgeted always and drawn only when refused; at 80 columns every refusal fits whole (ticket 40).

```journey-counts
# id                          keys  once-approved   # parts (whole rows) · best tool
a-merge-to-stop               2
a-stop-to-first-conflict      0                     # JetBrains 0
b-launch-to-first-conflict    1
one-file-one-conflict         1                    # JetBrains 2 · P11 (approved 2026-09-24): lands on the summary, count unchanged
three-conflicts-all-theirs    1                    # JetBrains Accept Theirs 1 · P11 (approved): lands on the summary, count unchanged
modify-delete-keep-yours      1                     # JetBrains 1
two-files-in-order            2                    # JetBrains 2 · P11 (approved): lands on the summary, count unchanged
two-files-out-of-order        3                    # P11 (approved): lands on the summary, count unchanged
a-finish                      2                     # lazygit 1 (bare confirm, rejected) · P11+P12 approved 2026-09-24 → 1: `y` in the summary the last file opened · lowered by last-file-opens-the-summary
b-finish                      2                     # P11+P12 approved → 1 (`y`) · lowered by last-file-opens-the-summary
summary-open-git-combined     1
back-after-esc                2
open-a-done-file              1                     # Fork #1304 asks for it (CR 1.10); no count in CR
walk-back-a-done-file         4                     # Tower reset-to-unmerged (CR 1.11): count (?) · P15 approved → 3 (`j d y`) · lowered by ticket 50
a-whole-one-file              5                    # a-merge-to-stop + a-stop-to-first-conflict + one-file-one-conflict + a-finish · P11+P12 → 4 (`m m O y`) · approved; lowered by last-file-opens-the-summary
a-whole-two-files             6                    # a-merge-to-stop + a-stop-to-first-conflict + two-files-in-order + a-finish · P11+P12 → 5 (`m m o T y`) · approved; lowered by last-file-opens-the-summary
b-whole-two-files             5                    # b-launch-to-first-conflict + two-files-in-order + b-finish · P11+P12 → 4 (`⏎ o T y`) · approved; lowered by last-file-opens-the-summary
editor-to-finish  5  # M3a (usability M3): the editor never opens to refuse, so the walk starts in the changes pane: c (refused in words) + 2×⇥ + c + ⏎, still 5 · P14 → 2 (c ⏎; c y with P12) · P14 approved 2026-09-24 → 2 (`c y`); lowered by ticket 52
b-finish-again-after-undo    2                     # from the u landing: c ⏎ (ticket 41) · P12: `c y`, still 2 · approved; keys re-sent by finish-summary-is-a-preview
b-abort-shell-merge           3                     # batch 3 (approved 2026-09-24): a → ⏎ from the banner · ticket 50 → 2 (a y) · lazygit 3 (?)
b-undo-a-take-in-the-shell    1                     # batch 3: u from the banner after a whole-file t · JetBrains ⌃Z 1
# last-block-to-finished      4                     # Amendment 2026-09-24 J13: `o ⏎ c ⏎` · P11+P12 → 3 (`o ⏎ y`) · JetBrains ~4 (?) · VS Code 3–4 · lazygit 2 (bare, rejected). Approved 2026-09-24; un-commented at 3 by last-file-opens-the-summary, which adds its scenario
```
The walk parses **this** block, the only fence tagged `journey-counts`. Text after `#` is a comment. P6's numbers were
moved into `keys` by the conductor on 2026-09-24, when ticket 17 built `O`/`T`.

### Interfaces and data shapes (amendment 2)
```rust
// graft-git/src/conflict.rs                                            fix-resolver-line-numbers (revised; SideLines dropped, never built)
pub enum Projection { Ours, Theirs, Result }
impl ConflictFile { pub fn first_lines(&self, p: Projection) -> Vec<usize>; }
// one entry per region, the 1-based first line of that region under `p`. Ours/Theirs: every block, open or taken, counts that
// side's lines. Result: Plain its lines, Resolved its taken lines, Conflict its ours lines (drawn as an unnumbered placeholder).
// An empty region gets the next line's number.

// graft-ui/src/conflict_resolver.rs                                    fix-resolver-line-numbers
pub struct SideNames { pub ours: SanitizedText, pub theirs: SanitizedText, pub ours_role: SanitizedText, pub theirs_role: SanitizedText } // "yours · main" / "yours"
pub struct BlockNumbers { pub ours: usize, pub theirs: usize, pub result: usize }   // this block's first line in each projection
pub struct DocView { …, pub names: SideNames }        // `explainer` removed
pub struct BlockView { …, pub numbers: BlockNumbers }  // `ours_label`/`theirs_label` removed: `DocView::names` carries them
// was: RegionView::Plain.first_line now = the Result projection's number — superseded 2026-09-24:
// RegionView::Plain { numbers: BlockNumbers, .. } replaces first_line (fix-resolver-three-panes, S1); the card draws numbers.result

// graft-ui/src/conflict_resolver.rs                                    fix-resolver-file-list (owns these, R1; three-panes consumes)
pub const THREE_PANE_MIN: u16 = 180;
pub enum Arrangement { ThreePane, Card }
pub fn arrangement(frame_columns: u16) -> Arrangement;
// graft-ui/src/conflict_resolver/files.rs (new)                         fix-resolver-file-list
pub struct FileRow { pub name: SanitizedText, pub state: FileState }
pub enum FileState { Open { open: usize, of: usize }, AllTaken, Done, WholeFile { how: SanitizedText }, Binary, Unread }
pub struct FileListView { pub rows: Vec<FileRow>, pub cursor: usize, pub unavailable: bool }
pub enum Placement { Column { width: u16 }, Strip { rows: u16 } }
pub fn placement(view: &FileListView, frame_columns: u16) -> Placement;          // Column iff arrangement(frame_columns) == ThreePane
pub struct ResolverAreas { pub list: Rect, pub doc: Rect }
pub fn resolver_areas(area: Rect, placement: Placement) -> ResolverAreas;        // draw and hit-test split the area through this
pub fn file_at(view: &FileListView, placement: Placement, width: u16, dx: u16, dy: u16) -> Option<usize>;  // offsets from `list`'s origin
pub struct FileList<'a> { pub view: &'a FileListView, pub placement: Placement, pub theme: &'a Theme, pub ascii: bool }  // Widget
pub struct DocView { …, pub files: FileListView }

// graft-ui/src/conflict_resolver.rs                                    fix-resolver-three-panes (THREE_PANE_MIN/Arrangement/arrangement moved to file-list above, 2026-09-24)
pub enum LineRef { Plain { region: usize, i: usize }, Ours { block: usize, i: usize }, Theirs { block: usize, i: usize }, Taken { block: usize, i: usize }, Base { block: usize, i: usize } }
pub enum Cell { Line { line_no: Option<usize>, text: LineRef }, Pad, Placeholder, Rule }
pub enum ScreenRow { /* existing */ PaneTitles, Aligned { block: Option<usize>, ours: Cell, result: Cell, theirs: Cell } }
pub fn layout(doc: &DocView, arrangement: Arrangement, columns: u16) -> Layout;   // was (doc, columns); Card == today's rows
impl Placement { pub const fn arrangement(self) -> Arrangement; }   // width from state.size.columns via resolver_split
pub fn three_panes(doc: Rect) -> [Rect; 3];   // doc minus two `│`, thirds, remainder to the result; gutters included (ADR-0043 §2)

// graft-ops/src/conflict.rs                                            fix-ops-conflict-set-and-summary
pub struct ConflictSet { pub paths: Vec<PathBuf>, pub recorded: bool }   // unmerged in git's status order, then resolved ones in MERGE_MSG order; recorded = git's list was found
pub enum ConflictSetError { Io(io::Error), Run(RunError) }
pub fn operation_conflicts(repo: &RepoHandle, facts: &RepoFacts) -> Result<ConflictSet, ConflictSetError>;
// graft-ops/src/finish.rs (new)
pub struct FinishSummary { pub into: Vec<u8>, pub from: Vec<u8>, pub by_you: Vec<ByYou>, pub by_git: Vec<ByGit>, pub message: Vec<u8>, pub recorded: bool }
pub struct ByYou { pub path: PathBuf, pub result: ResultIs }
pub enum ResultIs { Yours, Theirs, Deleted, Combined }   // the staged blob vs HEAD:path and MERGE_HEAD:path
pub struct ByGit { pub path: PathBuf, pub change: Change, pub added: usize, pub removed: usize }
pub enum Change { Added, Modified, Deleted, Renamed { from: PathBuf }, TypeChanged }
pub enum SummaryError { NotMerging, StillUnmerged { count: usize }, Facts(FactsError), Run(RunError), Io(io::Error) }
pub fn finish_summary(repo: &RepoHandle) -> Result<FinishSummary, SummaryError>;
// graft-ops/src/apply/resume.rs
pub enum Resume { Continue, ContinueWith { message: Vec<u8> }, Skip, Abort }   // ContinueWith: merge only → `git commit --cleanup=whitespace -F -` (review m6-amend-09 Q1)
pub enum Refusal { …, MessageNotHere { kind: InProgress } }                       // ContinueWith on a rebase / pick / revert (M7)
// fix-continue-a-merge-begun-in-the-shell: FinishMergeParams gains `pub message: Option<Vec<u8>>`

// graft (app)                                                          fix-resolver-file-list, fix-finish-summary-screen
pub struct ResolverState { …, pub done: Option<DoneShape> /* ticket 25 */, pub files: Loading<Arc<ConflictSet>>, pub set_job: Option<JobId>, pub seen: BTreeMap<PathBuf, RowState>, pub settling: Option<Take> /* ticket 17: an O/T waiting for its Stage; Some ⇒ in_flight.is_some() */ }
// was `pub file_cursor: usize` — superseded 2026-09-24 (ticket 13 R2): a function of the open path, cannot drift
// graft/src/update/resolver/files.rs (new)                              fix-resolver-file-list
pub enum RowState { Open { open: usize, of: usize }, AllTaken, Done, WholeFile { how: UnmergedHow }, Binary, Unread }
pub struct ListedFile { pub path: PathBuf, pub state: RowState }
pub fn listed_files(state: &AppState) -> Vec<ListedFile>;   // sorted once here, git's path order (byte-wise), over set ∪ extra unmerged ∪ open path; a done file keeps its row (was: ops' order, done files after open — superseded 2026-09-24, ticket 25 D1)
pub fn file_cursor(state: &AppState) -> usize;   // index of ResolverState::path in listed_files; 0 when absent
pub fn select_file(state: &mut AppState, commands: &mut Vec<Command>, index: usize, next_id: /* resolver's type */) -> bool;  // click; `j`/`k`/`↓`/`↑` step file_cursor(state) ± 1, no wrap (ticket 17 D4); a Done row opens read-only (ticket 25; 13's refusal dropped)
#[derive(Clone, Debug, PartialEq, Eq)] pub enum DoneShape { Text { unstaged: bool }, Binary { unstaged: bool }, Removed, Unreadable { why: String } }   // wrapped in DoneOpen { shape, start_over } (ticket 28)
// the Product answering Command::OpenConflict carries Result<graft_ops::conflict::Opening, …> (was Opened)
// graft-ops/src/conflict/done.rs (new; re-exported from `conflict`)     fix-stable-file-order-and-done-file-open
pub enum DoneFile { Text { file: graft_git::conflict::ConflictFile, unstaged: bool }, Binary { unstaged: bool }, Removed }
pub enum Opening { Unmerged(Arc<OpenedFile>), Done(Arc<DoneFile>, StartOver) }   // graft jobs::conflict (built: 25; StartOver: 28)
pub fn open_done(repo: &RepoHandle, status: &WorkingTreeStatus, path: &Path) -> Result<DoneFile, OpenError>;
pub fn open_listed(repo: &RepoHandle, path: &Path) -> Result<Opening, OpenError>;   // reads the status itself; open_conflict unchanged
// OpenError gains Status(<graft-git status reader's error>)
// graft-ui hints: HintContext::Resolver { base, restart, whole_file, done: bool, roles }
pub struct Choices { pub takes: Vec<Option<graft_git::conflict::Take>>, pub whole: Option<graft_ops::operation::conflict::Side>, pub edited: bool }
pub struct AppState { …, pub choices: BTreeMap<PathBuf, Choices>, pub finish: Option<FinishState>, pub finish_draft: Option<(String, String)> }   // (subject, body); apart from commit_draft (ticket 19 S7)
pub struct FinishState { pub summary: Loading<Arc<FinishSummary>>, pub job: Option<JobId>, pub cursor: usize, pub top: usize, pub expanded: Option<(usize, Loading<Arc<DiffView>>)>, pub diff_job: Option<JobId>,
    pub subject: TextField, pub body: TextArea, pub focus: FinishFocus, pub return_to: ReturnTo, pub committing: Option<JobId>, pub editing: Option<JobId>, pub notice: Option<(String, bool)> }
pub enum FinishFocus { Files, Subject, Body }   pub enum ReturnTo { Banner, Resolver }
pub fn open_finish(state: &mut AppState, commands: &mut Vec<Command>, from: ReturnTo, next_id: &mut impl FnMut() -> JobId) -> bool;
pub fn finish_key(state: &mut AppState, commands: &mut Vec<Command>, key: KeyPress, next_id: &mut impl FnMut() -> JobId) -> bool;    // modal; before resolver_key
pub enum GraphOverlay { Finish, … }   pub enum Command { …, ReadConflictSet(JobId), ReadFinishSummary(JobId), ReadFinishDiff(JobId, PathBuf) }   // own diff route (ticket 19 S4)
pub enum Product { …, ConflictSet(Result<Arc<ConflictSet>, String>), FinishSummary(Result<Arc<FinishSummary>, String>), FinishDiff(Result<Arc<DiffView>, String>) }
// graft-ui/src/finish_summary.rs (new)
pub struct SummaryRow { pub path: SanitizedText, pub what: SanitizedText }
pub struct FinishView<'a> { pub title: SanitizedText, pub by_you: Vec<SummaryRow>, pub by_git: Vec<SummaryRow>, pub recorded: bool, pub cursor: usize,
    pub expanded: Option<(usize, Arc<DiffView>)>, pub subject: FieldView<'a>, pub body: AreaView<'a>, pub focus: SummaryFocus, pub notice: Option<(SanitizedText, bool)>, pub busy: bool }
pub enum SummaryFocus { Files, Subject, Body }
pub struct FinishScreen<'a> { pub view: &'a FinishView<'a>, pub theme: &'a Theme, pub ascii: bool }   // Widget
pub const FINISH_MIN: (u16, u16) = (60, 14);   pub const TWO_COLUMN_MIN: u16 = 160;
pub enum HintContext { …, Finish { focus: SummaryFocus, expandable: bool } }
```

### Hard-to-reverse choices (amendment 2)
1. **What the yours and theirs panes show.** Options: projections of the conflicted file, or git's stage files with a
   three-way alignment. **Projections** (ADR-0043 Consequences). Cost: a change git merged in from the other side shows
   in both side panes. The summary's *combined by git* section names those changes.
2. **Where the operation's set of conflicted files comes from.** Options: this run's memory, or git. **Git**: the
   unmerged paths, plus `MERGE_MSG`'s `Conflicts:` list, plus the live entry's backup chain. With git, a relaunch and a
   merge begun in the shell list the same files. This run's memory adds only the per-block choices (`Choices`). Cost: we
   parse a comment block git writes. The builder verifies it per operation kind in a throwaway repo. When it is absent,
   `recorded: false`, and the screen says so instead of guessing.
3. **How an edited message reaches git.** Options: rewrite `MERGE_MSG` and run `--continue`, or
   `git commit --cleanup=whitespace -F` (m6-conflicts/22). **`commit -F`**: git's own state files are written only by git, and P4's inverse
   restores `MERGE_MSG` exactly as git left it. An unedited message uses today's `--continue`, byte for byte.

### Keymap rows owed for P1–P5 (the conductor writes them into `02-keymap.md`)
- **Ghost Preview** (a new row under §2.3 or its own section): `m` / `r`, when the preview was opened by that key and
  its plan has landed, is Confirm. The cursor still rests on Cancel, and `.`-opened previews are unchanged.
- **§2.6** (written, `02-keymap.md:267`): `n` / `N` become "next / previous block, taken ones included; at the file's
  last block, cross to the next file; past the end, wrap to the first open block". ~~past the end, the first open block;
  at the last block, cross to the next file~~ (order fixed 2026-09-24). A new line: `o`/`t`/`b` move the cursor to the next open block.
- **§2.6**: new row `Ctrl+O` / `Ctrl+T`: yours / theirs in every open block of this file (no staging).
- **§2.6a**: `c` also finishes a merge begun in the shell. For a merge, `c` opens the finish summary (§2.6b).
- **§2.6a**: replace "It takes focus the moment an operation stops" with "A stop with anything unmerged opens the
  resolver on the first conflicted file, and `Esc` there lands on the banner. A stop with nothing unmerged focuses the
  banner."

### Where I'd differ (Proposed, not decided; gated specs wait for them)
- **P6 — `O` / `T` in the resolver: yours / theirs in every open block of the current file, then mark it done and move
  to the next file**, in one key. On a whole-file card they are the same as `o`/`t`. This conflicts with §2.1:
  UPPERCASE is reserved for places, modes and the two direction pairs, and bulk actions use a modifier. It needs a named
  exception, like the banner's `B`/`X`. Why: the list puts the file on screen, so acting on it is 1 key (JetBrains Accept
  Theirs); the prototype walked it. Trade-off: a slipped `Shift` resolves a whole file. It can be recovered with `d`
  (start the file over, which reads the stages back from the backup chain).
- **P7 — `j`/`k` (and `↑`/`↓`) move between files in the resolver.** The document scrolls with `n`/`N`, `PgUp`/`PgDn`,
  `g`/`G` and the wheel. Why: the list is the resolver's selection, so §2.2's "move selection" applies. Trade-off: no
  line-by-line scrolling key, though the cursor block always scrolls itself into view.
- **P8 — `c` in the resolver finishes**: it opens the summary for a merge and continues for anything else. Refused while
  files are open. The resolver stays open after the last file is done (the prototype's frame 4). Why: the list shows
  that every file is done, and the finish key belongs where the eye already is. Same key count either way.
- **P9 — §2.6b "Finish summary"** rows: `⏎` commit and `Esc` back (his approval); `space` shows or hides the file's
  diff, and `j`/`k` move (§2.2); `⇥` moves focus files → subject → body; `Ctrl+Enter` commits and `Ctrl+E` opens
  `$EDITOR` (§2.10). This reuses existing meanings, so it is a doc edit.
- ADR-0043 is accepted, so rewriting `docs/design/01` §1.5 is a consequence of it, not a proposal.

### New UI (amendment 2) — resolver layout and finish summary
Mockups are hand-drawn and not to scale; the widths are stated in the text. The keys shown assume P6–P8 are approved.
Without them, `O/T`, `j/k` and `c finish` are left off the bar. The header keeps `MERGING` / `REBASING step n of m`.

**≥180 columns (200x50): list column + three panes.** The list column is `clamp(longest row, 22, 32)` wide, and each pane
is a third of the rest.
```
 graft ~/code/api                                                                                       MERGING
 FILES · 2 conflicted        │ ▸ notes.txt · 2 conflicts · 1 open
                             │   yours · main                 │   result                      │   theirs · feature
   gone.txt    done ✓        │    1 one                       │    1 one                      │    1 one
 ▸ notes.txt   1 of 2 open   │  ── not taken                  │ ── 1 of 2 · took theirs       │ ── taken ✓
                             │    2 two from main             │    2 TWO from feature         │    2 TWO from feature
                             │    3 three                     │    3 three                    │    3 three
                             │▏ ── o take                     │ ── 2 of 2 · open              │ ── t take
                             │▏   4 five from main            │      ? open · o t b           │    4 FIVE from feature
                             │    5 six                       │    5 six                      │    5 six
 o yours  t theirs  b both  ^O/^T all  O/T file  ⏎ done  j/k file  n next  B base  e edit  c finish  esc banner
```
- The eye goes first to the cursor block's rule row across all three panes (`▏` on its rows), then to the result
  pane's `? open` placeholder, then to the list's `▸`. The block rule reads `o take` / `t take` on the sides while
  open, and `taken ✓` / `not taken` once taken.
- `B` puts the block's BASE lines in the result placeholder, unnumbered, headed `base`. Text too long for a pane is cut
  with `…` (no horizontal scroll this milestone; gap row → M7).
- As built (ticket 14 D3, D5, 2026-09-24): with `B` on, the placeholder head reads `base` over muted base lines instead
  of `? open · o t b`; a binary result reads `? open · o t` (no `b`). The rule row starts at the pane's mark column + 1,
  padded, not `─`-filled to the edge; `│` separators draw only on rows with content.
- **Not yet built (ticket 14 D6):** a take does not advance the cursor; `n` is still needed after `t` until
  `fix-take-advances-to-next-block` (ticket 15, P2) lands. The captures press `t` then `n`.
- Mouse: a click on a list row selects that file; a click on any pane's block row makes it the cursor block; the wheel
  scrolls all three panes together.

**Below 180 columns (120x40, 160x48): list strip + card.** The strip is one row when every entry fits. Otherwise it is
a grid of `name  state` cells, filled row by row, up to 3 rows, keeping the cursor's row visible, with `… N more` last.
```
 graft ~/code/api                                                              MERGING
 files   gone.txt · done ✓    ▸ notes.txt · 1 of 2 open
 ────────────────────────────────────────────────────────────────────────────────────
 ▸ notes.txt · 2 conflicts · 1 open
     1 one
   ── conflict 1 of 2 · took theirs · feature ─────────────────────────────────
     2 TWO from feature
     3 three
▏  ── conflict 2 of 2 · open ── o yours  t theirs  b both ─────────────────────
▏       yours · main                     │       theirs · feature
▏     4 five from main                   │     4 FIVE from feature
▏  ──────────────────────────────────────┴──────────────────────────────────────
     5 six
 o yours  t theirs  b both  ^T all theirs  O/T file  ⏎ done  j/k file  n next  e edit  esc banner
many files:  files · 7 · 2 done   ▸ src/auth/token.rs 2 of 3 open   src/api/routes.rs done ✓   Cargo.lock ⏎ marks done
                                    docs/gone.md keep or delete      src/lib.rs 1 of 1 open     … 2 more · j/k
```
- At 160 the layout is the same as at 120, with wider halves and more of the bar at full rungs. At 80x24 the sides
  stack (below 100). The strip compacts a state to `1/2 open` / `✓` before it becomes a grid.
- The eye goes first to the file identity row (`▸ notes.txt · …`), which repeats the strip's `▸`, then to the cursor
  card.
- Empty and error states are unchanged from the ADR-0042 section above. A file the resolver has not read yet shows
  `Unread` as `…`. If the set could not be read, the strip reads `files: git's list unavailable · the unmerged ones below`.
- A done file clicked opens **read-only** (fix-stable-file-order-and-done-file-open D2): identity row
  `▸ <path> · done ✓ · d starts it over` (+ ` · edited since done` when unstaged), the working-tree file's plain lines;
  binary / removed draw a card `done ✓ · binary` / `done ✓ · removed`. Takes refused: `<path> is done · d starts it over`.
  Bar: `d start over  e edit  esc close`, or `e edit  esc close` where ops cannot start it over (Amendment 3). A done file is one card at every width (ticket 25, ruling 4).

**Finish summary** (full frame under the header, like the resolver; modal).
```
120x40                                                                  160x48 and 200x50: two columns
 graft ~/code/api                                           MERGING      files on the left (60 %), MESSAGE on the right:
 Finish · merging feature → main · nothing is written until ⏎            subject field + body area (§2.10's look)
 RESOLVED BY YOU · 2
   gone.txt      kept · yours · main
   notes.txt     theirs · feature in 2 of 2 conflicts
 COMBINED BY GIT ON ITS OWN · 3 · space shows the change
 ▸ src/lib.rs    modified  +12 −3
     │ @@ -10,6 +10,9 @@ …            (expanded by space: at most 12 rows, then `… 9 more lines · space hides`)
   README.md     modified  +1 −1
   src/new.rs    added     +40
 MESSAGE · ⇥ edit
   Merge branch 'feature' into main
 ⏎ commit  space diff  j/k move  ⇥ message  esc back
```
- The eye goes first to the title's `merging feature → main`, then to the cursor, which opens on the **first row git
  combined** (the part he has never seen), then to `⏎ commit`.
- How each row under *resolved by you* is worded. This run's `Choices` come first:
  - `theirs · feature in 2 of 2 conflicts`, or `yours · main in 1, theirs · feature in 1`;
  - `kept · yours · main` or `deleted · as theirs · feature`;
  - `edited in your editor`.

  Without `Choices`, the row falls back to `ResultIs`: `same as yours · main`, `same as theirs · feature`, `deleted`,
  or `combined`.
- Focus and keys:
  - `⇥` moves focus files → subject → body → files.
  - In the subject, `⏎` commits. In the body, `⏎` is a newline and `Ctrl+Enter` commits.
  - `Esc` goes back from any focus, keeping the draft in `finish_draft`.
  - A click on a row moves the cursor there and toggles its diff.
- States:
  - Loading: `reading what this merge changes…`, and `⏎` answers `still reading · wait`.
  - Git combined nothing: `COMBINED BY GIT ON ITS OWN · none`.
  - `recorded: false`: one section, `CHANGED BY THIS MERGE · n · which were conflicts is not recorded`.
  - Read failed: `summary unavailable: <why> · ⏎ commits with git's message`.
  - Empty subject: `⏎` answers `the message is empty · ⇥ to write one`.
  - Committing: the bar reads `committing…` and every key but `Ctrl+C` is swallowed.
  - A hook refuses: the reason appears above the bar.
  - Frame smaller than 60x14: `finish needs 60x14 · esc back`, and `⏎` does nothing.
- Colour and reverse video are **unverified** in captures (rule 5). The text tells are `▸` and the bar.

### Genuinely risky tickets (amendment 2)
- **fix-continue-a-merge-begun-in-the-shell (P4, approved)**: **Fable**, for the three reasons above. It is now also
  reached through the summary (`message`).
- **fix-resolver-three-panes**: Opus. The layout is a pure function checked against golden files, and the projection
  rule is decided here, not by the builder.
- **fix-ops-conflict-set-and-summary**: Opus. Whether git writes `Conflicts:` for each kind has to be checked in
  throwaway repos, and the diff alone cannot show it, but every decision is made here, so it does not pass Fable test 3.
- Fable builds this milestone: G1 (built) and P4, plus acceptance. The design is not underspecified.

### Specs (`.scratch/m6-conflicts/issues/`) — what supersedes what
- **Revised in place:**
  - `fix-resolver-line-numbers`: now one name per side plus projection numbers; `SideLines` is dropped.
  - `fix-take-every-open-block`: P3, un-gated.
  - `fix-take-advances-to-next-block`: P2, un-gated.
  - `fix-stop-opens-the-resolver`: P5, un-gated.
  - `fix-preview-verb-confirms`: P1, un-gated.
  - `fix-landing-after-a-stop`: item 5 is now rebase-only; a merge's `c` goes to the summary.
  - `fix-continue-a-merge-begun-in-the-shell`: un-gated, now takes a `message`.
  - `fix-acceptance-conflict-journeys`: rewritten.
- **New:**
  - `fix-ops-conflict-set-and-summary`: interface first.
  - `fix-resolver-file-list`.
  - `fix-resolver-three-panes`.
  - `fix-finish-summary-screen`: its Part B is gated on P8.
  - `fix-resolver-file-keys`: gated on P6 and P7.
- **Order.** Every one of these touches `graft`, so they run one after another in `graft`:
  1. `fix-ops-conflict-set-and-summary`
  2. `fix-resolver-line-numbers`
  3. `fix-resolver-file-list`
  4. `fix-resolver-three-panes`
  5. `fix-finish-summary-screen`
  6. P1, P2, P3 and P5, in any order
  7. P4, after step 1
  8. `fix-resolver-file-keys`, after the file list and P3
  9. acceptance, last

  The pure `graft-ui` and `graft-git` halves of steps 2–4 can be built ahead in a worktree limited to those crates.

---

## Amendment 3 — 2026-09-24 — a done file: read it, start it over (review m6-amend-25 M1, M2)

Read: `docs/reviews/m6-amend-25.md`; `docs/research/conflict-resolution.md` (**CR**) 1.10, 1.11, §3 :287; `docs/research/ux-principles.md` (**UX**) 1.3, 2, 3.1, 3.5, §5; 02-keymap :111, §2.6 `d`; ticket 25 (spec + NOTES); graft-ops, graft MAPs. No source. Ticket: `28-done-file-start-over-only-where-it-can`.

### Need
- **Who / task:** he pressed `⏎` on a file and then saw he took the wrong side. Or he wants to look again at what he chose before `c`.
- **What hurts:** 25 built it, but `d` is offered on every done file. ops refuses after the confirm on a whole-file shape, and on **any** done file of a merge begun in the shell (J3's start). No field note yet: this comes from review 25, not from his use.

### Prior art (CR · T take · R reject · I invent)
| Aspect | Best answer | Ours before | Rec · why |
|---|---|---|---|
| See a done file | Fork: an open request to show how each conflict was resolved in the file list (CR 1.10, #1304, **(?)** title only); JetBrains: a resolved file leaves the list (CR 1.1) | a done row keeps its place and opens read-only (25) | **T** the Fork ask, as built. H1: what he chose stays one click away |
| Walk back a `⏎` | Tower: "reset a merge conflict to its unmerged state, even after it has been resolved" (CR 1.11 :207; CR §3 :287 recommends **T**) | `d` on every done file; ops refuses two shapes after the confirm | **T** Tower's reset, **only where ops can do it**. **I**: ops' own gate decides the offer when the file opens (UX H5: block the impossible, don't warn after; keymap :111) |
| Why it can't | not in CR | a false promise, then a refusal | **I**: `d` is not drawn, and pressing it answers ops' words with no box (H9) |
| Confirm the walk-back | Tower: **(?)**, not in CR | the box, cursor on Cancel (ADR-0038 §1) | **Keep.** It is not undoable while the operation is open, so the confirm is required (UX 3.5) |

### Journeys (UX 1.3; `[M]` = a decision)
- **J11 walk back a mistaken `⏎`.** Start: the landing after `⏎` (the next file open, the done row above it). Steps: click the done row (or `k` once 17/P7 lands) · `d` [M: read the box] · `→` · `⏎`. **4**. Tower: the count is not in CR **(?)**.
  - Lands: the same file, unmerged again, open blocks, cursor on block 0, `▸` on its row in the same cell.
- **J12 open a done file to read it.** Click its row (or `k`/`j`, P7). **1**. Lands: the file's working-tree lines, read-only; bar `d start over  e edit  esc close`, or `e edit  esc close` where it cannot start over. The Fork ask; no count in CR.
- **As built, where it can't:** `d → ⏎` = 3 keys, the box's false promise, then the refusal. **Now:** no `d` in the bar, and `d` answers in words. **0** keys wasted.

**Where each event lands him (additions).**
- A done row opened → the done file, as J12.
- `d` where ops can → the box; `→ ⏎` → J11's landing.
- `d` where ops can't → no box. Status line `can't start over · <ops' words>`. Stays on the done file.
- ops refuses after the confirm (the repo changed behind the app) → stays on the done file, ops' words, the write released.

### Interfaces (amendment 3)
Ticket 28's Signatures: `graft_ops::apply::restart_gate`, `jobs::conflict::StartOver`, `Opening::Done(Arc<DoneFile>, StartOver)`, `OpenFailure.done: Option<StartOver>`, `DoneOpen`, `start_over_refusal`.

### Hard-to-reverse choices (amendment 3)
1. **Who decides "can `d` start this over"?** Options: the app mirrors ops' rules, or ops' own gate, called by both the offer and the act. **Ops' gate.** The mirror has drifted twice (m6-05 M1, m6-amend-25 M1).
2. **When is it asked?** Options: when the file opens (in the job), or when `d` is pressed (a check job before the box). **When it opens.** The bar is right before he looks (H6). Cost: it can go stale, and ops' refusal after the confirm is the backstop.

### Where I'd differ (Proposed, not decided)
- **P10: `d` starts a whole-file shape over**, in an app-begun merge. `restore::restore_stages` already puts a path's recorded stages back; the working tree goes back to the side git left. The box would read `the choice comes back: keep or delete`. Why: J11 fails today on every modify/delete, which is J5b's shape. Trade-off: a second restart path in ops, and new box words. A merge begun in the shell still cannot, because nothing kept the stages (gap M6/25; REUC rejected, graft-ops MAP §apply `stage_source`).

### New UI (amendment 3)
The done file per Amendment 2 "A done file clicked opens read-only". The identity row and the bar now follow the gate:
```
can, 120x40      ▸ both.txt · done ✓ · d starts it over
                 d start over  e edit  esc close
cannot, 120x40   ▸ gone.txt · done ✓
                 e edit  esc close
   after d       can't start over · gone.txt: deleted by them — no markers to take from      (status row, no box)
```
- The eye goes first to the identity row. Whether `d starts it over` is there is the answer.
- The card (removed / binary / unreadable) carries `d starts it over` as its keys only when the gate says yes.
- Other refused keys: `<path> is done · d starts it over` when it can, `<path> is done` when it can't.

### Genuinely risky tickets (amendment 3)
None. Ticket 28 goes on Opus: every rule is fixed here, and `d_is_in_the_bar_exactly_when_the_box_opens` judges it.

---

## Amendment 2026-09-24: look and finish

Read: `docs/research/conflict-visuals-and-finish.md` (**CV**, whole); `docs/research/ux-principles.md` (**UX**) 1.3, 1.4, 2,
3.1, 3.4, 3.5, §5; field notes :48–50, :54, :57, :58, :60, :61; 02-keymap §2.1, §2.2, §2.3b, §2.6, §2.6a, §2.6b; ADR-0038
§1, ADR-0041, ADR-0043 §2 §4; PROMPT.md §5; design 04 §4.6; CONTEXT.md "Ghost Preview"; graft-ui MAP (`theme`,
`conflict_resolver`, `finish_summary`, `hints`, `preview_box`); captures `docs/shots/m6/` m6-12, m6-14, m6-15 (187, 120,
80), `fix-finish-from-the-resolver-120x40`, `fix-preview-verb-120x40`; TESTING.md :2307–:2381;
`.scratch/m6-conflicts/user-batch.md` (his 11 open questions); `.scratch/m6-conflicts/user-facts-m632.md`;
docs/log/m6.md:78. No source. **Pipeline step 5 comes before specs:** no ticket is written from this until he has run
the prototype below. **Met 2026-09-24:** he ran it ("demo looks better than before", field notes :62-:63) and approved
the picks (:65, `.scratch/m6-conflicts/user-picks-2026-09-24.md`). Specs: "Specs, 2026-09-24" below.

### Need
- **Who / task:** unchanged. A merge stopped; he resolves it in the three panes (187 columns, Zed, dark theme — field
  notes :48–49, :57) and commits it.
- **His words** (field notes :60–61, 2026-09-24): *"after enter, the c part was confusing"*: he did not know to press
  `c`, and the finish screen did not read as a preview, so the `⏎` that committed surprised him. *"also the left middle
  and right. there was no color and lines like there on other tools on market."*
- **What the captures show** (textual, so they stand on the capture):
  - After the last `⏎` the bottom row is `stage b.txt ✓`, with no keys. `c finish` shows only after the next key
    (`fix-finish-from-the-resolver-120x40.txt:40` vs `:81`; at 187, `m6-14-last-file-done-187x50.txt:52`).
  - The only pointer to `c` is one plain notice row (`fix-finish-from-the-resolver-120x40.txt:5`), drawn with the same
    weight as every other row.
  - The summary opens with the cursor on a file row (`m6-15-finish-summary-187x50.txt:9`), where `⏎` commits. Everywhere
    else `⏎` acts on what is under the cursor (§2.2, ADR-0038 §1) and `y` confirms a preview (§2.3b, field notes :58).
  - The summary has no frame and no `Preview:` title. The merge preview he has already seen has both
    (`fix-preview-verb-120x40.txt:44–52`).
  - The panes' only styling is muted titles and a reversed cursor rule row (graft-ui MAP, `panes/draw.rs`).
- **Why this is not M8's colour pass:** whether a block is open or taken, and which side a result line came from, is
  **state**. That falls under ADR-0041's carve-out ("can the user tell") and heuristic H1. It is not a colour scheme.
- **His setup is known:** Zed's terminal, dark theme, 187 columns (field notes :49, :50, :57). The prototype runs there
  and does not ask.

### Prior art (CV · T take · R reject · I invent)
| Aspect | Best answer | Ours before | Rec · why |
|---|---|---|---|
| Side colours | VS Code teal / blue. BC teal / magenta. ediff red / green tints. JetBrains colours by change type, not by side (CV §1.1, §1.2, §2.3) | none | **T** BC's pair: the left pane in a sky blue, the right in a purple-magenta. Never red / green (colour-blind complaints, CV §1.3). **T** tint the background and leave the text in the theme's own colour (VS Code, ediff, Neovim; CV §1.3). **I** the exact values (Okabe–Ito-like). CV does not give them, so the prototype judges them |
| Open, taken, resolved | VS Code: the border goes orange → grey. Araxis: a resolved block loses its colour. JetBrains: the band goes from filled to a dotted outline (CV §1.1, §2.1) | a reversed cursor rule | **T**: an open block is amber (alarm). A taken block carries the chosen side's hue into the result, and the other side is muted. A done file has no tint (Araxis) |
| Without colour | KDiff3's `A/B/C/?` column; ediff's inverse video (CV §1.3) | — | **T**: every state has a glyph (`? ┃ ▶ ◀ ┊ ✓`), and position already names the side. Captures prove the glyphs, buffer tests prove the style (ADR-0041), and only legibility is left to his eye |
| Connectors | JetBrains: curved bands, dotted once resolved. VS Code: no bands, a gutter bracket. Meld: segments. No terminal tool draws links (CV §2) | a plain `│` | **R** curves: a cell grid cannot draw them, and no terminal tool tries. **I**: on a block's rows, the existing 1-cell pane separator becomes the link, and the taken side's tint runs through it into the result. That is JetBrains' band, one cell wide, in VS Code's state colours. No width change (ADR-0043 §2 holds) |
| Finding `c` | JetBrains enables Accept and Finish at 0; KDiff3 enables Save; lazygit asks on its own at 0 (CV §3) | a status hint, then a plain notice row | **T** lazygit's landing, but with the preview instead of the bare confirm (**P11**): finishing the last file opens the summary. **T** JetBrains' labelled control for when he is back in the resolver: a finish row, with `c finish` first in the bar. **R** a status-line hint alone (field notes :60) |
| The summary reads as a preview | git's editor and JetBrains' Commit dialog: the last screen is the commit, with its own verb (CV §3). Graft's merge preview: `Preview: …`, `1 merge commit`, `undo: u takes this back` | an unframed screen, `nothing is written until ⏎` | **T** our own preview's frame and words (H4): a border, `Preview: finish merging feature into main`, `1 merge commit on main`, `undo: u takes this back`. The key names its verb: `y commit` |
| `⏎` vs `y` | VS Code: the word on the button matters (#162043), and Complete Merge could not be reached from the keyboard (#183244) (CV §3) | `⏎` commits from the file list | **T** Graft's own rule (**P12**): `y` confirms every preview (§2.3b, field notes :58), and `⏎` acts on what is under the cursor, here a file row, so it opens that file's diff (§2.2, ADR-0038 §1). In the subject field `⏎` still commits: he has just typed the message, so the key is anticipated (KLM Rule 3) |
| Bare yes/no | lazygit (CV §3) | — | **R**, unchanged (note :422) |

### Journeys (UX 1.3; `[M]` = a decision)
- **J13 resolve the last block → a finished merge.** Start: a merge begun in the app, the last file open, the cursor on
  its last open block.
  - As built: `o` [M: which side] · `⏎` · [M: what now? He stalled here] `c` · [M: read] `⏎`. **4** keys.
  - With P11 and P12: `o` [M] · `⏎`, and the summary opens on its own · [M: read] `y`. **3**. Taking the whole file:
    `O` [M] → summary · `y`. **2** (was `O c ⏎`, 3).
  - Best tools: JetBrains about 4 (take, Apply, Accept and Finish, Commit; CV §3, `(?)`). VS Code 3–4 (accept, Complete
    Merge, commit, maybe a message). lazygit 2 (a bare confirm, rejected).
  - Decisions: 3. The one removed is the one he stalled on.
- **J6 `a-finish`.** The summary is already open: `y`. **1** (was `c ⏎`, 2).
- **J6b `b-finish`.** A merge begun in the shell, the last file done in the app: `y`. **1**. From the banner (resolved in
  the shell, or after `u`): `c` [M] · `y`. **2**, unchanged.
- **J14 back to a file from the summary, then finish.** `Esc` → the resolver, finish row showing · `c` · `y`. **3**. Not
  in the block: it is rare, and `walk-back-a-done-file` already covers the correction.
- **Unchanged:** `editor-to-finish` `Esc ⇥ ⇥ c y` **5**; `summary-open-git-combined` `space` (or `⏎`) **1**.
- **Whole journeys:** `a-whole-one-file` `m m O y` **4** (was 5); `a-whole-two-files` `m m o T y` **5** (was 6);
  `b-whole-two-files` `⏎ o T y` **4** (was 5).
- **The journey-counts block:** P11 and P12 approved 2026-09-24. The `keys` column still holds the as-built counts;
  each spec lowers its own rows in the commit that makes the walk send the new keys, because the walk fails a journey
  over its row and an id with no scenario. `last-block-to-finished` stays commented until
  `last-file-opens-the-summary` adds its scenario and un-comments it at 3.

- **J15 abort a merge begun in the shell** (batch 3, approved 2026-09-24). The banner focused: `a` · `→ ⏎` **3**; `a y`
  **2** once ticket 50 lands. Lands: the graph, the abort's `Done` line, no `MERGING`; one `AbortMerge` journal entry,
  and `u` puts the merge back, conflicted, his hand edits included. Best: lazygit `m` → abort → confirm, 3 (?).
- **J16 get a file back after a whole-file take, in a merge begun in the shell** (batch 3). A whole-file `t` on
  `gone.txt` deleted what he had kept by hand; the banner focused (after `Esc` from the resolver): `u` **1** (from the
  resolver: `Esc u`, 2 — the resolver does not bind `u`, keymap §2.6). Lands: the banner stays focused, status
  `undo: <the take's description> ✓`; the file on disk is his again. Only Graft's journaled steps come back this way
  (whole-file choices, `d`); a block take is an fs write inside the merge, not an entry. JetBrains: `⌃Z` in the merge
  dialog, 1.
- **J17 walk back a whole-file choice** (batch 7). The resolver on `notes.txt` after `gone.txt` was kept: `k` · `d` ·
  `→ ⏎` **4**; `k d y` **3** once ticket 50 lands. Lands: the card `deleted by them · o keeps the file  t deletes it`,
  the list `keep or delete`, `git ls-files -u` shows the file's stages again. Tower reset-to-unmerged, count (?).

**Where each event lands him (changes to Amendment 2's list).**
- **The last file of a merge done in the resolver** (by `⏎`, `O`/`T`, or `o`/`t` on a whole-file card): once its
  `Stage` lands, the summary opens.
  - Focus on the file list, the cursor on the first row git combined, or on the first row if git combined nothing.
  - Status: `notes.txt done ✓ · every file done`.
  - It opens only after a take still landing (`settling`, `in_flight`) has settled.
- **A stash pop:** → the graph, unchanged.
- **A rebase, cherry-pick or revert:** the resolver stays, with the finish row `✓ every file done — c continue the
  rebase`. The bar starts `c continue`. There is no auto-open, because `c` writes.
- **The last `Stage` fails:** → the resolver, with its own words, unchanged. No summary.
- **The last file done outside the resolver** (`space` on a `CONFLICTS` row): → the banner, `every file done ✓ · c
  finishes the merge`, unchanged.
  - **The trigger is the event, never the state.** A relaunch, a `u` or an `Esc` does not reopen the summary.
- **In the summary:**
  - `y` → as `⏎` did before: `committing…`, then the graph, cursor on the merge commit, `merged feature into main ✓`.
  - `⏎` on a file row → its diff opens or closes, as `space` does.
  - `Esc` → the resolver with the finish row; the bar starts `c finish`, and the draft is kept.
  - An unreadable summary → `summary unavailable: <why> · y commits with git's message`.
  - `y` while loading → `still reading · wait`.

### Design
**Colour roles: six new tokens (P13).** Colour follows **position**: the left pane is git's stage 2 (`yours · main`, or
`upstream · main` in a rebase) and the right pane is stage 3. So a pane's hue never moves, and the role word says whose
side it is (ADR-0043 §4).

| token | used for | dark, truecolor | 256 | 16 | `NO_COLOR` |
|---|---|---|---|---|---|
| `ours` | left title, left rule words, left separator glyphs | `#56B4E9` | 74 | 6 cyan | no colour: glyphs |
| `theirs` | right title, right rule words, right separator glyphs | `#CC79A7` | 175 | 5 magenta | no colour: glyphs |
| `open` | `? open`, separators of open blocks, list `n of m open` / `keep or delete` | `#E69F00` | 214 | 3 yellow | BOLD |
| `ours-tint` | background of the left block rows, and of result rows taken from the left | `#284559` | 23 | none | none |
| `theirs-tint` | the same for the right | `#4B3445` | 53 | none | none |
| `open-tint` | background of the result placeholder rows | `#533F13` | 58 | none | none |

- **Tint = mix(fg, `bg`, α).** α = 0.30, his pick 2026-09-24 ("medium"), shared by all three tints (ADR-0044 §1).
  - Targets: tint against `bg` between 1.5:1 and 2.5:1, and `text` on a tint at least 4.5:1.
  - Why that range: a cursor fill at 1.21:1 was invisible to him (field notes :48), and `selection` at 3.04:1 was seen
    (:50). A tint stays below `selection`, so "you are here" stays the strongest cue.
  - At 0.30 on `#14161B`: 1.8, 1.6 and 1.8 (computed with the WCAG formula).
- **Light theme:** the same α over the light `bg`, with darker foregrounds (`#0072B2`, `#A8467F`, `#B36B00`).
  **Unverified**, since he runs dark. The high-contrast theme uses the 16-colour rule.
- **256 colours:** the fixed entries above, not the nearest-colour mapping. The nearest entry to a dark tint is on the
  grey ramp (232–255), and the hue is lost (P13).
- **16 colours:** no background tints. ANSI backgrounds are full-strength blocks, and text on them is hard to read.
  Colour goes only on titles, rule words and separator glyphs, and the side not taken is `text-muted`. Precedent:
  `Theme::syntax` and the side-by-side diff go plain at `Palette16` (graft-ui MAP).
- **`NO_COLOR`:** glyphs and modifiers only. DIM on the side not taken, BOLD on `? open`, and the reversed cursor rule
  as built.
- **Not used:** `added` / `removed`, because they are red / green (CV §1.3), and the lane hues, which mean a branch, not
  a side.

**Three panes (≥180 columns).** The separator between yours | result and result | theirs stays 1 cell (ADR-0043 §2). On
plain rows it is `│` in `border`, as built. On a block's rows it is the link:

| block | left separator | right separator | yours rows | result rows | theirs rows |
|---|---|---|---|---|---|
| open | `?` on the rule row, `┃` below, in `open` | the same | `ours-tint` | `open-tint`; `? open · o t b` in `open`, BOLD | `theirs-tint` |
| took yours | `▶` on the rule row, `┃` below, in `ours` on `ours-tint` | `┊` in `text-muted` | `ours-tint` | `ours-tint` | no tint, `text-muted` |
| took theirs | `┊` in `text-muted` | `◀` on the rule row, `┃` below, in `theirs` on `theirs-tint` | no tint, `text-muted` | `theirs-tint` | `theirs-tint` |
| took both | `▶` / `┃` in `ours` | `◀` / `┃` in `theirs` | `ours-tint` | the first `ours.len()` lines `ours-tint`, then `theirs-tint` (git's order, `Take::Both`) | `theirs-tint` |
| file done | `│` | `│` | one card, as built, with no tint | | |

- **Tints fill the pane's full width** on those rows, separator cell included (ediff `:extend`). That makes the band
  across two panes.
- **The cursor block:** as built (`▏`, rule row reversed), with its separator glyphs BOLD. Other blocks get the same
  colours, not bold.
- **Titles:** `yours · main` in `ours` BOLD, `theirs · feature` in `theirs` BOLD, `result` in `text` BOLD.
- **Rule words:**
  - `── o take` in `ours`, `── t take` in `theirs`.
  - `── taken ✓` in the taken side's hue, `── not taken` in `text-muted`.
  - In the result: `── 2 of 2 · open` in `open`, `── 1 of 2 · took theirs` in `theirs`.
- **Both, then a hand edit** (`Choices.edited`): the result rows get no tint, because the line origin is unknown.
- **ASCII tier:** `?`, `|`, `>`, `<` and `:` stand for `?`, `┃`, `▶`, `◀` and `┊`.
- **Data:** `BlockState`, `TakeView` and the block's `ours.len()`. No new git or ops read.

The mockup is not to scale; panes are about 51 cells wide, as built:
```
187x50
MERGING
FILES · 2 conflicted        │▸ notes.txt · 2 conflicts · 1 open
                            │
   gone.txt   done ✓        │ yours · main                   │ result                          │ theirs · feature
▏▸ notes.txt  1 of 2 open   │ 1 one                          │ 1 one                           │ 1 one
                            │ ── not taken                   ┊ ── 1 of 2 · took theirs         ◀ ── taken ✓
                            │ 2 two from main                ┊ 2 TWO from feature              ┃ 2 TWO from feature
                            │ 3 three                        │ 3 three                         │ 3 three
                            │▏── o take                      ? ── 2 of 2 · open                ? ── t take
                            │▏7 seven from main              ┃   ? open · o t b                ┃ 7 SEVEN from feature
                            │ 8 eight                        │ 8 eight                         │ 8 eight
 o yours  t theirs  b both  ^O/^T all  O/T file  ⏎ done  j/k file  n next  B base  e edit  esc close
```
- **Block 1 (taken):** one purple band runs from theirs' row 2 through `◀┃` into the result's row 2. Yours' row 2 and
  `not taken` are muted.
- **Block 2 (open):** left rows sky blue, right rows purple, the result placeholder amber, and both separators amber
  `?` / `┃`.
- **Where the eye goes:** first to the amber `?` pair on the cursor block (the open work), then to the purple band (the
  finished work), then to the list's `▸`.

**Card (below 180 columns) and file list.**
- **No glyph changes in the card.** Its words already carry the state (`conflict 2 of 2 · open`, `took theirs ·
  feature`).
- **Colour at 120 and 160** (sides paired): the open block's left half `ours-tint` and its right half `theirs-tint`.
  Labels are in their hue, BOLD. The header's `open` is in `open`. A taken block's lines are in the taken side's tint,
  and its header's `theirs · feature` is in `theirs`.
- **Colour at 80** (stacked): each side's rows in its own tint, and the labels in their hue.
- **File list:**
  - `1 of 2 open`, `keep or delete` and `⏎ marks done` in `open`; `done ✓` in `text-muted`.
  - Once every file is done, the column head reads `FILES · all 2 done ✓`, and the strip label reads `files · all done
    ✓`.

**The finish row** (in the resolver when every file is done and the summary is not open: after `Esc`, for a rebase,
cherry-pick or revert, or if P11 is refused).
- It takes the notice row, under the identity row, and replaces `every file done ✓ · c finishes the merge`.
- **Text:**
  - At 187 and 120: `✓ every file done — c review and finish the merge`.
  - For a rebase: `✓ every file done — c continue the rebase`.
  - At 80: `✓ every file done — c review & finish`.
- **Style:** `accent` in BOLD on a full-width `selection` fill, the strongest fill because it is the next action.
  `NO_COLOR`: REVERSED.
- **Mouse:** a click on the row does what `c` does.
- **Bar:** `c finish` moves first (Hick, UX 1.4): `c finish  d start over  e edit  esc close`.
- **Status after the last `⏎`,** wherever there is no summary: `notes.txt done ✓ · every file done · c finishes the
  merge`. The key is named the moment he lands, never `stage notes.txt ✓` alone.
```
187x50, after Esc from the summary
MERGING
FILES · all 2 done ✓        │▸ notes.txt · done ✓ · d starts it over
                            │✓ every file done — c review and finish the merge
   gone.txt   done ✓        │  1 one
▏▸ notes.txt  done ✓        │  2 TWO from feature
 c finish  d start over  e edit  esc close
```

**The finish summary is a preview.** It fills the frame under the header inside the merge preview's box (`frame_box`,
rounded, `accent` border because it has focus), and uses the same words. The old title row `Finish · merging … ·
nothing is written until ⏎` goes.
```
187x50 (two columns inside the frame from 160, as built)
MERGING
╭ Preview: finish merging feature into main ─────────────────────────────────────────────────────────────────────╮
│ 1 merge commit on main · nothing is committed until y                                                          │
│ undo: u takes this back                                                                                        │
│ RESOLVED BY YOU · 2                                                  MESSAGE · ⇥ edit                          │
│   gone.txt   kept · yours · main                                     Subject Merge branch 'feature' into ┊ 32/50│
│   notes.txt  theirs · feature in 2 of 2 conflicts                    Body                                      │
│ COMBINED BY GIT ON ITS OWN · 1 · ⏎ shows the change                                                            │
│ ▸ README.md  modified  +1 −0                                                                                   │
╰ y commit  ⏎ diff  j/k move  ⇥ message  esc back ──────────────────────────────────────────────────────────────╯

80x24 (and 120x40: one column, same order, the message under the list)
MERGING
╭ Preview: finish merging feature into main ───────────────────────────────────╮
│ 1 merge commit on main · nothing is committed until y                        │
│ undo: u takes this back                                                      │
│ RESOLVED BY YOU · 2                                                          │
│   gone.txt   kept · yours · main                                             │
│   notes.txt  theirs · feature in 2 of 2 conflicts                            │
│ COMBINED BY GIT ON ITS OWN · 1 · ⏎ shows the change                          │
│ ▸ README.md  modified  +1 −0                                                 │
│ MESSAGE · ⇥ edit                                                             │
│  Subject Merge branch 'feature' into main                ┊           32/50   │
│  Body                                                                        │
╰ y commit  ⏎ diff  j/k move  ⇥ message  esc back ─────────────────────────────╯
```
- **Title:** `Preview: finish merging <from> into <into>`. Where the frame is narrower than the title, it becomes
  `Preview: merge <from> → <into>` and is then cut with `…`.
- **Where the eye goes:** first to the title (`accent` BOLD), then to the cursor row, then to `y commit`, which leads
  the footer border with its key in `accent` BOLD.
- **Colour:**
  - The words `yours · main` in `ours` and `theirs · feature` in `theirs`, which ties the rows back to the panes.
  - `+n` in `added` and `−n` in `removed`, as in diffs.
  - Section heads in `text-muted` BOLD.
- **Bar by focus:**
  - Files: `y commit  ⏎ diff  j/k move  ⇥ message  esc back`. `space` still toggles the diff, and is drawn as
    `⏎/space diff` if `every_bound_action_is_hinted_live_or_is_named_here` needs it named (as `^O/^T all` is).
  - Subject: `⏎ commit  ⇥ body  ⌃E editor  esc back`, as built.
  - Body: `⌃⏎ commit  ⇥ files  ⌃E editor  esc back`, as built.
  - In the subject and the body, `y` is typed text.
- **`FINISH_MIN`** grows by the frame's 2 columns and 4 rows (2 border rows, 2 preview lines) to (62, 18). Below that,
  as built: `finish needs 62x18 · esc back`.
- **`hit()`** is offset by the frame (1, 1) and the 2 preview lines, so a click still lands on the right row.

**Hard-to-reverse, this amendment.**
1. **The token names are config names he may set.** Options: named by position (`ours` / `theirs` = stage 2 / 3), or by
   owner (`yours` / `theirs`, which swap in a rebase). **By position:** a pane's hue never moves, and they are git's own
   stage names.
2. **Opening the summary on the last file (P11).** Options: stay in the resolver (P8), or open the summary. **Open it.**
   The code is easy to reverse, but it trains his hands, so he decides on the prototype. **Decided 2026-09-24: open it**
   (his pick; task-mode log: the summary opened by itself and his first keys there were `k`, `⏎`, then `y`).

### Touches (each, and how a builder checks it)
- **Crates:** for P11–P13, `graft-ui` and `graft` only. `graft-git`, `graft-core` and `graft-term` get no signature
  change, and `FinishSummary` is unchanged. Check: `git diff --stat` names none of them. The batch items 3 and 7
  (approved 2026-09-24) touch `graft-ops`; their specs say how.
- **graft-ui `theme`:** `Token`, `Token::ALL` and `Token::name`, the dark, light and high-contrast values, and the
  256 / 16 / `NO_COLOR` reduction. Also the status screen (F2, `RenderingView`) and any config round-trip that lists
  tokens. Check: grep the users of `Token::ALL`; run the theme tests.
- **graft-ui `conflict_resolver`:**
  - What changes: `panes/draw.rs` (separator per state, tints, titles), `draw/block.rs` (card colours), `files` (state
    colours, the all-done head) and the ASCII glyph gate.
  - Check: the golden and row tests, plus new buffer style assertions (ADR-0041): for each state row in the table
    above, the cell's fg, bg and modifier.
- **graft-ui `finish_summary`:**
  - What changes: the frame, the title, the preview lines, `FINISH_MIN`, `layout` and `hit`.
  - Check: a `hit` round trip at 80, 120 and 187.
- **graft-ui hints:**
  - What changes: the order of `Resolver { finishable }` (`c` first) and the list for `Finish { focus: Files }`.
  - Check: `every_context` and its coverage slots, `every_key_the_bar_draws_is_answered`, and
    `every_bound_action_is_hinted_live_or_is_named_here`.
- **graft app:**
  - What changes:
    - `finish_key`: `y` commits in Files, and `⏎` toggles the diff.
    - The path where the last file's `Stage` lands calls `open_finish(…, ReturnTo::Resolver, …)` for a merge only. It
      must wait on `settling` and `in_flight`, leave a stash pop closing and a rebase with no auto-open, and open
      nothing after `u` or a relaunch.
    - A click on the finish row does what `c` does.
    - The status words after the last `⏎`.
  - Check: every string that says `⏎ commit…` (grep `crates/graft` and `crates/graft-ui` for `⏎ commit`, including
    `summary unavailable…` and `still reading`).
- **Other screens that draw the same state:**
  - The banner's `every file done ✓ · c finishes the merge`: unchanged. Check: grep the literal.
  - The changes pane's `CONFLICTS` rows: unchanged.
  - The commit editor's refusal `esc, ⇥ banner, then c` (tickets 39 and 40): unchanged, because `c` still opens the
    summary, whose bar then says `y`.
  - The Ghost Preview box: unchanged, and `y` means the same thing there.
  - The header `MERGING`: unchanged.
- **Journal and undo:**
  - Nothing new: no new entry or step, and the summary writes nothing. `b-finish` still makes exactly one
    `FinishMerge` entry, and `u` still lands on the banner (ticket 41).
  - Check: the existing tests, such as `c_enter_finishes_a_shell_begun_merge`, moved to `y`, plus one new test: after
    `u`, no summary opens.
- **Keymap `02-keymap.md`** (edited in the spec pass, 2026-09-24; ADR-0044):
  - §2.1 :65 (`c` to finish).
  - §2.3b, the `y` row: add the finish summary.
  - §2.6, the `c` and `Enter` rows (the P8 text becomes P11).
  - §2.6b: `Enter` becomes "diff", and a new `y` row. `Ctrl+Enter` is unchanged.
- **This note:** Amendment 2's summary mockup, J6, and its `⏎`-commits event lines are superseded where P11 and P12
  are approved. Marked 2026-09-24 (spec pass, E2-E3).
- **TESTING.md, the M6 walk:**
  - `one-file-one-conflict`, `three-conflicts-all-theirs`, `two-files-in-order` and `two-files-out-of-order` now land
    on the summary, not "the resolver stays".
  - `a-finish`, `b-finish`, `b-finish-again-after-undo`, the `a-whole-*` rows, `b-whole-two-files` and
    `editor-to-finish` change `⏎` to `y`, and some of their counts change.
  - The capture table rows m6-14, m6-15 and m6-17.
  - The colour paragraph at :2364–:2366: buffer tests now prove the style, and legibility stays his to judge.
  - The "Times I left" row at :108: `c ⏎` becomes `c y`.
- **Captures in `docs/shots/m6/`:**
  - The separator glyphs change on the block rows of every three-pane capture: m6-10 to m6-13 at 187 and 200,
    `fix-three-panes-*`, `fix-advance-187`, `fix-file-keys-200`, `fix-stop-lands-in-resolver-187`,
    `fix-file-list-187` / `-200` (the list head), and `fix-done-*-200`.
  - The finish captures: m6-14 to m6-17 at all five sizes, `fix-finish-summary-*`, `fix-finish-from-the-resolver`,
    `fix-finish-shell-merge` and `fix-every-file-done-footer`.
  - Then the contact sheet `docs/shots/m6/m6-contact-sheet-<n>.txt`, rebuilt with `scripts/contact-sheet.sh
    --captions-order m6 docs/shots/m6`.
  - Check: each capture's "What right looks like" line against the new text, and the stem list before and after, so no
    stale stem is left.
- **The walk:**
  - The key strings in `.scratch/m6-conflicts/pty-walk-m6.py`.
  - The Rust test that parses `journey-counts`. Before uncommenting `last-block-to-finished`, the builder checks
    whether an id without a scenario fails it.
- **KNOWN_GAPS:** the conductor re-words the "colour unverified" rows for the resolver and the summary: the style is now
  proven by tests, and legibility is his to judge.
- **PROMPT.md §5:** the token list and the colour-depth exception (P13); exact text in ADR-0044 §5, applied by the conductor.

### Prototype (pipeline step 5): he answers his open questions by trying them
He wants to answer by trying, not by reading. One throwaway lets him try every open question that is a choice between
options: this amendment's look and finish (P11, P12, P13) and his list of 11 (`.scratch/m6-conflicts/user-batch.md`),
items 2, 3, 4, 6, 7, 8, 9, 10 and 11. Items 1 and 5 are his verdicts after trying it.

- **Where:** `prototypes/m6-conflict-look/look.py`. Python 3, standard library only (`tty`, `termios`, ANSI escapes),
  like `prototypes/m6-conflict-journey/walk.py`. It is outside the workspace, needs no cargo, and reads no repository:
  every frame is canned text. It is deleted when the specs land.
- **Run, one command:** `python3 prototypes/m6-conflict-look/look.py`. It draws at the window's width; his is 187.
- **What its builder copies:**
  - Graft's dark `bg` `#14161B`, and `text`, `text-muted`, `border`, `accent` and `selection` from `graft-ui`'s dark
    theme. The six new tokens come from the Design table above.
  - Every **as-built** string, byte for byte, from the capture or file named in its scene, or from the running app.
  - A string not built yet carries the tag `draft` on the knob row, so he knows the words are a proposal.

**Controls, the same in every scene**
| key | does |
|---|---|
| `]` / `[` | next / previous scene. `0` opens "What you're deciding" |
| `1`–`4` | cycle the scene's knobs, one knob per digit |
| `<` / `>` | redraw the scene at 80 · 100 · 120 · 160 · 187 columns (drawn from the left, the rest blank) |
| `*` | mark the option now on screen as his pick for this scene |
| `Backspace` | restart the scene |
| `q` | quit; prints the picks and writes them to `.scratch/m6-conflicts/look-picks.txt` |
| any other key | the app's own key, acted out on the canned frames (Wizard of Oz, UX §4) |

- **Row 1:** `scene n/10 · <the question> · keys k`. `k` counts his keys since the scene started (UX 1.3), so a
  journey can be counted as he walks it.
- **Row 2:** the knobs, `1 <name>: <value>`, with the current value reversed and tagged `as built` or `draft`.
- **Below:** the app's frame, exactly as it would draw.

**Task first (UX 4.1).** It opens in task mode: rows 1–2 are hidden, and scene 1 runs straight into scene 2 with the
recommended settings (the band, the Okabe pair, 30 %, the summary opens itself, `y` commits, the preview frame). The
conductor says only: *"the merge stopped on notes.txt — take theirs on what's left and get it committed."* The script
logs every key and every pause over 3 seconds to the picks file. The two that matter are his first key after the last
file is done, and his first key on the summary (`⏎` or `y`). When the commit lands, or on `]`, the rows appear and the
scenes follow in order.

**Scenes.** Each gives what he presses, what he sees, and the question it answers.

1. **Colours and lines between the panes** (field notes :61; P13).
   - Frame: `notes.txt` at the stop, both blocks open, the three-pane mockup above.
   - He presses `o`, `t`, `b`, `n`, `N`. Each take redraws the band live, and `⏎` once every block is taken goes to
     scene 2. The bottom row reads, for example, `tint 30% · 1.8:1 · text 7.0:1 · truecolor · lines A · pair okabe`.
   - Knobs:
     - `1` tint: 20 / 30 / 40 %.
     - `2` depth: truecolor / 256 fixed entries / 256 nearest / 16 / no colour. The 256 escapes draw the real palette
       entries, so he sees whether the hue survives.
     - `3` lines: glyphs and band / VS Code's bracket `┐│┘` beside each side's block / colour only, `│` unchanged.
     - `4` pair: Okabe `#56B4E9` / `#CC79A7`; teal–magenta `#2AA198` / `#D33682`; VS Code `#40C8AE` / `#40A6FF`.
   - Answers: without reading, which blocks are open and which taken? Which side did the middle's line 2 come from?
     Which tint shows without leaning in, and is any too loud? Which lines, which pair? At 256, fixed or nearest?
2. **Finishing the merge** (item 11; P11; P12; the summary's look).
   - Frame: `gone.txt` done, `notes.txt` open on its last open block, the cursor on it.
   - He presses `o ⏎`, then whatever he would press next. On the summary: `y` or `⏎` as the knob says, then a canned
     graph row `merged feature into main ✓`. `Esc` goes back to the resolver with the finish row. `⏎` on a file row
     opens a canned diff when `y` commits.
   - Knobs:
     - `1` after the last file:
       - as built: the bar reads `stage notes.txt ✓` until the next key, with the notice row `every file done ✓ · c
         finishes the merge` (`fix-finish-from-the-resolver-120x40.txt:5`, `:40`);
       - `c finish` at once: the finish row, and `c finish` first in the bar;
       - the summary opens by itself.
     - `2` commit key: `⏎` commits (as built) / `y` commits and `⏎` shows the file's change.
     - `3` summary look: as built, no frame, `nothing is written until ⏎` (`m6-15-finish-summary-187x50.txt`) / inside
       the preview frame.
   - Answers: after the last file, did you know what to press? Did the last screen read as "not committed yet"? Which
     key did your hand reach for?
3. **Keys on the "what will land" screen** (item 8).
   - Frame: the summary in scene 2's chosen look, with 30 canned files git combined, so it scrolls.
   - He presses `↑`, `↓`, `PgUp`, `PgDn`, `⇧⇥`, and the listed `j`, `k`, `⇥`. A key that is off flashes `does nothing
     here` on row 2, as the app does today (nothing).
   - Knobs: `1` `↑`/`↓` · `2` `PgUp`/`PgDn` · `3` `⇧⇥` back. Each is off (as built) or on.
   - Answers: which did your hand reach for, and should they work?
4. **The conflict screen's key row at 120 columns** (item 9).
   - Frame: the resolver at 120, a merge with `B base` and `d start over` offered.
   - He looks at the bottom row; `<` / `>` show it at 100 and 160.
   - Knobs:
     - `1`:
       - C, as built: `… ⏎ done  j/k file  M mergetool  d  esc` (`fix-stop-lands-in-resolver-120x40.txt`);
       - A, drop words from everyday keys first (`j/k`, `M` go bare), `d start over  esc close` kept whole (draft);
       - B, shorter words, `M tool`, `d restart`, everything whole (draft).
     - `2`: merge / rebase (137 columns wanted instead of 135, `user-facts-m632.md:3`).
   - Answers: at 120, does the row still tell you what `d` does and how to get out?
5. **Short key words on narrow screens** (item 4).
   - Frame: the changes pane's key row, its longest context. Here `<` / `>` step the width 10 columns at a time,
     from 187 down to 60.
   - Beside it, the 22 pairs of the 2026-09-21 draft
     (`.scratch/m5-history-and-ghost-preview/issues/24-the-ladder-and-the-stale-banner.md:24-36`). `j` / `k` move
     through them, `x` strikes a pair, and `w` retypes its short word. The bar redraws live.
   - Knob `1`: as built, the 9 short words kept (copied from `graft-ui`'s `short_form` table) / the whole draft /
     no short words (words drop whole).
   - Answers: which short words read without thinking? Which would you change or strike? Struck and retyped pairs go
     to the picks file.
6. **The stop line with the changes pane open** (item 6).
   - Frame: a merge begun in the shell, 13 files conflicted, the changes pane open, at 187. `<` shows 120.
   - Knobs:
     - `1` origin: `begun outside Graft` / `from a previous run`.
     - `2`: as built, the origin dropped so the count fits / keep the origin, and cut the count with `…` first
       (draft) / shorten both, `from the shell · 13 conflicted`, `from last run · 13 conflicted` (draft).
   - Answers: on that line, do you need to know where the merge began?
7. **What `n` says after the last block** (item 2).
   - Frame: the case at `docs/log/m6.md:78`: he walked back with `N`, a block behind him in `zdiff3.txt` is still
     open, and he presses `n` past the last one. The builder reproduces it on the running app first. If it no longer
     happens (the 2026-09-24 order fix, P2 above), the scene says so on row 2 and the item closes.
   - Knob `1`:
     - as built: `no more conflicts · ⏎ marks zdiff3.txt done`, also in the key row, so it shows twice;
     - `no more conflicts this way`;
     - `last block · 1 still open above · N goes back` (draft);
     - his own: `w`, type it, `⏎`, and it shows at once.
   - Answers: after `n`, did you think the file was finished?
8. **Starting over a file one side deleted or added whole** (item 7; P10 in Amendment 3).
   - Frame: `gone.txt` done (kept yours), opened read-only, in a merge begun in the app.
   - He presses `d`, then `→ ⏎` if a box opens.
   - Knobs:
     - `1`:
       - as built: no `d` in the bar; `d` answers `can't start over · gone.txt: deleted by them — no markers to take
         from`;
       - offered: bar `d start over  e edit  esc close`; `d` opens a box with the words `the choice comes back: keep or
         delete`, cursor on Cancel; `→ ⏎` → the card `deleted by them · o keeps the file  t deletes it` (draft).
     - `2`: begun in the app / begun in the shell (refused either way; it shows what he would get there).
   - Answers: would you walk back a keep-or-delete choice this way?
9. **Undo and abort on a merge begun in the shell** (item 3).
   - Frame: a `git pull` begun in the shell, stopped. In Graft, `t` on `notes.txt` overwrote his hand edit. The banner
     has focus.
   - He presses `u`, then `a`, then `y` if a preview opens.
   - Knob `1`:
     - keep the shell step (as built): both refuse, with the running app's words;
     - abort only: `u` refuses; `a` opens a preview `Preview: abort the merge · main goes back to before the pull`
       (draft), and `y` → the graph, `merge aborted ✓`;
     - both: `u` → `undo: take theirs on notes.txt · your edit is back ✓` (draft), then `a` as in "abort only".
   - Answers: before the merge is finished, which of these do you expect to work?
10. **The mark on a file you haven't opened** (item 10).
    - Frame: 9 files conflicted, 2 opened. The strip at 120 ends with `… 7 more`; `>` shows the column at 187.
    - Knob `1`: `…` (as built) / `○` / `new` / no mark.
    - Answers: does any name look cut off?

**After the scenes**, the conductor asks his two verdicts, one line each: does the conflict screen look right (item 1:
the whole screen, the note row on a whole-file card, the `┴` on a taken block's closing line)? Go on to the next
milestone (item 5)?

**Record.**
- The picks file and the task-mode log go to `docs/field-notes.md` the same day, one line per scene in his words.
- The colour decision (tokens, values, lines) becomes an ADR. His answers settle P11, P12 and P13 and items 2–11, and
  the specs follow them.
- Nothing goes to `docs/shots/`: the colour can't be captured (rule 5), and a prototype is not the product.

#### What you're deciding (the page `0` shows; his terms)
1. **Colours on the conflict screen:** which blue and purple, how strong, and what the line between the panes looks
   like.
2. **When the last file is done:** stay where you are, show `c finish` straight away, or go straight to the screen
   showing what will land.
3. **On that screen:** `y` commits and `⏎` shows a file's change, or `⏎` commits as now.
4. **How that screen looks:** framed like the merge preview, or as now.
5. **Arrow keys, Page Up / Page Down and Shift+Tab on that screen:** all, some, or none.
6. **The key row at 120 columns:** drop words from keys you use every day, use shorter words, or leave it.
7. **Short key words on narrow screens:** keep the draft, change some, or drop the idea.
8. **The line after a stop, with the changes pane open:** drop "begun outside Graft", keep it, or shorten both.
9. **What `n` says after the last conflict** in a file.
10. **Starting over a file where one side deleted it or added it whole:** yes or no.
11. **Undo and abort on a merge you started in the shell:** both, abort only, or keep doing it in the shell.
12. **The mark on a file you haven't opened yet.**

Then two answers: does the conflict screen look right? Go on to the next milestone?

### Proposed changes (P11–P13 approved 2026-09-24: "ok i guess", field notes :65; ADR-0044)
- **P11 — approved 2026-09-24: finishing the last file of a merge in the resolver opens the summary on its own.**
  - What it reverses: P8's "the resolver stays open after the last file is done", for merges only. P8 still holds for
    a rebase, cherry-pick or revert, which have no summary, and after `Esc`.
  - Why: the next action is finishing (UX 3.1); lazygit asks on its own at 0 (CV §3); he did not find `c` (field notes
    :60). It saves one key on every merge (J13: 4 → 3).
  - Trade-off: going back to a done file before committing costs an `Esc` first. The summary writes nothing, so nothing
    is lost.
- **P12 — approved 2026-09-24: the summary confirms with `y`, and `⏎` on a file row shows its diff.**
  - What it changes: his approved `⏎ commits` (field notes :54) and keymap §2.6b.
  - Why: `y` confirms every other preview (§2.3b, field notes :58), and `⏎` acts on what is under the cursor (§2.2,
    ADR-0038 §1), which here is a file row, not the commit. His surprise came from that clash (field notes :60).
  - No count changes. In the subject field `⏎` still commits.
  - Trade-off: the confirm key depends on focus: `y` in the list, `⏎` in the subject, `⌃⏎` anywhere. Each focus's bar
    shows its own.
- **P13 — approved 2026-09-24: six colour tokens added to PROMPT.md §5** (exact text: ADR-0044 §5).
  - The tokens: `ours`, `theirs`, `open`, `ours-tint`, `theirs-tint`, `open-tint`.
  - The tints get fixed 256-colour entries instead of the nearest-colour mapping in PROMPT.md §5 "Colour depth" and
    design 04 §4.6. Why: the nearest entry to a dark tint is on the grey ramp, and the hue is lost.
  - Trade-off: six more values to keep in every theme. The high-contrast theme drops the tints.
- **ADR-0041** puts "colour schemes" in M8. This is not that: whether a block is open or taken, and which side a line
  came from, is state, which falls under its carve-out. The M8 pass may retune these values, but the roles stay.
- **P14–P17:** see `docs/design-notes/m6-usability-fixes.md` (usability walk M1–M4).

### Genuinely risky tickets (for the spec pass)
- **No Fable build expected.** Every rule is fixed here, and the prototype picks the values.
- **The band** is a pure draw, judged by buffer tests.
- **P11's trigger** (the last `Stage` has landed, the operation is a merge, and the resolver is open) is one decision
  point in `graft`, judged by the walk. Its spec must name the `settling` / `in_flight` interplay, so that a take still
  landing cannot open the summary early.
- **Acceptance** stays on Fable.

```
### Specs, 2026-09-24 (unnumbered, `.scratch/m6-conflicts/issues/`)
Wave 1, two lanes in parallel (disjoint crates, ADR-0036):
- **Lane U — `graft-ui` only, in order:** `conflict-colour-tokens` → `conflict-band-and-state-colours` →
  `hint-words-keep-the-way-out`. One builder can take all three in one worktree. If a scout finds a `graft` source test
  (not a capture) that pins what a U spec changes, that spec reports `blocked: needs graft` and runs after lane O.
  Both lanes regenerate files under `docs/shots/m6/`; after the second lane merges, the conductor re-runs the capture
  tests once and regenerates any file both touched.
- **Lane O — `graft-ops` + `graft`, in order:** `undo-and-abort-a-shell-begun-merge` →
  `start-over-a-whole-file-choice`.
Wave 2, one at a time (`graft` + `graft-ui`): `finish-summary-is-a-preview` → `last-file-opens-the-summary` → 52
(P14) → 50 (P15) → 51 (P16) → `resolver-words-and-marks`.
Blocked: `short-words-from-the-draft` (batch 4 against the m5/27 prefixes-only ruling; a question).
Last: `acceptance-look-and-finish` (Fable).
Genuinely risky: `undo-and-abort-a-shell-begun-merge` (a git-write safety path: the undo gate while a merge is live
decides what `u` may rewrite under it; correctness is not visible in a diff). `last-file-opens-the-summary`'s trigger
(one decision point; the walk judges it).
