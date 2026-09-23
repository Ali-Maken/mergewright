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
| Line numbers | each pane its own file (JetBrains/KDiff3, **(?)**) | OURS only | **I**: every line that is on disk carries its disk number, both sides — what `e` and his editor show (H2) |
| Number on the marker | nobody numbers a marker | `2` on the header | **I**: rows that stand for markers carry no number |
| Finish | lazygit's "Continue the merge?" (1 key) | `c` (A); refused when begun in the shell (TESTING.md:108) | Keep A. **P4**: `c` finishes a shell-begun merge |
| Landing after finish | UX 3.1: cursor on the new commit | unspecified | **T**: cursor on the merge commit, `merged … ✓` |
| Three columns / AI | VS Code went back to inline (#160806) | document (ADR-0042) | **R**, unchanged |

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

```journey-counts
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
pub struct ResolverState { …, pub queued: VecDeque<(usize, Take)> }   // takes pressed while one is in flight, in order, cap 64

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
   ADR-0042 reason 1 refused.
3. **(P4) What `u` after finishing a shell-begun merge restores.** Options: the resolved result as uncommitted changes,
   not merging · the exact mid-merge state, `MERGE_HEAD` included. **Exact.** Every other `u` puts back what was; losing
   the second parent turns the next `c` into a plain commit, silently. Cost: a new inverse step writing git's own files.

### Where I'd differ (Proposed changes — not decided; fix specs marked *gated* wait for them)
- **P1 — preview: the verb key again confirms** (`m m`, `r r`), only once the plan has landed. ADR-0038 §1 is kept
  (cursor on Cancel everywhere, `⏎` = the choice under the cursor). Keymap: Ghost Preview gains `m`/`r` for `.`-less
  opens only (`.` still closes what `.` opened). Trade-off: a double tap merges after one glance — the same glance
  `→ ⏎` needed.
- **P2 — takes advance; `n`/`N` walk every block**, taken ones included (§2.6 row "next/previous *open* block").
  Past the last block: first open block if any (`wrapped · 2 still open`), else today's `no more conflicts` notice.
  TESTING.md scenario 1's `n t n b` becomes `t b`. Trade-off: `n` stops on taken blocks; the title's count says what
  is left.
- **P3 — `Ctrl+O` / `Ctrl+T`: ours / theirs in every open block of this file**, one write, then `⏎` as usual. No
  bulk-both (`Ctrl+B` is tmux's prefix). **(?)** Zed's terminal may claim `Ctrl+T` — check before approving.
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
