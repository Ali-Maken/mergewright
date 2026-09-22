# git-conflict-model: the conflict file as data — parse git's markers byte-exactly, take a side per block, write back without clobbering, read the three stages, reproduce git's diff3 blocks for base, name the unmerged shape, and the stopped-rebase corpus shape

Status: done — seam closed by the conductor's builder. `unmerged_how: None` added at all 24 `StatusEntry` literals in `graft`/`graft-ops` (list in NOTES); none of the 24 needed a real `UnmergedHow` (all generic test scaffolding, none asserting an XY pair). Full-workspace gates green 2026-09-22: `cargo fmt --check` clean, `cargo clippy --all-targets -- -D warnings` clean, `cargo nextest run` 2494/2494 passed (1 skipped), `cargo test --doc` all green. Every acceptance box below re-verified against this full-workspace run — same test names, still passing.
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

Every box names its test. All in `crates/graft-git/src/conflict/tests.rs` unless said otherwise; run 2026-09-22 under `cargo nextest run --workspace --exclude graft --exclude graft-ops` (1087 passed, 1 skipped).

- [x] Round trip: `render(parse(b)) == b` for every marker file in `StoppedRebase` (both, three, crlf, nofinal, md, zdiff3, both-added) plus eleven hand-written inputs (LF, CRLF, mixed, no final newline, newline-less end marker, label with spaces, empty label, empty sides, adjacent blocks, lone `\r`) — `conflict::tests::round_trip_is_byte_exact`.
- [x] `md.md` parses as one block whose theirs side holds the literal `=======` line, both from `git merge-file`'s verbatim output (`MERGE_FILE_MD`) and from the corpus file git wrote at the stop; ruling in NOTES — `conflict::tests::setext_underline_after_the_separator_is_content`.
- [x] `zdiff3.txt`: a base (`middle`) on every block; `both.txt`: none — `conflict::tests::diff3_and_zdiff3_carry_base`.
- [x] `take(Ours|Theirs)` + `render` equals `git checkout --ours|--theirs` bytes for five one-block files (both, crlf, md, nofinal, both-added); `Both` equals git's two checkouts spliced at line granularity (shared head, ours' own lines, theirs' own lines, shared tail) — `conflict::tests::take_matches_git_checkout_sides`.
- [x] Stale `expected` → `Changed { now }` with `now` the on-disk blob id and the bytes untouched; the right one → rendered bytes, mode bits kept (0o755 on unix), no `.graft-*` left — `conflict::tests::write_back_refuses_a_changed_file`, `write_back_keeps_mode_and_leaves_no_temp`.
- [x] `blob_id` equals `git hash-object --stdin` for empty, 1-byte and 1 MiB inputs — `conflict::tests::blob_id_is_gits`.
- [x] `read` on `pic.bin` → `Binary`; on a sparse 9 MiB file → `TooLarge { bytes: 9 MiB }` — `conflict::tests::read_names_binary_and_too_large`.
- [x] `stage_bytes` for `theirs-deleted.txt`: stage 3 → `Ok(None)`, stage 2 → `git show :2:` bytes, stage 1 → the base; stage 0 → `Err`; an unindexed path → `Ok(None)` — `conflict::tests::stage_bytes_reads_and_names_missing`.
- [x] `diff3_file` (the ticket's `diff3_blocks`, see NOTES) + `attach_base` on `three.txt` attaches 3 bases (`one`, `two`, `three`); after `take(0, Ours)` in memory and a hand edit of block 1's ours side → `BaseMismatch { block: 1 }`, nothing attached; whole-file shapes and unindexed paths → `Ok(None)` — `conflict::tests::base_attaches_by_equality_only`. A take **written back** removes its block while the stages keep it: the two remaining blocks still get their bases, a hand edit still mismatches — `base_survives_a_written_take_but_not_a_hand_edit`. A block git's default style **joined** from three `diff3` conflicts gets the whole run's base, plain lines in place, and a one-sided match is still a mismatch — `a_joined_block_gets_the_base_of_its_whole_run` (literals are `git merge-file` output, NOTES).
- [x] `UnmergedHow` for all **thirteen** (not eleven, NOTES) unmerged paths in both stopped shapes matches `git status --porcelain=v2` XY; the seven pairs by hand and `MM` → `Malformed` — `status::tests::unmerged_how_from_porcelain` (calls `stopped_shape_unmerged_how_matches_git` per shape).
- [x] Both shapes build into a temp dir in under 2 s each (0.42 s for the whole test), `.git/rebase-merge` / `.git/MERGE_HEAD` present, `git status --porcelain=v2` lists exactly the thirteen `(XY, path)` records, `zdiff3.txt` carries `||||||| ` — `graft_fixtures::shapes::conflicts::tests::stopped_shapes_are_as_documented`; names round-trip — `every_stopped_shape_has_a_name_and_is_found_by_it`.
- [x] Fuzz target: `cargo check --manifest-path fuzz/Cargo.toml --bin conflict_parse` passes on stable (no errors printed); **`cargo fuzz build` not run here** (nightly, the review's). The 1 000-iteration soup test passes; it and the shapes test together ran in 0.42 s under nextest — `conflict::tests::parse_never_panics_on_marker_soup`.
- [x] `first_marker_line` agrees with `blocks()[0].first_line` on every marker file in both shapes — `conflict::tests::first_marker_line_agrees`.
- [x] MAP.md for `graft-git` (`conflict` section, `status::UnmergedHow`, `StatusEntry.unmerged_how`) and `graft-fixtures` (both shapes, file by file, `STOPPED`/`every`) updated; NOTES below lists the `=======` ruling, the marker-size ruling, the scratch-dir choice.

Beyond the list: `marker_size_other_than_seven_is_plain`, `malformed_files_name_their_line` (`Unterminated` at the `<<<<<<<` line; every `StrayMarker` shape with its line), `a_second_base_marker_or_separator_inside_a_block_is_content`, `blocks_report_index_state_and_line`, `a_hand_built_block_renders_with_its_ends` (CRLF markers, `Terminator::None` end).

## Gates

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide. Fast tier only; fuzz is the review's. No `unwrap` on git output; no `let _ =`.

## Size

One context. graft-git `conflict.rs` ~600 lines with tests, `status.rs` +60, fixtures +250. Under 80k tokens if the builder reads the MAP.md sections named and not `graft-git/src/history_reads.rs` whole.

## NOTES

**Seam — why the workspace gate did not run whole.** Adding `unmerged_how` to `StatusEntry` breaks 24 struct literals outside the crate set (`cargo check --workspace --all-targets`: 24 × `E0063 missing field unmerged_how`, nothing else): `crates/graft-ops/src/operation/history/tests.rs:44`, `crates/graft-ops/src/operation/tests.rs:25`, `crates/graft/src/update/operate.rs:1119,1131,1143,1553,1713,1725,1848`, `crates/graft/src/update/tests.rs:2556,2565,2574,4134`, `crates/graft/src/view/tests.rs:529,540,551,562,2078`, `crates/graft/src/update/selection.rs:178,187,196`, `crates/graft/src/update/banner/tests.rs:24`, `crates/graft/src/update/commit_editor/tests.rs:47`, `crates/graft/src/update/stash/tests.rs:56`. Fix: `unmerged_how: None` at each (`Some(UnmergedHow::BothModified)` where the literal sets `unmerged: true`, if that site wants a shape). Gates ran with `--workspace --exclude graft --exclude graft-ops`: fmt, clippy `-D warnings`, nextest 1087/1087, doctests 2/2, all green. `blocked: needs graft, graft-ops`.

**The `=======` ruling.** The first `=======` after `<<<<<<<` is the separator; any later `=======` (and any `|||||||` after its section) before `>>>>>>>` is theirs' content. Evidence, `git merge-file -p -L ours -L base -L theirs` (git 2.54, 2026-09-22) on `md.md`'s three versions, kept verbatim in `conflict/tests.rs` as `MERGE_FILE_MD`:
```
Title
=====

<<<<<<< ours
Intro line ours
=======
Heading
=======

Intro line theirs
>>>>>>> theirs
```
`git checkout --theirs` on the corpus file gives the underline back as content (`take_matches_git_checkout_sides`). Inherent limit: a `=======` line in *ours'* content is read as the separator — nothing in the file distinguishes it. Not a gap to fix; documented in the module doc.

**Marker size ≠ 7.** A marker of any other width is not a marker: the file parses as plain text with zero blocks (`marker_size_other_than_seven_is_plain`), never `StrayMarker`. Chosen because a file with `conflict-marker-size = 12` must still open and read; the resolver then sees no blocks rather than an error. Gap, for KNOWN_GAPS: honouring `conflict-marker-size` (per-path gitattribute) is out; such a file shows as having no conflicts to take.

**Scratch dir for `diff3_file`.** A `tempfile::Builder` directory with prefix `graft-diff3-`, not `.git/graft/`: nothing is left in the user's repository if the process dies between write and merge, and it works on a read-only checkout — the sealed corpus is one, and `diff3_file` is tested on it. `merge-file` runs via `GitInvocation::in_directory(scratch)` with `run_raw`; exit 0..=127 is the conflict count, anything else is `Diff3Error::MergeFile`.

**`diff3_blocks` became `diff3_file`, and `attach_base` aligns runs, not blocks.** Measured (git 2.54, 2026-09-22, `/tmp` throwaway): git's default `merge` conflict style joins conflicts separated by three or fewer unchanged lines into one block; gap four keeps them apart; `diff3`/`zdiff3` style never joins. So a working-tree block is not one reproduced block but the union of one or more, with the plain lines between. `attach_base` therefore matches each file block against the shortest run of reproduced regions whose ours-side and theirs-side lines are byte-equal to it; the base is that run's base lines with the plain lines kept in place. That needs the plain regions, hence `diff3_file -> Option<ConflictFile>`. The gap-1 outputs are the `MERGE_STYLE_JOINED` / `DIFF3_STYLE_SEPARATE` literals in the tests. `three.txt` in the corpus has four filler lines between its changed lines so it really has three blocks (with one it had one, and the first cut of the test caught it).

**Thirteen unmerged paths, not eleven.** rename/rename(1to2) is the only recipe that yields `DD`, and it necessarily brings the two new names as `AU` and `UA`. All seven `UnmergedHow` variants are in the corpus. Rename/delete yields `UD`, not `DD`.

**`Shape::STOPPED`, not `ALL`.** `Shape::ALL` feeds the gallery, graph-snapshot and mistake-radar tests in `graft`, `graft-graph`, `graft-ops` (outside the crate set) that assume a clean tree. The stopped shapes live in `Shape::STOPPED`; `Shape::every()` = `ALL` ∪ `STOPPED` drives corpus build/read, `from_name` and the fixture identity tests.

**Dependencies.** `sha1 = "0.10"` added to graft-git (gix's hasher exposes only a fallible `try_finalize`); `tempfile` moved from dev-dependency to dependency for the scratch dir. `Cargo.lock` changed accordingly.

**Edges measured on git 2.54 (module doc).** CRLF files get CRLF-terminated markers; a side lacking a final newline gets one added *inside* the block, so a conflict file ends in a partial line only if its last plain line does; an empty label writes `<<<<<<< ` then newline.

**Unverified.** `cargo fuzz build` (nightly) — only `cargo check` of the target on stable. Windows path bytes (`path_bytes` non-unix arm is lossy, unexercised).

**Scout.** Sonnet `Explore` on graft-fixtures (shape/corpus/digest layout) before editing; graft-git read from MAP.md sections only.

**Compaction.** The session compacted once, after the tests first ran (the merge-vs-diff3 hunking finding); everything above was finished after it. Nothing remains.
