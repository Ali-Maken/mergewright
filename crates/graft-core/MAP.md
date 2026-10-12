# graft-core — map
Plain data only — no behaviour, no algorithms: commit and ref identity, events, the record shapes other crates read and write, errors for its own parsing.
May depend on: nothing internal (ARCHITECTURE.md crate table). One line per public item.

## capabilities
- `RenderingTier` (enum) — how Graft draws the commit graph: pixel, Unicode or ASCII.
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
- `Capabilities` (struct) — everything detection established about the terminal, incl. `answers_dsr` and `kitty_deflate`.

## commit
- `Timestamp` (struct) — when something happened, as git stores it.
- `Identity` (struct) — a name, an email and a moment.
- `Commit` (struct) — one commit: id, parents, author, committer, message.
- `Commit::date` (fn) — `-> Timestamp`: committer date.
- `Commit::subject` (fn) — `-> &[u8]`: first message line.

## commit_handle
- `CommitHandle` (struct) — `(pub u32)`: a commit as a dense index into its repository's id table.
- `CommitHandle::index` (fn) — `-> usize`.
- `Parents` (enum) — parents by handle, inline for up to two; `FromIterator`, `Default`.
- `Parents::iter` (fn) — `-> impl Iterator<Item = CommitHandle>`; also `len`, `is_empty`, `first`.
- `Link` (struct) — `commit` and its `parents`, by handle.
- `HandleSet` (struct) — bitset of handles; `Eq` by member, not representation.
- `HandleSet::new` (fn) — empty set; also `insert`, `contains`, `remove`, `len`, `is_empty`, `approximate_bytes`.

## error
- `Error` (enum) — a value a core constructor cannot represent: oid length/hex, ref name, entry id.
- `RefNameRule` (enum) — the `check-ref-format` rule a rejected ref broke.

## event
- `Event` (enum) — something that happened and that update must answer.
- `KeyPress` (struct) — a key going down, with modifiers.
- `KeyPress::plain` (fn) — `const (KeyCode) -> Self`; `control` builds one with Ctrl.
- `KeyCode` (enum) — the keys the keymap can name.
- `Modifiers` (struct) — control, alt, shift, command.
- `Modifiers::NONE` (const) — no modifier; `CONTROL` is Ctrl alone.
- `MouseInput` (struct) — a mouse event positioned in cells.
- `MouseKind` (enum) — down, up, drag, move, scroll.
- `MouseButton` (enum) — left, middle, right.

## job
- `JobId` (struct) — names one background job.
- `JobId::new` (fn) — `const (u64) -> Self`; `raw` reads it back.
- `JobFailure` (enum) — why a job produced no result: failed or panicked.

## journal
- `EntryId` (struct) — a journal entry's id, a ULID ordered by time; also the backup ref's last segment; serde as text.
- `EntryId::new` (fn) — `(unix_ms: i64, random: [u8; 10]) -> Self`; `parse` and `as_str` read and write the text.
- `OperationKind` (enum) — every journaled operation kind (stage … push, merge, rebase, rewrite, conflicts, undo, redo; `Recover`: refs or files put back; `Migrate`: the one-time rename move, never undone); serde snake_case.
- `EntryStatus` (enum) — `Pending`, `Committed`, `Aborted { reason }`.
- `INTERRUPTED` (const) — `&str` reason recovery records on an entry a dead process left open.
- `Touches` (struct) — what an operation changes: `refs`, `index`, `worktree`.
- `HeadRef` (enum) — `Branch(name)`, `Detached(Oid)`, `Unborn(name)`.
- `IndexState` (enum) — `Merged(Oid)` or `Unmerged { unmerged }`: an index as one tree id plus its shape.
- `IndexState::tree` (fn) — `-> Oid`; `is_unmerged` tells the shape.
- `JournalEntry` (struct) — one journaled operation, state-based: before/after of refs, head, index, worktree, config, merge state.
- `JournalEntry::is_interrupted` (fn) — `-> bool`: pending, or aborted with `INTERRUPTED`.
- `RefMove` (struct) — `{ before, after: Option<Oid> }`: one ref a fetch moved, outside the undo record.
- `PutBack` (struct) — `{ stash: Oid, why }`: the stash an operation put back into the working tree.
- `PutBackWhy` (enum) — `Pop`, `Apply`, `SetAside`.
- `MergeState` (struct) — git's merge in progress as files: `merge_head`, `other_heads`, `merge_msg`, `merge_mode`; byte-exact serde.
- `MergeState::heads` (fn) — every merge head, `merge_head` first.
- `MergeState::merge_head_bytes` (fn) — `-> Vec<u8>`: `MERGE_HEAD` as git writes it.

## mistake_radar
- `Severity` (enum) — `Info` < `Warning`; how loudly a finding is shown.
- `MistakeRadarKind` (enum) — the seven mistake kinds: backward, foxtrot, accidental pull merge, evil merge, duplicate, force-pushed, risky state.
- `MistakeRadarKind::ALL` (const) — every kind, in declaration order.
- `MistakeRadarKind::default_severity` (fn) — `-> Severity`; `name` is the kebab-case config name.

## oid
- `Oid` (struct) — a git object id, 20 or 32 bytes inline; serde as lowercase hex.
- `Oid::SHA1_BYTES` (const) — 20; `MAX_BYTES` is 32.
- `Oid::from_bytes` (fn) — `(&[u8]) -> Result<Self, Error>`; `from_hex` parses, `as_bytes` reads.
- `Oid::from_sha1` (fn) — `([u8; 20]) -> Self`: infallible wrap of a SHA-1 digest.

## on_disk
- `Generation` (enum) — `Graft` / `Tributary` / `Mergewright`, oldest first (derived `Ord`): one row per name the app shipped under (ADR-0057, Amendment 2026-10-11 §2).
- `Generation::CURRENT` / `GENERATIONS` (const) — the names this build writes (`Mergewright`); every generation, oldest first.
- `Generation::dir` / `title` / `command` / `env_prefix` / `ref_prefix` / `link_trailer` / `backup_email` (const fn) — the row's spellings: `mergewright`, `Mergewright`, `mwr`, `MWR_`, `refs/mergewright/`, …
- `Generation::has_marker` (const fn) — false for `Graft` only: with no marker present, Graft is in force.
- `Generation::older` (fn) — the generations released before this one, newest first.
- `Generation::of_ref` / `of_trailer` / `of_message` (fn) — which generation spelled a ref (by prefix), a trailer line (by key) or a git message (`<dir>:` or `<dir> `); `None` for anyone else's.
- `ASKPASS_SOCKET_VAR` (const) — `MWR_ASKPASS_SOCKET`, its one definition: set on askpass children by graft-git, re-exported by `graft::cli`.
- `MESSAGE_TAG` (const) — `Generation::CURRENT.dir()`: the tag every message this build writes into git starts with (stash, backup, index, absorb, forecast); earlier tags read by `of_message`.
- `CURRENT_DIR` (const) — `Generation::CURRENT.dir()`, short enough to inline in a path (`.git/{CURRENT_DIR}/...`).
- `MIGRATED_MARKER` (const) — `migrated`, in `<common_dir>/<dir>/` of each generation with `has_marker`: the newest one present (up to `CURRENT`) is in force.

## raster
- `RasterFrame` (struct) — the pixel renderer's strip: size in pixels and cells, premultiplied RGBA `pixels`.
- `RasterFrame::pixel` (fn) — `(x, y) -> Option<[u8; 4]>`: one pixel, `None` outside.

## ref_name
- `RefName` (struct) — a git ref's full name, validated.
- `RefName::new` (fn) — `(impl Into<Vec<u8>>) -> Result<Self, Error>`; also `as_bytes`, `shorthand`, `is_graft_internal` (true under any `Generation::ref_prefix`, whichever is in force).
