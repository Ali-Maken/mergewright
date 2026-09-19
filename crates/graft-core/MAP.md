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
- `EntryId` (struct) — a journal entry's id, a ULID (26 Crockford base32 chars, time then random), `Ord` by string = by time; also the backup ref's last segment (ADR-0010); `new(unix_ms, random)` / `parse` / `as_str` (fn); serde as its text (m3/02).
- `OperationKind` (enum) — `Stage`, `Unstage`, `Discard`, `Commit`, `Amend`, `StashPush`, `StashApply`, `StashPop`, `StashDrop`, `Undo`, `Redo` (m3/02).
- `EntryStatus` (enum) — `Pending`, `Committed`, `Aborted { reason }` (m3/02).
- `Touches` (struct) — what an operation changes: `refs` (full names, or `HEAD` when detached), `index`, `worktree`; the snapshot scope and the generic inverse's steps (m3/02).
- `HeadRef` (enum) — `Branch(full name)`, `Detached(Oid)`, `Unborn(full name)` (m3/02).
- `JournalEntry` (struct) — one journaled operation, state-based (ADR-0031): `id`, `at_unix_ms`, `kind`, `description`, `status`, `touches`, `head_before`/`head_after`, `refs_before`/`refs_after` (`BTreeMap<String, Option<Oid>>`), `index_before`/`index_after`, `worktree_before`/`worktree_after` (tree ids), `backup_ref`, `pins`, `commands`, `reverses`. Serde derives; the JSONL lines are `graft-ops::journal`'s (m3/02).

## mistake_radar
- `MistakeRadarKind` (enum) — the seven PROMPT §1.4 kinds: `BackwardMerge`, `FoxtrotMerge`, `AccidentalPullMerge`, `EvilMerge`, `DuplicateCommit`, `ForcePushedRemote`, `RiskyState`.
- `MistakeRadarKind::ALL` (const) — every kind, in declaration order (`graft-ops::mistake_radar::Policy` indexes by it).
- `MistakeRadarKind::default_severity` / `name` (fn) — PROMPT §1.4's severity table; the kebab-case config/CLI name.
- `Severity` (enum) — `Info` < `Warning`; how loudly a finding is shown.

## oid
- `Oid` (struct) — a git object id, 20 or 32 bytes inline; `SHA1_BYTES` / `MAX_BYTES` (const).
- `Oid::from_bytes` / `from_hex` / `as_bytes` (fn) — build, parse, read.
- `Oid` serde — as lowercase hex text (m3/02).

## raster
- `RasterFrame` (struct) — the pixel renderer's strip: `width`, `height`, `rows`, `columns`, premultiplied RGBA `pixels`, never a painted background.
- `RasterFrame::pixel` (fn) — the premultiplied RGBA at one pixel, `None` outside.

## ref_name
- `RefName` (struct) — a git ref's full name; `new` / `as_bytes` / `shorthand` (fn).
