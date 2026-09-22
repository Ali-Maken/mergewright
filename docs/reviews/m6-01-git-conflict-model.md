# Review: M6 group 01 `git-conflict-model` (47b55c2..f3353ea)

Reviewer: Fable, read-only against f3353ea. One round (ADR-0029).
Ticket: `.scratch/m6-conflicts/issues/01-git-conflict-model.md`. Design: `docs/design-notes/m6-conflicts.md` Decision 1, Interfaces, Hard-to-reverse 2 and 4; ADR-0042.

## What was verified, and how

**No cargo was run** (two builders hold worktrees; the conductor's merge gate stands: 2494 passed, 1 skipped, fmt/clippy clean). Every behavioural claim below was checked either by reading the code at the named line or by an experiment against **git 2.54.0 (Apple Git-157)** in throwaway repos under `/tmp`, with `GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1`. Timing claims in the ticket (0.42 s for the shape test, 1 000 soups under 1 s) and `cargo fuzz build` are **unverified**, not disproved.

Claims of the builder's that held against real git:

- The first `=======` after `<<<<<<<` is the separator; a later one is theirs' content. `git merge-file` (default, `--diff3`, `--zdiff3`) prints a setext `=======` inside theirs verbatim. A `=======` in *ours'* content is inherently ambiguous; the module doc says so.
- Default style joins conflicts three unchanged lines apart into one block and keeps four apart; `--diff3` never joins. Measured with a real `git merge`, gaps 3 and 4.
- CRLF files get CRLF markers; a side lacking a final newline gets one added inside the block; a conflict file ends in a partial line only when its last plain line does; `-L ""` writes `<<<<<<< ` with the space.
- `blob_id` = `sha1("blob <len>\0" + bytes)` matches `git hash-object --stdin` for empty and CRLF input.
- `merge-file`'s exit code is the conflict count capped at 127 (130 conflicts → exit 127); `conflict.rs:795` accepts 0..=127 inclusive, which is right.
- Porcelain v2 XY for all seven shapes, both after `git merge` and after `git rebase`, and `git status`'s long-form words, match `UnmergedHow::from_xy` / `words()` (`status.rs:171-202`). Rename/rename is what yields `DD` and brings `AU`+`UA`; thirteen paths, not eleven, is right.
- `ls-files -u -z` record shape is `mode SP oid SP stage TAB path NUL` as `stages` parses it (`conflict.rs:672-697`). `merge-file` runs outside a repository.

Claims **disproved** or shown incomplete (each is a finding below):

- "A working-tree block is the union of one or more reproduced blocks" (NOTES, `conflict.rs:908-915`) — it can also be a **fraction** of one: git's zealous refinement splits a conflict whose sides share interior lines into several blocks; `--diff3` and `--zdiff3` both leave it as one. → Major 1.
- Reproduced sides equal the working-tree bytes — not under `eol=crlf`/`core.autocrlf`: the stop writes CRLF (markers included) while the stages are LF. → Major 2.
- `conflict-marker-size ≠ 7` "parses as plain": a real merge with `conflict-marker-size=12` leaves a `UU` file of 12-char markers that this parser calls plain and `first_marker_line` cannot see. → Major 3.
- Git's own `rerere` preimage/postimage use bare `<<<<<<<` / `>>>>>>>` (no space, no label); this parser treats them as plain and then errors `StrayMarker` on the `=======`. → Major 5.

## Acceptance ticks

14 boxes checked, **13 held, 1 overstated**. Every box names a test that exists at f3353ea and tests the box's substance (`crates/graft-git/src/conflict/tests.rs`, `status/tests.rs:453`, `graft-fixtures/src/shapes/conflicts.rs:230`). The one that does not hold as written is box 13 — see Critical 1.

## Critical

### C1. Tick 13 claims "on every marker file **in both shapes**"; the test reads one shape

`crates/graft-git/src/conflict/tests.rs:548` — `first_marker_line_agrees` borrows `Shape::StoppedRebase` only. The ticket's box (`01-git-conflict-model.md`, acceptance list, box 13) says "in both shapes". The original box asked only "on every marker file", which the test does cover; the builder's tick added a claim no test verifies. Filed Critical by CLAUDE.md's rule ("a false tick is Critical, whatever its size"); the substance of the box holds and the fix is one loop over `[Shape::StoppedRebase, Shape::StoppedMerge]` or one word removed from the tick. The conductor may downgrade; I do not.

## Major

### M1. `attach_base` refuses pristine files that git's zealous merge split — and blames the user for editing

`conflict.rs:886-894` (`find_run`): a run is grown from one reproduced conflict upward and abandoned as soon as its sides are **longer** than the file block's. So a file block that is a *part* of one reproduced block never matches. Real git (experiment, `git merge` on base `1..7`, ours `A b c d e f B`, theirs `X b c d e f Y`) writes **two** blocks (`A`/`X`, then five plain lines, then `B`/`Y`); `merge-file --diff3` and `--zdiff3` both print **one** block with seven-line sides. `attach_base` then returns `Err(BaseMismatch { block: 0 })`, whose `Display` (`conflict.rs:827`) says "base unavailable after edits" on a file nobody edited. The NOTES' model ("the union of one or more reproduced blocks") is half of git's behaviour: `xdl_merge` at `XDL_MERGE_ZEALOUS` both joins (≤3-line gaps) and splits (refines conflicts whose sides share lines), and `--diff3` caps the level at `EAGER`, so it does neither.

Why it matters: both sides touching a region and agreeing on its middle (a reformat plus an edit at each end) is a common shape; `B` will fail on it with a wrong explanation, and G5 builds on this rule (hard-to-reverse choice 4).

Fix, either: (a) after the run search fails, try the inverse — a run of **file** regions (block, plain…, block) whose concatenated sides equal one reproduced block; give each file block in that run the whole reproduced base, flagged as shared (git itself cannot attribute base lines to the sub-blocks, which is why diff3 refuses to refine); or (b) keep "never a guess" and return a **typed** answer distinct from a hand edit (`NoBase::SplitByGit { first_block, last_block }`) so the UI never says "after edits" for a pristine file. Either way, add the experiment above as a literal pair test next to `MERGE_STYLE_JOINED`.

### M2. `attach_base` refuses every block of a pristine file under `eol=crlf` / `core.autocrlf`

`conflict.rs:721-730` (`blob_bytes`) copies the raw blob (`cat-file blob <oid>`) into the scratch directory; `conflict.rs:895-896` compares reproduced lines byte-for-byte with the working tree. Experiment: `.gitattributes` `f.txt text eol=crlf`, LF blobs, `git merge` → the working-tree conflict file is CRLF throughout, markers included; `git show :2:f.txt` is LF. The reproduced blocks are LF, the file's are CRLF, every block mismatches, "base unavailable after edits" again. Same mechanism for `core.autocrlf=true` and for `ident`/`filter` smudges.

Fix: write the scratch copies with the path's working-tree conversion — `git cat-file --filters --path=<path> <oid>` (verified: gives CRLF for the case above). Keep `stage_bytes` raw if G5 wants blob bytes for the whole-file view, but say which it is in the doc comment. Test: a corpus file with `eol=crlf` in `.gitattributes` (the corpus has `crlf.txt` with CRLF blobs, which is the case that *doesn't* exercise this).

### M3. A file with `conflict-marker-size ≠ 7` parses as clean, and nothing downstream can tell

`conflict.rs:267-281` (`Marker::of`) requires exactly seven marker characters plus a space; anything else is content, so the file is one `Plain` region with `open() == 0`. `history_reads.rs:465` (`first_marker_line`) has the same fixed prefix. Experiment: `.gitattributes` `f.txt conflict-marker-size=12`, `git merge` → working tree has `<<<<<<<<<<<< HEAD` / `============` / `>>>>>>>>>>>> feature`, status `UU`. Both the resolver (no blocks → `ConflictsOpen` cannot fire) and the `MarkersRemain` refusal are blind, so `⏎`/`space` on that file stages markers and the commit editor commits them. git.git itself sets this attribute on its own docs; Markdown/RST projects use it for the same setext reason `md.md` exists.

The builder recorded the gap honestly (NOTES "Marker size ≠ 7"; MAP.md). The choice "plain" is the unsafe one of the two the ticket offered. Fix now, small: treat a column-0 run of **eight or more** `<`/`>` followed by a space, or of `=` alone, as `ParseErrorKind::MarkerSize` (a genuine plain file with such a line is far rarer than a conflicted one). Fix right, later: `parse_with(bytes, marker_size)` fed by one `git check-attr conflict-marker-size -- <path>` in the opener, and the same for `first_marker_line`. Record the KNOWN_GAPS row as **broken** (a real conflict is invisible), not unverified.

### M4. Attaching a base changes `render()` and shifts `blocks().first_line`, so line numbers stop matching the file on disk

`conflict.rs:170-171` counts `base.len() + 1` lines for a block that has a base; `conflict.rs:542` accumulates that into `first_line`; `conflict.rs:476-483` renders `||||||| label` + base lines for any `Conflict` block with `base: Some`; `conflict.rs:951-952` sets both on a merge-style block. After `attach_base` on a merge-style file, `first_line` for every block after the first is larger than its line on disk (by the earlier blocks' base lengths + 1), and `blocks()`' doc (`conflict.rs:194-197`, "1-based line in `render()`") is now a line in a rendering the disk never had. `$EDITOR` (`e`) and the diff pane show disk lines. And a `write_back` of that `ConflictFile` (G2's `apply_take` is designed to re-read first — but the type does not stop anyone) would write a diff3-style file the user never had, with `base` as the base label.

Fix: keep a reproduced base out of the rendering — a separate `Block.reproduced: Option<(label, lines)>` (or an `attached: bool` on the base that `render`/`line_count` skip). `attach_base` then cannot change what `write_back` writes or what line numbers mean. Test: `blocks()` before and after `attach_base` on `three.txt` are equal; `render()` unchanged.

### M5. Bare `<<<<<<<` / `>>>>>>>` (no label, no space) are not markers — git's `rerere` writes exactly that

`conflict.rs:271` (`strip_prefix(b"<<<<<<< ")`) needs the space. Experiment: with `rerere.enabled`, a stop writes `.git/rr-cache/<id>/preimage` as `<<<<<<<\n b-o\n =======\n b-t\n >>>>>>>\n` — labels stripped, no trailing space. `parse` on that file: line 2 is content, line 4's `=======` is `StrayMarker`. The ticket names M7's rerere as a consumer of this spine; `xdl_merge` also writes the bare form whenever a label is NULL. Fix now, before G2-G5 build on `Vec<u8>` labels: `ours_label`/`theirs_label` as `Option<Vec<u8>>` (`None` renders `<<<<<<<`, `Some(l)` renders `<<<<<<< l`, so `Some(b"")` and `None` stay byte-distinct), and `Marker::of` accepts the bare form when the content is exactly the seven characters. Round-trip test for both.

## Minor

- **m1.** `attach_base` overwrites a base git already wrote (`conflict.rs:950-952`), replacing a zdiff3 file's verbatim label (`||||||| parent of f544e68 (t)` on a rebase — measured) with `base`; and because zdiff3 moves a hunk's common prefix/suffix lines outside the block while `--diff3` keeps them inside (measured: `p` shared → zdiff3 sides `O`/`T`, diff3 sides `p O`/`p T`), a pristine zdiff3 file with a shared line mismatches. Skip blocks that already have a base, and say so in the doc comment; G5 should not call it for them.
- **m2.** `tests.rs:367-373`: `MERGE_STYLE_JOINED` is `git merge-file -p` output, which runs at `XDL_MERGE_ZEALOUS_ALNUM`, not `git merge`'s `ZEALOUS`; they differ when the gap is non-alphanumeric (measured: four blank lines → `git merge` 2 blocks, `merge-file` default 1). The doc comment says "git's default style joins"; the literal proves it for `merge-file`, which is not what writes the working tree. Regenerate the literal from a real merge or reword.
- **m3.** `conflict.rs:338` and `:518`: two `unreachable!` in non-test code to satisfy the borrow checker. Restructure (`take()` the `Open` first at 337; match on `previous` without the placeholder at 515) so no panic path exists.
- **m4.** `conflict.rs:709`: a caller passing stage 0 gets `RunError::Unparsable`, whose contract (MAP.md) is "git ran and exited zero but the output did not match". Make `stage: u8` an enum `Stage { Base, Ours, Theirs }` and the error disappears.
- **m5.** `tests.rs:462`: `let _: Option<Diff3Error> = None;` does nothing (keeps an import alive). Drop it and the import.
- **m6.** `conflict.rs:858` `RunBase = Option<(Vec<u8>, Vec<Line>, Terminator)>` — a three-tuple read by position at `:950`. A small struct with names reads plainly.
- **m7.** Signature drift recorded in NOTES but not yet in the downstream tickets: `diff3_blocks -> Option<Vec<Block>>` became `diff3_file -> Option<ConflictFile>`, `attach_base(_, &[Block])` became `attach_base(_, &ConflictFile)`, `Diff3Error` gained `MergeFile`. G2's `base_for` and the design note's Interfaces block still name the old ones; the conductor should edit them before G2 reads them.
- **m8.** `conflict.rs:647`: `persist` renames over `path`; if `path` is a symlink the link is replaced by a regular file while `read` (`:593`) read the target. Not reachable through git today (git writes no markers into symlinks); one sentence in the doc comment is enough.
- **m9.** `conflict.rs:594-601`: the size cap is checked from metadata, then the file is read whole; a file growing between the two reads past the cap. Read with `take(MAX_BYTES + 1)` instead, or accept and note.
- **m10.** `diff3_file` labels the reproduced base `base` (`conflict.rs:786`); ADR-0042 wants git's own labels verbatim. The UI can caption it; note it in MAP.md so G3 does not think it is git's.
- **m11.** Ticket "Size" said ~600 lines with tests; `conflict.rs` is 971 plus 684 of tests. Not a defect — the module doc (33 lines) is the best-written part — but a reader looking for `attach_base` scrolls past `stages`/`blob_bytes`; a `conflict/stages.rs` split would help M7.

## Rule 3 (no swallowed errors)

Checked every `Result` in `conflict.rs`, `status.rs`, `shapes/conflicts.rs`: each surfaces typed (`ReadError`, `WriteError`, `Diff3Error`, `RunError`, `BuildError::Unexpected`). The two deliberate defaults (`first_line` → empty string `:812`; capacity hint `:600`) carry a comment saying why. `stop_on_conflict` (`conflicts.rs:163-177`) accepts exit 1 only and re-raises everything else, success included. No `let _ =` on a `Result`. Fine.

## Architecture, readability, safety

- Crate rules (ARCHITECTURE.md table): `graft-git` gains `sha1` and ships `tempfile`; both external, no internal edge changes. OK.
- No `unwrap`/`expect` outside tests; two `unreachable!` (m3).
- Terminal sanitising: not this crate's job; labels are raw bytes, and the design routes them through `SanitizedText` in `view::resolver`. G4 must not shortcut that — `ours_label` is user-controlled text from a branch name.
- UI thread: pure functions plus git spawns meant for a job. `find_run` is O(conflicts²) per unmatched block with a full recount each step (`:890-891`); fine under the 8 MiB cap, worth a note.
- Laptop-safety: the new tests build two small repos (<2 s budget asserted), one sparse 9 MiB file, one 1 MiB hash. Fine.
- Data loss: `write_back` refuses on a fingerprint change and renames a synced temp; a crash mid-write leaves the old file. The lease is a check, not a lock (window between `:628` and `:647`) — acceptable per design. The real data-safety hole is M3 (markers staged and committed unseen).

## Questions (no line, so not defects)

- Q1. Does G4 always re-read the file after `apply_take` and re-run `base_for`, so an `attach_base`-mutated `ConflictFile` never reaches `write_back`? If not, M4 becomes a write of diff3 markers into a merge-style file.

## Not committed

This worktree (`worktree-agent-a99c374539fc66396`) moved from f3353ea to ea8a14a while the review ran, with group 02's commits landing on it; CLAUDE.md forbids staging while another agent works the same worktree. The file is written, not committed. The conductor commits it.
