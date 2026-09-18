# M3 design pass: working tree, undo and stash (2026-09-19)

Read: CLAUDE.md; ADR-0029/0030/0031/0033 (+0002, 0009, 0010); PROGRESS.md "M3 design
pass brief"; PROMPT.md §2–§4 (Working changes, Committing, Stash), §6.1, §6.3, §7.2,
§8 M3, Technology; docs/design 01 §1.3/1.4/1.10/1.11/1.13, 02 §2.2/2.3/2.4/2.9–2.11,
06 whole, 08 §8.2–8.4; every `crates/*/MAP.md`; KNOWN_GAPS M3 rows; docs/log/m2.md;
M2 review "Does the promise hold". No source.

**Promise:** *you cannot lose work* — every write M3 makes is journaled with a backup
and comes back byte-for-byte on `u`; measured by "Times I left" (ADR-0033).
A usable day (ADR-0030): edit in Zed → switch to Graft → stage files/hunks → commit →
amend → stash/pop → discard a wrong edit → `u` when anything of that was a mistake.

**Sizing from M2 (mean 118 turns, 22 compactions).** Stated causes: builders read
source instead of scouting (10b); clippy `too_many_lines` splitting rounds (18);
tests + integration + view + capture in one ticket (06b, 07); root-causing git
behaviour by trial (05, 1c–1f: the index-write and rename/copy chain). Applied here:
groups are seams-first (ops before UI), every group names its new files and the
functions in them (no 60-line surprises), git behaviour claims are listed as **verify
in a throwaway repo before coding** items, captures come from a script, and each
group has a named stop seam.

## Interfaces and data shapes

**Seam rule (brief point 2): `ops` decides, `app` wires.** `app` never builds a
`GitInvocation` for a write, never touches the journal file, never chooses a
snapshot. A key becomes `Command::Operate(Operation)`; the job runner calls
`graft_ops::apply`; the answer is `Product::Operated(Result<Applied, ApplyError>)`;
`update` turns it into a refresh + a status line. Guard test: `GitInvocation` appears
under `crates/graft/src/` only inside `jobs/` (reads). ARCHITECTURE.md gets the rule.

**Four properties (brief point 1)** — one enum, four methods, one arm per kind:

```rust
// graft-ops/src/operation.rs
pub enum Operation { Stage(Target), Unstage(Target), Discard(Target), Commit(CommitParams),
    Amend(CommitParams), StashPush(StashParams),
    StashApply { index: usize, commit: Oid }, StashPop { index: usize, commit: Oid }, StashDrop { index: usize, commit: Oid } }
pub enum Target { Paths(Vec<PathBuf>), Patch { path: PathBuf, patch: Vec<u8> } }   // Patch = one hunk or selected lines
pub struct CommitParams { pub message: String, pub sign_off: bool, pub no_verify: bool, pub allow_empty: bool }
pub struct StashParams { pub message: Option<String>, pub include_untracked: bool, pub staged_only: bool }
impl Operation {
    pub fn kind(&self) -> OperationKind;
    pub fn describe(&self) -> String;                                   // journal + browser text
    pub fn check(&self, facts: &RepoFacts) -> Result<(), Refusal>;       // 1. preconditions
    pub fn plan(&self, facts: &RepoFacts) -> Vec<Invocation>;            // 2. typed git, never a string
    pub fn touches(&self, facts: &RepoFacts) -> Touches;                 // 3. refs/index/worktree → snapshot scope
}
pub fn inverse(entry: &JournalEntry, original: Option<&JournalEntry>) -> Result<InversePlan, NotInvertible>; // 4. undo.rs
```

**Journal entry (core; satisfies ADR-0031):** before/after for refs, HEAD, index tree,
worktree tree, per entry, timestamped. Point-in-time = replay backwards; `state_at`
proves the shape can answer, no key exposes it (M5).

```rust
// graft-core/src/journal.rs   (serde derives; core stays plain data)
pub struct EntryId(String);                                  // ULID: sortable, ref-name safe
pub enum OperationKind { Stage, Unstage, Discard, Commit, Amend, StashPush, StashApply, StashPop, StashDrop, Undo, Redo }
pub enum EntryStatus { Pending, Committed, Aborted { reason: String } }
pub struct Touches { pub refs: Vec<String>, pub index: bool, pub worktree: bool }
pub enum HeadRef { Branch(String), Detached(Oid), Unborn(String) }
pub struct JournalEntry {
    pub id: EntryId, pub at_unix_ms: i64, pub kind: OperationKind, pub description: String,
    pub status: EntryStatus, pub touches: Touches,
    pub head_before: HeadRef, pub head_after: Option<HeadRef>,
    pub refs_before: BTreeMap<String, Option<Oid>>, pub refs_after: BTreeMap<String, Option<Oid>>,
    pub index_before: Option<Oid>, pub index_after: Option<Oid>,        // tree ids
    pub worktree_before: Option<Oid>, pub worktree_after: Option<Oid>,  // tree ids: tracked (worktree content) + untracked
    pub backup_ref: Option<String>,   // refs/graft/backup/<worktree>/<ulid> → after-commit → before-commit → pins
    pub pins: Vec<Oid>,               // commits the inverse needs alive (dropped/popped stash)
    pub commands: Vec<String>,        // the Command Log (PROMPT principle 5)
    pub reverses: Option<EntryId>,
}
```

Backup commit tree is `{ index/ → index tree, worktree/ → worktree tree }`, so both
trees stay reachable and restore reads each by name. Undo = generic restore of the
before-state (refs by compare-and-swap, index by `update-index --index-info`,
worktree path-scoped to what the op changed) plus kind-specific stash-reflog steps.

`graft-git` additions: `status::read` (porcelain v2, one spawn; `git status` is the
one command that honours `GIT_OPTIONAL_LOCKS=0` — M2 review R2 Major 1), `diff::patch::
hunk_patch` (pure), `DiffSpec::{Staged, Unstaged}`, `stash::list`, `state::in_progress`,
`RepoHandle::{git_path, worktree_name}`, `GitInvocation::env`. Specs have the exact
signatures.

## Hard-to-reverse choices

1. **Inverse before the first key.** Options: (a) commit first, undo after; (b) model +
   inverse + execution proven on fixtures, then any UI write. **(b)**: the window where
   Graft writes history and can't take it back is exactly what the user would be
   testing in; ops-model → ops-execution are groups 1–3, no key is wired before them.
2. **State-based journal, not replay-based.** Options: (a) store op params, undo/redo by
   re-running; (b) store before/after refs+HEAD+index tree+worktree tree, undo/redo by
   restoring. **(b)**: one restore path serves undo, redo, aborted-mid-way recovery and
   ADR-0031's time query; a re-run can't reproduce a failed `stash pop`'s half state.
   Cost: a worktree-touching op pays two snapshots (before and after); index/ref-only
   ops reuse the before tree as after.
3. **Snapshot scope follows `touches`, but stays unconditional for now.** ADR-0009
   stands: every op touching index or worktree takes the full before-snapshot. The
   scope is one function (`snapshot::scope(&Touches)`), measured on the heavy fixture at
   review (`docs/perf/m3-snapshot.txt`, KNOWN_GAPS R2-3). If stage/unstage lag, the ADR
   narrowing index-only ops to an index-tree snapshot is a one-line flip, proposed with
   numbers, never silent.
4. **Every M3 overlay replaces the graph area** (the Map view's arm); the detail pane,
   if shown, stays. One layout rule for commit editor, stash list, journal browser at
   all three breakpoints; confirmations are a small centred box.

## Where I'd differ from the design docs (Proposed changes)

1. **§6.3 snapshot misses unstaged edits to tracked files.** "Index tree + untracked
   blobs" loses an unstaged modification on discard→undo. Built: worktree tree = real
   index copied to a temp `GIT_INDEX_FILE` (stat cache kept, so only dirty files hash)
   + `add -A` + `write-tree`; index tree separately. Strictly more; no trade-off but
   two trees per snapshot.
2. **Aborted entries with a backup are undoable.** §6.2 says "nothing needs undoing"; a
   `stash pop` that fails midway leaves a half-applied tree. `u` restores it.
3. **Commit key.** §2.10 `Ctrl+Enter` is indistinguishable from `Enter` outside the kitty
   keyboard protocol (Zed/VS Code). Added: `Tab` to a `[ Commit ]` button + `Enter`; the
   same caveat §2.2 records for `Ctrl+Shift+Z`. Ctrl+Enter still works where reported.
4. **Journal browser opens on `J`** (no opener in §2). In M3 `Enter` acts only on the
   entry `u` would undo next; older entries say "multi-step revert comes with Ghost
   Preview (M5)" (ADR-0031). `Space` expands an entry's Command Log lines.
5. **Stash reach:** no sidebar exists; `S` opens the stash overlay (§1.11's list), `s`
   inside it saves. "Branch from stash" → M5 (branches); "selected files" → M7.
6. **Pinned uncommitted row** (KNOWN_GAPS m2/06b): drawn as row 0 whenever the tree is
   dirty, wherever `HEAD` sits; attached to `HEAD`'s lane only when `HEAD` is topmost.
7. **Refresh on focus-gained** (Technology "Auto-refresh"): status re-read when the
   terminal regains focus; `notify` watcher deferred (M4), stated in KNOWN_GAPS.
8. **Hunk staging inside untracked files** (PROMPT §4) needs `add -N`; deferred to M7,
   whole-file stage only. `i` (.gitignore) deferred to M7. Both KNOWN_GAPS.
9. **Commit/amend refused while a merge/rebase/cherry-pick is in progress** — finishing
   one is M5/M6's; refusal names the shell command. Honest interim, KNOWN_GAPS.

## New UI

Common: overlays replace the graph area (choice 4). Every field-relevant fact is in
text, never colour only (captures carry no colour). Empty/error states named per view.

**Changes pane** (extends M2's working-tree detail pane; §1.4 gives the wide layout).
Files region rows: `STAGED (2)`, `UNSTAGED (1)`, `UNTRACKED (1)` section rows, files
under each with `+A −R` (untracked: `new`). Body: staged diff for a STAGED file,
unstaged diff otherwise, untracked shows the file's lines as added. Diff region gets a
line cursor (`▎` in the sign column, bold row): `Space` on a hunk heading stages/unstages
the hunk, on a `+`/`−` line that line, on context the enclosing hunk; `d` = discard
the same unit (confirm first). Section membership decides direction: STAGED → unstage,
else stage. `Ctrl+A` stage all; `c`/`a` open the editor. Eye lands on the section
counts, then the cursor row. Breakpoints as M2 (right / below / tab). Empty: `nothing
to commit — working tree clean`; a failed status read shows git's first stderr line.

**Confirm box** (`graft-ui::confirm`): centred, 7 rows × min(72, width−4): line 1 the
question in plain words (`Discard changes to src/a.rs?`), line 2 consequence (`12 added
/ 4 removed lines in the working tree; a backup is saved first, u undoes it`), line 3
the exact command (`git restore --worktree -- src/a.rs`), buttons `[ Keep ]  [ Discard ]`,
safe one selected. `←→`/`Tab` move, `Enter` activates, `Esc` cancels. Reused by stash
drop and the undo drift guard (`Since this commit, the index changed — undo anyway?`).

**Commit editor** (§2.10 keys + change 3). Rows: title `COMMIT ── ⎇ main ── 2 files
staged` (`AMEND a1b2c3d` when amending; `nothing staged` in the title when the commit
would be refused); `Subject` single line with `12/50` counter and a `┊` guide glyph at
column 51; `Body` multi-line, `┊` at 73, `Co-authored-by:` trailer inserted by
`Ctrl+Shift+C`; options row `[ ] amend  [ ] sign-off  [ ] no-verify  [ ] allow empty
[ Commit ]`; footer `⌃⏎/Commit commit  ⇥ next  ⌃E $EDITOR  ⌃S sign-off  esc cancel`.
`commit.template` prefills the body. Enter in Subject moves to Body. Refusal shows on
the title row (`nothing staged`, `subject empty`) and the button is drawn `[ Commit ]`
dimmed with the reason beside it. While committing: title `committing…`, keys ignored,
`Esc` cancels the job. Hook stderr appears in the status line and stays till `Esc`.

**Stash overlay** (`S`; §1.11 layout). Header `STASHES (2)`; rows `stash@{0} "msg" 2h
ago 3 files`; below, the selected stash's file list + diff on `Space`. Keys: `s` save
(a 2-line prompt: message input; `Tab` toggles `all incl. untracked / staged only`;
`Enter` saves — on a clean tree the prompt refuses `nothing to stash`), `Enter` apply,
`p` pop, `d` drop (confirm), `Esc` close. Empty: `no stashes — s stashes the working
tree`. A refused apply/pop lists the files it would overwrite.

**Journal browser** (`J`; §1.10 layout). Rows: age, description, `refs · backup`
summary, and `← u` on the entry `u` would undo next, `← U` on the redo target; an
Aborted entry reads `failed: <reason> · backup saved`; an interrupted one `interrupted`.
`Space` toggles the entry's Command Log lines under it (`$ git commit -F -`). Empty:
`nothing yet — every write lands here and u takes it back`. On launch with an
unresolved Pending entry the graph footer says `an operation was interrupted: <desc> ·
J to inspect`.

**Status line:** `Status::Done("committed a1b2c3d")` clears on the next key; `Status::
Failed("commit: hook rejected — <first stderr line>")` stays until `Esc`. Footer hints
gain `c commit  S stash  u undo` (width permitting; `?` is M8).

## Genuinely risky groups

- **ops-model** (signature, Fable): the per-kind inverse, the in-force/next-undo
  cursor and the entry shape are the product. Meets all three tests by standing rule.
- **ops-execution** (Opus): correctness *is* judged by tests (byte-for-byte on
  fixtures, linked worktree, crash mid-op), so not Fable — but wrong git primitives
  become structural. Every git claim in its spec is marked "verify in a throwaway
  repo first"; the reviewer runs the heavy snapshot-cost script.
- **external `$EDITOR` / suspend-resume** (in commit-editor, Opus): terminal protocol;
  a wrong restore breaks principle 8. Judged by the acceptance walk's pty run.

Fable builds: ops-model (signature) + acceptance. No third; the design is not
underspecified for M3's scope.
