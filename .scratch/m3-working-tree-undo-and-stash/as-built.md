# M3 as-built corrections (conductor)

Facts the build established that a group spec states differently. The spec is
`planner-fable`'s and is not edited; this file records what is true in the code
now. **Every M3 builder reads this before its spec.** Where the two disagree,
this file is the code and the spec is the intent.

| Spec says | Code is | Why | From |
|---|---|---|---|
| `hunk_patch` takes 3 arguments | 4 — a trailing `diff::PatchDirection { Forward, Reverse }` | The 3-arg form could only build the stage direction; `git apply -R` and `--cached -R` refuse it, so line-level unstage and discard were impossible. Both directions are now proved by applying with real git and comparing bytes | group 01 review fix, `b47ae51` |
| `status::read` returns on any output | `StatusError::TooLarge` past `output_cap` | `output.truncated` was ignored, so a truncated porcelain read parsed as a complete one | group 01 review fix, `b47ae51` |
| `stash::has_untracked_commit -> bool` | `-> Result<bool, StashError>` (new `StashError::Object`) | It swallowed errors to `false`, which CLAUDE.md forbids | group 01 review fix, `b47ae51` |

## Group 02 (ops-model), `d302765`

Eight rules differ from the spec text; all are written up in that ticket's
`## NOTES` section, which **group 03 must read in full** — it carries the
execution contract (`refs_after` recorded on abort as well as success, the
`Everything` scope, and the `Step` order refs → HEAD → index → worktree →
stash reflog). The headline ones:

| Spec says | Code is | Why |
|---|---|---|
| `parse` returns `Vec<JournalEntry>` | `Parsed { entries, torn_tail }`; `render_*` return `Result` | A torn last line is reported, never swallowed |
| `refs/stash` restored by `UpdateRef` | Never. `StashStore`/`StashDrop { commit }`, idempotent by commit | `update-ref` would *add* a reflog entry, not remove one; the reflog is the stash list |
| `hunk_patch` 3 args (group 01 row above) | 4, with `PatchDirection` | Group 01 review fix |
| `clean -f -d` | `clean -f`, always | With a pathspec `-d` is irrelevant, and `--untracked-files=all` means status never lists a bare directory |

Verified against **git 2.54 only**. The project's stated minimum is 2.38 — see
the KNOWN_GAPS row.

## Group 02 review fixes, `e490b34`

Rules 9–15 in the ticket's `## NOTES`, plus a revised group-03 contract —
**group 03 reads that section, not the original spec text, for the contract.**
What changed in the shape:

| Was | Is | Why |
|---|---|---|
| `took_effect(Aborted) = worktree_before.is_some()` | Aborted is in force only with a backup *and* a recorded change, or an unknown after-state. `Committed` stays always in force | An operation that failed changing nothing shadowed `next_undo`; an aborted undo mis-aimed the next `u` |
| Undo of `StashPush` always emits `StashDrop` | Reflog step only when `refs/stash` actually moved; an unrecorded side → `Incomplete` | `stash push` with nothing to save exits 0, so undo dropped the user's *previous* stash |
| `InversePlan` has no pins | `InversePlan.pins` | Redo of an undone stash push depended on the object surviving gc |
| Deviation 5, "a field never recorded cannot drift" | Replaced with `Drift.unknown` | The reviewer judged it a regression; argued in the ticket |
| Seam rule in ARCHITECTURE.md, unenforced | `crates/graft/tests/it/seam_rule.rs`, 2 tests | A rule with no test is a comment |

Also added: `PatchSide`/`PatchSideMismatch`, `StashScope` with `UnbornHead`/
`NothingToStash`, and `--literal-pathspecs` on every invocation (verified 2.54).
