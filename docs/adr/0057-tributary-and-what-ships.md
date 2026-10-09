# ADR-0057: Tributary, and what ships in v1

Status: accepted (user, 2026-10-02: "go ahead with your recommendation" — name, notarising, items past v1).
**Open item: a person must run a trademark search (USPTO classes 9 and 42) before anything is published.**
Amends ADR-0010 (the on-disk names), PROMPT.md §5 ("image previews in the pixel tier").

## Decision

1. **The name is Tributary; the command is `trib`.** From `docs/naming-research.md`: the graph is a river system, and
   `trib` was free on every registry checked. Until the trademark search is done, **nothing is published,
   registered, tagged or pushed under either name** — not crates.io, not a tap repository, not a release. `dist`
   runs `plan` and `build` locally only.
2. **Every on-disk and environment name moves to the new name, once, at launch.** `refs/graft/**` →
   `refs/tributary/**`, `<common_dir>/graft/` → `<common_dir>/tributary/`, the config dir, `GRAFT_*` → `TRIB_*`.
   The first launch of the renamed binary migrates: journaled, idempotent, the old copy kept until the new one reads
   back the same. A migration that cannot finish leaves the old names in force and says so. Runs alone, last before
   the acceptance ticket.
3. **No Apple notarisation.** `brew install` and the curl installer do not quarantine what they fetch; only a binary
   downloaded in a browser would warn. Not worth $99/year for one user. dist codesigns ad hoc as it does by default.
4. **Carried past v1** (not in his daily use; each is a screen of its own): pixel-tier image diffs (KNOWN_GAPS row
   371), Map view mouse and remote rows (372, 374), undo-backup pruning (205).

## Why

- "Graft" is another Git GUI's name (CLAUDE.md). His live journal and backup refs must survive the switch, so the
  rename is a migration, not a find-and-replace.
- Keeping internal `graft` names forever would leave another product's name in his repositories' refs.

## Consequences

- The rename group is the only M8 group that touches every crate; nothing builds beside it.
- Old backups stay readable: the migration moves refs with `git update-ref`, journaled like any write (ADR-0009's
  snapshot rule does not apply: no working-tree content is touched).
- KNOWN_GAPS rows 371, 372, 374 and 205 re-expire to "after v1".

## Amendment (2026-10-09, ticket 26)

How §2's migration works. The user approved the migration itself on 2026-10-09: copy, read back, then delete the old
names; nothing kept afterwards.

1. **One migration per repository, covering every worktree.** One pass moves the refs, `<common_dir>/graft/`, every
   `<common_dir>/worktrees/*/graft/` and `<git_dir>/graft/` (duplicate paths counted once). Each tree is copied whole,
   every file is compared byte for byte, and journals are also read back through the real reader.
   *Why:* refs are shared, so moving them one worktree at a time would leave a worktree's journal naming refs that
   have already moved. A copy of the whole tree means no file is deleted without first being read back.
   *Rejected:* a separate migration per worktree; a fixed list of files to move.
2. **A marker with two states: `switched`, then `clean`.** It lives at `<common_dir>/tributary/migrated`.
   - Before the switch: new refs are created and read back, the trees are copied and read back.
   - Writing `switched` is the moment the new names come into force.
   - After the switch: the `Migrate` entry is journaled, old refs are deleted with their old oids as expected values,
     the old trees are removed, and the marker becomes `clean`.

   A failure before the switch leaves the old names in force and deletes nothing. A failure after it keeps the new
   names; the next launch sees `switched` and finishes the clean-up. When the marker reads `clean`, opening a
   repository costs one file read and starts no `git` process.
   *Why:* with one marker state, a clean-up that failed part-way would either strand old refs for good or force
   every launch to check for them.
3. **A lock stops the migration while another copy of the app is open on the repository.** The lock file is
   `<common_dir>/tributary/lock`, an advisory OS lock (`File::lock`), not a polling loop. A process running under
   the old names holds it shared for its whole life. The migration needs it exclusive and only tries once; if
   another process holds it, this run stays on the old names and says `another trib is open on this repository`.
   *Why:* otherwise a second window still on the old names would keep writing them after the switch, and the marker
   would hide those writes from every later launch.
4. **`OnDiskNames` (`New` / `Old`) lives in `graft-core`.** It sits on `SharedRepo`/`RepoHandle`, which set it at
   open from whether the marker exists. Every reader and writer of the ref prefix, the dir name or the
   backup-commit trailer takes it from there. The check for "is this one of our refs" matches both prefixes.
   *Why:* `graft-git` needs the names (ref filter, commit-graph offer) and cannot depend on `graft-ops`. Every
   caller that opens a repository, tests and headless paths included, gets the same answer for free.

**Consequences:**
- Backup commits stay readable forever: their trailer is read as `Graft-Link:` or `Tributary-Link:`, and only the
  new spelling is written.
- Journal lines are never rewritten; ref names are mapped to the new prefix when read, only once the new names are
  in force.
- The crash-report dir and the backup author identity are renamed without moving anything (ticket 27).
