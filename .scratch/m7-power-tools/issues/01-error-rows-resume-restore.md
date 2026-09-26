# error-rows-resume-restore — the resume and restore paths stop hiding errors (rule 3)

Status: ready
Blocked by: none
Crates: graft-ops (`apply/resume.rs`, `apply.rs` `seal_after` / `chain_states`, `conflict/set.rs`, `restore`, `undo`),
graft (`jobs/conflict.rs` `worktree_lines`; the `UndoError::Drift::explain` caller).
Source: KNOWN_GAPS rows. They are cited by content, because line numbers drift:
- :494 `Drift::explain` says "u again" for a redo;
- :495 `RestoreError::Stopped`, a third stop shape;
- :496 `Ran::Failed` performs no roll-back;
- :497 ref names are not sanitised on the restore path;
- :572 `worktree_lines` treats every read error as "no lines";
- :601 two error mappings discard the real error;
- :613 a catch-all arm mislabels a failure as "stopped";
- :640 `live_entry` makes an unstoppable `rev-list`.

Note "Carried items" (the restore path moved here from G-rewrite).
Build with: Opus. These are rule-3 fixes. :496's roll-back must be designed against the recording protocol; the rest
are mechanical.
Scout: `scout` first (ADR-0035). Check that every cited site still says what the row says (lines predate M6 splits).
Needs: none. It is sequenced before `rewrite-engine` if both want graft-ops at once (disjoint crate sets, ADR-0036).
The conductor decides.

## Decided
- **:572:** empty lines only on `NotFound`; any other error goes to the card's notice in words (`can't read <path>:
  <reason>`). A relative path is joined onto the workdir, not the process cwd.
- **:601:**
  - `seal_after`: a failed `read_refs` is an error reported to the caller, never a silent fallback.
  - `chain_states`: map each `live_entry` error by kind. No `_ =>` arm.
- **:613:** `conflict/set.rs` matches `ApplyError::Stopped` explicitly, or `live_entry` returns `FactsError`. The
  builder picks one and names it. No catch-all.
- **:640:** `live_entry` calls `in_progress_detail_with_stop(repo, stop)`.
- **:495:** collapse `RestoreError::Stopped` into the shared stop shape (the scout names the two others). Callers match
  one variant.
- **:496:** `Ran::Failed` rolls back what the restore wrote before the failure, as `restore::execute`'s rollback does
  for other failures. It never leaves half a restore silently. If a full rollback is impossible, it stops, and the
  words name what's left (rule 2: no retry).
- **:497:** every ref name on the restore path is checked with `git check-ref-format` semantics (the existing
  sanitiser, if any; the scout checks) before any write. A bad name refuses in words.
- **:494:** the variant carries its direction (`Undo`/`Redo`), and `explain` says `u again` or `U again` to match.

## Read first
- KNOWN_GAPS is not read by builders: the rows' substance is quoted above.
- `docs/design/06-undo-journal-design.md` §6.2–6.4 (the recording protocol, rollback).
- `crates/graft-ops/MAP.md`: `apply::resume`, `apply` (`seal_after`, `chain_states`), `conflict::set`, `restore`,
  `undo`. `crates/graft/MAP.md`: `jobs` (`conflict.rs`), the undo status.

## Scout brief (≤ 1 page)
1. For each row: does the cited code still exist? File:line now, and one line saying what it does.
2. The three stop shapes (:495): their names and every match site.
3. `restore::execute`'s rollback: what it covers, and where `Ran::Failed` bypasses it.
4. Is there a ref-name sanitiser (`check-ref-format` or Rust)? Where?

## Signatures
```rust
// graft-ops: UndoError::Drift { …, direction: Direction }  // Direction { Undo, Redo }; explain() names u or U
// RestoreError: the Stopped variant removed in favour of the shared stop shape (the scout names it)
// no other new pub API; list each changed signature in NOTES
```

## Acceptance (one test per row, red first where the row is **broken**)
- [ ] :572 `a_permission_error_shows_in_the_card_not_empty` (graft, red first); `a_relative_path_reads_from_the_workdir`.
- [ ] :601 `a_failed_read_refs_in_seal_after_is_reported` (red first); `chain_states_maps_each_error_kind`.
- [ ] :613 `a_facts_failure_is_not_reported_as_stopped` (red first).
- [ ] :640 `live_entry_stops_when_asked` (red first: a stop flagged before the `rev-list` returns `Stopped` promptly).
- [ ] :495 `every_stop_is_one_shape`: a compile-level change; the test asserts the callers' single arm.
- [ ] :496 `a_restore_that_fails_mid_way_rolls_back` (red first; a fault injected on the second write).
- [ ] :497 `a_bad_ref_name_is_refused_before_any_write` (red first).
- [ ] :494 `drift_on_redo_says_capital_u` (red first).
- [ ] NOTES: each row → its test. The conductor closes the KNOWN_GAPS rows from them.
- [ ] MAP.md (graft-ops, graft).

## Touches (check each, report how)
- **Every caller of `live_entry`, `restore::execute`, `RestoreError`:** grep; each listed in NOTES. Check: the workspace
  builds, and the undo/resume tests are green.
- **The M6 resolver cards:** they now may show a notice. Check: m6 resolver captures byte-identical (no fixture has an
  unreadable file).
- **Undo/redo status words:** `U again` on redo. Check: the undo walk captures. List any that change.
- **Keymap / captures:** no key change.

## Rules / Gates
As rewrite-engine.

## Size
Medium (~50k).
