# 30q: `u` gives back his bytes, not git's re-encoding of them (KNOWN_GAPS `M7/30p-crlf`; ticket 30p R1, NOTES 30p-a test 6)

Status: 30q-a partial — `blocked: needs` 22 files outside its list for the `JournalEntry` field (NOTES 30q-a)
Build with (three parts; **a, then b and c concurrent**: b and c hold disjoint files):
- **30q-a: Opus, `builder-opus-xhigh`.** It changes the before-snapshot every undoable operation takes and adds one field to the
  journal's opening line: undo territory, persisted. Not Fable: the layout and the field are fixed in this spec (R2, R3),
  so the builder makes no structural choice, and real-git byte-exact tests judge it.
- **30q-b: Opus, `builder-opus-xhigh`.** It changes what `RestoreWorktree` writes, in `u`, redo and the roll-back of a
  cancelled undo. Same reasons.
- **30q-c: Sonnet.** A perf script and one example, modelled on an existing one.
Blocked by: nothing; 30p-a merged (a83555cf).
Scout: none. Every site is named by function; re-grep by name. MAP.md first (graft-ops `snapshot`, `undo`, `restore`,
`journal`; graft-core `JournalEntry`), source second.
Needs: b needs a's `Snapshot::worktree_raw` and `JournalEntry::worktree_raw_before`. c needs a's `snapshot::raw_record`.

Source: KNOWN_GAPS `M7/30p-crlf`; ticket 30p R1 and NOTES 30p-a (test 6, red at `tests/it/recover.rs:781`);
graft-ops MAP `snapshot::worktree_tree`, `snapshot::take`, `restore::execute` (`RestoreWorktree`'s four-way split, the
roll-back), `undo::inverse`; PROMPT.md §2 point 2 ("Never lose work"); `docs/research/ux-principles.md` §2 H3, H5, §3.5.

## Rulings (planner, 2026-09-30; not open)

**R0 (rule 7, both sides): the cause is shared, or this ticket is wrong. Verify before building.**
- The finding is observed for a put-back only. "Any snapshot-backed `u` since M3" is the planner's reading of MAP
  (`worktree_tree` = `add -A`, i.e. git's clean filter; `RestoreWorktree` = `restore --source` / `checkout-index`, i.e.
  git's smudge), **not an observation**.
- So 30q-a's first act, before any other code, is test A0 (below) on today's code: a plain discard, then `u`.
  - **Red** as expected → build on.
  - **Green** → stop. Report `blocked: a plain u holds LF under autocrlf; the cause is put-back-specific`, with the test
    body in NOTES. The design below assumes a shared cause and does not hold without it.

**R1: fix the snapshot. Do not refuse, do not accept.**
- **Why:**
  - PROMPT.md §2.2, the M3 promise: every destructive operation can be undone. His bytes are his work. `core.autocrlf` is
    the mild case: the lines come back and the endings change. A **one-way clean filter** is the real case. nbstripout,
    for instance, strips a notebook's outputs on clean and smudges with `cat`. Today, `u` over that notebook gives back
    a notebook with no outputs. That is content lost, and "accept and document" would ship it.
  - ux-principles §3.5: Graft asks no confirmation for discard, switch or reset **because** `u` is reliable. A `u` that
    silently changes bytes removes the reason for skipping each of those confirmations.
- **Rejected: refuse in `check`.** Detection would be the same test as R2's, so it costs no less. In an autocrlf repo,
  though, the refusal fires on every discard, switch, stash or rebase over a changed LF file. A teammate's
  Windows-shaped repo would block his daily tasks (H3, user control). The only remedy is a trip to the shell to convert
  endings, which is a "Times I left" row by construction.
- **Rejected: accept and document.** Above: a one-way filter makes this loss, not cosmetics. git's own `stash` has the
  same property. That is git's floor. Our promise is stronger than git's.

**R2: what counts as his work, and so what the snapshot keeps raw.**
- A path's bytes are kept raw when three things hold:
  1. The path differs between `HEAD`'s tree and the snapshot's worktree tree. That is uncommitted work: added,
     modified or type-changed, never deleted.
  2. git would convert it: `check-attr` gives `text`, `eol`, `crlf`, `filter`, `ident` or `working-tree-encoding` a
     value other than unspecified or unset, **or** `core.autocrlf` is `true` or `input` and `text` is not unset.
  3. Its bytes on disk differ from what git's smudge makes of the cleaned blob (`<worktree_tree>:<path>` through
     `cat-file --filters`). Symlinks and gitlinks are never kept: git filters neither.
- **A path that matches `HEAD` is restored as git's own checkout writes it.** Its content is in history, and git
  defines that file as the blob smudged (an LF file committed under `autocrlf=true` gets git's own warning, "LF will be
  replaced by CRLF the next time Git touches it"). This bound keeps the cost on the changed paths, usually a handful,
  never the repo: an LFS or autocrlf repo would otherwise smudge every file on every operation.
- **Cost, stated for the conductor's perf run (30q-c):**
  - Clean worktree: +1 spawn (`diff-tree`).
  - Changed paths with no conversion: +3 spawns (`diff-tree`, `config`, `check-attr`).
  - Conversion candidates: +1 `cat-file --batch --filters` and the raw reads of those files only. A mismatch adds
    `hash-object -w --no-filters` and one `write-tree` on a temp index.

**R3: the layout, additive, so every ref and journal line written since M3 reads as before.**
- **Backup commit:** `{ index/ | unmerged/, worktree/, raw/ }`. `raw/` is present **only** when non-empty, and it holds
  only the kept paths, at their own paths and modes. `worktree/` is unchanged: still git's cleaned tree, so drift,
  `replaced`, `lost_edits`, the preview's diffs and J all read what they read today.
- **Journal:** the opening line gains `worktree_raw_before: Option<Oid>` (the `raw/` tree), with the same serde rule as
  `merge_before` (`default`, `skip_serializing_if = Option::is_none`). An old line parses as `None`. A line with nothing
  kept renders byte-identical to today's.
- **Only `before` is kept.** Every `RestoreWorktree` writes a `worktree_before` tree:
  - `u` writes the operation's own;
  - redo writes the `Undo` entry's own;
  - the roll-back writes the restore's own.
  An after-snapshot never needs it. **This is unverified**: 30q-b's first grep checks it (below).
- **Hard to reverse**, so it gets an ADR (30q-a): `docs/adr/0052-the-snapshot-keeps-bytes-git-would-not-give-back.md`,
  written from R1-R3, ≤ 1 page. Check the number is free first.

**R4: restore writes the raw bytes after git writes the path.**
- `RestoreWorktree` runs its four-way split unchanged. Then, for each path in its `paths` that the step's `raw` tree
  holds, it writes that blob's bytes.
  - One `cat-file --batch` feeds them.
  - Each write goes to a temp file in the same directory, then `rename`, so a crash leaves git's version or his, never
    half of either.
  - The mode comes from the `raw/` entry (`100755` sets the exec bit).
- The index is not touched. The path reads stat-dirty until git refreshes it, and clean after, because
  clean(raw) is the tree's blob.

**Out of scope. Each is a KNOWN_GAPS row for the conductor, every one marked unverified:**
- **J's put-back (Recover `Files`) forward.** It is `restore --source=<backup_tree>`, a git invocation in a plan
  (`Vec<Invocation>`), and it cannot write raw bytes. So an LF lost edit under autocrlf likely comes back CRLF on the
  put-back itself. Its `u` is exact after this ticket.
- **The set-aside put-back** (`stash apply`, ADR-0045 §4). git's `stash` smudges, the same as git's own `--autostash`.
  The rebase's or switch's `u` is exact after this ticket. The forward put-back is not.
- **Drift is EOL-blind.** An endings-only edit after an operation is not drift (it compares cleaned trees), so `u`
  writes over it unasked.
- **`core.safecrlf=true`.** The snapshot's `add -A` may refuse an LF file, and then every operation over it fails at
  the snapshot. This is a question.
- **`chain_backup` links** (a resolver step inside a stopped operation) keep no `raw/`. Nothing restores a worktree from
  them today (`d` restores stages).

## Part 30q-a: the before-snapshot keeps raw bytes (R0, R2, R3)

### Files (exact; outside this list → stop, `blocked: needs <file>`)
- **graft-core:** the file defining `JournalEntry` (grep `merge_before` in `crates/graft-core/src`), its tests, `MAP.md`.
- **graft-ops:**
  - `src/snapshot.rs` (`Snapshot`, `take`); new `src/snapshot/raw.rs`; `src/snapshot/plumbing.rs` only for a temp
    index it already builds.
  - The journal line types and their parse/render tests (grep `PendingLine` and `merge_before`).
  - `src/apply/record.rs` (`begin` records it).
  - The restore file whose `begin` takes the undo's own snapshot (grep `snapshot::take` under `src/restore*`).
  - New `tests/it/snapshot_raw.rs` and its registration line.
  - `tests/it/undo_bytes.rs` (new) for A0 only, **uncommitted if red** (see A0).
  - `MAP.md` (`snapshot`: `Snapshot`, `raw_record`, the layout line; `journal`: the field).
- **Docs:** new ADR (R3).

### Signatures
```rust
// graft-core — JournalEntry gains one field (and so does graft-ops' PendingLine), serde as merge_before.
pub worktree_raw_before: Option<Oid>,

// graft-ops::snapshot
pub struct Snapshot { /* index, worktree_tree, commit as today */ pub worktree_raw: Option<Oid> }

// graft-ops::snapshot::raw (re-exported from snapshot, as every public item there is)
/// R2: the paths of `worktree_tree` that are uncommitted, that git would convert, and whose bytes on disk git's smudge
/// would not give back, as a tree of their raw bytes (`hash-object -w --no-filters`). `None` when there are none.
/// Leading parameters: exactly `worktree_tree`'s own (repo, git runner, stop), copied, not re-invented.
pub fn raw_record(/* worktree_tree's leading params */, head: Option<Oid>, worktree_tree: Oid)
    -> Result<Option<Oid>, SnapshotError>;
// `take` calls it after `worktree_tree` and mktrees `raw/` beside `worktree/` when Some. `commit_trees` is unchanged.
// read_state_now (drift) never calls it.
```

### Acceptance (name the verifying test at tick time; **(red)** = seen failing on today's code first)
- [x] (red seen, both: `undo_bytes::u_after_a_discard_brings_back_an_lf_file_under_autocrlf` at `undo_bytes.rs:44:5`, `undo_bytes::u_after_a_stash_brings_back_an_lf_file_under_autocrlf` at `undo_bytes.rs:64:5`; file uncommitted, NOTES 30q-a)
  **A0, R0's premise, first, before any code** (red expected): `undo_bytes::u_after_a_discard_brings_back_an_lf_file_under_autocrlf`
  (graft-ops, real git, through the same `GraftOps`/`apply`/`undo` path the recover tests use).
  - Setup: `core.autocrlf=true`. A committed `src/a.rs`, then `W` written with LF endings only.
  - Run: discard `src/a.rs`, then `u`.
  - Assert: `std::fs::read` is `W`, byte for byte.
  - Red → NOTES `red: <line>`. Leave the file **uncommitted**; 30q-b commits it green. Green → R0's stop.
  - Also `u_after_a_stash_brings_back_an_lf_file_under_autocrlf`: the same, with a stash push instead of the discard,
    so two ordinary kinds show it. Same rule.
- [ ] (red, by compile) `snapshot_raw::a_snapshot_with_nothing_to_convert_keeps_nothing_raw`. Plain repo, no
  attributes, `autocrlf` unset, two changed files:
  - `worktree_raw == None`;
  - `git ls-tree <commit>` lists exactly `index` and `worktree`;
  - the pending line has no `worktree_raw_before` key.
- [ ] (red) `snapshot_raw::a_snapshot_keeps_raw_only_what_would_not_round_trip`. `core.autocrlf=true`, with:
  - `c.rs`: changed, CRLF;
  - `l.rs`: changed, LF;
  - `run.sh`: changed, LF, `0o755`;
  - `old.rs`: committed from LF and unchanged;
  - `link`: a changed symlink.
  Assert `git ls-tree -r <raw>` lists exactly `l.rs` (`100644`) and `run.sh` (`100755`). `git cat-file blob <raw>:l.rs`
  is `l.rs`'s bytes on disk. The pending line's `worktree_raw_before` is that tree.
- [ ] (red) `snapshot_raw::a_one_way_clean_filter_is_kept_raw`. `.git/info/attributes` `nb.txt filter=strip`, with
  `filter.strip.clean = sed '/^#out/d'` and `smudge = cat`. `nb.txt` is changed and holds a `#out 42` line. The raw
  tree holds `nb.txt` with that line.
- [ ] (red) `snapshot_raw::autocrlf_input_keeps_a_crlf_file_raw`: `core.autocrlf=input`, a changed CRLF file → kept.
- [ ] `snapshot_raw::an_inverse_filter_keeps_nothing`: 30p-a test 7's `rot` filter, a changed file → `None`.
- [ ] graft-core/graft-ops journal: `a_line_without_raw_parses_as_none_and_renders_unchanged`: an existing golden
  pending line round-trips byte-identical. Also `a_line_with_raw_round_trips`.
- [ ] ADR-0052 written. MAP.md lines as listed. Gates workspace-wide.
- [x] (NOTES 30q-a "Spawn sites", each line-cited) NOTES: the spawn sites `raw_record` adds, each by file:line, against R2's cost list (a code claim, line-cited).

### Touches (check each; say how on the **Touches:** line)
- **`Snapshot` constructors and patterns**, and **`take`'s callers**: begin, restore's begin, `chain_backup` (which
  keeps no `raw/`, R3 out of scope), recovery. Grep `Snapshot {` and `take(` in graft-ops, and in graft by grep.
- **Everything that reads a backup commit's root tree** (grep `ls-tree`, `"worktree"`, `"index"`, `"unmerged"`,
  `recorded_index_state`, `backup_chain`): a `raw/` sibling must not change its answer. Name each and why it holds.
- **Journal readers** in graft (J, `lost_edits`, the browser): the field is optional. Every golden journal line in
  tests stays byte-identical (grep `worktree_before` in `tests/`).
- **Perf:** every undoable operation now runs `raw_record`. The measurement is 30q-c's; say nothing about time.
- **UI, keymap, captures: none.** No word changes.

## Part 30q-b: `RestoreWorktree` writes them (R3 "only before", R4)

### Files (exact)
- **graft-ops:**
  - `src/undo*` (`Step::RestoreWorktree`, `inverse`).
  - `src/restore/steps.rs` (the write) and `src/restore/rollback.rs` (`reverse_of` carries `raw`).
  - `tests/it/recover.rs`: test 6 restored.
  - `tests/it/undo_bytes.rs`: A0 committed, plus the tests below.
  - `MAP.md` (`undo::Step`, `restore::execute`).
- **graft:** only a pattern on `Step::RestoreWorktree { tree, paths }`, if one exists (grep). Add `..`, nothing else.

### Signatures
```rust
// graft-ops::undo
Step::RestoreWorktree { tree: Oid, paths: PathScope, raw: Option<Oid> }
// `inverse` sets `raw` from the same entry whose `worktree_before` it sets as `tree`. Never from another entry.
```

### First, before any code (R3's unverified claim)
- Grep every construction of `Step::RestoreWorktree` (and `reverse_of`'s). List each with its `tree` source, by
  file:line, in NOTES.
- All `worktree_before` → go on.
- Any `worktree_after` or other tree → stop. Report `blocked: RestoreWorktree at <file:line> writes an after-tree; R3's
  only-before does not hold`.

### Acceptance (red = seen failing before the write lands)
- [ ] (red) `recover::u_brings_back_an_lf_file_under_autocrlf`, 30p-a's test 6, **restored verbatim** at the marker
  comment (`tests/it/recover.rs:796`):
  ```rust
  /// 6. `W` with LF endings only under `core.autocrlf=true`.
  #[test]
  fn u_brings_back_an_lf_file_under_autocrlf() {
      u_under_autocrlf_brings_back(WORKING);
  }
  ```
  Green after the change, with `put_back_then_u`'s index asserts unchanged. Remove the marker comment.
- [ ] (red) A0's two tests, green and committed.
- [ ] (red) `undo_bytes::u_brings_back_an_lf_executable_under_autocrlf`: `run.sh` holds `#!/bin/sh\n…`, LF, `0o755`,
  and is changed. Discard it, then `u`. Its bytes are exact and the exec bit is set.
- [ ] (red) `undo_bytes::u_brings_back_what_a_one_way_clean_filter_strips`: part a's `strip` filter, `nb.txt` with its
  `#out 42` line. Discard it, then `u`. The bytes are exact and hold `#out 42`.
- [ ] (red) `undo_bytes::u_brings_back_a_crlf_file_under_autocrlf_input`.
- [ ] `undo_bytes::u_brings_back_mixed_endings_under_autocrlf`: one CRLF line and one LF line. Expected red. If it is
  green on today's code, say so in NOTES and keep it as a guard.
- [ ] (red) `undo_bytes::redo_after_a_forced_undo_brings_back_an_lf_drift_under_autocrlf`:
  1. An operation.
  2. His LF edit over a path it wrote (drift).
  3. A forced `u` (it replaces the edit).
  4. Redo. The redo restores the `Undo` entry's own before.
  After redo, the path is his LF bytes.
- [ ] Untracked arm, **if** a graft-ops operation removes an untracked file that `u` re-creates (grep: stash push with
  untracked, or the put-back's `go`): `u_brings_back_an_lf_untracked_file_under_autocrlf`. Else say which grep found
  none.
- [ ] **Unchanged, green in the full run:**
  - 30p-a's seven;
  - `u_after_a_file_put_back_brings_back_what_it_replaced`;
  - every `tests/it/` test naming `RestoreWorktree`, `undo` or `redo`;
  - the M3 walks. Grep `m3_walk` or `m3` under graft `tests/it`, and name them.
- [ ] Mutation: make the write skip the exec bit → `u_brings_back_an_lf_executable_under_autocrlf` fails. Restore it and
  say so.
- [ ] MAP.md. Gates workspace-wide.

### Touches (check each; say how)
- **Every snapshot-backed `u`** (discard, stash push/apply/pop, checkout, reset `--hard`, merge, pull, rebase, rewrite,
  cherry-pick, revert, put-back): all reach the one `RestoreWorktree` executor. Show it with a grep: one executor arm,
  no other writer of a snapshot tree to disk in `src/restore*`.
- **Roll-back** of a cancelled or failed undo: `reverse_of` carries `raw` from the restore's own before. Existing
  cancel/fail tests stay green; name them.
- **Drift and `replaced`:** unchanged code. They read cleaned trees (R3).
- **The M3 walks and captures:** no screen or word changes, so every `docs/shots/` file stays byte-identical. Check with
  `git status docs/shots` after the full run.
- **Journal/undo semantics:** no new operation, kind or touched field.

## Part 30q-c: the snapshot's cost (R2's cost, measured by the conductor)

### Files (exact)
- An existing snapshot-cost example or script, if one exists: grep `worktree_tree` under `crates/*/examples` and
  `scripts/` (the m3/03 15 ms figure came from somewhere). Extend it. Otherwise create new
  `crates/graft-ops/examples/snapshot_cost.rs`.
- New `scripts/m7-30q-perf.sh`, modelled on `scripts/m7-30p-perf.sh`.
- graft-ops `MAP.md` (the example).

### Scope
- Three cases, each on a 5 000-file fixture (graft-fixtures' smallest `Scale` at or above that; name it), 20 runs.
  Report `take`'s p50 and p99 and `raw_record`'s alone:
  1. no conversion, 5 changed files;
  2. no conversion, clean worktree;
  3. `core.autocrlf=true`, 50 changed LF files.
- The script: a release build, then one run writing `docs/perf/30q-snapshot.txt` (or `$1`). It writes a header (date,
  HEAD, machine, git), a `SUMMARY case<k> …` line per case, then `SUMMARY raw-record gate(case1 p50<=10ms)=ok|OVER`.
  `$HOME` is replaced with `~`.
- The builder runs the example **once in debug** as a smoke test, to scratch. It reads only the exit line and reports
  no numbers. The release run is the conductor's.

### Acceptance
- [ ] Example and script exist, and the smoke run exits 0 (NOTES).
- [ ] Clippy with `--all-targets` is green (it compiles the example). Gates workspace-wide.
- [ ] On OVER: the conductor files the follow-up. The gate is not relaxed in this ticket.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run` and `cargo test --doc`, all
workspace-wide.

## Size
- **30q-a** ~50k.
- **30q-b** ~50k.
- **30q-c** ~20k.
If any part compacts, say so; the conductor splits the rest.

## Rules
- Worktree only. `CARGO_TARGET_DIR` in the worktree.
- Never read `PROGRESS.md` or `KNOWN_GAPS.md`.
- No perf or release run in a builder's context.
- KNOWN_GAPS is the conductor's:
  - `M7/30p-crlf` closes on 30q-b green;
  - R1's five out-of-scope rows are added as **unverified**.

## NOTES (builder)

### 30q-a (2026-09-30, builder Opus xhigh; no scout: the ticket names every site, re-grepped by function)

**Partial. `blocked: needs` 22 files outside 30q-a's list** to add `JournalEntry.worktree_raw_before` (list below),
one of them the debugger's. Committed at the seam: `snapshot::raw_record` (R2) with its tests, ADR-0052, MAP. Not
built: the field, `Snapshot.worktree_raw`, `take` writing `raw/`, `begin` recording it. No compaction.

**R0: red, both kinds → the cause is shared; built on.** On 8c3f4b1b, `core.autocrlf=true`, committed
`src/a.rs`, then `W` = `pub fn a() -> u32 {\n    4\n}\n`:
- discard, then `u`: red `crates/graft-ops/tests/it/undo_bytes.rs:44:5`; after `u` the file is `…{\r\n    4\r\n}\r\n`.
- stash push (`All { include_untracked: false }`), then `u`: red `undo_bytes.rs:64:5`, the same CRLF bytes.
- Before `u`, both assert the operation wrote over `W` (passed). Through `support::run` / `run_undo` (`apply`,
  `undo::undo`, unforced).
- The file is **uncommitted** in this worktree (`crates/graft-ops/tests/it/undo_bytes.rs`); its `mod undo_bytes;` line
  was taken back out of `tests/it/main.rs` so the committed gates are green. 30q-b re-adds both. If the worktree is
  gone, the body in short (imports `graft_ops::operation::{Operation, StashParams, StashScope, Target}`,
  `crate::support::{NOW, TempRepo, run, run_undo}`):
  ```rust
  const COMMITTED: &[u8] = b"pub fn a() -> u32 {\n    1\n}\n";
  const WORKING: &[u8] = b"pub fn a() -> u32 {\n    4\n}\n";
  fn lf_edit_under_autocrlf() -> TempRepo {
      let repo = TempRepo::init();
      repo.git(&["config", "core.autocrlf", "true"]);
      repo.write("src/a.rs", COMMITTED);
      repo.commit_all("first");
      repo.write("src/a.rs", WORKING);
      repo
  }
  // each test: repo = lf_edit_under_autocrlf(); run(&repo, &op, NOW);
  // assert_ne!(repo.read("src/a.rs"), WORKING); run_undo(&repo, NOW + 1, false).unwrap_or_else(..);
  // assert_eq!(repo.read("src/a.rs"), WORKING, "W is back, byte for byte");
  // op: Operation::Discard(Target::Paths(vec![PathBuf::from("src/a.rs")])), or
  //     Operation::StashPush(StashParams { message: Some("wip".to_owned()),
  //                          scope: StashScope::All { include_untracked: false } })
  ```

**The block (rule: outside the list → stop).** Adding the field fails E0063 (exhaustive struct literals) at, by
`cargo check --workspace --all-targets --keep-going` with the field in place:
- graft-ops, outside the list: `src/apply/fold.rs:151`; `src/journal_list/tests.rs:65` (**the debugger's**).
- graft (19 files, all test code): `tests/it/undo_by_key.rs:217`, `tests/it/conflict_interim.rs:973`,
  `src/view/tests.rs:2660`, `src/view/journal/tests.rs:14`, `src/update/undo.rs:885,1538`, `src/update/tests.rs:5461`,
  `src/update/switch/tests.rs:351`, `src/update/rewrite_editor/tests.rs:1051`,
  `src/update/resolver/tests/words_and_c.rs:22`, `src/update/remote/tests/lease.rs:237`,
  `src/update/remote/tests.rs:897`, `src/update/operate/tests.rs:20`, `src/update/journal/tests.rs:22`,
  `src/update/journal/find/tests.rs:24`, `src/update/clash/tests.rs:37`, `src/update/banner/tests.rs:57`,
  `src/update/amend_into/tests.rs:282`, `src/jobs/undo.rs:715`, `src/jobs/operate/tests.rs:13,57`.
- In the list (fine): `journal/lines.rs:150`, `apply/record.rs:111`, `journal/tests.rs:29,63`.
- Each fix is one line, `worktree_raw_before: None,`. graft-ops `tests/it` has no exhaustive literal (0 errors).
- Also outside: `take` has no `HEAD` today, and R2's `raw_record` takes it. Recommended: `take` gains
  `head: Option<Oid>`; `begin` derives it with no spawn (`head_before`: `Detached(o)` → `Some(o)`, `Unborn` → `None`,
  `Branch(n)` → `facts::read_refs(repo, &[n])`, in-process). Its one other caller, `tests/it/unmerged_index.rs:68`, is
  outside the list (22nd file). Reading `HEAD` inside `take` instead needs a new `SnapshotError` variant.

**Remaining (the continuation, exact):**
1. `graft-core` `JournalEntry.worktree_raw_before` (after `worktree_after`, serde as `merge_before`), the 22 literal
   lines above, `PendingLine` field + `of` + `into_entry` (`journal/lines.rs`).
2. `Snapshot.worktree_raw: Option<Oid>`; `take(.., head, ..)` calls `raw::raw_record(repo, scratch, stop, head,
   worktree_tree)` after `both_trees`; when `Some`, joins `{ index/|unmerged/, raw/, worktree/ }`. `plumbing::mktree`
   lists only index and worktree: either it gains `raw: Option<Oid>` (plumbing.rs is listed "only for a temp index", so
   say so) or `take` gets its own three-entry join. `commit_trees` unchanged.
3. `begin` (`apply/record.rs:111`): `worktree_raw_before: backup.and_then(|taken| taken.worktree_raw)`.
4. Tests still owed from the boxes: `ls-tree <commit>` lists exactly `index worktree` (none kept) / `index raw
   worktree`; the pending line has no `worktree_raw_before` key / holds the raw tree; the two journal round-trip tests.
   `snapshot_raw.rs`'s `raw_of` then goes through `take` (or `apply`) for those.

**Spec deviation, stated (R2's mechanism, not its rule).** R2 planned one `cat-file --batch --filters`. Seen on git
2.54 (a probe repo, `core.autocrlf=true`, blob `a\nb\n`): input `<blob> <path>\n` prints header `<oid> blob 4` then
6 bytes `a\r\nb\r\n` — the header size is **pre-filter**, so records cannot be split. With `-z` (git 2.38+), two records
gave one output (seen twice, in either order). `<tree>:<path>` input splits at the first space (`missing`). So question 3 checks out the
cleaned blobs with `checkout-index -a -f --prefix=<scratch>/` from a fresh temp index (the same conversion a restore
runs) and compares streamed. The ADR says so.

**Spawn sites `raw_record` adds** (`crates/graft-ops/src/snapshot/raw.rs`), against R2's list:
- Always (changed or not): `diff-tree -r -z --no-renames` `:114`; unborn `HEAD` instead `ls-tree -r -z` `:125`. R2: +1. ✓
- Changed regular files: `config --get core.autocrlf` `:191`, `check-attr -z --stdin` `:229`. R2: +3 total. ✓
- Conversion candidates: `update-index -z --index-info` (`fill_index` `:378`, called `:299`) and `checkout-index`
  `:308`, plus a scratch copy of those files. R2 said +1 (`cat-file`): **+2**.
- Mismatches: `hash-object -w --no-filters` `:335`, `update-index` (`:357` → `:378`), `write-tree` `:358`. R2 said
  +2 (it left out the `update-index` a temp index needs): **+3**.
- Worst case 8 spawns (R2: 6). Nothing calls `raw_record` yet, so no operation pays any of it today.

**Tests (new, all green):** `snapshot_raw::` `a_snapshot_with_nothing_to_convert_keeps_nothing_raw`,
`a_snapshot_keeps_raw_only_what_would_not_round_trip`, `a_one_way_clean_filter_is_kept_raw`,
`autocrlf_input_keeps_a_crlf_file_raw`, `an_inverse_filter_keeps_nothing`, `an_unborn_head_keeps_an_lf_file_raw`,
`a_path_with_a_newline_in_it_is_kept_raw`, `a_handle_opened_in_a_subdirectory_keeps_the_same_path` (each also asserts
the scratch dir is left empty); unit `snapshot::raw::tests::` six (check-attr rule, torn triple, diff-tree parse).
They assert the raw tree (`ls-tree -r`, `cat-file blob`), not the commit layout or the pending line (item 4).
Mutation: question 3 skipped (`if true ||` before `same_bytes`) → 3 red (`keeps_raw_only…`, `unborn_head…`,
`inverse_filter…`); restored.

**Touches:** `snapshot` gains one re-export (`raw_record`) and a doc line; `Snapshot`, `take`, `commit_trees`,
`mktree`, `plumbing.rs` unchanged (`git diff` on `src/`: `snapshot.rs` mod/re-export/doc only, new `snapshot/raw.rs`).
No caller of `raw_record` (grep `raw_record` in `crates/`: raw.rs, snapshot.rs, tests), so no backup commit, journal
line, drift read or `u` changes behaviour, and no operation pays its spawns. UI, keymap, captures: none.
