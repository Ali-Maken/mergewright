# Paste at the M1 checkpoint, before M2's design pass

Before M2's design pass and ticket specs, four rules. Add to CLAUDE.md, commit.

1. **Size from evidence, not intuition.** M1's session log records 13 compactions and several tickets at 93k–200k against an 80k target. planner-fable reads those entries first, says what made them overrun, and uses that to size M2's tickets. If a ticket can't be made to fit, say so when it's written rather than discovering it mid-build.
2. **Quiet gates.** Builders run `cargo test --quiet`, `cargo clippy --message-format=short`, `cargo build --quiet`. Full output only on failure, and only the failing part.
3. **Measurement never runs in a helper's context.** A script runs the passes and writes results to `docs/perf/<ticket>.txt`; the helper reads the summary lines only. Applies to perf runs, benchmarks, heavy-tier runs and memory curves.
4. **Stop at 40 tool calls or ~100k tokens.** The helper commits at the next clean point, writes a handoff note in the ticket, reports partial, and the conductor splits the rest.

Also for M2 planning:
- Before building, map every item in PROMPT.md §8 for M2 to the ticket covering it, and name anything uncovered.
- M2's ticket set ends with its acceptance ticket (Fable), proving M2's promise the way 35 proved M1's.
- graft-ops still has no MAP.md; create it when a ticket next touches that crate.
