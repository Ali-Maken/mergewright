# the-diff-parser-swallows-what-it-does-not-know: a combined diff parses as "no changes"

Status: **done**
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

- [x] A `diff --cc` reply is an error, not `Ok(vec![])` — test:
      `diff_cases::a_combined_diff_is_an_error_not_an_empty_success`
      (`crates/graft-git/tests/it/diff_cases.rs`), plus
      `a_combined_section_after_a_normal_file_is_an_error_too` for a `diff
      --cc` section arriving *after* a readable file, which the top-level
      guard alone would have swallowed into the previous file.
- [x] A reply with an unrecognised section before the first `diff --git` is an
      error — test: `diff_cases::an_unreadable_line_before_the_first_file_is_an_error`.
- [x] The `diff-tree` commit-id line is still skipped and still parses — test:
      `diff_cases::the_diff_tree_commit_id_line_is_still_skipped_and_the_diff_still_parses`
      (sha1 and sha256 ids, and the id-only reply that legitimately parses to
      nothing), and the real-git round-trips
      `diff_fixture::parse_unified_matches_numstat_for_every_shape` and
      `diff::patch::tests::whole_hunk_reproduces_gits_own_bytes_on_every_corpus_shape`,
      which feed actual `diff-tree -p` output — commit id included — for every
      commit of every corpus shape.
- [x] `parse_unified` never returns `Ok` with an empty vec for a non-empty
      input it did not understand — the property, not a case — test:
      `diff_cases::properties::an_empty_ok_means_every_line_was_a_bare_object_id`
      (proptest, 64 fast-tier cases): inputs are drawn from 16 line families —
      real diff lines, `diff --cc`/`diff --combined`/`@@@`, both id widths,
      git noise and an empty line — and an `Ok(vec![])` is only allowed when
      *every* line of the input is a bare object id, which the test decides
      with its own predicate rather than the parser's.
- [x] `status::counts` removed, or a written reason it stays — **removed**
      (`crates/graft-git/src/status.rs`). Eight tests called it: six went
      with it, and two — the bare-repository case and
      `ignored_files_are_not_counted_as_untracked` — were re-pointed at
      `read`, which answers both. Test: the workspace compiles and
      `cargo nextest run` is 2213/2213 with no reference to it left; the
      coverage it carried is `status::tests::read_matches_gits_own_counts_and_letters_on_every_shape`
      (git's own output as the oracle, every corpus shape) and
      `status::tests::all_six_dirty_states_on_a_repository_built_for_them`.
      The one case it answered without spawning git — a bare repository — is
      kept as `status::tests::a_bare_repository_is_reported_clean`, now
      through `read`, which short-circuits bare itself.

## Mutation table

Each mutation applied by editing the file, run, then edited back (no
`git checkout`). Workspace green before and after.

| Mutation | Expected | Observed |
|---|---|---|
| Restore the blanket `else` in `parse_unified` (skip every non-`diff --git` line) | the `diff --cc` case goes red | **3 red**: `a_combined_diff_is_an_error_not_an_empty_success` (fails with exactly `Ok([])`, the ticket's symptom), `an_unreadable_line_before_the_first_file_is_an_error`, and the property. The mixed-section case stayed green — it is caught by the second guard, below. |
| Drop `reject_combined_diff` from `parse_file_section` | the mixed case goes red | **1 red**: `a_combined_section_after_a_normal_file_is_an_error_too`. |
| `is_bare_object_id` → always `false` | the commit-id skip is load-bearing on real git output | **3 red**: the commit-id case and both real-git round-trips (`diff_fixture::parse_unified_matches_numstat_for_every_shape`, `diff::patch::tests::whole_hunk_reproduces_gits_own_bytes_on_every_corpus_shape`). |

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

## NOTES

**Cause, not symptom.** The swallow itself is gone: `parse_unified` now skips
exactly one named thing outside a `diff --git` section — a bare object id, 40
or 64 hex digits, which is what `diff-tree -p` prints before the first file —
and errors on everything else, naming the 1-indexed line.

**Chosen for `diff --cc`: refuse, don't parse.** `diff --cc`,
`diff --combined` and `@@@` hunk headers are a named error,
`"combined diff (`diff --cc`) is not supported here"`. Parsing combined
diffs did not fall out easily — a `@@@` header carries one range per parent
and its body has one marker column per parent, so `Hunk`/`DiffLine` would
have to grow a parent dimension that every consumer (`pairs`, `hunk_patch`,
the detail pane) then has to understand. That is a design change, not this
ticket. Refusing costs a caller that hits an unmerged path an error message
instead of a body; before, it got silence.

The guard is in two places, and the mutation table shows both are
load-bearing: at the top level (where a whole combined reply arrives, since
it contains no `diff --git` line at all) and at the top of
`parse_file_section` (where a combined section following a readable file
would otherwise be absorbed into that file's headers).

**Deliberately left alone:** the other skip in `parse_file_section`, for
extended header lines (`index <old>..<new> <mode>` and whatever a future git
adds beside it). It cannot hide a reply — the file's `diff --git` line has
already been read, so the file is reported either way — and the two shapes
that *could* hide one are now refused above it. Its comment says that, rather
than claiming a general licence.

**`status::counts` removed; nothing broke.** Only its own eight tests called it
(the app reads `WorkingTreeStatus::counts()`), and ticket 21's finding stands:
walking gix items, it could not fill an `unmerged` field honestly. Workspace
compiles, 2213/2213.

**Gates** (workspace-wide, `CARGO_TARGET_DIR=/tmp/graft-target-g22`, deleted
after): `cargo fmt --all --check` clean, `cargo clippy --all-targets -D
warnings` exit 0 with no diagnostics, `cargo nextest run` 2213/2213 (baseline
2214 measured here first; −7 gix-path `counts` tests, +1 bare-repo test on
`read`, +5 new parser tests), `cargo test --doc` ok.

**Unverified** (not broken): the `diff --cc` input in the test is hand-written
in the file's established style, from git's documented combined format — it
was not captured from a live conflicted repository in this session, because
that would need `graft`'s fixtures, which another builder holds. The shape
that matters (no `diff --git` line anywhere) is what the guard keys on, and
it is what ticket 21 observed. The fuzz target was not run; fuzzing is a
milestone-review tier, and this change adds only `Err` paths, no new
indexing.

No scout: the ticket quoted the offending lines and named the file.
