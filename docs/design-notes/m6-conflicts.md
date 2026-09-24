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
  new merge commit, `merged feature into main ✓`. Begun in the shell: in-spec refused with `merge begun in the shell —
  finish it there: git commit` (Times I left stays open); P4 **1**. CR: lazygit 1.
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
pub struct ResolverState { …, pub in_flight: Option<(usize, Take)>, pub queued: VecDeque<(usize, Take)> }   // in graft/src/update/resolver.rs; in_flight = the take whose TakeSide is out; queued = takes pressed while one is in flight, in order, cap 64, excluding the one in flight

// graft-ui/src/preview_box.rs                                                   fix-preview-verb-confirms (P1)
pub struct PreviewBox { …, pub again: Option<(SanitizedText, SanitizedText)> }   // ("m", "merge"), drawn first in the footer border

// graft-git conflict.rs + graft-ops conflict.rs                                  fix-take-every-open-block (P3)
impl ConflictFile { pub fn take_all(&mut self, take: Take) -> usize; }   // every Conflict region; how many
pub fn apply_take_all(repo: &RepoHandle, path: &Path, expected: Oid, take: Take) -> Result<ConflictFile, TakeError>;

// graft-core journal + graft-ops                                                fix-continue-a-merge-begun-in-the-shell (P4)
pub enum OperationKind { …, FinishMerge }
pub enum Operation { …, FinishMerge(FinishMergeParams) }   pub struct FinishMergeParams { pub merge_head: Oid }
pub struct MergeState { pub merge_head: Oid, pub merge_msg: Vec<u8>, pub merge_mode: Option<Vec<u8>> }  // recorded in the entry's before-state
// restore: InversePlan step `RestoreMergeState(MergeState)` — writes MERGE_HEAD/MERGE_MSG/MERGE_MODE back
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
  R4/R5). After `fix-finish-summary-screen` Part B (P8) it stays in the resolver instead, the words on its notice row; a
  stash pop still closes to the graph, status line `every file done ✓`.
- **J5 one file, 3 conflicts, theirs everywhere.** `Ctrl+T ⏎` **2** (the P2 route, `t t t ⏎`, is 4); P6 `T` **1**.
  JetBrains Accept Theirs 1.
- **J5b modify/delete, keep yours.** `o` on the whole-file card (a git write, marks it done). **1**. JetBrains 1.
- **J8 two files in order.** gone `o` · notes `Ctrl+T ⏎`. **3**; P6 `O T` **2**. JetBrains 2.
- **J9 two files, notes first.** Resolver on gone: `n` (the card crosses to the next file) · `Ctrl+T ⏎` (lands on
  gone, the next file not done) · `o`. **4**; P6 `n T O` **3** (P7's `j T O` is also 3).
- **J6 finish, merge begun in the app.** From `every file done`: `c` → summary, cursor on the first row git combined ·
  `⏎` → commit. **2**. lazygit 1: the extra key is his choice. Lands: graph focused, cursor on the merge commit,
  `merged feature into main ✓`.
- **J6b finish, merge begun in the shell (P4).** `c ⏎`. **2**. Same landing, and one `FinishMerge` entry.
- **J10 see a change git combined on its own.** In the summary: `space` expands that file's diff under its row. **1**.
- **J7 back after `Esc`.** Resolver `Esc` → banner, `Esc` → graph with `⇥ banner` leading the bar: `⇥ ⏎`. **2**.
- **Whole journeys.** A, one file, one conflict: `m m o ⏎ c ⏎` **6** (P6: 5). A, two files: `m m o ^T ⏎ c ⏎` **7**
  (P6 `m m O T c ⏎`: **6**; v1 in-spec was 11). B, two files: `⏎ o ^T ⏎ c ⏎` **6** (P6: 5).

**Where each event lands him (additions).**
- ~~A take → the cursor moves to the next open block (P2).~~ A take → the cursor moves to the next open block **at the
  keypress** (P2); takes typed faster than the disk queue, and a failed one drops the queue with a count.
- A file done (`⏎`, `O`/`T`, or `o`/`t` on a whole-file card) → the next file not done.
- The last file done → the banner, status line `every file done ✓ · c finishes the merge` (rebase / cherry-pick /
  revert: `c continues the …`); stash pop → the graph, `every file done ✓` (ticket 13 R4/R5). ~~P8: the resolver, with
  the notice~~ superseded 2026-09-24: that is ticket 19 Part B's, words on the resolver's notice row; stash pop still closes.
- `c` while files are still open → refused with `1 file still conflicted · ⏎ marks done`.
- `c` on a rebase, cherry-pick or revert → continues with no summary (merge only this milestone; gap row → M7), and the
  next stop lands in the resolver.
- In the summary:
  - `Esc` → back to where `c` was pressed, with the draft message kept.
  - `⏎` → `committing…`, then the graph with the cursor on the merge commit.
  - A hook refuses → the summary stays, the reason goes above its bar, and the draft is kept.
  - The summary could not be read → `summary unavailable: <why> · ⏎ commits with git's message`.

```journey-counts
# id                          keys  once-approved   # parts (whole rows) · best tool
a-merge-to-stop               2
a-stop-to-first-conflict      0                     # JetBrains 0
b-launch-to-first-conflict    1
one-file-one-conflict         2     P6:1            # JetBrains 2
three-conflicts-all-theirs    2     P6:1            # JetBrains Accept Theirs 1
modify-delete-keep-yours      1                     # JetBrains 1
two-files-in-order            3     P6:2            # JetBrains 2
two-files-out-of-order        4     P6:3
a-finish                      2                     # lazygit 1 (bare confirm, rejected)
b-finish                      2
summary-open-git-combined     1
back-after-esc                2
open-a-done-file              1                     # Fork #1304 asks for it (CR 1.10); no count in CR
walk-back-a-done-file         4                     # Tower reset-to-unmerged (CR 1.11): count (?)
a-whole-one-file              6     P6:5            # a-merge-to-stop + a-stop-to-first-conflict + one-file-one-conflict + a-finish
a-whole-two-files             7     P6:6            # a-merge-to-stop + a-stop-to-first-conflict + two-files-in-order + a-finish
b-whole-two-files             6     P6:5            # b-launch-to-first-conflict + two-files-in-order + b-finish
```
The walk parses **this** block, the only fence tagged `journey-counts`. Text after `#` is a comment. When P6 is
approved, the conductor moves the `P6:` numbers into `keys`.

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
pub struct ResolverState { …, pub done: Option<DoneShape> /* ticket 25 */, pub files: Loading<Arc<ConflictSet>>, pub set_job: Option<JobId>, pub seen: BTreeMap<PathBuf, RowState> }
// was `pub file_cursor: usize` — superseded 2026-09-24 (ticket 13 R2): a function of the open path, cannot drift
// graft/src/update/resolver/files.rs (new)                              fix-resolver-file-list
pub enum RowState { Open { open: usize, of: usize }, AllTaken, Done, WholeFile { how: UnmergedHow }, Binary, Unread }
pub struct ListedFile { pub path: PathBuf, pub state: RowState }
pub fn listed_files(state: &AppState) -> Vec<ListedFile>;   // sorted once here, git's path order (byte-wise), over set ∪ extra unmerged ∪ open path; a done file keeps its row (was: ops' order, done files after open — superseded 2026-09-24, ticket 25 D1)
pub fn file_cursor(state: &AppState) -> usize;   // index of ResolverState::path in listed_files; 0 when absent
pub fn select_file(state: &mut AppState, commands: &mut Vec<Command>, index: usize, next_id: /* resolver's type */) -> bool;  // click; `j`/`k` if P7; a Done row opens read-only (ticket 25; 13's refusal dropped)
#[derive(Clone, Debug, PartialEq, Eq)] pub enum DoneShape { Text { unstaged: bool }, Binary { unstaged: bool }, Removed, Unreadable { why: String } }   // wrapped in DoneOpen { shape, start_over } (ticket 28)
// the Product answering Command::OpenConflict carries Result<graft_ops::conflict::Opening, …> (was Opened)
// graft-ops/src/conflict/done.rs (new; re-exported from `conflict`)     fix-stable-file-order-and-done-file-open
pub enum DoneFile { Text { file: graft_git::conflict::ConflictFile, unstaged: bool }, Binary { unstaged: bool }, Removed }
pub enum Opening { Unmerged(Arc<OpenedFile>), Done(Arc<DoneFile>, StartOver) }   // graft jobs::conflict (built: 25; StartOver: 28)
pub fn open_done(repo: &RepoHandle, status: &WorkingTreeStatus, path: &Path) -> Result<DoneFile, OpenError>;
pub fn open_listed(repo: &RepoHandle, path: &Path) -> Result<Opening, OpenError>;   // reads the status itself; open_conflict unchanged
// OpenError gains Status(<graft-git status reader's error>)
// graft-ui hints: HintContext::Resolver { base, restart, whole_file, done: bool, roles }
pub struct Choices { pub takes: Vec<Option<Take>>, pub whole: Option<Side>, pub edited: bool }
pub struct AppState { …, pub choices: BTreeMap<PathBuf, Choices>, pub finish: Option<FinishState>, pub finish_draft: Option<(String, String)> }
pub struct FinishState { pub summary: Loading<Arc<FinishSummary>>, pub job: Option<JobId>, pub cursor: usize, pub expanded: Option<(usize, Loading<Arc<DiffView>>)>,
    pub subject: TextField, pub body: TextArea, pub focus: FinishFocus, pub return_to: ReturnTo, pub committing: Option<JobId> }
pub enum FinishFocus { Files, Subject, Body }   pub enum ReturnTo { Banner, Resolver }
pub fn open_finish(state: &mut AppState, commands: &mut Vec<Command>, from: ReturnTo, next_id: /* same */) -> bool;  // `c` on a merge, nothing unmerged
pub fn finish_key(state: &mut AppState, commands: &mut Vec<Command>, key: KeyPress, next_id: /* same */) -> bool;    // modal; before resolver_key
pub enum GraphOverlay { …, Finish }   pub enum Command { …, ReadConflictSet(JobId), ReadFinishSummary(JobId) }
pub enum Product { …, ConflictSet(Result<Arc<ConflictSet>, String>), FinishSummary(Result<Arc<FinishSummary>, String>) }
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
   `git commit --cleanup=strip -F`. **`commit -F`**: git's own state files are written only by git, and P4's inverse
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
