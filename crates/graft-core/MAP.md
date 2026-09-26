# graft-core — map
One line per public item. Update when a public item changes.

## capabilities
- `RenderingTier` (enum) — how Graft draws the commit graph.
- `PixelProtocol` (enum) — the image protocol the pixel tier would use.
- `Supported` (enum) — answer to one yes/no capability query, silence kept distinct.
- `CellSize` (struct) — one character cell in pixels, and where the number came from.
- `CellSizeSource` (enum) — reported, window size, or assumed.
- `Rgb` (struct) — an 8-bit-per-channel colour.
- `Theme` (enum) — dark or light.
- `ThemeHint` (struct) — light/dark hint, always usable, always carrying its source.
- `ThemeSource` (enum) — where a `ThemeHint` came from: background or assumed.
- `ColorDepth` (enum) — how much colour the terminal has: true colour down to none.
- `TmuxStatus` (enum) — whether Graft is inside tmux and the outer terminal is reachable.
- `ProbeOutcome` (enum) — what became of the probe round: answered, timed out, skipped.
- `Capabilities` (struct) — everything detection established about the terminal.

## commit
- `Timestamp` (struct) — when something happened, as git stores it.
- `Identity` (struct) — a name, an email and a moment.
- `Commit` (struct) — one commit: id, parents, author, committer, message.
- `Commit::date` / `subject` (fn) — committer date; first message line.

## commit_handle
- `CommitHandle` (struct) — a commit as a dense `u32` from its repository's id table; `index` (fn).
- `Parents` (enum) — parents by handle, inline for ≤ 2; `iter` / `len` / `is_empty` / `first` (fn), `FromIterator`, `Default` (`None`).
- `Link` (struct) — `commit` and its `parents`, by handle.
- `HandleSet` (struct) — bitset of handles; `new` / `insert` / `contains` / `remove` / `len` / `is_empty` / `approximate_bytes` (fn). `PartialEq`/`Eq` by member, not by representation (`remove` leaves its word behind).

## error
- `Error` (enum) — a value a core constructor cannot represent (`OidLength`, `OidHex`, `RefName`, `EntryId` (m3/02)).
- `RefNameRule` (enum) — the `check-ref-format` rule a rejected ref broke.

## event
- `Event` (enum) — something that happened and that update must answer.
- `KeyPress` (struct) — a key going down, with modifiers; `plain` / `control` (fn) build one.
- `KeyCode` (enum) — the keys the keymap can name.
- `Modifiers` (struct) — control, alt, shift, command; `NONE` / `CONTROL` (const).
- `MouseInput` (struct) — a mouse event positioned in cells.
- `MouseKind` (enum) — down, up, drag, move, scroll.
- `MouseButton` (enum) — left, middle, right.

## job
- `JobId` (struct) — names one background job; `new` / `raw` (fn).
- `JobFailure` (enum) — why a job produced no result: failed or panicked.

## journal
- `EntryId` (struct) — a journal entry's id, a ULID (26 Crockford base32 chars, time then random), `Ord` by string = by time; also the backup ref's last segment (ADR-0010); lives in `journal/entry_id.rs`, re-exported (m6-conflicts/41); `new(unix_ms, random)` / `parse` / `as_str` (fn); serde as its text (m3/02).
- `OperationKind` (enum) — `Stage`, `Unstage`, `Discard`, `Commit`, `Amend`, `StashPush`, `StashApply`, `StashPop`, `StashDrop`, `Fetch`, `Pull`, `Push` (m4/04; serde `fetch`/`pull`/`push`), `Merge`, `Rebase`, `Rewrite` (m7/02, serde `rewrite`; ADR-0047: a `rebase -i` rewrite, undo has no per-kind arm, so it restores the recorded refs as for a rebase — unverified until rewrite-engine-b's undo test), `CherryPick`, `Revert`, `Reset`, `Checkout`, `BranchCreate`, `BranchDelete`, `TagCreate`, `TagDelete`, `BranchFromStash`, `RevertTo` (m5/07; serde snake_case like the rest: `cherry_pick`, `branch_from_stash`, `revert_to`), `ResolveConflict`, `RestartConflict` (m6-conflicts/03; serde `resolve_conflict`, `restart_conflict`), `FinishMerge` (m6-conflicts/20; serde `finish_merge`: a merge begun in the shell, finished from Graft), `AbortMerge` (m6-conflicts/57; serde `abort_merge`: a merge begun in the shell, aborted from Graft), `Undo`, `Redo` (m3/02). **`RevertTo` is never written**: §6.6 journals the time machine as one `Undo` entry per step, so a summary entry would be the thing that decision rejected — see the variant's own doc (m5/07 job 2).
- `EntryStatus` (enum) — `Pending`, `Committed`, `Aborted { reason }` (m3/02).
- `INTERRUPTED` (const `&str`) — `"interrupted"`: the reason recovery records on an entry a dead process left open. One definition, because `ops` writes it and the journal browser matches on it (m3/06 review, Major 2).
- `JournalEntry::is_interrupted` (fn) — `(&self) -> bool`: `Pending` (what the file holds) or `Aborted` with `INTERRUPTED` (what a reader is handed, recovery having closed it first). Keying on `Pending` alone looks right and never fires.
- `Touches` (struct) — what an operation changes: `refs` (full names, or `HEAD` when detached), `index`, `worktree`; the snapshot scope and the generic inverse's steps (m3/02).
- `HeadRef` (enum) — `Branch(full name)`, `Detached(Oid)`, `Unborn(full name)` (m3/02).
- `JournalEntry` (struct) — one journaled operation, state-based (ADR-0031): `id`, `at_unix_ms`, `kind`, `description`, `status`, `touches`, `head_before`/`head_after`, `refs_before`/`refs_after` (`BTreeMap<String, Option<Oid>>`), `index_before`/`index_after` (`Option<IndexState>`), `worktree_before`/`worktree_after` (tree ids), `backup_ref`, `pins`, `commands`, `reverses`, `merge_before`/`merge_after` (`Option<MergeState>`, m6-conflicts/20: git's merge state on **every** entry; absent on an old line = `None`, and `skip_serializing_if` keeps a line with none byte-identical to before; also the bound KNOWN_GAPS M6/25's fix needs). Serde derives; the JSONL lines are `graft-ops::journal`'s (m3/02).
- `MergeState` (struct) — lives in `journal/merge_state.rs` with its private `text_bytes` serde module, re-exported so `graft_core::journal::MergeState` is unchanged (m6-conflicts/41). git's merge in progress, as files (m6-conflicts/20): `merge_head: Oid` (`MERGE_HEAD`'s first line), `other_heads: Vec<Oid>` (an octopus merge's further lines in order; empty otherwise, and then absent from the JSON so older lines read and write back unchanged — m6-conflicts/38), `merge_msg: Vec<u8>` and `merge_mode: Option<Vec<u8>>` (exact bytes; an empty `MERGE_MODE` is `Some(vec![])`). Bytes serialise as a JSON string when UTF-8, else `{"hex":"…"}` — exact both ways. `AUTO_MERGE` is left out (a tree; anchoring it would change the backup layout) — see its doc. `heads()` (every commit, `merge_head` first) and `merge_head_bytes()` (`MERGE_HEAD` as git writes it: one full hex id + `\n` per head; byte-exact for a git-written file, a hand-edited one comes back in git's form) (m6-conflicts/38).
- `PutBack` (struct) / `PutBackWhy` (enum) — m6-conflicts/72, ADR-0045 §1: `PutBack { stash: Oid, why }`, the stash an operation put back into the working tree; `PutBackWhy::{Pop, Apply, SetAside}` (serde snake_case). `JournalEntry.put_back: Option<PutBack>`, `serde(default, skip_serializing_if = None)` like `merge_before`: an old line reads `None` and re-renders byte-identically (`journal::tests::a_journal_line_without_put_back_re_renders_byte_identically`). Written on the `Pending` line by a pop or apply (the entry's own stash commit); `graft_ops::clash::live_clash` reads it.
- `IndexState` (enum) — what an index was, as one tree id plus its shape: `Merged(Oid)` (the plain `write-tree` tree; serde is the **bare oid string**, so every line M3-M5 wrote parses and re-renders byte-identically) or `Unmerged { unmerged: Oid }` (the stage tree `{ 0/<path>, 1/<path>, 2/<path>, 3/<path> }`, every entry at stage 0 so the tree is writable at all; serde `{"unmerged":"<oid>"}`). `#[serde(untagged)]`, `Merged` first. `tree()`, `is_unmerged()`. An unmerged index is a **state**, not an absence: `None` on the fields above means only *never recorded* (m6-conflicts/08).

## mistake_radar
- `MistakeRadarKind` (enum) — the seven PROMPT §1.4 kinds: `BackwardMerge`, `FoxtrotMerge`, `AccidentalPullMerge`, `EvilMerge`, `DuplicateCommit`, `ForcePushedRemote`, `RiskyState`.
- `MistakeRadarKind::ALL` (const) — every kind, in declaration order (`graft-ops::mistake_radar::Policy` indexes by it).
- `MistakeRadarKind::default_severity` / `name` (fn) — PROMPT §1.4's severity table; the kebab-case config/CLI name.
- `Severity` (enum) — `Info` < `Warning`; how loudly a finding is shown.

## oid
- `Oid` (struct) — a git object id, 20 or 32 bytes inline; `SHA1_BYTES` / `MAX_BYTES` (const).
- `Oid::from_bytes` / `from_hex` / `as_bytes` (fn) — build, parse, read.
- `Oid::from_sha1` (fn) — wrap a 20-byte SHA-1 digest; infallible where `from_bytes` cannot be, because a caller that hashed something itself has 20 bytes by construction (m5/07, Ghost Preview's synthetic ids).
- `Oid` serde — as lowercase hex text (m3/02).

## raster
- `RasterFrame` (struct) — the pixel renderer's strip: `width`, `height`, `rows`, `columns`, premultiplied RGBA `pixels`, never a painted background.
- `RasterFrame::pixel` (fn) — the premultiplied RGBA at one pixel, `None` outside.

## ref_name
- `RefName` (struct) — a git ref's full name; `new` / `as_bytes` / `shorthand` / `is_graft_internal` (fn).
- `GRAFT_INTERNAL` (const) — `refs/graft/`, the namespace Graft writes its own refs under.
