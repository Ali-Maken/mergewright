# 49r: review fixes for ticket 49 (package and command)

Status: ready
Blocked by: none (49 merged at 7ce123a0)
Build with: **Sonnet**. Script paths, record restores, one test split; held by grep tests.
Brownfield: ticket 49 NOTES serve as the scout; the review names every line.
Needs: none. Before 50 (same test files: `rename_names.rs`).

## Read first
- `docs/reviews/m8-mergewright-49.md`, whole
- Ticket 49: Decisions, NOTES

## Scope
- **Major 1:** `docs/shots/39-capture.py:15`, `40-capture.py:19` run `target/release/graft` → the `mwr` binary. Extend
  `rename_names::scripts_run_the_command` (`rename_names.rs:362`) to catch paths built with `os.path.join` (red first).
- **Major 2:** restore the records the sweep rewrote (`27a:47,70`, `28-accept-m8.md:73`, `scouts/27-rename-sweep.md:4`,
  `49:27`) to their original text; a record says what was true then. Add them to the sweep's records allow-list.
- **Major 3:** `ARCHITECTURE.md:234`: the shipping package is `mergewright`, crates stay `graft-*` (ADR-0057 §9).
- **Major 4:** split `rename_names.rs` under 400 lines; remove its over_400 row.
- **Minors:** fix each as the review says, or say in NOTES why not.
- Ticket 49 NOTES: correct "the original regex isn't recorded" (it is, `27a:130`).

## Acceptance (each box: the test named at tick time)
- [ ] `scripts_run_the_command` red on the old 39/40 scripts, green after (test named).
- [ ] Records grep: the five sites read as before 49 (diff quoted).
- [ ] `over_400.txt` has no `rename_names.rs` row; file-size test named.
- [ ] Every minor: fixed with test/grep named, or reason in NOTES.

## Touches
Capture scripts; records allow-list; ARCHITECTURE.md; over_400.txt.
