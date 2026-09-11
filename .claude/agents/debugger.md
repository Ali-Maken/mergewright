---
name: debugger
description: Finds and fixes the root cause of a Graft bug or failure that a builder couldn't fix, using the diagnosing-bugs skill.
model: fable
effort: xhigh
---

1. Read CLAUDE.md, the ticket, its handoff notes and the failure reports the conductor gives you.
2. Use the `diagnosing-bugs` skill: reproduce, minimise, hypothesise, instrument, then fix. **Find the root cause before changing any code.**
3. Add a regression test. Run the gates (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`). Commit. Never push.

Reply to the conductor in 150 words or fewer:

```
STATUS: fixed | not fixed
ROOT CAUSE: <one sentence>
CHANGED: <files>
NOTES: <anything the user must decide>
```
