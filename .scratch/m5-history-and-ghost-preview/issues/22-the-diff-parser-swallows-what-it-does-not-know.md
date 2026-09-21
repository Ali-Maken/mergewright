# the-diff-parser-swallows-what-it-does-not-know: a combined diff parses as "no changes"

Status: ready
Blocked by: none — `graft-git` only, runs alongside ticket 15 (`graft`)
Build with: **Opus.** Small diff, but it is a rule 3 violation on the path
every diff in the app travels, and the right answer needs a judgement about
which sections are benign.
Crates: **`graft-git` only.** Another builder holds `graft`. Need it: commit at
the seam, report `blocked: needs graft`, stop.
Scout: **not needed.** The lines are quoted below.

## The defect

`crates/graft-git/src/diff/unified.rs:28-46`. Any line not starting
`diff --git ` falls into the `else` and is skipped:

```rust
} else {
    // Before the first file, `diff-tree` prints the commit id on its
    // own line; nothing else here is ours to fail on.
    i += 1;
}
```

**That comment justifies exactly one line and is then applied to every line.**
It was true for `diff-tree`'s commit id. It is false for a combined diff: git
answers an unmerged path with `diff --cc` and `@@@` headers, the whole reply
falls through this branch, and `parse_unified` returns `Ok(vec![])` — success,
with nothing in it.

**This is what made ticket 21's blank conflict body invisible.** The pipeline
reported success, so nothing anywhere had an error to show. Ticket 21 routed
the one caller that hit it (it now reads against `HEAD`), so the symptom is
gone — **but the swallow is still here for the next caller.** CLAUDE.md rule 3:
a catch-all that carries on is exactly this.

## Scope

**Make the parser say what it does not understand.** A section it cannot read
must be an error, not an empty success. Keep skipping only what is genuinely
benign, and make the comment say *which* thing and *why* — one named case, not
a general licence.

Decide and state in the report:

- `diff --cc` and `diff --combined`: recognise them explicitly. Erroring with
  "combined diffs are not supported here" is an acceptable answer for now and
  is far better than silence; parsing them is not in this ticket's scope
  unless it falls out easily.
- Anything else unrecognised before the first `diff --git`: name what is
  legitimately there (the `diff-tree` commit id) and fail on the rest.

**Do not widen the fix into a parser rewrite.** The requirement is that the
function stops lying about success.

## Also here

`graft_git::status::counts(handle)` (`status.rs:423`) has **no callers left**
in the workspace — the app reads `WorkingTreeStatus::counts()` instead. Remove
it, or say why it must stay. Note ticket 21's finding when you decide: it could
not fill an `unmerged` field honestly, because it walks gix items, which is
why the app stopped using it.

## Acceptance (every tick names its verifying test inline, at tick time)

- [ ] A `diff --cc` reply is an error, not `Ok(vec![])` — test:
- [ ] A reply with an unrecognised section before the first `diff --git` is an
      error — test:
- [ ] The `diff-tree` commit-id line is still skipped and still parses — test:
- [ ] `parse_unified` never returns `Ok` with an empty vec for a non-empty
      input it did not understand — the property, not a case — test:
- [ ] `status::counts` removed, or a written reason it stays — test:

**Mutation-check each.** The one that matters: restore the blanket `else` and
show the `diff --cc` test going red. If it stays green the fix is cosmetic.

## Gates

Workspace-wide, never `-p`. `CARGO_TARGET_DIR=/tmp/graft-target-g22`, deleted
before hand-back. Baseline **2214/2214** — check it yourself.

**Never `git checkout` or `git restore` across your worktree** — a builder
destroyed five uncommitted files that way this milestone. Revert a mutation by
editing the file back.

Commit, never push.

## Report

Cause or symptom. What you chose for `diff --cc` and why. Whether removing
`status::counts` broke anything. Anything unverified — which is not broken.
