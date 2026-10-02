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
