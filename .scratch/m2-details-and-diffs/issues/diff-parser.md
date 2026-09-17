# diff-parser: unified-diff and file-list parsers, line pairing, fuzz targets

Status: draft
Build with: Sonnet — pure byte parsing with git's own output as the oracle; mechanical once the shapes are fixed here.
Needs: none (pure; `diff-reads` calls these)

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — Sizing from M1; Interfaces (`graft-git::diff`); Proposed change "word and syntax spans in ui"
- `PROMPT.md` §5 Diffs (lines 371–394), §6.3 (lines 460–487), §7.2 item 5 (fuzz)
- `KNOWN_GAPS.md` row at line 127 (cargo-fuzz targets → M2)
- `crates/graft-git/MAP.md` whole (short)
- `crates/graft-fixtures/MAP.md` — `borrow`, hermetic `git` runner, `proptest_cases`

## Scope

In:
- New module `crates/graft-git/src/diff/` with `mod.rs` (types), `unified.rs` (`parse_unified`), `file_list.rs` (`parse_file_list`), `pairs.rs` (`pairs`). Types live here; `diff-reads` adds `read.rs` next to them.
- `parse_unified` accepts what `git diff-tree -p -M -C --no-color --no-ext-diff` emits for one or many files: `diff --git a/x b/x`, `old mode`/`new mode`, `similarity index`, `rename from/to`, `copy from/to`, `new file mode`, `deleted file mode`, `index …`, `Binary files … differ` / `GIT binary patch` (skip its body), `--- a/x` / `+++ b/x` (also `/dev/null`), `@@ -a,b +c,d @@ heading`, `\ No newline at end of file`. Quoted paths (`"a/\303\251"`) unquoted to bytes. Line numbers assigned while parsing. Unknown header lines before the first `@@` are kept out of hunks and don't fail the parse; a malformed `@@` fails with `ParseError { line, what }`.
- `parse_file_list` joins `diff-tree --raw -z` (status, modes, similarity) with `--numstat -z` (counts, `-` = binary) by path order. Renames/copies carry both paths.
- `pairs`: split each hunk into runs of `Removed` followed by `Added`; pair them index-wise (`min(len)` pairs, the rest unpaired); a `Pair` covers one run, with ranges into `hunk.lines`.
- `fuzz/` directory at the workspace root (cargo-fuzz layout, **not a workspace member**): targets `parse_unified`, `parse_file_list`; `fuzz/README.md` (how to run, never in `cargo test`); `cargo fuzz build` proven once by the builder, the run itself is a milestone-review task (`KNOWN_GAPS.md` row 127 closed with that wording).
- Tests: see Acceptance; proptest for `pairs` (never overlapping, never out of range).

Out: running git (`diff-reads`), word spans (`syntax-highlighting`), sanitising, any UI type, image or symlink special cases beyond what the raw headers say (`TypeChanged` for mode 120000 ↔ 100644).

## Signatures

```rust
// crates/graft-git/src/diff/mod.rs
pub enum FileStatus { Added, Deleted, Modified, Renamed { similarity: u8 }, Copied { similarity: u8 }, TypeChanged, Unmerged, Untracked }
pub struct FileEntry { pub path: Vec<u8>, pub old_path: Option<Vec<u8>>, pub status: FileStatus, pub added: Option<u32>, pub removed: Option<u32> }   // None → binary
pub struct FileList { pub files: Vec<FileEntry>, pub added: u64, pub removed: u64 }
pub enum LineKind { Context, Added, Removed }
pub struct DiffLine { pub kind: LineKind, pub text: Vec<u8>, pub old_no: Option<u32>, pub new_no: Option<u32>, pub no_newline_at_eof: bool }   // text without its '\n'; a trailing '\r' stays in `text`
pub struct Hunk { pub old_start: u32, pub old_lines: u32, pub new_start: u32, pub new_lines: u32, pub heading: Vec<u8>, pub lines: Vec<DiffLine> }
pub enum BinaryNote { Differ, Patch }                      // "Binary files differ" / a GIT binary patch (body skipped)
pub struct Truncated { pub shown_lines: usize, pub total_lines: usize }   // filled by diff-reads; parser leaves None
pub struct FileDiff { pub entry: FileEntry, pub hunks: Vec<Hunk>, pub binary: Option<BinaryNote>, pub truncated: Option<Truncated>, pub old_mode: Option<u32>, pub new_mode: Option<u32> }
pub struct ParseError { pub line: usize, pub what: &'static str }
pub fn parse_unified(bytes: &[u8]) -> Result<Vec<FileDiff>, ParseError>;
pub fn parse_file_list(raw_z: &[u8], numstat_z: &[u8]) -> Result<FileList, ParseError>;
pub struct Pair { pub removed: std::ops::Range<usize>, pub added: std::ops::Range<usize> }
pub fn pairs(hunk: &Hunk) -> Vec<Pair>;
```

## Acceptance

- [ ] Fixture round-trip: for every `Shape` in `Shape::ALL` and every commit in it (hermetic `git log --format=%H`), `parse_unified(git diff-tree -p -M -C --no-color --no-ext-diff --root <sha>)` is `Ok`, and per file the count of `Added`/`Removed` lines equals `--numstat` (binary files excluded). One test, loops over shapes.
- [ ] Hand-written cases (`tests/diff_cases.rs`, input strings ≤ 40 lines each): rename with hunks; copy; mode-only change (no hunks); binary; new file; deleted file; `\ No newline`; CRLF line kept with `\r`; quoted non-UTF-8 path unquoted to the right bytes; a hunk heading with `@@` inside it; two files in one stream; hunk count mismatch → `ParseError { line }`.
- [ ] `parse_file_list` on the `unicode-names` and `non-utf8-path` fixtures gives paths byte-equal to `git ls-tree -r -z` names.
- [ ] `pairs`: proptest (`proptest_cases`) — ranges within `lines`, no two pairs overlap, every `Removed`/`Added` line lies in exactly one pair's range or is unpaired; `Context` never inside a range.
- [ ] `cargo fuzz build` succeeds once (command and its last line in the report); `fuzz/` excluded from the workspace; `cargo test --quiet` time unchanged within noise.
- [ ] MAP.md: `diff` section, one line per public item.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet`.

## Size

One crate, four files (~400 lines) + tests (~250) + fuzz scaffold (~40). Fits under 80k if the builder writes the hand-written cases from the list above rather than from exploring git output; ≤ 25 tool calls. The fixture round-trip test is the only place git runs.
