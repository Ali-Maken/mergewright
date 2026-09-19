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
