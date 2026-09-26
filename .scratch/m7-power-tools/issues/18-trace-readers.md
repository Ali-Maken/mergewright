# trace-readers — git blame, `log -L` and `--follow` readers; the whole file with one commit's adds and removes

Status: ready
Blocked by: none
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
- [ ] **Blame:**
  - `blame_matches_git_blame_line_for_line`: every line's commit and orig line against `git blame --line-porcelain`, run
    separately, on the fixture at 3 revs and the working tree.
  - `ignore_revs_skips_the_listed_commit_and_counts_it`.
  - `ignore_revs_false_blames_the_rustfmt_commit`.
  - `ignore_whitespace_is_on_by_default`.
  - `a_skipped_line_is_marked_skipped`.
  - `previous_names_the_parent_and_old_path_across_the_rename`.
  - `working_tree_blame_leaves_new_lines_uncommitted` (`commit: None`).
  - `binary_is_refused_before_spawning`.
  - `untracked_is_refused`.
  - `blame_honours_stop` (the child is killed; `Stopped`).
- [ ] **Function range:**
  - `a_rust_method_is_its_fn_not_the_impl` (red first: without the attributes file, git's default names the `impl`).
  - `a_python_def_is_found`.
  - `no_function_is_none`.
  - `over_400_lines_is_none`.
  - `a_user_attributes_file_is_never_overridden` (a repo with `core.attributesFile` set; assert Graft passed no `-c`).
- [ ] **Traces:**
  - `line_history_matches_git_log_L`: the ids, in order, against `git log -L… --format=%H`.
  - `range_at_moves_with_the_code`.
  - `file_history_follows_the_rename` (`renamed_from` on Priya's commit).
  - `trace_end_is_start_on_the_adding_commit`.
  - `trace_capped_says_how_many`.
  - `stop_kills_git_and_keeps_the_rows` (the rows sent before the stop are the sink's; returns `Stopped`).
- [ ] **Whole file:**
  - `whole_file_is_one_hunk_with_every_line`.
  - `whole_file_of_a_root_commit_is_all_adds`.
  - `whole_file_of_a_merge_diffs_its_first_parent`.
  - `whole_file_respects_max_lines`.
- [ ] **`map_to_head`:** `map_to_head_skips_added_lines`, `map_to_head_shifts_past_a_removal`.
- [ ] **Parsers:**
  - Unit tests on git 2.38 and current output samples, checked in as bytes.
  - `cargo-fuzz` targets `blame_porcelain`, `line_log` and `follow_log` beside the existing ones. Each is built, and run
    60 s each in the review (not here).
- [ ] **Measured:** `scripts/m7-perf-blame.sh` → `docs/perf/trace-readers.txt`.
  - It blames and traces the largest source file of the `synthetic::Scale::HEAVY` fixture (or the largest repo
    fixture), release, 10 runs.
  - Summary line: `trace-readers: blame p95=<ms> line_history p95=<ms> file_history p95=<ms>`.
  - The builder reads only the summary. Over 1 000 ms for blame → NOTES, and ask. The screen is async either way.
- [ ] graft-git MAP.md: the `blame` and `trace` modules. graft-fixtures MAP.md: the new shape.

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
