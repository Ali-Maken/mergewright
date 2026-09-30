# ADR-0052: The snapshot keeps the bytes git would not give back

Status: accepted (planner rulings R1-R3 of m7-power-tools/30q, 2026-09-30). Extends ADR-0009's snapshot.

## Context

The before-snapshot's `worktree/` is git's **cleaned** tree (`add -A` on an index copy), and `u` writes it back through
git's **smudge**. Under `core.autocrlf=true` an LF file cleans to the same blob a CRLF one does, so `u` gives it back
CRLF. Seen on real git for a put-back (30p-a test 6) and for a plain discard and a stash push (30q A0, ticket
NOTES). A one-way clean filter (nbstripout strips a notebook's outputs; smudge is `cat`) is worse:
the outputs never come back. PROMPT.md §2.2 promises every destructive operation can be undone, and ux-principles §3.5
skips confirmations for discard, switch and reset **because** `u` is reliable.

## Decision

1. **Fix the snapshot.** Not a refusal in `check`: in a Windows-shaped repo it would fire on every discard, switch,
   stash or rebase over a changed LF file, and the only way out is the shell. Not "accept and document": a one-way filter
   makes it lost content. git's `stash` has the same property; our promise is stronger than git's.
2. **What is kept raw.** A path, when all three hold:
   - it differs between `HEAD`'s tree and the worktree tree (added, modified or type-changed; never deleted);
   - git would convert it: `check-attr` gives `text`, `eol`, `crlf`, `filter`, `ident` or `working-tree-encoding` a
     value other than unspecified/unset, or `core.autocrlf` is on and `text` is not unset. Every doubt is a yes: a wrong
     yes costs a comparison, a wrong no loses bytes;
   - its bytes on disk differ from what git's own checkout (`checkout-index` into scratch) writes from the cleaned blob.
   Symlinks and gitlinks are never kept. A path that matches `HEAD` comes back as git's own checkout writes it: it is in
   history, and the bound keeps the cost on the changed paths, never the repo.
3. **Layout, additive.** The backup commit is `{ index/ | unmerged/, raw/, worktree/ }`; `raw/` exists only when
   non-empty and holds only the kept paths at their own paths and modes. `worktree/` is unchanged, so drift, `replaced`,
   `lost_edits`, the preview and J read what they read before. The journal's opening line gains
   `worktree_raw_before: Option<Oid>`, `serde(default, skip_serializing_if = None)` like `merge_before`: an old line
   parses as `None`, and a line with nothing kept renders byte-identically.
4. **Only `before`.** Every restore writes a `worktree_before` tree (`u` the operation's, redo the `Undo` entry's, the
   roll-back the restore's own), so no after-snapshot keeps `raw/`. `RestoreWorktree` runs as today, then writes each
   kept path's raw bytes over git's (temp file + rename, mode from `raw/`), leaving the index untouched.

## Consequences

- Cost per undoable operation (amended 2026-09-30, m7-power-tools/30r, after 30q-c measured changed-with-no-conversion
  over its 10 ms gate at three spawns): a clean worktree 0 spawns; changed paths git does not convert +1 (`check-attr`);
  conversion candidates +2 (`update-index`, `checkout-index`) and a scratch copy of those files; a mismatch +3
  (`hash-object -w --no-filters`, `update-index`, `write-tree`). The `diff-tree` and `config --get core.autocrlf` reads
  moved in process (`graft_git::snapshot_reads`): the tree compare is exact on two tree objects, and `core.autocrlf` is
  re-read from disk on every call, never the handle's launch-time config. `check-attr` stays git's: its attribute sources
  are git's to know. Counts pinned by `snapshot_raw` tests; times measured by 30q-c's script.
- `cat-file --batch --filters` was the planned smudge read; its header prints the pre-filter size (git 2.54, seen), so
  its output cannot be split. `checkout-index` runs the same conversion a restore's checkout does.
- Not covered: J's forward put-back and the set-aside's `stash apply` (git writes those), EOL-only drift, `chain_backup`
  links. Each is a KNOWN_GAPS row.
