# trace-readers — git blame, `log -L` and `--follow` readers; the whole file with one commit's adds and removes

Status: done
Blocked by: none
Crates: graft-git, graft-fixtures. Nothing in graft or graft-ui. `scripts/m7-perf-blame.sh` is this ticket's too.
Source: `docs/design-notes/m7-power-tools.md`: "Pass 2b" B1, H1, "Interfaces and data shapes (2b)"; pass 1b's
"Blame" and "History" screens.
Build with: **Opus**. It is new read-only modules with oracles (`git blame`, `git log -L`), so a wrong answer shows in a
test. It decides nothing structural.
Scout: none. It is greenfield: new modules beside `diff` and `history`. The builder reads graft-git's MAP instead.
Needs: none.

## Decided
- **Blame runs CLI git through `run`**, which is async and cancellable (study §4.10: libgit2 blame is slow).
  - The command: `git blame --porcelain -M [-w] [--ignore-revs-file <f>] <rev> -- <path>`. For `Rev::WorkingTree` it
    either uses `--contents` or passes no rev (the path read from disk); the builder picks one and says which in NOTES.
  - `-C` is off (cost).
  - `ignore_revs: true` passes `.git-blame-ignore-revs` only when that file exists at the repo root. A repo whose
    `blame.ignoreRevsFile` config names another file is honoured by git itself.
  - With `ignore_revs: false`, pass `--ignore-revs-file ""`, which clears the config.
  - `IgnoreRevs.revisions` counts the file's non-comment, non-blank lines.
- **Line marks:** git's porcelain marks a line it could pass to no one with `?` / `*` (`blame.markIgnoredLines`,
  `blame.markUnblamableLines`). Run with both `-c` set, and map them to `LineMark::Skipped` / `Unblamable`.
- **Refusals before spawning:** a binary file (the diff's attribute check, as `file_diff` decides binary) →
  `BlameError::Binary`. A path not in the rev → `Untracked`.
- **`enclosing_function`:** find the function header git would use for the line.
  - A zero-context diff touching the line, under the file's funcname driver, names the function in its hunk header.
  - The function's end is the next header at the same or shallower indentation, or the file's end. Over 400 lines →
    `None`.
  - When the repo's attributes set no `diff=` driver for the path, pass `-c core.attributesFile=<a Graft-written file
    mapping the extension to git's built-in driver>`. Do this **only** when the user has no `core.attributesFile` of
    their own.
  - The builder may pick another method, as long as the oracle tests below hold. Say which in NOTES.
- **`line_history`:** `git log -L<start>,<end>:<path> --format=<fields> <at>`, parsed to `TraceCommit`s.
  - `range_at` is the range's position in that commit, from the `-L` hunk headers.
  - `added`/`removed` count the range's lines only.
  - Streamed to `sink` in batches as git prints, if `run` can stream stdout. If it can't, one batch at the end, noted
    in NOTES (the app shows `tracing…` either way).
- **`file_history`:** `git log --follow --numstat --format=<fields> <at> -- <path>`. `renamed_from` comes from
  `--name-status` R rows.
- **`TraceEnd`:**

  | Case | `TraceEnd` |
  |---|---|
  | The last commit added the file (or range) | `Start{added_by}` |
  | git exits early with a message | `GitStops(first line)` |
  | `cap` reached | `Capped{shown}` |
  | `stop` fired | `Stopped` (the rows so far were already sent) |
- **`whole_file`:** `file_diff` with context = every line, giving one hunk from line 1 to the end. A merge is diffed
  against its first parent; a root commit is all adds. `old_path` is for a rename. It respects `max_lines` (§6.3's
  limit) with `truncated`.
- **`map_to_head`:** pure. It walks the worktree-vs-HEAD hunks. It returns `None` for a line inside an added run, and
  otherwise the HEAD line number.

## Read first
- The note: "Pass 2b" B1, H1; "Interfaces and data shapes (2b)" (graft-git::blame and ::trace).
- `docs/research/blame-line-history.md` §3, §4.1–4.3, §4.6–4.10.
- `crates/graft-git/MAP.md`:
  - `run` (every entry);
  - `diff` (`file_diff`, `parse_unified`, `Hunk`, `FileDiff`, `DiffError`);
  - `history` (`read_commit`);
  - `history_reads` (`parse_rev_list`, `ParseError`);
  - `Stop`.
- `crates/graft-fixtures/MAP.md`: the `Shape` list and how a scripted repo is built.

## Signatures (exact; add to graft-git's `lib.rs` exports)
```rust
// graft-git::blame
pub enum Rev { Commit(Oid), WorkingTree }
pub struct BlameOptions { pub ignore_whitespace: bool, pub ignore_revs: bool }   // Default: both true
pub struct BlameCommit { pub id: Oid, pub author: String, pub author_time: i64, pub summary: String,
                         pub previous: Option<(Oid, PathBuf)>, pub boundary: bool }
pub enum LineMark { Plain, Skipped, Unblamable }
pub struct BlameLine { pub commit: Option<Oid>, pub orig_line: u32, pub final_line: u32, pub text: Vec<u8>,
                       pub mark: LineMark }
pub struct IgnoreRevs { pub file: PathBuf, pub revisions: usize, pub applied: bool }
pub struct Blame { pub path: PathBuf, pub rev: Rev, pub lines: Vec<BlameLine>,
                   pub commits: BTreeMap<Oid, BlameCommit>, pub ignore_revs: Option<IgnoreRevs> }
pub enum BlameError { Run(RunError), Parse(ParseError), Binary, Untracked, Stopped }
pub fn blame(repo: &RepoHandle, rev: &Rev, path: &Path, opts: &BlameOptions, stop: &dyn Stop)
    -> Result<Blame, BlameError>;
pub fn parse_porcelain(bytes: &[u8]) -> Result<(Vec<BlameLine>, BTreeMap<Oid, BlameCommit>), ParseError>;
// graft-git::trace
pub enum LineRange { Function { name: String, start: u32, end: u32 }, Block { start: u32, end: u32 } }
pub fn enclosing_function(repo: &RepoHandle, at: Oid, path: &Path, line: u32, stop: &dyn Stop)
    -> Result<Option<LineRange>, RunError>;
pub fn map_to_head(worktree_vs_head: &[Hunk], line: u32) -> Option<u32>;
pub struct TraceCommit { pub id: Oid, pub author: String, pub when: i64, pub subject: String, pub path: PathBuf,
                         pub renamed_from: Option<PathBuf>, pub added: u32, pub removed: u32,
                         pub range_at: Option<(u32, u32)> }
pub enum TraceEnd { Start { added_by: Oid }, GitStops(String), Capped { shown: usize }, Stopped }
pub enum TraceError { Run(RunError), Parse(ParseError) }
pub fn line_history(repo: &RepoHandle, at: Oid, path: &Path, range: &LineRange, cap: usize,
                    sink: &mut dyn FnMut(Vec<TraceCommit>), stop: &dyn Stop) -> Result<TraceEnd, TraceError>;
pub fn file_history(repo: &RepoHandle, at: Oid, path: &Path, cap: usize,
                    sink: &mut dyn FnMut(Vec<TraceCommit>), stop: &dyn Stop) -> Result<TraceEnd, TraceError>;
pub fn whole_file(repo: &RepoHandle, commit: Oid, path: &Path, old_path: Option<&Path>, max_lines: usize,
                  stop: &dyn Stop) -> Result<FileDiff, DiffError>;
pub fn parse_line_log(bytes: &[u8]) -> Result<Vec<TraceCommit>, ParseError>;
pub fn parse_follow_log(bytes: &[u8]) -> Result<Vec<TraceCommit>, ParseError>;
```
If `ParseError` or `Hunk` lives under another path, use that one and say so in NOTES; don't duplicate it. All
`Display`s are one line.

## The fixture (graft-fixtures; every blame and history walk in M7 uses it)
A scripted repo, "session", matching the note's mock-ups:
- **`src/auth/session.rs`, ~60 lines.**
  - Commits:
    - Tom, "Add token refresh", 10 months ago: lines 18–29;
    - Lina, "Retry refresh once on 401", 7 months ago: adds **line 19** `let mut attempt = 0;` and changes line 22 (F3:
      history's mock-up is right);
    - Priya, "Move session TTL to config", 4 months ago;
    - you, "Handle expired tokens", today.
  - `refresh_token` is a method inside an `impl Session` block.
- **A rename:** the file was `src/session.rs` until Priya's commit.
- **Ignore-revs:** `.git-blame-ignore-revs` lists one "rustfmt" commit that reformatted lines 30–33.
- **A long line:** line 27 is ≥ 150 characters, which forces the trailer fallback.
- **Binary and untracked:** `assets/pic.bin` (binary) and an untracked `src/new.rs`.
- **Fixed author dates**, so ages are stable against the walks' fixed clock. The builder finds how the existing walks
  fix "now".

Add it as a new `Shape` (the builder names it) and document it in graft-fixtures' MAP.

## Acceptance (name the verifying test at tick time; red first where it says so)
- [x] **Blame:** all in `crates/graft-git/tests/it/blame.rs`, all passing.
  - `blame_matches_git_blame_line_for_line`: every line's commit and orig line against `git blame --line-porcelain`, run
    separately, on the fixture at 3 revs and the working tree. **Test: `blame_matches_git_blame_line_for_line`** (you,
    priya, lina via the old path, and the modified working tree).
  - `ignore_revs_skips_the_listed_commit_and_counts_it`. **Test: same name.**
  - `ignore_revs_false_blames_the_rustfmt_commit`. **Test: same name** (+ `ignore_revs_false_clears_a_configured_list_too`).
  - `ignore_whitespace_is_on_by_default`. **Test: same name.**
  - `a_skipped_line_is_marked_skipped`. **Test: same name.**
  - `previous_names_the_parent_and_old_path_across_the_rename`. **Test: same name.**
  - `working_tree_blame_leaves_new_lines_uncommitted` (`commit: None`). **Test: same name.**
  - `binary_is_refused_before_spawning`. **Test: same name.**
  - `untracked_is_refused`. **Test: same name.**
  - `blame_honours_stop` (the child is killed; `Stopped`). **Test: same name** — see NOTES: the stop fires on the
    pre-check spawn, not the blame child.
- [x] **Function range:** in `crates/graft-git/tests/it/trace.rs`.
  - `a_rust_method_is_its_fn_not_the_impl` (red first: without the attributes file, git's default names the `impl`).
    **Test: same name.** Red observed: with `graft_attributes` forced to `None` it failed with `name: "impl Session {",
    start: 13, end: 42`; restored, green.
  - `a_python_def_is_found`. **Test: same name.**
  - `no_function_is_none`. **Test: same name** (an import line, past the end, a binary).
  - `over_400_lines_is_none`. **Test: same name** (a 452-line fn → `None`; the small fn after it still found).
  - `a_user_attributes_file_is_never_overridden` (a repo with `core.attributesFile` set; assert Graft passed no `-c`).
    **Test: same name** — asserted by effect, not by argv: the user file names no rust driver, so the result is the
    `impl` (a Graft `-c` would have named the method). Plus `a_repository_driver_is_used`.
- [x] **Traces:** in `crates/graft-git/tests/it/trace.rs`.
  - `line_history_matches_git_log_L`: the ids, in order, against `git log -L… --format=%H`. **Test: same name.**
  - `range_at_moves_with_the_code`. **Test: same name** ((18,29), (17,28), (17,27); counts range-only).
  - `file_history_follows_the_rename` (`renamed_from` on Priya's commit). **Test: same name.**
  - `trace_end_is_start_on_the_adding_commit`. **Test: same name** (line → Tom, file → Ana). Plus `git_stopping_says_why`.
  - `trace_capped_says_how_many`. **Test: same name** (cap 2 → `Capped{2}`; cap 6 = exactly all → `Start`).
  - `stop_kills_git_and_keeps_the_rows` (the rows sent before the stop are the sink's; returns `Stopped`). **Test: same
    name** — see NOTES: no rows are ever sent before a stop (one batch at exit), so it asserts `Stopped` + empty sink.
- [x] **Whole file:** in `crates/graft-git/tests/it/trace.rs`.
  - `whole_file_is_one_hunk_with_every_line`. **Test: same name** (61 lines = 60 + 1 removed; also Priya's rename with
    `old_path`).
  - `whole_file_of_a_root_commit_is_all_adds`. **Test: same name.**
  - `whole_file_of_a_merge_diffs_its_first_parent`. **Test: same name** (+ an unchanged file is all context).
  - `whole_file_respects_max_lines`. **Test: same name.**
- [x] **`map_to_head`:** **Tests: `map_to_head_skips_added_lines`, `map_to_head_shifts_past_a_removal`**
  (`crates/graft-git/src/trace/tests.rs`).
- [x] **Parsers:**
  - Unit tests on git 2.38 and current output samples, checked in as bytes. **Tests:** `blame/tests.rs` (5: 2.54
    sample, 2.38-shaped sample, truncation/garbage, missing headers, `count_revisions`); `trace/tests.rs`
    (`the_2_54_line_log_reads_ranges_and_counts`, `the_2_38_line_log_reads_a_rename_and_an_added_range`,
    `the_2_54_follow_log_reads_the_rename`, `the_2_38_follow_log_reads_a_binary_rename`,
    `garbage_is_a_parse_error_not_a_panic`). 2.54 samples captured from the fixture in `src/testdata/git-2.54/`;
    2.38 samples hand-written (see NOTES).
  - `cargo-fuzz` targets `blame_porcelain`, `line_log` and `follow_log` beside the existing ones. Each is built, and run
    60 s each in the review (not here). **Verified: `cargo +nightly fuzz build` in `fuzz/` built all targets.** Not run.
- [x] **Measured:** `scripts/m7-perf-blame.sh` → `docs/perf/trace-readers.txt`.
  - It blames and traces the largest source file of the `synthetic::Scale::HEAVY` fixture (or the largest repo
    fixture), release, 10 runs. **Harness: `crates/graft-git/examples/trace_perf.rs`** (`required-features =
    heavy-tests`), on `trunk.txt`.
  - Summary line: `trace-readers: blame p95=14.3 line_history p95=252.6 file_history p95=18.5` (ms; cap 1000, both
    traces capped).
  - The builder reads only the summary. Over 1 000 ms for blame → NOTES, and ask. The screen is async either way.
    Blame 14.3 ms: under. See NOTES on what this does not measure.
- [x] graft-git MAP.md: the `blame` and `trace` modules. graft-fixtures MAP.md: the new shape. **Done** (both files;
  doc, not tested).

## Touches (check each, report how)
- `graft-git` exports: new modules only. `diff::file_diff` is called, not changed. Check: `git diff --stat` shows no
  edit in `diff/`.
- Fixtures: a new shape must not join any "every shape" loop whose tests would slow down. Check: grep the loops;
  NOTES names them.
- Fuzz: `fuzz/Cargo.toml` gains 3 targets. Check: the repo's fuzz build command, if the builder can run it; otherwise
  NOTES says it was not built (unverified, not broken).
- Captures, keymap, journal: none.

## Rules
Worktree only. `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md`. Measurement runs from
the script, never in your context.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
~70k. If it compacts, split:
- **trace-readers-a:** `blame`, `IgnoreRevs`, the fixture, `parse_porcelain`. This unblocks `blame-place`.
- **trace-readers-b:** `enclosing_function`, the traces, `whole_file`, `map_to_head`, the perf script. This unblocks
  `history-place`.

No stubs: each half lands its own signatures working.

## NOTES (builder, 2026-09-26)

**Compacted: yes**, once, mid-ticket (after blame + fixture + parsers, before the trace integration tests). Nothing
remains; no split needed. Scout: none — greenfield modules; read graft-git's MAP.md and the `diff` reader instead.

Decisions the ticket asked for:
- **Working-tree blame:** no rev passed (git reads the path from disk), not `--contents`. The all-zero id → `commit: None`.
- **Refusals** (`Binary`, `Untracked`): `git diff-tree -r --no-renames --numstat <empty tree> <rev|HEAD> -- path`
  (empty → Untracked, `-\t-` → Binary); the working tree also checks the content on disk with `diff-index --numstat`.
- **Function method:** the ticket's sketch, for every line at once — `git diff --no-index -U0` of the blob against a
  copy with a marker after every line; each hunk heading is the funcname governing that line. Driver: user
  `core.attributesFile` set → Graft passes nothing; repo names a driver → `-c core.attributesFile=<tmp>` with
  `* diff=<that driver>` (so `src/**` patterns still apply to temp paths); none → git's built-in for the extension.
  `a_repository_driver_is_used` covers the middle case. The temp-file IO errors surface as `RunError::Spawn` (the
  function returns `RunError`); the message reads as a spawn failure — a cosmetic mismatch, not a swallowed error.
- **Streaming:** `run` does not stream stdout, so both traces send **one batch at exit**. Consequence:
  `stop_kills_git_and_keeps_the_rows` can only assert `Stopped` and an empty sink — there are never rows before a stop.
  If the history screen needs rows as git prints, `run` needs a streaming stdout mode (a graft-git change, own ticket).
- **`blame_honours_stop`:** the stop is observed on the first spawn (the `diff-tree` refusal check), so the blame
  child itself is never started, not killed mid-run. The runner's kill path is the same `run` code either way.
- **`ParseError`** is `crate::diff::ParseError`, reused, not duplicated. `Hunk`/`FileDiff` are `crate::diff`'s.
- **`whole_file`** runs its own `diff-tree -p -U1000000000` rather than `diff::file_diff`: `DiffOptions::context` is a
  `u8`. `diff/` untouched.
- **`-L` and renames:** `git log -L` follows renames without `rename from` lines; `renamed_from` falls back to the
  `--- a/` path when it differs.
- Extra public item: `LineRange::lines()`; `blame::IGNORE_REVS_FILE`. Both in MAP.md.

Fixture: `Shape::Session` (name `session`), in a new `Shape::TRACED`, **not** `ALL` (it has an untracked file).
Loops over `every()` it does join: `crates/graft-fixtures/src/corpus.rs` (build loop and `from_name`),
`crates/graft-fixtures/tests/it/corpus.rs` (identity and accounted-for checks) — one small deterministic repo, no slow
test. Loops over `ALL` in other crates (gallery, snapshots, radar) are not joined. Its rustfmt lines are 31–33 and
35–37 (the design note's mock said 30–33). No `unblamable` line occurs in it; the hand-made 2.38 sample covers that.

Parser samples: 2.54 captured from the fixture by git 2.54.0. **2.38 samples are hand-written** to the documented
format: no 2.38 binary on this machine — unverified against a real 2.38.

Perf: `blame p95=14.3 line_history p95=252.6 file_history p95=18.5` ms. The heavy fixture's biggest file is
`trunk.txt`, **one line** rewritten by every trunk commit: this measures deep history (both traces hit the 1000 cap),
not blame over a long file. A long-file blame is unmeasured.

**Finding (out of scope, not fixed):** `crates/graft-git/src/diff/unified/primitives.rs:54` splits an unquoted
`diff --git` header on `" b/"`, and its doc (line 38) says git quotes both paths whenever either needs it. git 2.54
emits a **mixed** header for a rename to a non-ASCII name in `log -L`: `diff --git a/old.rs "b/n\303\251w.rs"`
(reproduced with a throwaway repo). `parse_unified` then fails "malformed `diff --git` header", so `line_history`
across such a rename returns `TraceError::Parse`. The 2.38 sample quotes both sides to stay inside today's parser.
Fix belongs in `diff/` (quoted second path after an unquoted first).

Gates: fmt, clippy `--all-targets -D warnings`, `cargo nextest run` (3602 passed, 1 skipped), `cargo test --doc` —
all workspace-wide. First nextest run had one failure outside my crates,
`graft::it two_boxes_over_content::capture_two_boxes_over_real_content_for_docs_shots`; it passed alone and on a
full re-run. Intermittent under load; cause not investigated.
