# git-conflict-model: the conflict file as data — parse git's markers byte-exactly, take a side per block, write back without clobbering, read the three stages, reproduce git's diff3 blocks for base, name the unmerged shape, and the stopped-rebase corpus shape

Status: ready
Blocked by: **none** — starts immediately.
Build with: Fable (signature) — `ConflictFile` is the spine every M6 group and M7's rerere/interactive-rebase stops hang on; the parse rules (what is a marker, what is a label, what a `=======` inside a block means, how CRLF and a missing final newline survive) are decided here and are not readable from a diff; the builder chooses the rules, the fixture shapes and the fuzz target. Meets all three tests (design note "Genuinely risky tickets").
Crates: graft-git, graft-fixtures
Needs: none

## Read first

- `docs/design-notes/m6-conflicts.md` — Decision 1, Interfaces (the `graft-git` block), Hard-to-reverse choices 2 and 4.
- PROMPT.md §4 Conflicts (lines 334–341); §7.1 (laptop-safe tests, fuzz in the heavy tier).
- `crates/graft-git/MAP.md` whole (`status`, `state`, `history_reads::first_marker_line`, `RepoHandle`, `run`/`RunError`, the `--literal-pathspecs` convention); `crates/graft-fixtures/MAP.md` §git (`run`, `run_at`, `run_isolated`), §Shape, §Scratch.
- `KNOWN_GAPS.md` rows 121 (conflicting-rebase corpus shape → this group), 472 (`FileStatusView::Unmerged { how }` has no XY pair — this group supplies it), 526/527 (`diff --cc` refused — **out**, stays M7).
- `docs/rules-rationale.md` "Never swallow an error" — every `RunError` and `io::Error` surfaces typed.

## Scope

In:

1. **`graft_git::conflict`** (new module): `Line`, `Block`, `Region`, `Take`, `ConflictFile`, `parse`, `render`, `take`, `blocks`, `open`, `read`, `write_back`, `stage_bytes`, `diff3_blocks`, `attach_base`, the error types — signatures below. Byte-exact: `render(parse(b)) == b` for every input `parse` accepts, including CRLF, mixed endings, no final newline, a trailing partial line inside a block. Markers: exactly seven marker chars at column 0 followed by a space and a label (`<<<<<<< `, `||||||| `, `>>>>>>> `) or `=======` alone (the label-less form git writes), terminator excluded from the label; `conflict-marker-size` other than 7 is **out** (the file parses as `StrayMarker`/`Unterminated` or as plain — builder decides and tests which, and says so in NOTES). zdiff3 parses as diff3. A second `<<<<<<<` inside an open block is `StrayMarker`. A bare `=======` **after** the separator and before `>>>>>>>` is theirs' content (git writes exactly one separator per block and never escapes content; the first one wins) — builder confirms this against `git merge-file` output for `md.md` and records the ruling. `take(Both)` = ours then theirs (git's `smerge` order). `fingerprint` = git's blob id of the bytes (`sha1("blob <len>\0" + bytes)`, computed locally, no git process) so `write_back`'s `expected` check and git's own index compare the same thing. `write_back`: temp file in the same directory, same mode bits, `rename` over the path; refuses `Changed { now }` when the file's current blob id differs from `expected`. `read`: `Binary` when a NUL is in the first 8 KiB (git's own rule); `TooLarge` above 8 MiB.
2. **Stages and base**: `stage_bytes(repo, path, N)` = `git show :N:<path>` (`Ok(None)` when the stage is missing — a whole-file shape); `diff3_blocks` writes the three stages to a scratch dir (under `.git/graft/` or a `tempfile` dir — builder decides, says why), runs `git merge-file -p --diff3 -L ours -L base -L theirs`, parses the output with `parse`, returns its `Block`s; `attach_base` walks `file.regions` and `diff3` in order and attaches a base to a `Conflict` block only when its `(ours, theirs)` lines are byte-equal to the reproduced block's; any mismatch → `Err(BaseMismatch { block })`, nothing attached. Never a guess.
3. **`UnmergedHow`** on `StatusEntry`: porcelain v2 `u` lines carry the XY pair; map `UU/AA/DD/AU/UA/DU/UD` to the seven variants; `words()` gives git's own phrases (`both modified`, `both added`, `both deleted`, `added by us`, `added by them`, `deleted by us`, `deleted by them`). Existing `unmerged: bool` stays true in exactly those cases.
4. **Fixture `Shape::StoppedRebase`**: a repo left at a rebase conflict (`.git/rebase-merge/` present, stages 1–3 in the index) with, in one stop: `both.txt` (UU, one block), `three.txt` (UU, three blocks, the middle one with a base that differs from both sides), `crlf.txt` (UU, CRLF line endings), `nofinal.txt` (UU, no final newline on one side), `md.md` (UU, a `=======` line inside theirs' side as content — Markdown setext underline), `zdiff3.txt` (UU, produced with `merge.conflictStyle=zdiff3`), `pic.bin` (UU, binary), `theirs-deleted.txt` (UD), `ours-deleted.txt` (DU), `both-added.txt` (AA), `both-deleted.txt` (DD — needs a rename on one side; builder decides the recipe). Also `Shape::StoppedMerge` (same files, via `git merge`, `MERGE_HEAD` present) so both `in_progress` kinds have a corpus. Recipes through `graft_fixtures::git::run` only. Document each file's purpose in `crates/graft-fixtures/MAP.md`.
5. **Fuzz target** `fuzz/fuzz_targets/conflict_parse.rs` (heavy tier, not in nextest): `parse` never panics; `render(parse(b)) == b` when `Ok`.
6. MAP.md entries for both crates; `KNOWN_GAPS.md` untouched (the conductor moves rows 121/472).

Out: any UI; any `Operation` (G2 `ops-conflict-operations`); staging (`add`) — that is `Operation::Stage`; `diff --cc` parsing (M7); `conflict-marker-size` ≠ 7 (record as a gap in NOTES); rerere.

## Signatures

```rust
// graft-git/src/conflict.rs
pub struct Line(pub Vec<u8>);
pub struct Block { pub ours_label: Vec<u8>, pub ours: Vec<Line>, pub base: Option<(Vec<u8>, Vec<Line>)>, pub theirs_label: Vec<u8>, pub theirs: Vec<Line> }
pub enum Region { Plain(Vec<Line>), Conflict(Block), Resolved { block: Block, take: Take } }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Take { Ours, Theirs, Both }
pub struct ConflictFile { pub regions: Vec<Region>, pub fingerprint: Oid }
pub struct BlockAt { pub index: usize, pub state: BlockState, pub first_line: usize }   // first_line: 1-based line in render()
pub enum BlockState { Open, Took(Take) }
#[derive(Debug, PartialEq, Eq)] pub struct ParseError { pub line: usize, pub what: ParseErrorKind }
pub enum ParseErrorKind { Unterminated, StrayMarker }
pub struct NoSuchBlock(pub usize);
pub fn parse(bytes: &[u8]) -> Result<ConflictFile, ParseError>;
pub fn blob_id(bytes: &[u8]) -> Oid;
impl ConflictFile {
    pub fn render(&self) -> Vec<u8>;
    pub fn take(&mut self, block: usize, take: Take) -> Result<(), NoSuchBlock>;   // block index counts Conflict and Resolved regions in order; re-taking a Resolved block replaces its take
    pub fn blocks(&self) -> Vec<BlockAt>;
    pub fn open(&self) -> usize;
}
pub enum ReadError { Io(std::io::Error), Binary, TooLarge { bytes: u64 }, Parse(ParseError) }
pub fn read(path: &Path) -> Result<ConflictFile, ReadError>;
pub enum WriteError { Io(std::io::Error), Changed { now: Oid } }
pub fn write_back(path: &Path, file: &ConflictFile, expected: Oid) -> Result<Oid, WriteError>;   // Ok(new fingerprint)
pub fn stage_bytes(repo: &RepoHandle, path: &Path, stage: u8) -> Result<Option<Vec<u8>>, RunError>;
pub fn diff3_blocks(repo: &RepoHandle, path: &Path) -> Result<Option<Vec<Block>>, Diff3Error>;   // None: a stage is missing
pub enum Diff3Error { Run(RunError), Io(std::io::Error), Parse(ParseError) }
pub struct BaseMismatch { pub block: usize }
pub fn attach_base(file: &mut ConflictFile, diff3: &[Block]) -> Result<usize, BaseMismatch>;   // Ok(blocks given a base)

// graft-git/src/status.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum UnmergedHow { BothModified, BothAdded, BothDeleted, AddedByUs, AddedByThem, DeletedByUs, DeletedByThem }
impl UnmergedHow { pub fn words(self) -> &'static str; pub fn has_markers(self) -> bool /* BothModified | BothAdded */ }
pub struct StatusEntry { /* existing */ pub unmerged_how: Option<UnmergedHow> }

// graft-fixtures/src/shape.rs
pub enum Shape { /* existing */ StoppedRebase, StoppedMerge }
```

## Acceptance

Every box names its test.

- [ ] Round trip: `render(parse(b)) == b` for every file in `StoppedRebase` with markers, plus hand-written LF / CRLF / mixed / no-final-newline / label-with-spaces inputs — `conflict::tests::round_trip_is_byte_exact`.
- [ ] `md.md` parses as one block whose theirs side contains the literal `=======` line; the ruling is in NOTES with the `git merge-file` output it was checked against — `conflict::tests::setext_underline_after_the_separator_is_content`.
- [ ] `parse` on `zdiff3.txt` gives a `base` on every block; on `both.txt` no base — `conflict::tests::diff3_and_zdiff3_carry_base`.
- [ ] `take(Ours|Theirs|Both)` then `render` equals `git checkout --ours|--theirs` bytes for a one-block file, and ours-then-theirs for `Both` — `conflict::tests::take_matches_git_checkout_sides`.
- [ ] `write_back` with a stale `expected` returns `Changed { now }` and leaves the file's bytes untouched; with the right one the file has the rendered bytes, the old mode bits, and no temp file remains in the directory — `conflict::tests::write_back_refuses_a_changed_file`, `write_back_keeps_mode_and_leaves_no_temp`.
- [ ] `blob_id(bytes)` equals `git hash-object --stdin` for empty, 1-byte and 1 MiB inputs — `conflict::tests::blob_id_is_gits`.
- [ ] `read` on `pic.bin` → `Binary`; on a 9 MiB file → `TooLarge { bytes }` — `conflict::tests::read_names_binary_and_too_large`.
- [ ] `stage_bytes` for `theirs-deleted.txt` stage 3 → `Ok(None)`, stage 2 → `Some(bytes)` equal to `git show :2:` — `conflict::tests::stage_bytes_reads_and_names_missing`.
- [ ] `diff3_blocks` + `attach_base` on `three.txt` attaches 3 bases; after `take` on block 1 and a hand edit of ours in block 2 the reproduced blocks no longer match and the answer is `BaseMismatch { block: 1 }`, nothing attached — `conflict::tests::base_attaches_by_equality_only`.
- [ ] `UnmergedHow` for all eleven fixture files matches `git status --porcelain=v2` XY — `status::tests::unmerged_how_from_porcelain`.
- [ ] `Shape::StoppedRebase` and `StoppedMerge` build in under 2 s each and `git status` in them lists exactly the eleven paths as unmerged — `graft_fixtures::tests::stopped_shapes_are_as_documented`.
- [ ] Fuzz target compiles (`cargo fuzz build conflict_parse` runs in the review, not here); a 1 000-iteration property test with random marker soup runs in nextest under 1 s — `conflict::tests::parse_never_panics_on_marker_soup`.
- [ ] `first_marker_line` (existing) agrees with `blocks()[0].first_line` on every marker file — `conflict::tests::first_marker_line_agrees`.
- [ ] MAP.md for `graft-git` (`conflict`, `status::UnmergedHow`) and `graft-fixtures` (both shapes, file by file) updated; NOTES lists: the `=======` ruling, the marker-size gap, the scratch-dir choice.

## Gates

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide. Fast tier only; fuzz is the review's. No `unwrap` on git output; no `let _ =`.

## Size

One context. graft-git `conflict.rs` ~600 lines with tests, `status.rs` +60, fixtures +250. Under 80k tokens if the builder reads the MAP.md sections named and not `graft-git/src/history_reads.rs` whole.
