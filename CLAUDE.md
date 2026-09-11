# Graft: rules for Claude

"Graft" is only a **codename**: the name is taken by another Git GUI. The real name gets chosen in M8. Don't publish, register or tag anything public under this name.

## Where things live

| File | Purpose |
|---|---|
| `PROMPT.md` | The product spec. The note above its first `---` is for humans. |
| `DESIGN.md` | The approved design (exists after step 2). |
| `CONTEXT.md` | The glossary from `/grill-with-docs`. Use its terms everywhere. |
| ADRs | Decision records, in the folder `docs/agents/domain.md` names (default `docs/adr/`). **Wherever PROMPT.md says `DECISIONS.md`, write an ADR instead.** |
| `PROGRESS.md` | The plan and the current step. |
| Tickets | Local markdown, wherever `docs/agents/issue-tracker.md` says. |
| `KNOWN_GAPS.md` | Anything unfinished or stubbed. |
| `docs/reviews/` | Milestone reviews. |

## How work runs

- The user runs `/conduct` in a main session on **Opus**. The conductor hands every piece of work to a fresh helper in `.claude/agents/` and stops at checkpoints.
- **The one exception is step 1** (`/grill-with-docs`), which runs in the main session because it needs the user.
- Do only the next unchecked step. Never start a new milestone without the user's OK at a checkpoint.

## Rules for every agent

- **Never mark stubbed or half-done work as done.** List it in `KNOWN_GAPS.md`.
- **Rust code is done only when** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` all pass.
- **Commit** after each ticket. Never push or force.
- **Ask the user** (through the conductor) before changing the spec or cutting scope. Record agreed changes as ADRs.
- **Keep your context lean:** read what you need, not everything.
- **Tests must be laptop-safe** (PROMPT.md 7.1). Builders run only the fast tier (`cargo test`). The heavy tier, fuzzing and benchmarks run only in milestone reviews.
- **One heavy job at a time:** never run two cargo commands or test suites at once. Never generate large fixtures outside the heavy tier.

## Agent skills

### Issue tracker

Local markdown: tickets live as files under `.scratch/<milestone-slug>/issues/NN-<slug>.md`. There is no remote. See `docs/agents/issue-tracker.md`.

### Domain docs

Single-context: `CONTEXT.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
